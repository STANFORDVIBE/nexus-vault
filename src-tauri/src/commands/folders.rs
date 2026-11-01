use tauri::State;

use crate::commands::{SharedState, with_unlocked};
use crate::core::error::AppResult;
use crate::core::folders::FolderView;

#[tauri::command]
pub async fn list_folders(state: State<'_, SharedState>) -> AppResult<Vec<FolderView>> {
    with_unlocked(&state, |_, vault| Ok(vault.list_folders()))
}

#[tauri::command]
pub async fn create_folder(state: State<'_, SharedState>, name: String) -> AppResult<String> {
    with_unlocked(&state, |db, vault| vault.create_folder(db, &name))
}

#[tauri::command]
pub async fn rename_folder(
    state: State<'_, SharedState>,
    id: String,
    name: String,
) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.rename_folder(db, &id, &name))
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, SharedState>, id: String) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.delete_folder(db, &id))
}
