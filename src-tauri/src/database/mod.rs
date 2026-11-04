//! SQLite storage. Rows hold ciphertext for anything sensitive; this layer never sees
//! plaintext secrets, only opaque blobs plus non-sensitive flags and timestamps.

mod migrations;
pub mod repo;

use std::path::Path;

use rusqlite::Connection;

use crate::core::error::AppResult;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens (or creates) the database file and brings the schema up to date.
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// Checks, without modifying the file, that `path` is a Nexus Vault database.
    /// Used before registering a file the user picked, so a random SQLite file is
    /// never migrated into a "vault".
    pub fn is_vault_file(path: &Path) -> bool {
        use rusqlite::OpenFlags;
        let Ok(conn) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
            return false;
        };
        conn.query_row("SELECT COUNT(*) FROM vault_meta WHERE id = 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .is_ok_and(|n| n == 1)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        // secure_delete overwrites freed pages so deleted ciphertexts don't linger on disk;
        // FULL sync trades a little speed for durability of every committed change.
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA secure_delete = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;",
        )?;
        let mut db = Database { conn };
        migrations::apply(&mut db.conn)?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Runs `f` inside a transaction: either every change commits or none does.
    pub fn transaction<T>(
        &mut self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        let tx = self.conn.transaction()?;
        let value = f(&tx)?;
        tx.commit()?;
        Ok(value)
    }

    /// Flushes the WAL into the main file so it can be safely moved or copied.
    pub fn checkpoint(&self) -> AppResult<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }

    /// Verifies the file is a healthy SQLite database (used before trusting a vault).
    /// `quick_check` skips only index-vs-table cross checks, so it stays fast on big
    /// vaults full of images; ciphertexts are authenticated by AEAD anyway.
    pub fn integrity_check(&self) -> AppResult<bool> {
        let result: String = self
            .conn
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        Ok(result == "ok")
    }
}
