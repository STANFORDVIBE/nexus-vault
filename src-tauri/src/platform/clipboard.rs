//! Clipboard with automatic clearing.
//!
//! Secrets are written with the Windows flags that keep them out of clipboard history
//! (Win+V) and cloud sync. We remember only a hash of what we copied, and clear the
//! clipboard later only if it still holds exactly that value, so other applications'
//! content is never wiped.

use std::sync::Mutex;
use std::time::Duration;

use arboard::{Clipboard, SetExtWindows};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use zeroize::Zeroizing;

use crate::core::error::{AppError, AppResult};
use crate::platform::events;

struct Copied {
    digest: [u8; 32],
    generation: u64,
}

static LAST_COPY: Mutex<Option<Copied>> = Mutex::new(None);
static GENERATION: Mutex<u64> = Mutex::new(0);

fn digest(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}

fn next_generation() -> u64 {
    let mut generation = GENERATION.lock().unwrap_or_else(|p| p.into_inner());
    *generation += 1;
    *generation
}

/// Copies `value`. When `secret` is set it is hidden from history and remembered for
/// clearing; `clear_after_secs > 0` schedules the automatic clear.
pub fn copy(app: &AppHandle, value: String, secret: bool, clear_after_secs: u32) -> AppResult<()> {
    let value = Zeroizing::new(value);
    let mut clipboard = Clipboard::new().map_err(|_| AppError::Internal)?;
    let set = clipboard.set();
    let set = if secret {
        set.exclude_from_history().exclude_from_cloud()
    } else {
        set
    };
    set.text(value.as_str()).map_err(|_| AppError::Internal)?;

    let generation = next_generation();
    let mut last = LAST_COPY.lock().unwrap_or_else(|p| p.into_inner());
    *last = secret.then(|| Copied {
        digest: digest(&value),
        generation,
    });
    drop(last);

    if secret && clear_after_secs > 0 {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(u64::from(clear_after_secs)));
            if clear_if_ours(Some(generation)) {
                let _ = app.emit(events::CLIPBOARD_CLEARED, ());
            }
        });
    }
    Ok(())
}

/// Clears the clipboard if it still contains the secret we put there.
/// With `generation`, only that specific copy is considered (a newer copy wins).
pub fn clear_if_ours(generation: Option<u64>) -> bool {
    let mut last = LAST_COPY.lock().unwrap_or_else(|p| p.into_inner());
    let Some(copied) = last.as_ref() else {
        return false;
    };
    if generation.is_some_and(|g| g != copied.generation) {
        return false;
    }

    let Ok(mut clipboard) = Clipboard::new() else {
        return false;
    };
    let current = clipboard.get_text().map(Zeroizing::new);
    let ours = matches!(&current, Ok(text) if digest(text) == copied.digest);
    *last = None;
    if ours {
        return clipboard.clear().is_ok();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable_and_distinct() {
        assert_eq!(digest("abc"), digest("abc"));
        assert_ne!(digest("abc"), digest("abd"));
    }
}
