use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::commands::{SharedState, blocking, with_unlocked};
use crate::core::attachments;
use crate::core::error::{AppError, AppResult};
use crate::core::time::{date_stamp, now_millis};

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

/// Saves a decrypted image to a file the user picks (the copy is not encrypted).
/// Returns the file name, or `None` if the dialog was cancelled.
#[tauri::command]
pub async fn save_attachment(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
) -> AppResult<Option<String>> {
    // decrypt first, so a locked vault or a missing image fails before any dialog
    let plain = with_unlocked(&state, |db, vault| vault.read_attachment(db, &id))?;
    let ext = attachments::split_plaintext(&plain)
        .map(|(mime, _)| attachments::extension(mime))
        .ok_or(AppError::Integrity)?;

    let picked = blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title("Сохранить картинку")
            .add_filter(ext.to_uppercase(), &[ext])
            .set_file_name(format!("картинка-{}.{ext}", date_stamp(now_millis())))
            .blocking_save_file())
    })
    .await?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };

    blocking(move || {
        let (_, data) = attachments::split_plaintext(&plain).ok_or(AppError::Integrity)?;
        std::fs::write(&path, data)?;
        Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()))
    })
    .await
}
