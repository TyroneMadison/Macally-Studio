pub mod actions;
pub mod commands;
pub mod control;
pub mod engine;
pub mod foreground;
pub mod keymap;
pub mod profiles;
pub mod version;

use std::sync::Arc;

use tauri::Manager;

use commands::{AppState, NoopEngine};
use profiles::Store;

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        // must be the first plugin registered
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let state = AppState::load(Store::open()?, Arc::new(NoopEngine::default()))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_layout,
            commands::get_profiles,
            commands::get_profile,
            commands::save_profile,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::import_profile,
            commands::export_profile,
            commands::set_active_profile,
            commands::get_settings,
            commands::save_settings,
            commands::set_paused,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Macally Studio");
}
