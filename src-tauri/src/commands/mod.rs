//! IPC surface exposed to the WebView. Every command the frontend may call lives here;
//! commands stay thin and delegate to `core` / `crypto` / `database`.
//!
//! Vault commands are `async` so they run off the main thread: Argon2id takes most of
//! a second, and synchronous commands would freeze the window meanwhile.

pub mod app;
pub mod backup;
pub mod folders;
pub mod generator;
pub mod items;
pub mod settings;
pub mod vault;
pub mod vaults;

use std::sync::Arc;

use crate::core::error::{AppError, AppResult};
use crate::core::state::AppState;
use crate::core::vault::UnlockedVault;
use crate::database::Database;

pub type SharedState = Arc<AppState>;

/// Runs `f` against the unlocked vault and counts the call as user activity.
pub(crate) fn with_unlocked<T>(
    state: &AppState,
    f: impl FnOnce(&mut Database, &mut UnlockedVault) -> AppResult<T>,
) -> AppResult<T> {
    let mut inner = state.lock();
    inner.touch();
    let (db, vault) = inner.unlocked()?;
    f(db, vault)
}

/// Runs blocking work (Argon2id) on the blocking thread pool.
pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| AppError::Internal)?
}
