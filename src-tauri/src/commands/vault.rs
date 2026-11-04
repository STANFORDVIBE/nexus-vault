use secrecy::{ExposeSecret, SecretString};
use tauri::{AppHandle, State};

use crate::commands::{SharedState, blocking};
use crate::core::error::AppResult;
use crate::core::state::VaultStatus;
use crate::core::vault;
use crate::platform::lock::on_vault_locked;

#[tauri::command]
pub async fn get_vault_status(state: State<'_, SharedState>) -> AppResult<VaultStatus> {
    Ok(state.lock().status())
}

#[tauri::command]
pub async fn create_vault(
    state: State<'_, SharedState>,
    password: SecretString,
) -> AppResult<VaultStatus> {
    let state = state.inner().clone();
    blocking(move || state.lock().create_vault(password.expose_secret())).await
}

#[tauri::command]
pub async fn unlock_vault(
    state: State<'_, SharedState>,
    password: SecretString,
) -> AppResult<VaultStatus> {
    let state = state.inner().clone();
    blocking(move || state.lock().unlock(password.expose_secret())).await
}

#[tauri::command]
pub async fn lock_vault(app: AppHandle, state: State<'_, SharedState>) -> AppResult<VaultStatus> {
    let status = {
        let mut inner = state.lock();
        inner.lock_vault();
        inner.status()
    };
    on_vault_locked(&app);
    Ok(status)
}

#[tauri::command]
pub async fn change_master_password(
    state: State<'_, SharedState>,
    current: SecretString,
    new: SecretString,
) -> AppResult<()> {
    let state = state.inner().clone();
    blocking(move || {
        // Argon2id runs twice here: keep the app lock only for the quick reads and writes
        let (check, params) = {
            let mut inner = state.lock();
            inner.touch();
            let params = inner.kdf_params;
            let (db, _) = inner.unlocked()?;
            (vault::password_check(db)?, params)
        };
        let (key, envelope) =
            check.prepare_change(current.expose_secret(), new.expose_secret(), params)?;
        let mut inner = state.lock();
        let (db, unlocked) = inner.unlocked()?;
        vault::apply_password_change(db, unlocked, &key, envelope)
    })
    .await
}

/// Called by the UI on user input (throttled) to postpone auto-lock.
#[tauri::command]
pub async fn report_activity(state: State<'_, SharedState>) -> AppResult<()> {
    state.lock().touch();
    Ok(())
}
