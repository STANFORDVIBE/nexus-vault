pub mod backup;
mod commands;
pub mod core;
pub mod crypto;
pub mod database;
mod platform;

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

use crate::core::state::AppState;
use crate::platform::autostart::MINIMIZED_ARG;
use crate::platform::{lock, tray, window};

const VAULT_FILE: &str = "vault.db";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let quick_search = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL);

    tauri::Builder::default()
        // must be first: a second launch just focuses the running instance
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([quick_search])
                .expect("valid shortcut")
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        window::quick_search(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let state: commands::SharedState = Arc::new(AppState::new(data_dir.join(VAULT_FILE)));
            app.manage(state);

            let handle = app.handle();
            tray::create(handle)?;
            lock::start_auto_lock(handle);
            lock::watch_system_lock(handle);

            let start_hidden = std::env::args().any(|arg| arg == MINIMIZED_ARG);
            if !start_hidden {
                window::show_main_window(handle);
            }
            Ok(())
        })
        .on_window_event(window::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::vault::get_vault_status,
            commands::vault::create_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::change_master_password,
            commands::vault::report_activity,
            commands::items::list_items,
            commands::items::get_item,
            commands::items::get_dashboard,
            commands::items::create_item,
            commands::items::update_item,
            commands::items::set_favorite,
            commands::items::move_item_to_folder,
            commands::items::trash_item,
            commands::items::restore_item,
            commands::items::delete_item_permanently,
            commands::items::empty_trash,
            commands::items::reveal_password,
            commands::items::copy_item_field,
            commands::items::open_item_website,
            commands::folders::list_folders,
            commands::folders::create_folder,
            commands::folders::rename_folder,
            commands::folders::delete_folder,
            commands::generator::generate_password,
            commands::generator::copy_generated,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::backup::create_backup,
            commands::backup::pick_backup_file,
            commands::backup::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
