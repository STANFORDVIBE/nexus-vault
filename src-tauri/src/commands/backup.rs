use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::backup::{self, BackupInfo, EXTENSION, RestoreSummary, auto};
use crate::commands::{SharedState, blocking, verify_master_password};
use crate::core::error::{AppError, AppResult};
use crate::core::export::{self, ExportFormat};
use crate::core::import::{self, ImportSummary};
use crate::core::time::{date_stamp, now_millis};
use crate::platform::lock::on_vault_locked;

const FILTER_NAME: &str = "Резервная копия Nexus Vault";

fn default_file_name() -> String {
    format!("nexus-vault-{}.{EXTENSION}", date_stamp(now_millis()))
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
/// current vault with it. The replaced vault is kept as `<name>.previous-<time>.db`.
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
        let staging = backup::sibling(&live, "restore");
        let previous = backup::sibling(&live, &format!("previous-{}", now_millis()));
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackupStatus {
    pub last_at: Option<i64>,
    pub folder: String,
    pub count: usize,
}

/// Where automatic copies go, when the last one was made and how many exist.
#[tauri::command]
pub async fn get_auto_backup_status(state: State<'_, SharedState>) -> AppResult<AutoBackupStatus> {
    let mut inner = state.lock();
    let folder = inner.auto_backup_folder();
    let prefix = inner.auto_backup_prefix();
    let (db, _) = inner.unlocked()?;
    Ok(AutoBackupStatus {
        last_at: auto::last_backup_at(db),
        folder: folder.to_string_lossy().into_owned(),
        count: auto::list(&folder, &prefix).len(),
    })
}

/// "Make a copy now" in the automatic backups folder. Returns the file name.
#[tauri::command]
pub async fn run_auto_backup_now(state: State<'_, SharedState>) -> AppResult<String> {
    let state = state.inner().clone();
    blocking(move || {
        let mut inner = state.lock();
        inner.touch();
        let path = inner.run_auto_backup(true)?.ok_or(AppError::Internal)?;
        Ok(path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    })
    .await
}

/// Folder picker for automatic backups. Only returns the path; the UI saves it in
/// settings. `None` if cancelled.
#[tauri::command]
pub async fn choose_auto_backup_folder(app: AppHandle) -> AppResult<Option<String>> {
    let picked = blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title("Папка для автоматических копий")
            .blocking_pick_folder())
    })
    .await?;
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn open_auto_backup_folder(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> AppResult<()> {
    let folder = state.lock().auto_backup_folder();
    std::fs::create_dir_all(&folder)?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|_| AppError::NotFound)
}

/// Unencrypted export (CSV or JSON). Requires the master password again; returns the
/// file name, or `None` if the save dialog was cancelled.
#[tauri::command]
pub async fn export_vault(
    app: AppHandle,
    state: State<'_, SharedState>,
    password: SecretString,
    format: ExportFormat,
) -> AppResult<Option<String>> {
    // verify first (slow Argon2id), so a wrong password fails before any dialog
    let verify_state = state.inner().clone();
    blocking(move || verify_master_password(&verify_state, password.expose_secret())).await?;

    let picked = blocking(move || {
        let name = format!(
            "nexus-vault-export-{}.{}",
            date_stamp(now_millis()),
            format.extension()
        );
        Ok(app
            .dialog()
            .file()
            .set_title("Экспорт без шифрования")
            .add_filter(format.extension().to_uppercase(), &[format.extension()])
            .set_file_name(name)
            .blocking_save_file())
    })
    .await?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };

    let state = state.inner().clone();
    blocking(move || {
        let mut inner = state.lock();
        let (_, unlocked) = inner.unlocked()?;
        let text = export::render(unlocked, format, now_millis())?;
        // UTF-8 BOM so Excel shows Cyrillic correctly in the CSV
        let mut bytes = zeroize::Zeroizing::new(Vec::with_capacity(text.len() + 3));
        if format == ExportFormat::Csv {
            bytes.extend_from_slice(b"\xEF\xBB\xBF");
        }
        bytes.extend_from_slice(text.as_bytes());
        std::fs::write(&path, &*bytes)?;
        Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()))
    })
    .await
}

/// Adds entries from another password manager's export (CSV) or our own JSON export.
/// Returns what was imported, or `None` if the dialog was cancelled.
#[tauri::command]
pub async fn import_items(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> AppResult<Option<ImportSummary>> {
    // fail fast if locked, before showing a dialog
    state.lock().unlocked()?;

    let picked = blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title("Импорт из другого менеджера паролей")
            .add_filter("CSV или JSON", &["csv", "json"])
            .blocking_pick_file())
    })
    .await?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };

    let state = state.inner().clone();
    blocking(move || {
        if std::fs::metadata(&path)?.len() > import::MAX_IMPORT_BYTES {
            return Err(AppError::Validation("import_too_large"));
        }
        let bytes = zeroize::Zeroizing::new(std::fs::read(&path)?);
        let parsed = import::parse(&bytes)?;
        let mut inner = state.lock();
        inner.touch();
        let (db, vault) = inner.unlocked()?;
        vault.import(db, parsed).map(Some)
    })
    .await
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
