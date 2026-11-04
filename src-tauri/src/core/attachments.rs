//! Images attached to notes (screenshots, photos of documents...).
//!
//! Each image is sealed with the Vault Key in its own row of `attachments`, outside the
//! item payload, so unlocking never decrypts them: an image is opened only when a note
//! that shows it is rendered. The note text refers to it as `![alt](attachment:<id>)`.
//!
//! Sealed plaintext: `mime length (u8) | mime | image bytes`. The type is detected from
//! the bytes themselves, never taken from the client, and only raster formats the
//! WebView renders as plain pixels are accepted (no SVG: it can carry scripts).

use std::collections::HashSet;

use zeroize::Zeroizing;

use crate::core::error::{AppError, AppResult};
use crate::core::model::ItemPayload;
use crate::core::time::now_millis;
use crate::core::vault::UnlockedVault;
use crate::crypto::{CRYPTO_VERSION, aead, random};
use crate::database::Database;
use crate::database::repo::{self, AttachmentRow};

/// The frontend shrinks big screenshots before sending; this is the hard limit.
pub const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;
const MAX_ATTACHMENTS_PER_ITEM: usize = 200;
/// How a note refers to an attachment: `![alt](attachment:<id>)`.
pub const REFERENCE_PREFIX: &str = "attachment:";

pub(crate) fn attachment_aad(vault_id: &str, item_id: &str, attachment_id: &str) -> Vec<u8> {
    format!("nexus-vault/v1/attachment/{vault_id}/{item_id}/{attachment_id}").into_bytes()
}

/// MIME type by magic bytes; `None` for anything that is not a supported image.
fn sniff_image(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else {
        None
    }
}

fn encode(mime: &str, data: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut plain = Zeroizing::new(Vec::with_capacity(1 + mime.len() + data.len()));
    plain.push(mime.len() as u8);
    plain.extend_from_slice(mime.as_bytes());
    plain.extend_from_slice(data);
    plain
}

/// File extension for a stored image type.
pub fn extension(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    }
}

/// `(mime, image bytes)` of a decrypted attachment.
pub fn split_plaintext(plain: &[u8]) -> Option<(&str, &[u8])> {
    let (&len, rest) = plain.split_first()?;
    let (mime, data) = rest.split_at_checked(len as usize)?;
    Some((std::str::from_utf8(mime).ok()?, data))
}

/// Checks a decrypted attachment has the expected layout (used on restore too).
pub(crate) fn is_valid_plaintext(plain: &[u8]) -> bool {
    let Some((&len, rest)) = plain.split_first() else {
        return false;
    };
    let Some((mime, data)) = rest.split_at_checked(len as usize) else {
        return false;
    };
    sniff_image(data).is_some_and(|sniffed| sniffed.as_bytes() == mime)
}

