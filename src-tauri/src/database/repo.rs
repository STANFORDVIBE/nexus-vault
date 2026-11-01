//! Typed row access. Every query is a prepared statement with bound parameters.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::core::error::AppResult;
use crate::crypto::kdf::KdfParams;

pub struct VaultMetaRow {
    pub vault_id: String,
    pub format_version: i64,
    pub kdf_algorithm: String,
    pub kdf: KdfParams,
    pub salt: Vec<u8>,
    pub created_at: i64,
}

pub struct EnvelopeRow {
    pub wrapped_key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub crypto_version: i64,
}

pub struct ItemRow {
    pub id: String,
    pub item_type: String,
    pub encrypted_payload: Vec<u8>,
    pub nonce: Vec<u8>,
    pub crypto_version: i64,
    pub folder_id: Option<String>,
    pub is_favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

pub struct FolderRow {
    pub id: String,
    pub encrypted_name: Vec<u8>,
    pub nonce: Vec<u8>,
    pub crypto_version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub const MASTER_ENVELOPE: &str = "master";
/// Vault Key sealed with a key derived from a Windows Hello signature.
pub const HELLO_ENVELOPE: &str = "hello";

// ---- vault_meta -------------------------------------------------------------

pub fn insert_meta(conn: &Connection, meta: &VaultMetaRow) -> AppResult<()> {
    conn.execute(
        "INSERT INTO vault_meta (id, vault_id, format_version, kdf_algorithm, kdf_memory_kib,
             kdf_iterations, kdf_parallelism, kdf_salt, created_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            meta.vault_id,
            meta.format_version,
            meta.kdf_algorithm,
            meta.kdf.memory_kib,
            meta.kdf.iterations,
            meta.kdf.parallelism,
            meta.salt,
            meta.created_at
        ],
    )?;
    Ok(())
}

pub fn load_meta(conn: &Connection) -> AppResult<Option<VaultMetaRow>> {
    Ok(conn
        .query_row(
            "SELECT vault_id, format_version, kdf_algorithm, kdf_memory_kib, kdf_iterations,
                    kdf_parallelism, kdf_salt, created_at
             FROM vault_meta WHERE id = 1",
            [],
            |row| {
                Ok(VaultMetaRow {
                    vault_id: row.get(0)?,
                    format_version: row.get(1)?,
                    kdf_algorithm: row.get(2)?,
                    kdf: KdfParams {
                        memory_kib: row.get(3)?,
                        iterations: row.get(4)?,
                        parallelism: row.get(5)?,
                    },
                    salt: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()?)
}

pub fn update_kdf(conn: &Connection, kdf: KdfParams, salt: &[u8]) -> AppResult<()> {
    conn.execute(
        "UPDATE vault_meta SET kdf_memory_kib = ?1, kdf_iterations = ?2, kdf_parallelism = ?3,
             kdf_salt = ?4 WHERE id = 1",
        params![kdf.memory_kib, kdf.iterations, kdf.parallelism, salt],
    )?;
    Ok(())
}

// ---- key_envelopes ------------------------------------------------------------

pub fn upsert_envelope(conn: &Connection, envelope: &EnvelopeRow, now: i64) -> AppResult<()> {
    upsert_envelope_as(conn, MASTER_ENVELOPE, envelope, now)
}

pub fn upsert_envelope_as(
    conn: &Connection,
    id: &str,
    envelope: &EnvelopeRow,
    now: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO key_envelopes (id, wrapped_key, nonce, crypto_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT (id) DO UPDATE SET wrapped_key = excluded.wrapped_key,
             nonce = excluded.nonce, crypto_version = excluded.crypto_version,
             updated_at = excluded.updated_at",
        params![
            id,
            envelope.wrapped_key,
            envelope.nonce,
            envelope.crypto_version,
            now
        ],
    )?;
    Ok(())
}

pub fn load_envelope(conn: &Connection) -> AppResult<Option<EnvelopeRow>> {
    load_envelope_as(conn, MASTER_ENVELOPE)
}

pub fn delete_envelope(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM key_envelopes WHERE id = ?1", [id])?;
    Ok(())
}

pub fn load_envelope_as(conn: &Connection, id: &str) -> AppResult<Option<EnvelopeRow>> {
    Ok(conn
        .query_row(
            "SELECT wrapped_key, nonce, crypto_version FROM key_envelopes WHERE id = ?1",
            [id],
            |row| {
                Ok(EnvelopeRow {
                    wrapped_key: row.get(0)?,
                    nonce: row.get(1)?,
                    crypto_version: row.get(2)?,
                })
            },
        )
        .optional()?)
}

// ---- items ----------------------------------------------------------------------

const ITEM_COLUMNS: &str = "id, item_type, encrypted_payload, nonce, crypto_version, folder_id,
    is_favorite, created_at, updated_at, deleted_at";

fn item_from_row(row: &Row<'_>) -> rusqlite::Result<ItemRow> {
    Ok(ItemRow {
        id: row.get(0)?,
        item_type: row.get(1)?,
        encrypted_payload: row.get(2)?,
        nonce: row.get(3)?,
        crypto_version: row.get(4)?,
        folder_id: row.get(5)?,
        is_favorite: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        deleted_at: row.get(9)?,
    })
}

pub fn insert_item(conn: &Connection, item: &ItemRow) -> AppResult<()> {
    conn.execute(
        &format!(
            "INSERT INTO items ({ITEM_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        ),
        params![
            item.id,
            item.item_type,
            item.encrypted_payload,
            item.nonce,
            item.crypto_version,
            item.folder_id,
            item.is_favorite,
            item.created_at,
            item.updated_at,
            item.deleted_at
        ],
    )?;
    Ok(())
}

pub fn load_items(conn: &Connection) -> AppResult<Vec<ItemRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {ITEM_COLUMNS} FROM items"))?;
    let rows = stmt.query_map([], item_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Returns false when no row matched (the item does not exist).
pub fn update_item_payload(
    conn: &Connection,
    id: &str,
    encrypted_payload: &[u8],
    nonce: &[u8],
    crypto_version: i64,
    folder_id: Option<&str>,
    updated_at: i64,
) -> AppResult<bool> {
    let changed = conn.execute(
        "UPDATE items SET encrypted_payload = ?2, nonce = ?3, crypto_version = ?4,
             folder_id = ?5, updated_at = ?6 WHERE id = ?1",
        params![
            id,
            encrypted_payload,
            nonce,
            crypto_version,
            folder_id,
            updated_at
        ],
    )?;
    Ok(changed == 1)
}

pub fn set_item_favorite(conn: &Connection, id: &str, favorite: bool) -> AppResult<bool> {
    Ok(conn.execute(
        "UPDATE items SET is_favorite = ?2 WHERE id = ?1",
        params![id, favorite],
    )? == 1)
}

pub fn set_item_folder(
    conn: &Connection,
    id: &str,
    folder_id: Option<&str>,
    updated_at: i64,
) -> AppResult<bool> {
    Ok(conn.execute(
        "UPDATE items SET folder_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, folder_id, updated_at],
    )? == 1)
}

pub fn set_item_deleted(conn: &Connection, id: &str, deleted_at: Option<i64>) -> AppResult<bool> {
    Ok(conn.execute(
        "UPDATE items SET deleted_at = ?2 WHERE id = ?1",
        params![id, deleted_at],
    )? == 1)
}

pub fn delete_item(conn: &Connection, id: &str) -> AppResult<bool> {
    Ok(conn.execute("DELETE FROM items WHERE id = ?1", [id])? == 1)
}

pub fn delete_trashed_items(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute("DELETE FROM items WHERE deleted_at IS NOT NULL", [])?)
}

// ---- folders --------------------------------------------------------------------

pub fn insert_folder(conn: &Connection, folder: &FolderRow) -> AppResult<()> {
    conn.execute(
        "INSERT INTO folders (id, encrypted_name, nonce, crypto_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            folder.id,
            folder.encrypted_name,
            folder.nonce,
            folder.crypto_version,
            folder.created_at,
            folder.updated_at
        ],
    )?;
    Ok(())
}

pub fn update_folder_name(
    conn: &Connection,
    id: &str,
    encrypted_name: &[u8],
    nonce: &[u8],
    updated_at: i64,
) -> AppResult<bool> {
    Ok(conn.execute(
        "UPDATE folders SET encrypted_name = ?2, nonce = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, encrypted_name, nonce, updated_at],
    )? == 1)
}

pub fn delete_folder(conn: &Connection, id: &str) -> AppResult<bool> {
    // items.folder_id is ON DELETE SET NULL, so contained items move to "no folder"
    Ok(conn.execute("DELETE FROM folders WHERE id = ?1", [id])? == 1)
}

pub fn load_folders(conn: &Connection) -> AppResult<Vec<FolderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, encrypted_name, nonce, crypto_version, created_at, updated_at FROM folders",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(FolderRow {
            id: row.get(0)?,
            encrypted_name: row.get(1)?,
            nonce: row.get(2)?,
            crypto_version: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// ---- settings -------------------------------------------------------------------

pub fn load_settings(conn: &Connection) -> AppResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
