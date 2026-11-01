//! Non-secret preferences, stored in plaintext in the `settings` table so they are
//! available before unlock (close behaviour, auto-lock).

use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::database::Database;
use crate::database::repo;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    /// Lock the vault and keep running in the tray (safest default).
    LockToTray,
    /// Hide to the tray, vault stays unlocked until auto-lock.
    MinimizeToTray,
    /// Quit the application.
    Exit,
}

// `default`: settings saved by an older version (without newer fields) still load.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Minutes of inactivity before auto-lock; 0 disables it.
    pub auto_lock_minutes: u32,
    /// Seconds before a copied secret is cleared from the clipboard; 0 disables it.
    pub clipboard_clear_seconds: u32,
    pub lock_on_system_lock: bool,
    pub close_behavior: CloseBehavior,
    pub launch_at_startup: bool,
    /// Encrypted `.nexus` copies written automatically while the vault is unlocked.
    pub auto_backup: bool,
    pub auto_backup_interval_hours: u32,
    /// How many automatic copies to keep; older ones are deleted.
    pub auto_backup_keep: u32,
    /// Custom folder; `None` = `<data dir>/backups`.
    pub auto_backup_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_lock_minutes: 5,
            clipboard_clear_seconds: 30,
            lock_on_system_lock: true,
            close_behavior: CloseBehavior::LockToTray,
            launch_at_startup: false,
            auto_backup: true,
            auto_backup_interval_hours: 24,
            auto_backup_keep: 10,
            auto_backup_dir: None,
        }
    }
}

const KEY: &str = "app_settings";

impl Settings {
    pub fn validate(&self) -> AppResult<()> {
        if self.auto_lock_minutes > 24 * 60 {
            return Err(AppError::Validation("auto_lock_minutes"));
        }
        if self.clipboard_clear_seconds > 10 * 60 {
            return Err(AppError::Validation("clipboard_clear_seconds"));
        }
        if !(1..=24 * 30).contains(&self.auto_backup_interval_hours) {
            return Err(AppError::Validation("auto_backup_interval_hours"));
        }
        if !(1..=100).contains(&self.auto_backup_keep) {
            return Err(AppError::Validation("auto_backup_keep"));
        }
        if self
            .auto_backup_dir
            .as_deref()
            .is_some_and(|d| d.is_empty() || d.len() > 1024)
        {
            return Err(AppError::Validation("auto_backup_dir"));
        }
        Ok(())
    }

    /// Loads stored settings; unknown or broken values fall back to defaults.
    pub fn load(db: &Database) -> Settings {
        repo::load_settings(db.conn())
            .ok()
            .and_then(|rows| rows.into_iter().find(|(k, _)| k == KEY))
            .and_then(|(_, value)| serde_json::from_str(&value).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, db: &mut Database) -> AppResult<()> {
        self.validate()?;
        let json = serde_json::to_string(self).map_err(|_| AppError::Internal)?;
        db.transaction(|tx| repo::set_setting(tx, KEY, &json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_then_roundtrip() {
        let mut db = Database::open_in_memory().unwrap();
        assert_eq!(Settings::load(&db), Settings::default());

        let custom = Settings {
            auto_lock_minutes: 15,
            close_behavior: CloseBehavior::Exit,
            ..Settings::default()
        };
        custom.save(&mut db).unwrap();
        assert_eq!(Settings::load(&db), custom);
    }

    #[test]
    fn settings_from_an_older_version_keep_their_values() {
        let mut db = Database::open_in_memory().unwrap();
        let old = r#"{"autoLockMinutes":15,"clipboardClearSeconds":30,"lockOnSystemLock":true,"closeBehavior":"exit","launchAtStartup":false}"#;
        db.transaction(|tx| repo::set_setting(tx, KEY, old))
            .unwrap();
        let loaded = Settings::load(&db);
        assert_eq!(loaded.auto_lock_minutes, 15);
        assert_eq!(loaded.close_behavior, CloseBehavior::Exit);
        assert!(loaded.auto_backup);
    }

    #[test]
    fn rejects_out_of_range_values() {
        let bad = Settings {
            clipboard_clear_seconds: 100_000,
            ..Settings::default()
        };
        assert!(bad.validate().is_err());
    }
}
