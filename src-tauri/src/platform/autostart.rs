use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::core::error::{AppError, AppResult};

/// Command-line flag used when Windows launches the app at sign-in: start in the tray.
pub const MINIMIZED_ARG: &str = "--minimized";

pub fn is_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

pub fn set_enabled(app: &AppHandle, enabled: bool) -> AppResult<()> {
    if enabled == is_enabled(app) {
        return Ok(());
    }
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|_| AppError::Internal)
}
