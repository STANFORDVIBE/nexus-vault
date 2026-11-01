use secrecy::{ExposeSecret, SecretString};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::backup::{self, BackupInfo, EXTENSION, RestoreSummary};
use crate::commands::{SharedState, blocking};
use crate::core::error::{AppError, AppResult};
use crate::core::time::now_millis;
use crate::platform::lock::on_vault_locked;

const FILTER_NAME: &str = "Резервная копия Nexus Vault";

fn default_file_name() -> String {
    // yyyy-mm-dd from unix days (civil-from-days, Howard Hinnant)
    let days = now_millis().div_euclid(86_400_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("nexus-vault-{year:04}-{month:02}-{day:02}.{EXTENSION}")
}

/// Asks where to save, then writes an encrypted backup. Returns the file name,
/// or `None` if the user cancelled the dialog.
#[tauri::command]
pub async fn create_backup(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> AppResult<Option<String>> {
    // fail fast if locked, before showing a dialog
    state.lock().unlocked()?;

    let picked = blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title("Сохранить резервную копию")
            .add_filter(FILTER_NAME, &[EXTENSION])
            .set_file_name(default_file_name())
            .blocking_save_file())
    })
    .await?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };

    let state = state.inner().clone();
    blocking(move || {
        let mut inner = state.lock();
        inner.touch();
        let (db, vault) = inner.unlocked()?;
        backup::write_backup(&path, db, &vault.key)?;
        Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()))
    })
    .await
}

/// Lets the user choose a `.nexus` file and returns its basic info.
#[tauri::command]
pub async fn pick_backup_file(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> AppResult<Option<BackupInfo>> {
    let picked = blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title("Выберите резервную копию")
            .add_filter(FILTER_NAME, &[EXTENSION])
            .blocking_pick_file())
    })
    .await?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    let info = backup::inspect(&path)?;
    state.lock().pending_restore = Some(path);
    Ok(Some(info))
}

/// Verifies the chosen backup with its master password and atomically replaces the
/// current vault with it. The replaced vault is kept as `vault.previous.db`.
#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    state: State<'_, SharedState>,
    password: SecretString,
) -> AppResult<RestoreSummary> {
    let state = state.inner().clone();
    let summary = blocking(move || {
        let (path, live) = {
            let inner = state.lock();
            let path = inner.pending_restore.clone().ok_or(AppError::Cancelled)?;
            (path, inner.db_path.clone())
        };

        // 1. verify everything (slow: Argon2id + decrypt every row), outside the lock
        let bytes = backup::read_file(&path)?;
        let verified = backup::verify(&bytes, password.expose_secret())?;
        let summary = verified.summary();

        // 2. write a complete new database next to the live one
        if let Some(dir) = live.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let staging = live.with_file_name("vault.restore.db");
        let previous = live.with_file_name("vault.previous.db");
        backup::write_database(&staging, &verified)?;

        // 3. switch: close the live vault, swap files, reload
        let mut inner = state.lock();
        inner.close_database();
        let swapped = backup::swap_in(&staging, &live, &previous);
        inner.pending_restore = None;
        inner.reload();
        swapped?;
        Ok(summary)
    })
    .await?;
    on_vault_locked(&app);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_name_looks_like_a_date() {
        let name = super::default_file_name();
        assert!(name.starts_with("nexus-vault-20"));
        assert!(name.ends_with(".nexus"));
        assert_eq!(name.len(), "nexus-vault-2026-11-01.nexus".len());
    }
}
