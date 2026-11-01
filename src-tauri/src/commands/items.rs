use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::commands::{SharedState, with_unlocked};
use crate::core::error::{AppError, AppResult};
use crate::core::items::{CopyField, DashboardData, ItemDetail, ItemFilter, ItemSummary};
use crate::core::model::ItemPayload;
use crate::platform::clipboard;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInput {
    #[serde(flatten)]
    pub content: ItemPayload,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
}

#[tauri::command]
pub async fn list_items(
    state: State<'_, SharedState>,
    filter: ItemFilter,
) -> AppResult<Vec<ItemSummary>> {
    with_unlocked(&state, |_, vault| Ok(vault.list_items(&filter)))
}

#[tauri::command]
pub async fn get_item(state: State<'_, SharedState>, id: String) -> AppResult<ItemDetail> {
    with_unlocked(&state, |_, vault| vault.get_item(&id))
}

#[tauri::command]
pub async fn get_dashboard(state: State<'_, SharedState>) -> AppResult<DashboardData> {
    with_unlocked(&state, |_, vault| Ok(vault.dashboard()))
}

#[tauri::command]
pub async fn create_item(state: State<'_, SharedState>, input: ItemInput) -> AppResult<String> {
    with_unlocked(&state, |db, vault| {
        vault.create_item(db, input.content, input.folder_id, input.favorite)
    })
}

#[tauri::command]
pub async fn update_item(
    state: State<'_, SharedState>,
    id: String,
    input: ItemInput,
) -> AppResult<()> {
    with_unlocked(&state, |db, vault| {
        vault.update_item(db, &id, input.content, input.folder_id)
    })
}

#[tauri::command]
pub async fn set_favorite(
    state: State<'_, SharedState>,
    id: String,
    favorite: bool,
) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.set_favorite(db, &id, favorite))
}

#[tauri::command]
pub async fn move_item_to_folder(
    state: State<'_, SharedState>,
    id: String,
    folder_id: Option<String>,
) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.move_to_folder(db, &id, folder_id))
}

#[tauri::command]
pub async fn trash_item(state: State<'_, SharedState>, id: String) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.move_to_trash(db, &id))
}

#[tauri::command]
pub async fn restore_item(state: State<'_, SharedState>, id: String) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.restore_item(db, &id))
}

#[tauri::command]
pub async fn delete_item_permanently(state: State<'_, SharedState>, id: String) -> AppResult<()> {
    with_unlocked(&state, |db, vault| vault.delete_permanently(db, &id))
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, SharedState>) -> AppResult<usize> {
    with_unlocked(&state, |db, vault| vault.empty_trash(db))
}

/// Returns the login password for display ("show password" / edit form).
#[tauri::command]
pub async fn reveal_password(state: State<'_, SharedState>, id: String) -> AppResult<String> {
    with_unlocked(&state, |_, vault| {
        vault.field_value(&id, CopyField::Password)
    })
}

/// Opens the login's website in the default browser. Only http(s) URLs are allowed,
/// so a crafted entry cannot launch arbitrary programs or local files.
#[tauri::command]
pub async fn open_item_website(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;

    let website = with_unlocked(&state, |_, vault| {
        vault.field_value(&id, CopyField::Website)
    })?;
    let url = if website.contains("://") {
        website
    } else {
        format!("https://{website}")
    };
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(AppError::Validation("website"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| AppError::Internal)
}

/// Copies a field to the clipboard from Rust, so secrets never pass through JS.
/// Secret fields are excluded from Windows clipboard history and auto-cleared.
#[tauri::command]
pub async fn copy_item_field(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
    field: CopyField,
) -> AppResult<u32> {
    let (value, clear_after) = {
        let mut inner = state.lock();
        let clear_after = inner.settings.clipboard_clear_seconds;
        inner.touch();
        let (_, vault) = inner.unlocked()?;
        (vault.field_value(&id, field)?, clear_after)
    };
    let clear_after = if field.is_secret() { clear_after } else { 0 };
    clipboard::copy(&app, value, field.is_secret(), clear_after)?;
    Ok(clear_after)
}
