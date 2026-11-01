//! Windows Hello (face, fingerprint or PIN) through `KeyCredentialManager`.
//!
//! Each vault gets its own Hello key. The key never leaves Windows; we only ask it to
//! sign the vault's challenge, which shows the system Hello prompt.
//!
//! Unsafe code is limited to two Win32 calls that bring the prompt to the front:
//! it is created by a separate system process and otherwise may open behind our window.
#![allow(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Security::Credentials::{
    KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus,
};
use windows::Security::Cryptography::CryptographicBuffer;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
use windows::core::{Array, HSTRING, w};
use zeroize::Zeroizing;

use crate::core::error::{AppError, AppResult};

fn credential_name(vault_id: &str) -> HSTRING {
    HSTRING::from(format!("Nexus Vault {vault_id}"))
}

/// Hello is set up on this PC (a PIN at least) and usable by apps.
pub fn is_available() -> bool {
    KeyCredentialManager::IsSupportedAsync()
        .and_then(|op| op.join())
        .unwrap_or(false)
}

fn status_error(status: KeyCredentialStatus) -> AppError {
    match status {
        KeyCredentialStatus::UserCanceled | KeyCredentialStatus::UserPrefersPassword => {
            AppError::Cancelled
        }
        KeyCredentialStatus::NotFound => AppError::HelloFailed,
        _ => AppError::HelloUnavailable,
    }
}

/// While a Hello prompt is pending, keeps pulling it to the foreground.
struct FocusPrompt(Arc<AtomicBool>);

impl FocusPrompt {
    fn start() -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let flag = done.clone();
        std::thread::spawn(move || {
            for _ in 0..50 {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                // SAFETY: plain Win32 calls with a static, NUL-terminated class name;
                // a missing window just returns an error.
                unsafe {
                    if let Ok(hwnd) = FindWindowW(w!("Credential Dialog Xaml Host"), None) {
                        let _ = SetForegroundWindow(hwnd);
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        FocusPrompt(done)
    }
}

impl Drop for FocusPrompt {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// Creates (or replaces) the vault's Hello key. Shows the Hello prompt.
pub fn create(vault_id: &str) -> AppResult<()> {
    let _focus = FocusPrompt::start();
    let result = KeyCredentialManager::RequestCreateAsync(
        &credential_name(vault_id),
        KeyCredentialCreationOption::ReplaceExisting,
    )
    .and_then(|op| op.join())
    .map_err(|_| AppError::HelloUnavailable)?;
    match result.Status().map_err(|_| AppError::HelloUnavailable)? {
        KeyCredentialStatus::Success => Ok(()),
        status => Err(status_error(status)),
    }
}

/// Signs `challenge` with the vault's Hello key. Shows the Hello prompt.
pub fn sign(vault_id: &str, challenge: &[u8]) -> AppResult<Zeroizing<Vec<u8>>> {
    let _focus = FocusPrompt::start();
    let opened = KeyCredentialManager::OpenAsync(&credential_name(vault_id))
        .and_then(|op| op.join())
        .map_err(|_| AppError::HelloUnavailable)?;
    match opened.Status().map_err(|_| AppError::HelloUnavailable)? {
        KeyCredentialStatus::Success => {}
        status => return Err(status_error(status)),
    }
    let credential = opened
        .Credential()
        .map_err(|_| AppError::HelloUnavailable)?;
    let data =
        CryptographicBuffer::CreateFromByteArray(challenge).map_err(|_| AppError::Internal)?;
    let signed = credential
        .RequestSignAsync(&data)
        .and_then(|op| op.join())
        .map_err(|_| AppError::HelloUnavailable)?;
    match signed.Status().map_err(|_| AppError::HelloUnavailable)? {
        KeyCredentialStatus::Success => {}
        status => return Err(status_error(status)),
    }
    let buffer = signed.Result().map_err(|_| AppError::Internal)?;
    let mut bytes = Array::<u8>::new();
    CryptographicBuffer::CopyToByteArray(&buffer, &mut bytes).map_err(|_| AppError::Internal)?;
    Ok(Zeroizing::new(bytes.to_vec()))
}

/// Removes the vault's Hello key (best effort: it may already be gone).
pub fn delete(vault_id: &str) {
    if let Ok(op) = KeyCredentialManager::DeleteAsync(&credential_name(vault_id)) {
        let _ = op.join();
    }
}
