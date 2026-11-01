//! Main window behaviour: show/focus, close-to-tray and the quick-search hotkey.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, Window, WindowEvent};
use tauri_plugin_notification::NotificationExt;

use crate::commands::SharedState;
use crate::core::settings::CloseBehavior;
use crate::platform::{clipboard, events, lock};

static TRAY_HINT_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Ctrl+Shift+L: bring the app forward and focus search.
pub fn quick_search(app: &AppHandle) {
    show_main_window(app);
    let _ = app.emit(events::QUICK_SEARCH, ());
}

/// Exits cleanly: our clipboard secret is cleared and the vault dropped first.
pub fn exit_app(app: &AppHandle) {
    clipboard::clear_if_ours(None);
    app.state::<SharedState>().lock().lock_vault();
    app.exit(0);
}

fn hide_to_tray(window: &Window) {
    let _ = window.hide();
    // tell the user once per session where the app went
    if !TRAY_HINT_SHOWN.swap(true, Ordering::Relaxed) {
        let _ = window
            .app_handle()
            .notification()
            .builder()
            .title("Nexus Vault работает в трее")
            .body("Откройте приложение значком в области уведомлений или Ctrl+Shift+L.")
            .show();
    }
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        let app = window.app_handle();
        let behavior = app.state::<SharedState>().lock().settings.close_behavior;
        match behavior {
            CloseBehavior::Exit => exit_app(app),
            CloseBehavior::MinimizeToTray => {
                api.prevent_close();
                hide_to_tray(window);
            }
            CloseBehavior::LockToTray => {
                api.prevent_close();
                lock::lock_now(app);
                hide_to_tray(window);
            }
        }
    }
}
