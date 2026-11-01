//! Several vault files: list, create, open, switch, rename, forget, and the
//! "forgot master password" reset.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::commands::{SharedState, blocking};
use crate::core::error::{AppError, AppResult};
use crate::core::registry::{self, VAULT_EXTENSION};
use crate::core::state::VaultStatus;
use crate::database::Database;
use crate::platform::lock::on_vault_locked;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultListEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_opened_at: Option<i64>,
    pub exists: bool,
    pub active: bool,
}

#[tauri::command]
pub async fn list_vaults(state: State<'_, SharedState>) -> AppResult<Vec<VaultListEntry>> {
    let inner = state.lock();
    Ok(inner
        .registry
        .vaults
        .iter()
        .map(|v| VaultListEntry {
            id: v.id.clone(),
            name: v.name.clone(),
            path: v.path.to_string_lossy().into_owned(),
            last_opened_at: v.last_opened_at,
            exists: v.path.exists(),
            active: v.id == inner.registry.active,
        })
        .collect())
}

/// Switches with the side effects of a lock (clipboard wipe, UI reset).
fn switch(app: &AppHandle, state: &SharedState, id: &str) -> AppResult<VaultStatus> {
    let status = {
        let mut inner = state.lock();
        inner.switch_to(id)?;
        inner.status()
    };
    on_vault_locked(app);
    Ok(status)
}

#[tauri::command]
pub async fn switch_vault(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
) -> AppResult<VaultStatus> {
    if state.lock().registry.active == id {
        return Ok(state.lock().status());
    }
    switch(&app, &state, &id)
}

/// Registers a new, not yet created vault and makes it active: the UI then shows the
/// onboarding screen for it. With `choose_folder` the user picks where the file goes.
/// Returns `None` if the folder dialog was cancelled.
#[tauri::command]
pub async fn add_new_vault(
    app: AppHandle,
    state: State<'_, SharedState>,
    name: String,
    choose_folder: bool,
) -> AppResult<Option<VaultStatus>> {
    let name = registry::validate_name(&name)?;
    let folder: PathBuf = if choose_folder {
        let dialog_app = app.clone();
        let picked = blocking(move || {
            Ok(dialog_app
                .dialog()
                .file()
                .set_title("Где хранить новое хранилище")
                .blocking_pick_folder())
        })
        .await?;
        match picked.and_then(|p| p.into_path().ok()) {
            Some(folder) => folder,
            None => return Ok(None),
        }
    } else {
        state.lock().registry.default_folder()
    };
    std::fs::create_dir_all(&folder)?;

    let id = {
        let mut inner = state.lock();
        let path = inner.registry.new_vault_path(&folder, &name);
        let id = inner.registry.add(name, path)?;
        inner.registry.save()?;
        id
    };
    switch(&app, &state, &id).map(Some)
}

/// Lets the user pick an existing vault file, registers it and makes it active.
#[tauri::command]
pub async fn open_vault_file(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> AppResult<Option<VaultStatus>> {
    let dialog_app = app.clone();
    let picked = blocking(move || {
        Ok(dialog_app
            .dialog()
            .file()
            .set_title("Открыть хранилище")
            .add_filter("Хранилище Nexus Vault", &[VAULT_EXTENSION, "db"])
            .blocking_pick_file())
    })
    .await?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };

    let existing = state
        .lock()
        .registry
        .find_by_path(&path)
        .map(|v| v.id.clone());
    let id = match existing {
        Some(id) => id,
        None => {
            if !Database::is_vault_file(&path) {
                return Err(AppError::Validation("not_a_vault"));
            }
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Хранилище".into());
            let name: String = name.chars().take(registry::MAX_VAULT_NAME).collect();
            let mut inner = state.lock();
            let id = inner.registry.add(name, path)?;
            inner.registry.save()?;
            id
        }
    };
    switch(&app, &state, &id).map(Some)
}

#[tauri::command]
pub async fn rename_vault(
    state: State<'_, SharedState>,
    id: String,
    name: String,
) -> AppResult<VaultStatus> {
    let name = registry::validate_name(&name)?;
    let mut inner = state.lock();
    inner.registry.rename(&id, name)?;
    inner.registry.save()?;
    Ok(inner.status())
}

/// Removes a vault from the list. The file stays on disk.
#[tauri::command]
pub async fn forget_vault(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
) -> AppResult<VaultStatus> {
    let was_active = state.lock().registry.active == id;
    if was_active {
        let other = state
            .lock()
            .registry
            .vaults
            .iter()
            .find(|v| v.id != id)
            .map(|v| v.id.clone())
            .ok_or(AppError::Validation("last_vault"))?;
        switch(&app, &state, &other)?;
    }
    let mut inner = state.lock();
    inner.registry.forget(&id)?;
    inner.registry.save()?;
    Ok(inner.status())
}

/// Opens Explorer with the vault file selected (or its folder if not created yet).
#[tauri::command]
pub async fn reveal_vault(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
) -> AppResult<()> {
    let path = state
        .lock()
        .registry
        .get(&id)
        .map(|v| v.path.clone())
        .ok_or(AppError::NotFound)?;
    let result = if path.exists() {
        app.opener().reveal_item_in_dir(&path)
    } else {
        let folder = path.parent().ok_or(AppError::NotFound)?;
        app.opener()
            .open_path(folder.to_string_lossy(), None::<&str>)
    };
    result.map_err(|_| AppError::NotFound)
}

/// "Forgot the master password": the locked vault file is renamed (kept next to the
/// original) and the vault can be created anew. Returns the archived file name.
#[tauri::command]
pub async fn reset_forgotten_vault(state: State<'_, SharedState>) -> AppResult<String> {
    state.lock().reset_forgotten()
}
