//! Encrypted `.nexus` backups.
//!
//! Layout:
//! ```text
//! MAGIC (8 bytes) | header length (u32 LE) | header JSON | sealed body
//! ```
//! The header carries what is needed to unwrap the Vault Key with the master password
//! (KDF parameters, salt, key envelope). The body is a snapshot of all rows (whose
//! payloads are already encrypted) sealed again with the Vault Key, using
//! `MAGIC || header` as associated data, so any change to either part is detected.
//!
//! Body plaintext since version 3: `JSON length (u32 LE) | JSON | image bytes`. Images
//! are appended raw, in the order of `attachments` in the JSON, instead of being hex
//! inside it — that would double their size. Versions 1–2 are plain JSON.
//!
//! Restore is atomic: the file is fully verified and written to a new database first,
//! and only then swapped in place of the current vault.

pub mod auto;
pub mod hex;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::attachments;
use crate::core::error::{AppError, AppResult};
use crate::core::model::ItemType;
use crate::core::time::now_millis;
use crate::core::vault::{self, VAULT_FORMAT_VERSION};
use crate::crypto::CRYPTO_VERSION;
use crate::crypto::aead::{self, KEY_LEN, SecretKey};
use crate::crypto::kdf::{self, KDF_ALGORITHM, KdfParams};
use crate::database::Database;
use crate::database::repo::{self, AttachmentRow, EnvelopeRow, FolderRow, ItemRow, VaultMetaRow};

pub const MAGIC: &[u8; 8] = b"NXVBAK\x00\x01";
/// 2: note images (hex in the JSON). 3: images as raw bytes after the JSON.
/// Older files are still restored.
pub const BACKUP_FORMAT_VERSION: u32 = 3;
pub const EXTENSION: &str = "nexus";
const MAX_HEADER: usize = 64 * 1024;
/// Restore reads the whole file into memory; creating refuses to go past this too,
/// so no copy is ever written that could not be restored.
const MAX_BACKUP_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const JSON_LEN_BYTES: usize = 4;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    format_version: u32,
    app_version: String,
    created_at: i64,
    vault_id: String,
    vault_format_version: i64,
    kdf_algorithm: String,
    kdf_memory_kib: u32,
    kdf_iterations: u32,
    kdf_parallelism: u32,
    salt: String,
    wrapped_key: String,
    key_nonce: String,
    key_crypto_version: i64,
    body_nonce: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupItem {
    id: String,
    item_type: String,
    encrypted_payload: String,
    nonce: String,
    crypto_version: i64,
    folder_id: Option<String>,
    is_favorite: bool,
    created_at: i64,
    updated_at: i64,
    deleted_at: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFolder {
    id: String,
    encrypted_name: String,
    nonce: String,
    crypto_version: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupAttachment {
    id: String,
    item_id: String,
    /// Version 2 only: the ciphertext as hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encrypted_data: Option<String>,
    /// Version 3: length of the ciphertext in the raw section after the JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<u64>,
    nonce: String,
    crypto_version: i64,
    created_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    vault_created_at: i64,
    folders: Vec<BackupFolder>,
    items: Vec<BackupItem>,
    settings: Vec<(String, String)>,
    #[serde(default)]
    attachments: Vec<BackupAttachment>,
}

/// What the UI shows before asking for the password.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub created_at: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSummary {
    pub items: usize,
    pub folders: usize,
}

fn header_aad(header_bytes: &[u8]) -> Vec<u8> {
    let mut aad = MAGIC.to_vec();
    aad.extend_from_slice(header_bytes);
    aad
}

/// Writes atomically: a temp file next to the target, flushed, then renamed.
fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension(format!("{EXTENSION}.tmp"));
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(())
}

