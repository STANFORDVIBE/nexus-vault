//! Encrypted `.nexus` backups.
//!
//! Layout:
//! ```text
//! MAGIC (8 bytes) | header length (u32 LE) | header JSON | sealed body
//! ```
//! The header carries what is needed to unwrap the Vault Key with the master password
//! (KDF parameters, salt, key envelope). The body is a JSON snapshot of all rows
//! (whose payloads are already encrypted) sealed again with the Vault Key, using
//! `MAGIC || header` as associated data, so any change to either part is detected.
//!
//! Restore is atomic: the file is fully verified and written to a new database first,
//! and only then swapped in place of the current vault.

pub mod auto;
pub mod hex;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::core::model::ItemType;
use crate::core::time::now_millis;
use crate::core::vault::{self, VAULT_FORMAT_VERSION};
use crate::crypto::CRYPTO_VERSION;
use crate::crypto::aead::{self, KEY_LEN, SecretKey};
use crate::crypto::kdf::{self, KDF_ALGORITHM, KdfParams};
use crate::database::Database;
use crate::database::repo::{self, EnvelopeRow, FolderRow, ItemRow, VaultMetaRow};

pub const MAGIC: &[u8; 8] = b"NXVBAK\x00\x01";
pub const BACKUP_FORMAT_VERSION: u32 = 1;
pub const EXTENSION: &str = "nexus";
const MAX_HEADER: usize = 64 * 1024;
const MAX_BACKUP_SIZE: u64 = 1024 * 1024 * 1024;

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
struct Body {
    vault_created_at: i64,
    folders: Vec<BackupFolder>,
    items: Vec<BackupItem>,
    settings: Vec<(String, String)>,
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
    };
    let body_json =
        zeroize::Zeroizing::new(serde_json::to_vec(&body).map_err(|_| AppError::Internal)?);

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
    let ciphertext = aead::seal_with_nonce(
        vault_key,
        &body_nonce,
        &body_json,
        &header_aad(&header_bytes),
    )?;

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

    let body_json = aead::open(
        &vault_key,
        &decode(&header.body_nonce)?,
        ciphertext,
        &header_aad(header_bytes),
    )
    .map_err(|_| AppError::InvalidBackup)?;
    let body: Body = serde_json::from_slice(&body_json).map_err(|_| AppError::InvalidBackup)?;

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
pub fn swap_in(new_db: &Path, live_db: &Path, previous_db: &Path) -> AppResult<()> {
    remove_db_files(previous_db)?;
    if live_db.exists() {
        fs::rename(live_db, previous_db)?;
    }
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(sidecar(live_db, suffix));
    }
    fs::rename(new_db, live_db)?;
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
        let text =
            String::from_utf8_lossy(&bytes).replace("\"formatVersion\":1", "\"formatVersion\":9");
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
