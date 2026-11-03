use tauri::State;
use tauri::ipc::{InvokeBody, Request, Response};

use crate::commands::{SharedState, with_unlocked};
use crate::core::error::{AppError, AppResult};

/// Header carrying the note id; the body is the raw image (no JSON/base64 round trip).
const ITEM_ID_HEADER: &str = "x-item-id";

/// Encrypts an image for a note. Returns the attachment id to put in the markdown.
#[tauri::command]
pub async fn add_attachment(
    state: State<'_, SharedState>,
    request: Request<'_>,
) -> AppResult<String> {
    let InvokeBody::Raw(data) = request.body() else {
        return Err(AppError::Validation("image"));
    };
    let item_id = request
        .headers()
        .get(ITEM_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Validation("item"))?;
    with_unlocked(&state, |db, vault| vault.add_attachment(db, item_id, data))
}

/// The decrypted image as raw bytes: `mime length (u8) | mime | image`.
#[tauri::command]
pub async fn get_attachment(state: State<'_, SharedState>, id: String) -> AppResult<Response> {
    with_unlocked(&state, |db, vault| {
        let plain = vault.read_attachment(db, &id)?;
        Ok(Response::new(plain.to_vec()))
    })
}