/// Serializes the whole vault into the `.nexus` format.
pub fn create_backup_bytes(db: &Database, vault_key: &SecretKey) -> AppResult<Vec<u8>> {
    let meta = repo::load_meta(db.conn())?.ok_or(AppError::VaultNotInitialized)?;
    let envelope = repo::load_envelope(db.conn())?.ok_or(AppError::Integrity)?;

    let rows = repo::load_attachments(db.conn())?;
    let body = Body {
        vault_created_at: meta.created_at,
        folders: repo::load_folders(db.conn())?
            .into_iter()
            .map(|f| BackupFolder {
                id: f.id,
                encrypted_name: hex::encode(&f.encrypted_name),
                nonce: hex::encode(&f.nonce),
                crypto_version: f.crypto_version,
                created_at: f.created_at,
                updated_at: f.updated_at,
            })
            .collect(),
        items: repo::load_items(db.conn())?
            .into_iter()
            .map(|i| BackupItem {
                id: i.id,
                item_type: i.item_type,
                encrypted_payload: hex::encode(&i.encrypted_payload),
                nonce: hex::encode(&i.nonce),
                crypto_version: i.crypto_version,
                folder_id: i.folder_id,
                is_favorite: i.is_favorite,
                created_at: i.created_at,
                updated_at: i.updated_at,
                deleted_at: i.deleted_at,
            })
            .collect(),
        settings: repo::load_settings(db.conn())?,
        attachments: rows
            .iter()
            .map(|a| BackupAttachment {
                id: a.id.clone(),
                item_id: a.item_id.clone(),
                encrypted_data: None,
                length: Some(a.encrypted_data.len() as u64),
                nonce: hex::encode(&a.nonce),
                crypto_version: a.crypto_version,
                created_at: a.created_at,
            })
            .collect(),
    };
    let json = serde_json::to_vec(&body).map_err(|_| AppError::Internal)?;
    let images: usize = rows.iter().map(|a| a.encrypted_data.len()).sum();
    if (json.len() + images) as u64 > MAX_BACKUP_SIZE - MAX_HEADER as u64 {
        return Err(AppError::Validation("backup_too_large"));
    }
    let mut plain =
        zeroize::Zeroizing::new(Vec::with_capacity(JSON_LEN_BYTES + json.len() + images));
    plain.extend_from_slice(&(json.len() as u32).to_le_bytes());
    plain.extend_from_slice(&json);
    for row in &rows {
        plain.extend_from_slice(&row.encrypted_data);
    }

    // The body nonce must be part of the authenticated header, so draw it first.
    let body_nonce: [u8; aead::NONCE_LEN] = crate::crypto::random::bytes()?;
    let header = Header {
        format_version: BACKUP_FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at: now_millis(),
        vault_id: meta.vault_id,
        vault_format_version: meta.format_version,
        kdf_algorithm: meta.kdf_algorithm,
        kdf_memory_kib: meta.kdf.memory_kib,
        kdf_iterations: meta.kdf.iterations,
        kdf_parallelism: meta.kdf.parallelism,
        salt: hex::encode(&meta.salt),
        wrapped_key: hex::encode(&envelope.wrapped_key),
        key_nonce: hex::encode(&envelope.nonce),
        key_crypto_version: envelope.crypto_version,
        body_nonce: hex::encode(&body_nonce),
    };
    let header_bytes = serde_json::to_vec(&header).map_err(|_| AppError::Internal)?;
    let ciphertext =
        aead::seal_with_nonce(vault_key, &body_nonce, &plain, &header_aad(&header_bytes))?;

    let mut out = Vec::with_capacity(MAGIC.len() + 4 + header_bytes.len() + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn write_backup(path: &Path, db: &Database, vault_key: &SecretKey) -> AppResult<()> {
    let bytes = create_backup_bytes(db, vault_key)?;
    write_atomic(path, &bytes)
}

fn split(bytes: &[u8]) -> AppResult<(Header, &[u8], &[u8])> {
    let rest = bytes
        .strip_prefix(MAGIC.as_slice())
        .ok_or(AppError::InvalidBackup)?;
    let (len, rest) = rest.split_at_checked(4).ok_or(AppError::InvalidBackup)?;
    let len = u32::from_le_bytes(len.try_into().map_err(|_| AppError::InvalidBackup)?) as usize;
    if len > MAX_HEADER {
        return Err(AppError::InvalidBackup);
    }
    let (header_bytes, ciphertext) = rest.split_at_checked(len).ok_or(AppError::InvalidBackup)?;
    let header: Header =
        serde_json::from_slice(header_bytes).map_err(|_| AppError::InvalidBackup)?;
    if header.format_version > BACKUP_FORMAT_VERSION
        || header.vault_format_version > VAULT_FORMAT_VERSION
    {
        return Err(AppError::UnsupportedVersion);
    }
    Ok((header, header_bytes, ciphertext))
}

pub fn read_file(path: &Path) -> AppResult<Vec<u8>> {
    let size = fs::metadata(path)?.len();
    if size > MAX_BACKUP_SIZE {
        return Err(AppError::InvalidBackup);
    }
    Ok(fs::read(path)?)
}

pub fn inspect(path: &Path) -> AppResult<BackupInfo> {
    let bytes = read_file(path)?;
    let (header, _, _) = split(&bytes)?;
    Ok(BackupInfo {
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        created_at: header.created_at,
        size_bytes: bytes.len() as u64,
    })
}

/// A backup that passed every check and is ready to be written as a new vault.
pub struct VerifiedBackup {
    meta: VaultMetaRow,
    envelope: EnvelopeRow,
    folders: Vec<FolderRow>,
    items: Vec<ItemRow>,
    attachments: Vec<AttachmentRow>,
    settings: Vec<(String, String)>,
}

impl VerifiedBackup {
    pub fn summary(&self) -> RestoreSummary {
        RestoreSummary {
            items: self.items.len(),
            folders: self.folders.len(),
        }
    }
}

/// Decrypts and fully verifies a backup: header, body and every single row.
pub fn verify(bytes: &[u8], password: &str) -> AppResult<VerifiedBackup> {
    let (header, header_bytes, ciphertext) = split(bytes)?;
    let decode = |s: &str| hex::decode(s).ok_or(AppError::InvalidBackup);

    if header.kdf_algorithm != KDF_ALGORITHM || header.key_crypto_version > CRYPTO_VERSION {
        return Err(AppError::UnsupportedVersion);
    }
    let params = KdfParams {
        memory_kib: header.kdf_memory_kib,
        iterations: header.kdf_iterations,
        parallelism: header.kdf_parallelism,
    };
    params
        .validate_stored()
        .map_err(|_| AppError::InvalidBackup)?;

    let salt = decode(&header.salt)?;
    let wrapped_key = decode(&header.wrapped_key)?;
    let key_nonce = decode(&header.key_nonce)?;
    let kek = kdf::derive_kek(password.as_bytes(), &salt, params)?;
    let raw_key = aead::open(
        &kek,
        &key_nonce,
        &wrapped_key,
        format!("nexus-vault/v1/envelope/{}", header.vault_id).as_bytes(),
    )
    .map_err(|_| AppError::InvalidPassword)?;
    let key_bytes: [u8; KEY_LEN] = raw_key
        .as_slice()
        .try_into()
        .map_err(|_| AppError::InvalidBackup)?;
    let vault_key = SecretKey::from_bytes(key_bytes);

    let plain = aead::open(
        &vault_key,
        &decode(&header.body_nonce)?,
        ciphertext,
        &header_aad(header_bytes),
    )
    .map_err(|_| AppError::InvalidBackup)?;
    let (body_json, mut raw) = if header.format_version >= 3 {
        let (len, rest) = plain
            .split_at_checked(JSON_LEN_BYTES)
            .ok_or(AppError::InvalidBackup)?;
        let len = u32::from_le_bytes(len.try_into().map_err(|_| AppError::InvalidBackup)?);
        rest.split_at_checked(len as usize)
            .ok_or(AppError::InvalidBackup)?
    } else {
        (plain.as_slice(), &[][..])
    };
    let body: Body = serde_json::from_slice(body_json).map_err(|_| AppError::InvalidBackup)?;

    let folders: Vec<FolderRow> = body
        .folders
        .into_iter()
        .map(|f| {
            Ok(FolderRow {
                encrypted_name: decode(&f.encrypted_name)?,
                nonce: decode(&f.nonce)?,
                id: f.id,
                crypto_version: f.crypto_version,
                created_at: f.created_at,
                updated_at: f.updated_at,
            })
        })
        .collect::<AppResult<_>>()?;
    let items: Vec<ItemRow> = body
        .items
        .into_iter()
        .map(|i| {
            Ok(ItemRow {
                encrypted_payload: decode(&i.encrypted_payload)?,
                nonce: decode(&i.nonce)?,
                id: i.id,
                item_type: i.item_type,
                crypto_version: i.crypto_version,
                folder_id: i.folder_id,
                is_favorite: i.is_favorite,
                created_at: i.created_at,
                updated_at: i.updated_at,
                deleted_at: i.deleted_at,
            })
        })
        .collect::<AppResult<_>>()?;

    // Every row must authenticate under this vault's key before we accept the backup.
    for folder in &folders {
        aead::open(
            &vault_key,
            &folder.nonce,
            &folder.encrypted_name,
            &vault::folder_aad(&header.vault_id, &folder.id),
        )
        .map_err(|_| AppError::InvalidBackup)?;
    }
    for item in &items {
        let item_type = ItemType::parse(&item.item_type).map_err(|_| AppError::InvalidBackup)?;
        let plain = aead::open(
            &vault_key,
            &item.nonce,
            &item.encrypted_payload,
            &vault::item_aad(&header.vault_id, &item.id, item_type),
        )
        .map_err(|_| AppError::InvalidBackup)?;
        vault::decode_payload(&plain, item_type).map_err(|_| AppError::InvalidBackup)?;
    }

    let attachments: Vec<AttachmentRow> = body
        .attachments
        .into_iter()
        .map(|a| {
            let encrypted_data = match (a.encrypted_data, a.length) {
                (Some(hex_data), None) => decode(&hex_data)?,
                (None, Some(len)) => {
                    let (data, rest) = raw
                        .split_at_checked(
                            usize::try_from(len).map_err(|_| AppError::InvalidBackup)?,
                        )
                        .ok_or(AppError::InvalidBackup)?;
                    raw = rest;
                    data.to_vec()
                }
                _ => return Err(AppError::InvalidBackup),
            };
            Ok(AttachmentRow {
                encrypted_data,
                nonce: decode(&a.nonce)?,
                id: a.id,
                item_id: a.item_id,
                crypto_version: a.crypto_version,
                created_at: a.created_at,
            })
        })
        .collect::<AppResult<_>>()?;
    if !raw.is_empty() {
        // bytes nobody accounts for: not a file this app wrote
        return Err(AppError::InvalidBackup);
    }
    for attachment in &attachments {
        if !items.iter().any(|i| i.id == attachment.item_id) {
            return Err(AppError::InvalidBackup);
        }
        let plain = aead::open(
            &vault_key,
            &attachment.nonce,
            &attachment.encrypted_data,
            &attachments::attachment_aad(&header.vault_id, &attachment.item_id, &attachment.id),
        )
        .map_err(|_| AppError::InvalidBackup)?;
        if !attachments::is_valid_plaintext(&plain) {
            return Err(AppError::InvalidBackup);
        }
    }

    Ok(VerifiedBackup {
        meta: VaultMetaRow {
            vault_id: header.vault_id,
            format_version: header.vault_format_version,
            kdf_algorithm: header.kdf_algorithm,
            kdf: params,
            salt,
            created_at: body.vault_created_at,
        },
        envelope: EnvelopeRow {
            wrapped_key,
            nonce: key_nonce,
            crypto_version: header.key_crypto_version,
        },
        folders,
        items,
        attachments,
        settings: body.settings,
    })
}

/// Writes a verified backup into a brand-new database file at `path`.
pub fn write_database(path: &Path, backup: &VerifiedBackup) -> AppResult<()> {
    remove_db_files(path)?;
    let mut db = Database::open(path)?;
    let now = now_millis();
    db.transaction(|tx| {
        repo::insert_meta(tx, &backup.meta)?;
        repo::upsert_envelope(tx, &backup.envelope, now)?;
        for folder in &backup.folders {
            repo::insert_folder(tx, folder)?;
        }
        for item in &backup.items {
            repo::insert_item(tx, item)?;
        }
        for attachment in &backup.attachments {
            repo::insert_attachment(tx, attachment)?;
        }
        for (key, value) in &backup.settings {
            repo::set_setting(tx, key, value)?;
        }
        Ok(())
    })?;
    db.checkpoint()?;
    Ok(())
}

/// `dir/vault.db` + "previous" -> `dir/vault.previous.db` (works for any vault name).
pub fn sibling(live_db: &Path, tag: &str) -> PathBuf {
    let stem = live_db
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vault".into());
    live_db.with_file_name(format!("{stem}.{tag}.db"))
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Removes a database file together with its WAL/SHM side files.
pub fn remove_db_files(path: &Path) -> AppResult<()> {
    for p in [
        path.to_path_buf(),
        sidecar(path, "-wal"),
        sidecar(path, "-shm"),
    ] {
        match fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Swaps `new_db` into `live_db`, keeping the replaced vault as `previous_db`.
/// The live database must be closed (and checkpointed) by the caller.
/// `previous_db` must be a fresh name: an existing file there is never overwritten,
/// so restoring twice in a row cannot destroy the vault kept by the first restore.
pub fn swap_in(new_db: &Path, live_db: &Path, previous_db: &Path) -> AppResult<()> {
    if previous_db.exists() {
        return Err(AppError::Internal);
    }
    let had_live = live_db.exists();
    if had_live {
        fs::rename(live_db, previous_db)?;
    }
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(sidecar(live_db, suffix));
    }
    fs::rename(new_db, live_db).inspect_err(|_| {
        // put the old vault back: the user must never be left without a vault file
        if had_live {
            let _ = fs::rename(previous_db, live_db);
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{ItemPayload, LoginPayload};
    use crate::core::vault::UnlockedVault;

    const PASSWORD: &str = "correct horse battery";

    fn vault_with_data() -> (Database, UnlockedVault) {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, PASSWORD, KdfParams::TEST).unwrap();
        let folder = v.create_folder(&mut db, "Работа").unwrap();
        v.create_item(
            &mut db,
            ItemPayload::Login(LoginPayload {
                title: "GitHub".into(),
                password: Some("BackupSecret-123".into()),
                ..Default::default()
            }),
            Some(folder),
            true,
        )
        .unwrap();
        (db, v)
    }

    #[test]
    fn roundtrip_restores_everything() {
        let (db, v) = vault_with_data();
        let bytes = create_backup_bytes(&db, &v.key).unwrap();
        let needle = b"BackupSecret-123";
        assert!(!bytes.windows(needle.len()).any(|w| w == needle));

        let verified = verify(&bytes, PASSWORD).unwrap();
        assert_eq!(verified.summary().items, 1);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("restored.db");
        write_database(&path, &verified).unwrap();
        let restored_db = Database::open(&path).unwrap();
        let restored = vault::unlock(&restored_db, PASSWORD).unwrap();
        assert_eq!(restored.items.len(), 1);
        assert_eq!(restored.list_folders()[0].name, "Работа");
        let item = restored.items.values().next().unwrap();
        assert!(item.favorite);
    }

    /// The vault from `vault_with_data` plus a note with a 64 KiB "screenshot".
    fn vault_with_image() -> (Database, UnlockedVault, String, Vec<u8>) {
        use crate::core::model::SecureNotePayload;
        let (mut db, mut v) = vault_with_data();
        let note = v
            .create_item(
                &mut db,
                ItemPayload::SecureNote(SecureNotePayload {
                    title: "Скрин".into(),
                    content: String::new(),
                    tags: vec![],
                }),
                None,
                false,
            )
            .unwrap();
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend((0..64 * 1024).map(|i| (i % 251) as u8));
        let id = v.add_attachment(&mut db, &note, &png).unwrap();
        (db, v, id, png)
    }

    fn restore(bytes: &[u8]) -> (tempfile::TempDir, Database) {
        let verified = verify(bytes, PASSWORD).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("restored.db");
        write_database(&path, &verified).unwrap();
        let db = Database::open(&path).unwrap();
        (dir, db)
    }

    /// Rewrites a current backup the way an older app version wrote it: plain JSON
    /// body, images as hex inside it (version 2) or no images at all (version 1).
    fn as_older_version(bytes: &[u8], key: &SecretKey, version: u32) -> Vec<u8> {
        let (mut header, header_bytes, ciphertext) = split(bytes).unwrap();
        let nonce: [u8; aead::NONCE_LEN] =
            hex::decode(&header.body_nonce).unwrap().try_into().unwrap();
        let plain = aead::open(key, &nonce, ciphertext, &header_aad(header_bytes)).unwrap();
        let len = u32::from_le_bytes(plain[..4].try_into().unwrap()) as usize;
        let mut body: Body = serde_json::from_slice(&plain[4..4 + len]).unwrap();
        let mut raw = &plain[4 + len..];
        for a in &mut body.attachments {
            let n = a.length.take().unwrap() as usize;
            a.encrypted_data = Some(hex::encode(&raw[..n]));
            raw = &raw[n..];
        }
        if version == 1 {
            body.attachments.clear();
        }

        header.format_version = version;
        let header_bytes = serde_json::to_vec(&header).unwrap();
        let json = serde_json::to_vec(&body).unwrap();
        let sealed = aead::seal_with_nonce(key, &nonce, &json, &header_aad(&header_bytes)).unwrap();
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&header_bytes);
        out.extend_from_slice(&sealed);
        out
    }

    #[test]
    fn note_images_survive_backup_and_restore() {
        let (db, v, id, png) = vault_with_image();
        let bytes = create_backup_bytes(&db, &v.key).unwrap();
        let (_dir, restored_db) = restore(&bytes);
        let restored = vault::unlock(&restored_db, PASSWORD).unwrap();
        assert_eq!(
            &restored.read_attachment(&restored_db, &id).unwrap()[10..],
            png
        );
    }

    #[test]
    fn images_are_not_inflated_in_the_backup() {
        let (db, v, _, png) = vault_with_image();
        let current = create_backup_bytes(&db, &v.key).unwrap();
        let hex_based = as_older_version(&current, &v.key, 2);
        // the image itself plus a little overhead, not twice its size
        assert!(current.len() < png.len() + 8 * 1024, "{}", current.len());
        assert!(hex_based.len() > 2 * png.len());
    }

    #[test]
    fn older_backups_still_restore() {
        let (db, v, id, png) = vault_with_image();
        let current = create_backup_bytes(&db, &v.key).unwrap();

        let (_dir, restored_db) = restore(&as_older_version(&current, &v.key, 2));
        let restored = vault::unlock(&restored_db, PASSWORD).unwrap();
        assert_eq!(
            &restored.read_attachment(&restored_db, &id).unwrap()[10..],
            png
        );

        let (_dir, restored_db) = restore(&as_older_version(&current, &v.key, 1));
        let restored = vault::unlock(&restored_db, PASSWORD).unwrap();
        assert_eq!(restored.items.len(), 2);
    }

    #[test]
    fn unaccounted_image_bytes_are_rejected() {
        let (db, v, _, _) = vault_with_image();
        let bytes = create_backup_bytes(&db, &v.key).unwrap();
        let (header, header_bytes, ciphertext) = split(&bytes).unwrap();
        let nonce: [u8; aead::NONCE_LEN] =
            hex::decode(&header.body_nonce).unwrap().try_into().unwrap();
        let mut plain = aead::open(&v.key, &nonce, ciphertext, &header_aad(header_bytes))
            .unwrap()
            .to_vec();
        plain.extend_from_slice(b"smuggled");
        let sealed =
            aead::seal_with_nonce(&v.key, &nonce, &plain, &header_aad(header_bytes)).unwrap();
        let mut forged = bytes[..bytes.len() - ciphertext.len()].to_vec();
        forged.extend_from_slice(&sealed);
        assert!(matches!(
            verify(&forged, PASSWORD),
            Err(AppError::InvalidBackup)
        ));
    }

    #[test]
    fn wrong_password_is_rejected() {
        let (db, v) = vault_with_data();
        let bytes = create_backup_bytes(&db, &v.key).unwrap();
        assert!(matches!(
            verify(&bytes, "wrong password!!"),
            Err(AppError::InvalidPassword)
        ));
    }

    #[test]
    fn any_tampering_is_detected() {
        let (db, v) = vault_with_data();
        let bytes = create_backup_bytes(&db, &v.key).unwrap();
        let header_end = 12 + u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;

        // flip a byte in the body and in the authenticated header (created_at digit)
        let mut body = bytes.clone();
        let last = body.len() - 20;
        body[last] ^= 1;
        assert!(verify(&body, PASSWORD).is_err());

        let mut header = bytes.clone();
        let pos = bytes[..header_end]
            .windows(12)
            .position(|w| w == b"\"createdAt\":")
            .unwrap()
            + 12;
        header[pos] = if header[pos] == b'1' { b'2' } else { b'1' };
        assert!(verify(&header, PASSWORD).is_err());

        assert!(verify(&bytes[..bytes.len() / 2], PASSWORD).is_err());
        assert!(matches!(
            verify(b"garbage", PASSWORD),
            Err(AppError::InvalidBackup)
        ));
    }

    #[test]
    fn newer_format_is_refused() {
        let (db, v) = vault_with_data();
        let bytes = create_backup_bytes(&db, &v.key).unwrap();
        let text = String::from_utf8_lossy(&bytes).replace(
            &format!("\"formatVersion\":{BACKUP_FORMAT_VERSION}"),
            "\"formatVersion\":9",
        );
        assert!(matches!(
            verify(text.as_bytes(), PASSWORD),
            Err(AppError::UnsupportedVersion | AppError::InvalidBackup)
        ));
    }

    #[test]
    fn atomic_swap_keeps_previous_vault() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("vault.db");
        let new = dir.path().join("vault.restore.db");
        let previous = dir.path().join("vault.previous.db");
        fs::write(&live, b"old").unwrap();
        fs::write(&new, b"new").unwrap();

        swap_in(&new, &live, &previous).unwrap();
        assert_eq!(fs::read(&live).unwrap(), b"new");
        assert_eq!(fs::read(&previous).unwrap(), b"old");
        assert!(!new.exists());
    }

    #[test]
    fn second_restore_keeps_the_first_previous_vault() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("vault.db");
        let first = dir.path().join("vault.previous-1.db");
        let second = dir.path().join("vault.previous-2.db");
        fs::write(&live, b"original").unwrap();
        fs::write(dir.path().join("a.db"), b"wrong backup").unwrap();
        swap_in(&dir.path().join("a.db"), &live, &first).unwrap();
        fs::write(dir.path().join("b.db"), b"right backup").unwrap();
        swap_in(&dir.path().join("b.db"), &live, &second).unwrap();
        assert_eq!(fs::read(&first).unwrap(), b"original");
        assert_eq!(fs::read(&second).unwrap(), b"wrong backup");
        assert_eq!(fs::read(&live).unwrap(), b"right backup");
        // an existing file under the "previous" name is never overwritten
        fs::write(dir.path().join("c.db"), b"x").unwrap();
        assert!(swap_in(&dir.path().join("c.db"), &live, &first).is_err());
        assert_eq!(fs::read(&first).unwrap(), b"original");
        assert_eq!(fs::read(&live).unwrap(), b"right backup");
    }

    #[test]
    fn failed_swap_puts_the_live_vault_back() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("vault.db");
        let previous = dir.path().join("vault.previous-1.db");
        fs::write(&live, b"old").unwrap();
        // the new database is missing, so the second rename fails
        assert!(swap_in(&dir.path().join("missing.db"), &live, &previous).is_err());
        assert_eq!(fs::read(&live).unwrap(), b"old");
        assert!(!previous.exists());
    }

    #[test]
    fn write_backup_is_atomic_file() {
        let (db, v) = vault_with_data();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("my.nexus");
        write_backup(&path, &db, &v.key).unwrap();
        assert!(path.exists());
        assert!(!dir.path().join("my.nexus.tmp").exists());
        assert_eq!(inspect(&path).unwrap().file_name, "my.nexus");
    }
}
