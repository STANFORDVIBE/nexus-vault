use tauri::{AppHandle, State};

use crate::commands::SharedState;
use crate::core::error::AppResult;
use crate::core::settings::Settings;
use crate::platform::autostart;

#[tauri::command]
pub async fn get_settings(app: AppHandle, state: State<'_, SharedState>) -> AppResult<Settings> {
    let mut settings = state.lock().settings.clone();
    // the OS registry is the source of truth for autostart
    settings.launch_at_startup = autostart::is_enabled(&app);
    Ok(settings)
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, SharedState>,
    settings: Settings,
) -> AppResult<Settings> {
    settings.validate()?;
    autostart::set_enabled(&app, settings.launch_at_startup)?;
    let mut inner = state.lock();
    inner.touch();
    // settings live in the vault database, so they can only be changed while unlocked
    let (db, _) = inner.unlocked()?;
    settings.save(db)?;
    inner.settings = settings.clone();
    Ok(settings)
}
