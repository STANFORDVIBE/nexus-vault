//! Locking: manual, after inactivity, and when Windows itself is locked.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::SharedState;
use crate::platform::{clipboard, events};

/// Side effects shared by every way of locking: wipe our clipboard secret and tell
/// the UI to drop decrypted data and close sensitive dialogs.
pub fn on_vault_locked(app: &AppHandle) {
    clipboard::clear_if_ours(None);
    let _ = app.emit(events::VAULT_LOCKED, ());
}

/// Locks the vault if it is unlocked. Returns whether it was.
pub fn lock_now(app: &AppHandle) -> bool {
    let locked = app.state::<SharedState>().lock().lock_vault();
    if locked {
        on_vault_locked(app);
    }
    locked
}

/// Background watcher that enforces the auto-lock timeout.
pub fn start_auto_lock(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(5));
            let expired = {
                let state = app.state::<SharedState>();
                let inner = state.lock();
                let minutes = inner.settings.auto_lock_minutes;
                inner.vault.is_some()
                    && minutes > 0
                    && inner.last_activity.elapsed() >= Duration::from_secs(u64::from(minutes) * 60)
            };
            if expired {
                lock_now(&app);
            }
        }
    });
}

/// Background writer of automatic backups: checks once a minute while unlocked.
pub fn start_auto_backup(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let result = {
                let state = app.state::<SharedState>();
                let mut inner = state.lock();
                if inner.vault.is_none() {
                    continue;
                }
                inner.run_auto_backup(false)
            };
            if result.is_err() {
                let _ = app.emit(events::AUTO_BACKUP_FAILED, ());
            }
        }
    });
}

/// Locks the vault when the Windows session is locked (Win+L, lid close, ...).
pub fn watch_system_lock(app: &AppHandle) {
    #[cfg(windows)]
    crate::platform::session::watch(app, |app| {
        let enabled = app
            .state::<SharedState>()
            .lock()
            .settings
            .lock_on_system_lock;
        if enabled {
            lock_now(app);
        }
    });
    #[cfg(not(windows))]
    let _ = app;
}
