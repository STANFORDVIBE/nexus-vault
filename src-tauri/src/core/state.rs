//! Process-wide application state shared by IPC commands and background tasks.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use serde::Serialize;

use crate::core::error::{AppError, AppResult};
use crate::core::registry::Registry;
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

/// The vault the status refers to (name and file location are not secret).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveVault {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub state: VaultState,
    pub corrupted_items: usize,
    pub vault: ActiveVault,
    /// How many vaults are registered (the UI offers switching when > 1).
    pub vault_count: usize,
}

pub struct AppState {
    inner: Mutex<Inner>,
}

pub struct Inner {
    /// File of the active vault (`registry.active_entry().path`).
    pub db_path: PathBuf,
    pub registry: Registry,
    db: Option<Database>,
    pub vault: Option<UnlockedVault>,
    pub settings: Settings,
    pub last_activity: Instant,
    pub kdf_params: KdfParams,
    /// Backup file chosen in the restore dialog, waiting for its password.
    pub pending_restore: Option<PathBuf>,
    /// Wrong master passwords in a row, and when the next attempt is allowed.
    /// shortcut: kept in memory only, a restart resets it; persist it if that matters.
    failed_unlocks: u32,
    retry_at: Option<Instant>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let registry = Registry::load(&data_dir);
        let mut inner = Inner {
            db_path: registry.active_entry().path.clone(),
            registry,
            db: None,
            vault: None,
            settings: Settings::default(),
            last_activity: Instant::now(),
            kdf_params: KdfParams::DEFAULT,
            pending_restore: None,
            failed_unlocks: 0,
            retry_at: None,
        };
        if let Ok(db) = inner.database() {
            inner.settings = Settings::load(db);
        }
        AppState {
            inner: Mutex::new(inner),
        }
    }

    /// Test state whose default vault is `db_path` (its file name must be `vault.db`).
    #[cfg(test)]
    pub fn for_tests(db_path: PathBuf) -> Self {
        assert!(db_path.ends_with(crate::core::registry::DEFAULT_VAULT_FILE));
        let state = AppState::new(db_path.parent().unwrap().to_path_buf());
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
        let entry = self.registry.active_entry();
        VaultStatus {
            state,
            corrupted_items: self.vault.as_ref().map_or(0, |v| v.corrupted_items),
            vault: ActiveVault {
                id: entry.id.clone(),
                name: entry.name.clone(),
                path: entry.path.to_string_lossy().into_owned(),
            },
            vault_count: self.registry.vaults.len(),
        }
    }

    /// Folder for automatic copies of the active vault.
    pub fn auto_backup_folder(&self) -> PathBuf {
        self.settings
            .auto_backup_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.registry.backups_folder())
    }

    pub fn auto_backup_prefix(&self) -> String {
        crate::backup::auto::file_prefix(&self.db_path, &self.registry.active)
    }

    /// Writes an automatic copy if one is due (or always with `force`).
    /// Returns the written file, or `None` when nothing was due.
    pub fn run_auto_backup(&mut self, force: bool) -> AppResult<Option<PathBuf>> {
        use crate::backup::auto;
        if !force && !self.settings.auto_backup {
            return Ok(None);
        }
        let folder = self.auto_backup_folder();
        let prefix = self.auto_backup_prefix();
        let keep = self.settings.auto_backup_keep as usize;
        let interval = self.settings.auto_backup_interval_hours;
        let now = crate::core::time::now_millis();
        let vault = self.vault.as_ref().ok_or(AppError::VaultLocked)?;
        let db = self.db.as_mut().ok_or(AppError::VaultLocked)?;
        if !force && !auto::is_due(auto::last_checked_at(db), interval, now) {
            return Ok(None);
        }
        auto::run(&folder, &prefix, keep, db, &vault.key, now, force)
    }

    /// Locks and closes the current vault, then makes `id` the active one.
    pub fn switch_to(&mut self, id: &str) -> AppResult<()> {
        self.registry.set_active(id)?;
        self.close_database();
        self.db_path = self.registry.active_entry().path.clone();
        self.pending_restore = None;
        self.reload();
        self.registry.save()
    }

    /// "Forgot the master password": moves the locked vault file aside (it is kept,
    /// not deleted) so a new vault can be created in its place. Returns the new name.
    pub fn reset_forgotten(&mut self) -> AppResult<String> {
        if self.vault.is_some() {
            return Err(AppError::Validation("vault_unlocked"));
        }
        if !self.db_path.exists() {
            return Err(AppError::VaultNotInitialized);
        }
        self.close_database();
        let archived = archive_path(&self.db_path);
        std::fs::rename(&self.db_path, &archived)?;
        for suffix in ["-wal", "-shm"] {
            let mut sidecar = self.db_path.clone().into_os_string();
            sidecar.push(suffix);
            let _ = std::fs::remove_file(sidecar);
        }
        self.settings = Settings::default();
        Ok(archived
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
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
        // persist the registry now: until it is saved, the default vault gets a new id on
        // every start, and automatic backups are named after that id
        self.registry.mark_opened();
        let _ = self.registry.save();
        Ok(self.status())
    }

    pub fn unlock(&mut self, password: &str) -> AppResult<VaultStatus> {
        if self.vault.is_some() {
            return Ok(self.status());
        }
        if let Some(retry_at) = self.retry_at {
            let wait = retry_at.saturating_duration_since(Instant::now());
            if !wait.is_zero() {
                return Err(AppError::TooManyAttempts(wait.as_secs_f64().ceil() as u64));
            }
        }
        let db = self.database()?;
        if !db.integrity_check()? {
            return Err(AppError::Integrity);
        }
        match vault::unlock(db, password) {
            Ok(unlocked) => self.finish_unlock(unlocked),
            Err(AppError::InvalidPassword) => {
                self.failed_unlocks += 1;
                self.retry_at = Some(Instant::now() + unlock_delay(self.failed_unlocks));
                Err(AppError::InvalidPassword)
            }
            Err(error) => Err(error),
        }
    }

    /// Unlock with a Windows Hello signature of the challenge read for `vault_id`.
    pub fn unlock_with_hello(
        &mut self,
        vault_id: &str,
        signature: &[u8],
    ) -> AppResult<VaultStatus> {
        if self.vault.is_some() {
            return Ok(self.status());
        }
        let db = self.database()?;
        if !db.integrity_check()? {
            return Err(AppError::Integrity);
        }
        // the user may have switched to another vault while the prompt was open
        if vault::hello_challenge(db)?.0 != vault_id {
            return Err(AppError::HelloFailed);
        }
        let unlocked = vault::unlock_with_hello(db, signature)?;
        self.finish_unlock(unlocked)
    }

    fn finish_unlock(&mut self, unlocked: UnlockedVault) -> AppResult<VaultStatus> {
        self.failed_unlocks = 0;
        self.retry_at = None;
        self.settings = Settings::load(self.database()?);
        if let Some(db) = self.db.as_mut() {
            // housekeeping only: a failure here must not keep the user out of the vault
            let _ = unlocked.prune_attachments(db);
        }
        self.vault = Some(unlocked);
        self.touch();
        self.registry.mark_opened();
        let _ = self.registry.save();
        Ok(self.status())
    }

    pub fn is_unlocked(&self) -> bool {
        self.vault.is_some()
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

/// Pause after `failures` wrong master passwords in a row: none for the first two typos,
/// then 2, 4, 8 ... seconds, at most a minute. Makes guessing through the app pointless.
fn unlock_delay(failures: u32) -> std::time::Duration {
    let secs = match failures {
        0..=2 => 0,
        n => (1u64 << (n - 2).min(6)).min(60),
    };
    std::time::Duration::from_secs(secs)
}

/// `vault.db` -> `vault.forgotten-<millis>.db`, next to the original.
fn archive_path(path: &std::path::Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vault".into());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    path.with_file_name(format!(
        "{stem}.forgotten-{}{ext}",
        crate::core::time::now_millis()
    ))
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
    fn wrong_passwords_in_a_row_slow_down_unlocking() {
        assert_eq!(unlock_delay(1).as_secs(), 0);
        assert_eq!(unlock_delay(2).as_secs(), 0);
        assert_eq!(unlock_delay(3).as_secs(), 2);
        assert_eq!(unlock_delay(4).as_secs(), 4);
        assert_eq!(unlock_delay(20).as_secs(), 60);

        let dir = tempfile::tempdir().unwrap();
        let state = AppState::for_tests(dir.path().join("vault.db"));
        let mut inner = state.lock();
        inner.create_vault("correct horse battery").unwrap();
        inner.lock_vault();
        for _ in 0..3 {
            assert!(matches!(
                inner.unlock("wrong password!"),
                Err(AppError::InvalidPassword)
            ));
        }
        // even the right password waits until the pause is over
        assert!(matches!(
            inner.unlock("correct horse battery"),
            Err(AppError::TooManyAttempts(2))
        ));
        inner.retry_at = None;
        inner.unlock("correct horse battery").unwrap();
        assert_eq!(inner.failed_unlocks, 0);
    }

    #[test]
    fn vault_id_is_stable_after_creation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let state = AppState::for_tests(path.clone());
        let id = {
            let mut inner = state.lock();
            inner.create_vault("correct horse battery").unwrap();
            inner.status().vault.id
        };
        // restarting before the first unlock must keep the id (auto backups use it)
        let reopened = AppState::for_tests(path);
        assert_eq!(reopened.lock().status().vault.id, id);
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

    #[test]
    fn switching_vaults_keeps_each_one_separate() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::for_tests(dir.path().join("vault.db"));
        let mut inner = state.lock();
        inner.create_vault("correct horse battery").unwrap();
        let first = inner.registry.active.clone();

        let path = inner
            .registry
            .new_vault_path(&inner.registry.default_folder(), "Работа");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let work = inner.registry.add("Работа".into(), path).unwrap();
        inner.switch_to(&work).unwrap();
        assert!(inner.vault.is_none(), "switching locks the previous vault");
        assert_eq!(inner.status().state, VaultState::Uninitialized);
        assert_eq!(inner.status().vault.name, "Работа");
        inner.create_vault("another long password").unwrap();

        inner.switch_to(&first).unwrap();
        assert_eq!(inner.status().state, VaultState::Locked);
        assert!(inner.unlock("another long password").is_err());
        inner.unlock("correct horse battery").unwrap();
        assert_eq!(inner.status().vault_count, 2);
    }

    #[test]
    fn forgotten_vault_is_moved_aside_not_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let state = AppState::for_tests(path.clone());
        let mut inner = state.lock();
        inner.create_vault("correct horse battery").unwrap();
        assert!(
            inner.reset_forgotten().is_err(),
            "only a locked vault can be reset"
        );
        inner.lock_vault();

        let archived = inner.reset_forgotten().unwrap();
        assert!(archived.starts_with("vault.forgotten-") && archived.ends_with(".db"));
        assert!(dir.path().join(&archived).exists());
        assert!(!path.exists());
        assert_eq!(inner.status().state, VaultState::Uninitialized);
        inner.create_vault("a brand new password").unwrap();
    }
}