/// Attachment ids mentioned in a note's markdown.
fn referenced_ids(content: &str) -> impl Iterator<Item = &str> {
    content
        .match_indices(REFERENCE_PREFIX)
        .filter_map(|(at, _)| {
            let rest = &content[at + REFERENCE_PREFIX.len()..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .unwrap_or(rest.len());
            (end > 0).then(|| &rest[..end])
        })
}

impl UnlockedVault {
    /// Encrypts and stores an image for a note. Returns the new attachment id.
    pub fn add_attachment(
        &mut self,
        db: &mut Database,
        item_id: &str,
        data: &[u8],
    ) -> AppResult<String> {
        let item = self.items.get(item_id).ok_or(AppError::NotFound)?;
        if !matches!(item.payload, ItemPayload::SecureNote(_)) {
            return Err(AppError::Validation("type"));
        }
        if data.len() > MAX_ATTACHMENT_BYTES {
            return Err(AppError::Validation("image_too_large"));
        }
        let mime = sniff_image(data).ok_or(AppError::Validation("image"))?;
        let count = repo::list_attachment_ids(db.conn())?
            .iter()
            .filter(|(_, owner)| owner == item_id)
            .count();
        if count >= MAX_ATTACHMENTS_PER_ITEM {
            return Err(AppError::Validation("too_many_images"));
        }

        let id = random::new_id()?;
        let sealed = aead::seal(
            &self.key,
            &encode(mime, data),
            &attachment_aad(&self.vault_id, item_id, &id),
        )?;
        let row = AttachmentRow {
            id: id.clone(),
            item_id: item_id.to_string(),
            encrypted_data: sealed.ciphertext,
            nonce: sealed.nonce.to_vec(),
            crypto_version: CRYPTO_VERSION,
            created_at: now_millis(),
        };
        db.transaction(|tx| repo::insert_attachment(tx, &row))?;
        Ok(id)
    }

    /// Decrypted attachment in the stored layout (`mime length | mime | bytes`).
    pub fn read_attachment(&self, db: &Database, id: &str) -> AppResult<Zeroizing<Vec<u8>>> {
        let row = repo::load_attachment(db.conn(), id)?.ok_or(AppError::NotFound)?;
        if row.crypto_version > CRYPTO_VERSION {
            return Err(AppError::UnsupportedVersion);
        }
        let plain = aead::open(
            &self.key,
            &row.nonce,
            &row.encrypted_data,
            &attachment_aad(&self.vault_id, &row.item_id, &row.id),
        )?;
        if !is_valid_plaintext(&plain) {
            return Err(AppError::Integrity);
        }
        Ok(plain)
    }

    /// Deletes images no note refers to any more (the reference was erased, or the
    /// app closed between pasting and saving). Run on unlock, never while editing, so
    /// undoing a deletion in the editor still finds the image.
    pub fn prune_attachments(&self, db: &mut Database) -> AppResult<usize> {
        let referenced: HashSet<&str> = self
            .items
            .values()
            .filter_map(|item| match &item.payload {
                ItemPayload::SecureNote(p) => Some(p.content.as_str()),
                _ => None,
            })
            .flat_map(referenced_ids)
            .collect();
        let unused: Vec<String> = repo::list_attachment_ids(db.conn())?
            .into_iter()
            // an owner that failed to decrypt keeps its images: nothing is known about it
            .filter(|(id, owner)| {
                self.items.contains_key(owner) && !referenced.contains(id.as_str())
            })
            .map(|(id, _)| id)
            .collect();
        if unused.is_empty() {
            return Ok(0);
        }
        db.transaction(|tx| {
            for id in &unused {
                repo::delete_attachment(tx, id)?;
            }
            Ok(unused.len())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::items::{ItemFilter, ItemSort, ItemView};
    use crate::core::model::{LoginPayload, SecureNotePayload};
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;

    const PASSWORD: &str = "correct horse battery";
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake-png-body";

    fn setup() -> (Database, UnlockedVault, String) {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, PASSWORD, KdfParams::TEST).unwrap();
        let note = v
            .create_item(
                &mut db,
                ItemPayload::SecureNote(SecureNotePayload {
                    title: "Скрины".into(),
                    content: String::new(),
                    tags: vec![],
                }),
                None,
                false,
            )
            .unwrap();
        (db, v, note)
    }

    fn set_content(db: &mut Database, v: &mut UnlockedVault, id: &str, content: String) {
        v.update_item(
            db,
            id,
            ItemPayload::SecureNote(SecureNotePayload {
                title: "Скрины".into(),
                content,
                tags: vec![],
            }),
            None,
        )
        .unwrap();
    }

    #[test]
    fn stored_encrypted_and_read_back() {
        let (mut db, mut v, note) = setup();
        let id = v.add_attachment(&mut db, &note, PNG).unwrap();

        let raw: Vec<u8> = db
            .conn()
            .query_row("SELECT encrypted_data FROM attachments", [], |r| r.get(0))
            .unwrap();
        assert!(!raw.windows(13).any(|w| w == b"fake-png-body"));

        let plain = v.read_attachment(&db, &id).unwrap();
        assert_eq!(&plain[..10], b"\x09image/png");
        assert_eq!(&plain[10..], PNG);

        drop(v);
        let v = vault::unlock(&db, PASSWORD).unwrap();
        assert_eq!(&v.read_attachment(&db, &id).unwrap()[10..], PNG);
    }

    #[test]
    fn only_images_for_notes_are_accepted() {
        let (mut db, mut v, note) = setup();
        assert!(matches!(
            v.add_attachment(&mut db, &note, b"<svg onload=alert(1)>"),
            Err(AppError::Validation("image"))
        ));
        let login = v
            .create_item(
                &mut db,
                ItemPayload::Login(LoginPayload {
                    title: "x".into(),
                    ..Default::default()
                }),
                None,
                false,
            )
            .unwrap();
        assert!(v.add_attachment(&mut db, &login, PNG).is_err());
        assert!(matches!(
            v.add_attachment(&mut db, "missing", PNG),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn moved_ciphertext_does_not_open() {
        let (mut db, mut v, note) = setup();
        let a = v.add_attachment(&mut db, &note, PNG).unwrap();
        let b = v.add_attachment(&mut db, &note, PNG).unwrap();
        // swap a's ciphertext into b's row: the AAD binds it to its own id
        db.conn()
            .execute(
                "UPDATE attachments SET encrypted_data = (SELECT encrypted_data FROM attachments WHERE id = ?1),
                     nonce = (SELECT nonce FROM attachments WHERE id = ?1) WHERE id = ?2",
                [&a, &b],
            )
            .unwrap();
        assert!(v.read_attachment(&db, &b).is_err());
    }

    #[test]
    fn unreferenced_images_are_pruned() {
        let (mut db, mut v, note) = setup();
        let kept = v.add_attachment(&mut db, &note, PNG).unwrap();
        let dropped = v.add_attachment(&mut db, &note, PNG).unwrap();
        set_content(
            &mut db,
            &mut v,
            &note,
            format!("до\n![скрин](attachment:{kept})\nпосле"),
        );

        assert_eq!(v.prune_attachments(&mut db).unwrap(), 1);
        assert!(v.read_attachment(&db, &kept).is_ok());
        assert!(matches!(
            v.read_attachment(&db, &dropped),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn deleting_the_note_deletes_its_images() {
        let (mut db, mut v, note) = setup();
        let id = v.add_attachment(&mut db, &note, PNG).unwrap();
        v.move_to_trash(&mut db, &note).unwrap();
        assert!(v.read_attachment(&db, &id).is_ok(), "trash keeps them");
        v.empty_trash(&mut db).unwrap();
        assert!(v.read_attachment(&db, &id).is_err());
        let all = ItemFilter {
            view: ItemView::All,
            folder_id: None,
            query: None,
            sort: ItemSort::Title,
        };
        assert!(v.list_items(&all).is_empty());
    }

    #[test]
    fn finds_references_in_markdown() {
        let ids: Vec<&str> =
            referenced_ids("![a](attachment:abc-1) text attachment: ![b](attachment:DEF)")
                .collect();
        assert_eq!(ids, vec!["abc-1", "DEF"]);
    }

    #[test]
    fn plaintext_layout_is_checked() {
        assert!(is_valid_plaintext(&encode("image/png", PNG)));
        assert!(!is_valid_plaintext(&encode("image/gif", PNG)));
        assert!(!is_valid_plaintext(b""));
        assert!(!is_valid_plaintext(b"\xFFimage/png"));
    }
}
