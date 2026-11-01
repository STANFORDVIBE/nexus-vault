//! Process-wide application state shared by IPC commands and background tasks.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use serde::Serialize;

use crate::core::error::{AppError, AppResult};
use crate::core::settings::Settings;
use crate::core::vault::{self, UnlockedVault};
use crate::crypto::kdf::KdfParams;
use crate::database::Database;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VaultState {
    Uninitialized,
    Locked,
    Unlocked,
    /// The vault file exists but cannot be opened (damaged or not a vault).
    Corrupted,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub state: VaultState,
    pub corrupted_items: usize,
}

pub struct AppState {
    inner: Mutex<Inner>,
}

pub struct Inner {
    pub db_path: PathBuf,
    db: Option<Database>,
    pub vault: Option<UnlockedVault>,
    pub settings: Settings,
    pub last_activity: Instant,
    pub kdf_params: KdfParams,
    /// Backup file chosen in the restore dialog, waiting for its password.
    pub pending_restore: Option<PathBuf>,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Self {
        let mut inner = Inner {
            db_path,
            db: None,
            vault: None,
            settings: Settings::default(),
            last_activity: Instant::now(),
            kdf_params: KdfParams::DEFAULT,
            pending_restore: None,
        };
        if let Ok(db) = inner.database() {
            inner.settings = Settings::load(db);
        }
        AppState {
            inner: Mutex::new(inner),
        }
    }

    #[cfg(test)]
    pub fn for_tests(db_path: PathBuf) -> Self {
        let state = AppState::new(db_path);
        state.lock().kdf_params = KdfParams::TEST;
        state
    }

    /// The mutex only guards in-memory data; a panic elsewhere must not brick the app.
    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Inner {
    /// Opens the vault database if the file exists. The file is created only by
    /// `create_vault`, so a fresh install leaves nothing on disk until then.
    pub fn database(&mut self) -> AppResult<&mut Database> {
        if self.db.is_none() {
            if !self.db_path.exists() {
                return Err(AppError::VaultNotInitialized);
            }
            self.db = Some(Database::open(&self.db_path)?);
        }
        self.db.as_mut().ok_or(AppError::Internal)
    }

    /// Closes the database handle (needed before replacing the file on restore).
    pub fn close_database(&mut self) {
        self.vault = None;
        if let Some(db) = self.db.take() {
            let _ = db.checkpoint();
        }
    }

    /// Reloads settings after the database file was replaced (restore).
    pub fn reload(&mut self) {
        self.settings = self
            .database()
            .map(|db| Settings::load(db))
            .unwrap_or_default();
    }

    pub fn status(&mut self) -> VaultStatus {
        let state = if self.vault.is_some() {
            VaultState::Unlocked
        } else {
            match self.database().and_then(|db| vault::vault_exists(db)) {
                Ok(true) => VaultState::Locked,
                Ok(false) | Err(AppError::VaultNotInitialized) => VaultState::Uninitialized,
                Err(_) => VaultState::Corrupted,
            }
        };
        VaultStatus {
            state,
            corrupted_items: self.vault.as_ref().map_or(0, |v| v.corrupted_items),
        }
    }

    pub fn create_vault(&mut self, password: &str) -> AppResult<VaultStatus> {
        vault::validate_new_master_password(password)?;
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if self.db.is_none() {
            self.db = Some(Database::open(&self.db_path)?);
        }
        let params = self.kdf_params;
        let settings = self.settings.clone();
        let db = self.database()?;
        let unlocked = vault::create(db, password, params)?;
        settings.save(db)?;
        self.vault = Some(unlocked);
        self.touch();
        Ok(self.status())
    }

    pub fn unlock(&mut self, password: &str) -> AppResult<VaultStatus> {
        if self.vault.is_some() {
            return Ok(self.status());
        }
        let db = self.database()?;
        if !db.integrity_check()? {
            return Err(AppError::Integrity);
        }
        let unlocked = vault::unlock(db, password)?;
        self.settings = Settings::load(db);
        self.vault = Some(unlocked);
        self.touch();
        Ok(self.status())
    }

    /// Drops the unlocked vault, wiping keys and decrypted data. Returns whether
    /// anything was actually locked.
    pub fn lock_vault(&mut self) -> bool {
        self.vault.take().is_some()
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Database and unlocked vault together, or `VaultLocked`.
    pub fn unlocked(&mut self) -> AppResult<(&mut Database, &mut UnlockedVault)> {
        let vault = self.vault.as_mut().ok_or(AppError::VaultLocked)?;
        let db = self.db.as_mut().ok_or(AppError::VaultLocked)?;
        Ok((db, vault))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("vault.db");
        let state = AppState::for_tests(path.clone());

        {
            let mut inner = state.lock();
            assert_eq!(inner.status().state, VaultState::Uninitialized);
            assert!(!path.exists(), "no file before the vault is created");
            assert!(inner.create_vault("short").is_err());
            assert!(!path.exists());

            inner.create_vault("correct horse battery").unwrap();
            assert_eq!(inner.status().state, VaultState::Unlocked);
            assert!(inner.lock_vault());
            assert_eq!(inner.status().state, VaultState::Locked);
            assert!(matches!(inner.unlocked(), Err(AppError::VaultLocked)));
            assert!(matches!(
                inner.unlock("wrong password!"),
                Err(AppError::InvalidPassword)
            ));
            inner.unlock("correct horse battery").unwrap();
        }

        // A fresh process sees a locked vault
        let reopened = AppState::for_tests(path);
        assert_eq!(reopened.lock().status().state, VaultState::Locked);
    }

    #[test]
    fn corrupted_file_is_reported_not_trusted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        std::fs::write(&path, b"this is not a sqlite database at all").unwrap();
        let state = AppState::for_tests(path);
        let mut inner = state.lock();
        assert_eq!(inner.status().state, VaultState::Corrupted);
        assert!(inner.unlock("correct horse battery").is_err());
        // creating a new vault must not silently overwrite the damaged file
        assert!(inner.create_vault("correct horse battery").is_err());
    }
}
