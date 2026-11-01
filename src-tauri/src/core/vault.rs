//! Vault lifecycle: creation, unlock, master password change and the per-row
//! encryption scheme.
//!
//! Key hierarchy: master password --Argon2id--> KEK --XChaCha20-Poly1305--> Vault Key
//! --XChaCha20-Poly1305--> every item payload and folder name.

use std::collections::HashMap;

use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::core::error::{AppError, AppResult};
use crate::core::model::{ItemPayload, ItemType};
use crate::core::time::now_millis;
use crate::crypto::aead::{self, KEY_LEN, SecretKey};
use crate::crypto::kdf::{self, KDF_ALGORITHM, KdfParams, SALT_LEN};
use crate::crypto::{CRYPTO_VERSION, random};
use crate::database::Database;
use crate::database::repo::{self, EnvelopeRow, VaultMetaRow};

/// Version of the overall vault layout (tables + AAD formats).
pub const VAULT_FORMAT_VERSION: i64 = 1;
pub const MIN_MASTER_PASSWORD_CHARS: usize = 10;
const MAX_MASTER_PASSWORD_BYTES: usize = 1024;
/// Ciphertext length leaks plaintext length; padding to buckets blurs it.
const PADDING_BUCKET: usize = 64;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct VaultItem {
    pub id: String,
    #[zeroize(skip)]
    pub item_type: ItemType,
    pub folder_id: Option<String>,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub payload: ItemPayload,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Everything that exists only while the vault is unlocked. Dropping it (on lock)
/// wipes the Vault Key and every decrypted payload from memory.
pub struct UnlockedVault {
    pub vault_id: String,
    pub(crate) key: SecretKey,
    pub items: HashMap<String, VaultItem>,
    pub folders: HashMap<String, Folder>,
    /// Rows that failed authentication on unlock (tampered or corrupted).
    pub corrupted_items: usize,
}

// ---- associated data ------------------------------------------------------------

fn envelope_aad(vault_id: &str) -> Vec<u8> {
    format!("nexus-vault/v1/envelope/{vault_id}").into_bytes()
}

pub(crate) fn item_aad(vault_id: &str, item_id: &str, item_type: ItemType) -> Vec<u8> {
    format!(
        "nexus-vault/v1/item/{vault_id}/{item_id}/{}",
        item_type.as_str()
    )
    .into_bytes()
}

pub(crate) fn folder_aad(vault_id: &str, folder_id: &str) -> Vec<u8> {
    format!("nexus-vault/v1/folder/{vault_id}/{folder_id}").into_bytes()
}

// ---- payload encoding -------------------------------------------------------------

fn pad(mut bytes: Zeroizing<Vec<u8>>) -> Zeroizing<Vec<u8>> {
    // JSON ignores trailing whitespace, so spaces are a format-neutral padding
    let target = bytes.len().div_ceil(PADDING_BUCKET).max(1) * PADDING_BUCKET;
    bytes.resize(target, b' ');
    bytes
}

pub(crate) fn encode_payload(payload: &ItemPayload) -> AppResult<Zeroizing<Vec<u8>>> {
    let json = Zeroizing::new(serde_json::to_vec(payload).map_err(|_| AppError::Internal)?);
    Ok(pad(json))
}

pub(crate) fn decode_payload(bytes: &[u8], expected: ItemType) -> AppResult<ItemPayload> {
    let payload: ItemPayload = serde_json::from_slice(bytes).map_err(|_| AppError::Integrity)?;
    if payload.item_type() != expected {
        return Err(AppError::Integrity);
    }
    Ok(payload)
}

pub(crate) fn encode_text(text: &str) -> Zeroizing<Vec<u8>> {
    pad(Zeroizing::new(serde_json::to_vec(text).unwrap_or_default()))
}

fn decode_text(bytes: &[u8]) -> AppResult<String> {
    serde_json::from_slice(bytes).map_err(|_| AppError::Integrity)
}

// ---- master password ----------------------------------------------------------------

pub fn validate_new_master_password(password: &str) -> AppResult<()> {
    if password.chars().count() < MIN_MASTER_PASSWORD_CHARS {
        return Err(AppError::Validation("master_password_too_short"));
    }
    if password.len() > MAX_MASTER_PASSWORD_BYTES {
        return Err(AppError::Validation("master_password_too_long"));
    }
    Ok(())
}

fn wrap_vault_key(
    vault_id: &str,
    vault_key: &SecretKey,
    password: &str,
    params: KdfParams,
) -> AppResult<([u8; SALT_LEN], EnvelopeRow)> {
    let salt: [u8; SALT_LEN] = random::bytes()?;
    let kek = kdf::derive_kek(password.as_bytes(), &salt, params)?;
    let sealed = aead::seal(&kek, vault_key.expose(), &envelope_aad(vault_id))?;
    Ok((
        salt,
        EnvelopeRow {
            wrapped_key: sealed.ciphertext,
            nonce: sealed.nonce.to_vec(),
            crypto_version: CRYPTO_VERSION,
        },
    ))
}

/// Derives the KEK from `password` and unwraps the Vault Key. A wrong password and a
/// tampered envelope are indistinguishable by design: both fail authentication.
fn unwrap_vault_key(
    meta: &VaultMetaRow,
    envelope: &EnvelopeRow,
    password: &str,
) -> AppResult<SecretKey> {
    if meta.format_version > VAULT_FORMAT_VERSION
        || envelope.crypto_version > CRYPTO_VERSION
        || meta.kdf_algorithm != KDF_ALGORITHM
    {
        return Err(AppError::UnsupportedVersion);
    }
    meta.kdf.validate_stored()?;
    if meta.salt.len() < SALT_LEN {
        return Err(AppError::Integrity);
    }

    let kek = kdf::derive_kek(password.as_bytes(), &meta.salt, meta.kdf)?;
    let raw = aead::open(
        &kek,
        &envelope.nonce,
        &envelope.wrapped_key,
        &envelope_aad(&meta.vault_id),
    )
    .map_err(|_| AppError::InvalidPassword)?;
    let bytes: [u8; KEY_LEN] = raw.as_slice().try_into().map_err(|_| AppError::Integrity)?;
    Ok(SecretKey::from_bytes(bytes))
}

pub fn vault_exists(db: &Database) -> AppResult<bool> {
    Ok(repo::load_meta(db.conn())?.is_some())
}

/// Creates a brand new vault in an empty database and returns it unlocked.
pub fn create(db: &mut Database, password: &str, params: KdfParams) -> AppResult<UnlockedVault> {
    validate_new_master_password(password)?;
    if vault_exists(db)? {
        return Err(AppError::VaultAlreadyExists);
    }

    let vault_id = random::new_id()?;
    let vault_key = SecretKey::generate()?;
    let (salt, envelope) = wrap_vault_key(&vault_id, &vault_key, password, params)?;
    let now = now_millis();

    db.transaction(|tx| {
        repo::insert_meta(
            tx,
            &VaultMetaRow {
                vault_id: vault_id.clone(),
                format_version: VAULT_FORMAT_VERSION,
                kdf_algorithm: KDF_ALGORITHM.into(),
                kdf: params,
                salt: salt.to_vec(),
                created_at: now,
            },
        )?;
        repo::upsert_envelope(tx, &envelope, now)
    })?;

    Ok(UnlockedVault {
        vault_id,
        key: vault_key,
        items: HashMap::new(),
        folders: HashMap::new(),
        corrupted_items: 0,
    })
}

/// Verifies the master password and decrypts the whole vault into memory.
pub fn unlock(db: &Database, password: &str) -> AppResult<UnlockedVault> {
    let meta = repo::load_meta(db.conn())?.ok_or(AppError::VaultNotInitialized)?;
    let envelope = repo::load_envelope(db.conn())?.ok_or(AppError::Integrity)?;
    let key = unwrap_vault_key(&meta, &envelope, password)?;
    load_contents(db, meta.vault_id, key)
}

/// Decrypts folders and items with an already unwrapped Vault Key.
pub(crate) fn load_contents(
    db: &Database,
    vault_id: String,
    key: SecretKey,
) -> AppResult<UnlockedVault> {
    let mut folders = HashMap::new();
    for row in repo::load_folders(db.conn())? {
        let name = aead::open(
            &key,
            &row.nonce,
            &row.encrypted_name,
            &folder_aad(&vault_id, &row.id),
        )
        .and_then(|plain| decode_text(&plain))
        .unwrap_or_else(|_| "⚠ Повреждённая папка".into());
        folders.insert(
            row.id.clone(),
            Folder {
                id: row.id.clone(),
                name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
        );
    }

    let mut items = HashMap::new();
    let mut corrupted_items = 0;
    for row in repo::load_items(db.conn())? {
        let decrypted = ItemType::parse(&row.item_type).and_then(|item_type| {
            if row.crypto_version > CRYPTO_VERSION {
                return Err(AppError::UnsupportedVersion);
            }
            let plain = aead::open(
                &key,
                &row.nonce,
                &row.encrypted_payload,
                &item_aad(&vault_id, &row.id, item_type),
            )?;
            Ok((item_type, decode_payload(&plain, item_type)?))
        });
        match decrypted {
            Ok((item_type, payload)) => {
                items.insert(
                    row.id.clone(),
                    VaultItem {
                        id: row.id.clone(),
                        item_type,
                        folder_id: row.folder_id.clone(),
                        favorite: row.is_favorite,
                        created_at: row.created_at,
                        updated_at: row.updated_at,
                        deleted_at: row.deleted_at,
                        payload,
                    },
                );
            }
            // A single damaged row must not lock the user out of everything else.
            Err(_) => corrupted_items += 1,
        }
    }

    Ok(UnlockedVault {
        vault_id,
        key,
        items,
        folders,
        corrupted_items,
    })
}

/// Confirms `password` is the master password of this unlocked vault (used before
/// sensitive actions such as a plaintext export).
pub fn check_master_password(
    db: &Database,
    vault: &UnlockedVault,
    password: &str,
) -> AppResult<()> {
    let meta = repo::load_meta(db.conn())?.ok_or(AppError::VaultNotInitialized)?;
    let envelope = repo::load_envelope(db.conn())?.ok_or(AppError::Integrity)?;
    let key = unwrap_vault_key(&meta, &envelope, password)?;
    if key.expose() != vault.key.expose() {
        return Err(AppError::Integrity);
    }
    Ok(())
}

/// Re-wraps the Vault Key under a new master password. Items are untouched, so the
/// change is a single small transaction and cannot leave the vault half-migrated.
pub fn change_master_password(
    db: &mut Database,
    vault: &UnlockedVault,
    current: &str,
    new: &str,
    params: KdfParams,
) -> AppResult<()> {
    validate_new_master_password(new)?;
    let meta = repo::load_meta(db.conn())?.ok_or(AppError::VaultNotInitialized)?;
    let envelope = repo::load_envelope(db.conn())?.ok_or(AppError::Integrity)?;
    let current_key = unwrap_vault_key(&meta, &envelope, current)?;
    if current_key.expose() != vault.key.expose() {
        return Err(AppError::Integrity);
    }

    let (salt, envelope) = wrap_vault_key(&vault.vault_id, &vault.key, new, params)?;
    let now = now_millis();
    db.transaction(|tx| {
        repo::update_kdf(tx, params, &salt)?;
        repo::upsert_envelope(tx, &envelope, now)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "correct horse battery";

    fn new_vault() -> (Database, UnlockedVault) {
        let mut db = Database::open_in_memory().unwrap();
        let vault = create(&mut db, PASSWORD, KdfParams::TEST).unwrap();
        (db, vault)
    }

    #[test]
    fn create_then_unlock() {
        let (db, vault) = new_vault();
        let unlocked = unlock(&db, PASSWORD).unwrap();
        assert_eq!(unlocked.vault_id, vault.vault_id);
        assert_eq!(unlocked.key.expose(), vault.key.expose());
    }

    #[test]
    fn wrong_password_does_not_unlock() {
        let (db, _) = new_vault();
        assert!(matches!(
            unlock(&db, "correct horse batterz"),
            Err(AppError::InvalidPassword)
        ));
        assert!(matches!(unlock(&db, ""), Err(AppError::InvalidPassword)));
    }

    #[test]
    fn cannot_create_twice() {
        let (mut db, _) = new_vault();
        assert!(matches!(
            create(&mut db, PASSWORD, KdfParams::TEST),
            Err(AppError::VaultAlreadyExists)
        ));
    }

    #[test]
    fn rejects_short_master_password() {
        let mut db = Database::open_in_memory().unwrap();
        assert!(matches!(
            create(&mut db, "short", KdfParams::TEST),
            Err(AppError::Validation(_))
        ));
        assert!(!vault_exists(&db).unwrap());
    }

    #[test]
    fn unlock_without_vault_reports_uninitialized() {
        let db = Database::open_in_memory().unwrap();
        assert!(matches!(
            unlock(&db, PASSWORD),
            Err(AppError::VaultNotInitialized)
        ));
    }

    #[test]
    fn change_password_keeps_access() {
        let (mut db, vault) = new_vault();
        change_master_password(
            &mut db,
            &vault,
            PASSWORD,
            "a brand new secret",
            KdfParams::TEST,
        )
        .unwrap();

        assert!(matches!(
            unlock(&db, PASSWORD),
            Err(AppError::InvalidPassword)
        ));
        let unlocked = unlock(&db, "a brand new secret").unwrap();
        assert_eq!(unlocked.key.expose(), vault.key.expose());
    }

    #[test]
    fn change_password_requires_current_password() {
        let (mut db, vault) = new_vault();
        assert!(matches!(
            change_master_password(
                &mut db,
                &vault,
                "not the password",
                "another secret!",
                KdfParams::TEST
            ),
            Err(AppError::InvalidPassword)
        ));
        assert!(unlock(&db, PASSWORD).is_ok());
    }

    #[test]
    fn tampered_envelope_does_not_unlock() {
        let (db, _) = new_vault();
        db.conn()
            .execute(
                "UPDATE key_envelopes SET wrapped_key = zeroblob(length(wrapped_key))",
                [],
            )
            .unwrap();
        assert!(unlock(&db, PASSWORD).is_err());
    }

    #[test]
    fn downgraded_kdf_parameters_are_refused() {
        let (db, _) = new_vault();
        db.conn()
            .execute(
                "UPDATE vault_meta SET kdf_memory_kib = 1024, kdf_iterations = 1",
                [],
            )
            .unwrap();
        assert!(matches!(unlock(&db, PASSWORD), Err(AppError::Integrity)));
    }

    #[test]
    fn newer_format_is_refused() {
        let (db, _) = new_vault();
        db.conn()
            .execute("UPDATE vault_meta SET format_version = 99", [])
            .unwrap();
        assert!(matches!(
            unlock(&db, PASSWORD),
            Err(AppError::UnsupportedVersion)
        ));
    }

    #[test]
    fn padding_hides_exact_length() {
        let short = encode_text("a");
        let longer = encode_text("abcdefghij");
        assert_eq!(short.len(), longer.len());
        assert_eq!(short.len() % PADDING_BUCKET, 0);
        assert_eq!(decode_text(&short).unwrap(), "a");
    }
}
