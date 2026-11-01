//! App updates from GitHub Releases. The only network access of the app: it fetches a
//! small `latest.json` and, on request, the signed installer. Nothing from the vault
//! is ever sent. The installer is verified against the public key in `tauri.conf.json`.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::commands::SharedState;
use crate::core::error::{AppError, AppResult};
use crate::platform::events;

/// First automatic check after start, then once a day while the app runs.
const FIRST_CHECK: Duration = Duration::from_secs(15);
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

async fn find_update(app: &AppHandle) -> AppResult<Option<tauri_plugin_updater::Update>> {
    let updater = app.updater().map_err(|_| AppError::UpdateFailed)?;
    updater.check().await.map_err(|_| AppError::UpdateFailed)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    Ok(find_update(&app).await?.map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
    }))
}

/// Downloads, verifies and runs the installer; the app exits while it installs and
/// the installer starts it again.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    let update = find_update(&app).await?.ok_or(AppError::NotFound)?;
    // lock first: the vault must not stay decrypted while the installer replaces the app
    app.state::<SharedState>().lock().lock_vault();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|_| AppError::UpdateFailed)?;
    app.restart();
}

/// Background check, if the user keeps it on in settings.
pub fn start_auto_check(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            let enabled = app.state::<SharedState>().lock().settings.check_updates;
            if enabled
                && let Ok(Some(info)) = tauri::async_runtime::block_on(check_update(app.clone()))
            {
                let _ = app.emit(events::UPDATE_AVAILABLE, info);
            }
            std::thread::sleep(CHECK_EVERY);
        }
    });
}
