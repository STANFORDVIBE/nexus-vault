//! Folders: a flat list whose names are encrypted with the Vault Key.

use serde::Serialize;

use crate::core::error::{AppError, AppResult};
use crate::core::time::now_millis;
use crate::core::vault::{Folder, UnlockedVault, encode_text, folder_aad};
use crate::crypto::{CRYPTO_VERSION, aead, random};
use crate::database::Database;
use crate::database::repo::{self, FolderRow};

const MAX_FOLDER_NAME: usize = 64;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderView {
    pub id: String,
    pub name: String,
    pub item_count: usize,
    pub created_at: i64,
}

fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_FOLDER_NAME {
        return Err(AppError::Validation("folder_name"));
    }
    Ok(name.to_string())
}

impl UnlockedVault {
    fn ensure_unique_name(&self, name: &str, except: Option<&str>) -> AppResult<()> {
        let taken = self
            .folders
            .values()
            .any(|f| Some(f.id.as_str()) != except && f.name.to_lowercase() == name.to_lowercase());
        if taken {
            return Err(AppError::Validation("folder_exists"));
        }
        Ok(())
    }

    pub fn list_folders(&self) -> Vec<FolderView> {
        let mut folders: Vec<FolderView> = self
            .folders
            .values()
            .map(|f| FolderView {
                id: f.id.clone(),
                name: f.name.clone(),
                item_count: self
                    .items
                    .values()
                    .filter(|i| i.deleted_at.is_none() && i.folder_id.as_deref() == Some(&f.id))
                    .count(),
                created_at: f.created_at,
            })
            .collect();
        folders.sort_by_key(|f| f.name.to_lowercase());
        folders
    }

    pub fn create_folder(&mut self, db: &mut Database, name: &str) -> AppResult<String> {
        let name = clean_name(name)?;
        self.ensure_unique_name(&name, None)?;
        let id = random::new_id()?;
        let sealed = aead::seal(
            &self.key,
            &encode_text(&name),
            &folder_aad(&self.vault_id, &id),
        )?;
        let now = now_millis();
        let row = FolderRow {
            id: id.clone(),
            encrypted_name: sealed.ciphertext,
            nonce: sealed.nonce.to_vec(),
            crypto_version: CRYPTO_VERSION,
            created_at: now,
            updated_at: now,
        };
        db.transaction(|tx| repo::insert_folder(tx, &row))?;
        self.folders.insert(
            id.clone(),
            Folder {
                id: id.clone(),
                name,
                created_at: now,
                updated_at: now,
            },
        );
        Ok(id)
    }

    pub fn rename_folder(&mut self, db: &mut Database, id: &str, name: &str) -> AppResult<()> {
        if !self.folders.contains_key(id) {
            return Err(AppError::NotFound);
        }
        let name = clean_name(name)?;
        self.ensure_unique_name(&name, Some(id))?;
        let sealed = aead::seal(
            &self.key,
            &encode_text(&name),
            &folder_aad(&self.vault_id, id),
        )?;
        let now = now_millis();
        db.transaction(|tx| {
            repo::update_folder_name(tx, id, &sealed.ciphertext, &sealed.nonce, now)
        })?;
        if let Some(folder) = self.folders.get_mut(id) {
            folder.name = name;
            folder.updated_at = now;
        }
        Ok(())
    }

    /// Deletes the folder; its items stay in the vault without a folder.
    pub fn delete_folder(&mut self, db: &mut Database, id: &str) -> AppResult<()> {
        if !self.folders.contains_key(id) {
            return Err(AppError::NotFound);
        }
        db.transaction(|tx| repo::delete_folder(tx, id))?;
        self.folders.remove(id);
        for item in self.items.values_mut() {
            if item.folder_id.as_deref() == Some(id) {
                item.folder_id = None;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::model::{ItemPayload, SecureNotePayload};
    use crate::core::vault;
    use crate::crypto::kdf::KdfParams;
    use crate::database::Database;

    #[test]
    fn folder_lifecycle() {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();

        let id = v.create_folder(&mut db, "  Работа ").unwrap();
        assert!(v.create_folder(&mut db, "работа").is_err());
        assert!(v.create_folder(&mut db, " ").is_err());

        let note = ItemPayload::SecureNote(SecureNotePayload {
            title: "n".into(),
            ..Default::default()
        });
        let item = v
            .create_item(&mut db, note, Some(id.clone()), false)
            .unwrap();
        assert_eq!(v.list_folders()[0].item_count, 1);

        v.rename_folder(&mut db, &id, "Личное").unwrap();
        drop(v);
        let mut v = vault::unlock(&db, "correct horse battery").unwrap();
        assert_eq!(v.list_folders()[0].name, "Личное");

        v.delete_folder(&mut db, &id).unwrap();
        assert!(v.folders.is_empty());
        assert_eq!(v.items[&item].folder_id, None);
        drop(v);
        let v = vault::unlock(&db, "correct horse battery").unwrap();
        assert_eq!(v.items[&item].folder_id, None);
    }

    #[test]
    fn folder_names_are_encrypted() {
        let mut db = Database::open_in_memory().unwrap();
        let mut v = vault::create(&mut db, "correct horse battery", KdfParams::TEST).unwrap();
        v.create_folder(&mut db, "TopSecretProject").unwrap();
        let rows = crate::database::repo::load_folders(db.conn()).unwrap();
        let needle = b"TopSecretProject";
        assert!(
            !rows[0]
                .encrypted_name
                .windows(needle.len())
                .any(|w| w == needle)
        );
    }
}
