//! Windows session lock notifications (WM_WTSSESSION_CHANGE / WTS_SESSION_LOCK).
//!
//! This is the only module that needs `unsafe`: registering for session notifications
//! and subclassing the main window are plain Win32 calls with no safe wrapper.
#![allow(unsafe_code)]

use std::sync::OnceLock;

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK};

type Handler = fn(&AppHandle);

static CALLBACK: OnceLock<(AppHandle, Handler)> = OnceLock::new();
const SUBCLASS_ID: usize = 0x4E58_5654; // "NXVT"

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_WTSSESSION_CHANGE
        && wparam.0 as u32 == WTS_SESSION_LOCK
        && let Some((app, handler)) = CALLBACK.get()
    {
        handler(app);
    }
    // SAFETY: forwarding the unmodified message to the next handler in the chain.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

pub fn watch(app: &AppHandle, handler: Handler) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(raw) = window.hwnd() else {
        return;
    };
    if CALLBACK.set((app.clone(), handler)).is_err() {
        return;
    }
    // tauri may link a different `windows` crate version, so convert via the raw pointer
    let hwnd = HWND(raw.0);
    // SAFETY: hwnd is the live main window owned by this process; the subclass proc and
    // the callback it reads are 'static.
    unsafe {
        let _ = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
        let _ = SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0);
    }
}
