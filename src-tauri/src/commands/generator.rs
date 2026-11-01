use tauri::AppHandle;

use crate::core::error::AppResult;
use crate::crypto::generator::{self, GeneratedPassword, GeneratorOptions};
use crate::platform::clipboard;

/// Works while locked too: generating a password reveals nothing about the vault.
#[tauri::command]
pub async fn generate_password(options: GeneratorOptions) -> AppResult<GeneratedPassword> {
    generator::generate(&options)
}

/// Copies a freshly generated value (it is not stored anywhere). Treated as a secret:
/// hidden from clipboard history and cleared after the configured timeout.
#[tauri::command]
pub async fn copy_generated(
    app: AppHandle,
    state: tauri::State<'_, crate::commands::SharedState>,
    value: String,
) -> AppResult<u32> {
    let clear_after = state.lock().settings.clipboard_clear_seconds;
    clipboard::copy(&app, value, true, clear_after)?;
    Ok(clear_after)
}
