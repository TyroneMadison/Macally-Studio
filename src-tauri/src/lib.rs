pub mod actions;
pub mod catalog;
pub mod commands;
pub mod control;
pub mod engine;
pub mod foreground;
pub mod hotkeys;
pub mod keymap;
pub mod profiles;
pub mod tray;
pub mod updater;
pub mod version;

use tauri::Manager;

use commands::AppState;
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
        .on_window_event(tray::on_window_event)
        .setup(|app| {
            let engine = engine::start(app.handle().clone());
            app.manage(engine.clone());
            // loads settings and profiles and hands the active profile to the engine
            app.manage(AppState::load(Store::open()?, engine)?);
            tray::build(app.handle())?;
            control::resync(app.handle()); // hotkeys and tray
            control::sync_autostart(app.handle()); // start at login, run as administrator
            foreground::start(app.handle().clone());
            // the window starts hidden (tauri.conf.json), so a login start never flashes it
            if !std::env::args().any(|a| a == "--minimized") {
                show_main(app.handle());
            }
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
            commands::get_status,
            commands::start_record,
            commands::stop_record,
            commands::set_monitor,
            commands::get_catalog,
            commands::refresh_catalog,
            commands::add_catalog_entry,
            updater::check_update,
            updater::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Macally Studio")
        .run(|_app, _event| {
            // macOS: clicking the Dock icon brings back a window hidden by close-to-tray
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_main(_app);
            }
        });
}
