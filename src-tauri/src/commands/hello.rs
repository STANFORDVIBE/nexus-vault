//! Windows Hello unlock. The Hello prompt can stay open for a long time, so it is
//! always shown with the state mutex released.

use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use tauri::State;

use crate::commands::{SharedState, blocking};
use crate::core::error::{AppError, AppResult};
use crate::core::state::VaultStatus;
use crate::core::vault;
use crate::platform::hello;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloStatus {
    /// Hello is set up on this PC.
    pub available: bool,
    /// The active vault can be unlocked with Hello.
    pub enabled: bool,
}

#[tauri::command]
pub async fn get_hello_status(state: State<'_, SharedState>) -> AppResult<HelloStatus> {
    let enabled = {
        let mut inner = state.lock();
        inner
            .database()
            .and_then(|db| vault::hello_enabled(db))
            .unwrap_or(false)
    };
    let available = blocking(|| Ok(hello::is_available())).await?;
    Ok(HelloStatus { available, enabled })
}

#[tauri::command]
pub async fn enable_hello(state: State<'_, SharedState>, password: SecretString) -> AppResult<()> {
    let vault_id = {
        let mut inner = state.lock();
        let (db, unlocked) = inner.unlocked()?;
        vault::check_master_password(db, unlocked, password.expose_secret())?;
        unlocked.vault_id.clone()
    };
    let challenge = vault::new_hello_challenge()?;
    let id = vault_id.clone();
    let signature = blocking(move || {
        hello::create(&id)?;
        hello::sign(&id, &challenge)
    })
    .await?;

    let mut inner = state.lock();
    let (db, unlocked) = inner.unlocked()?;
    if unlocked.vault_id != vault_id {
        // locked or switched to another vault while the prompt was open
        return Err(AppError::VaultLocked);
    }
    vault::enable_hello(db, unlocked, &challenge, &signature)
}

#[tauri::command]
pub async fn disable_hello(state: State<'_, SharedState>) -> AppResult<()> {
    let vault_id = {
        let mut inner = state.lock();
        let (db, unlocked) = inner.unlocked()?;
        vault::disable_hello(db)?;
        unlocked.vault_id.clone()
    };
    blocking(move || {
        hello::delete(&vault_id);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn unlock_with_hello(state: State<'_, SharedState>) -> AppResult<VaultStatus> {
    let (vault_id, challenge) = {
        let mut inner = state.lock();
        if inner.is_unlocked() {
            return Ok(inner.status());
        }
        vault::hello_challenge(inner.database()?)?
    };
    let id = vault_id.clone();
    let signature = blocking(move || hello::sign(&id, &challenge)).await?;
    let state = state.inner().clone();
    blocking(move || state.lock().unlock_with_hello(&vault_id, &signature)).await
}
