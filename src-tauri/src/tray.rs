//! Tray / menu bar: status line, profile list, pause, open, check for updates, quit.
//! Rebuilt on "profile-changed" / "paused-changed" and by `control::resync`.

use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Listener, Manager, Window, WindowEvent, Wry};

use crate::commands::AppState;
use crate::control;

const TRAY_ID: &str = "main-tray";

pub fn build(app: &AppHandle) -> anyhow::Result<()> {
    let icon = app.default_window_icon().ok_or_else(|| anyhow::anyhow!("no app icon"))?.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Macally Studio")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    for event in ["profile-changed", "paused-changed"] {
        let handle = app.clone();
        app.listen_any(event, move |_| refresh(&handle));
    }
    Ok(())
}

pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    match menu(app) {
        Ok(m) => {
            let _ = tray.set_menu(Some(m));
        }
        Err(e) => eprintln!("tray menu: {e}"),
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    // copy everything out first: no lock is held while menu items are created on the main thread
    let (paused, active, pause_key, profiles) = match app.try_state::<AppState>() {
        Some(state) => {
            let s = state.settings.lock().unwrap().clone();
            let list: Vec<(String, String)> =
                state.profiles.lock().unwrap().iter().map(|p| (p.id.clone(), p.name.clone())).collect();
            (state.engine.is_paused(), s.active_profile, s.hotkeys.pause, list)
        }
        None => (false, String::new(), String::new(), Vec::new()),
    };
    let version = crate::version::display(&app.package_info().version.to_string());
    let (status, pause_label) = labels(paused, &version, &pause_key);

    let mut b = MenuBuilder::new(app)
        .item(&MenuItem::with_id(app, "status", status, false, None::<&str>)?)
        .separator();
    for (id, name) in &profiles {
        b = b.item(&CheckMenuItem::with_id(app, format!("profile:{id}"), name, true, *id == active, None::<&str>)?);
    }
    b.separator()
        .item(&CheckMenuItem::with_id(app, "pause", pause_label, true, paused, None::<&str>)?)
        .item(&MenuItem::with_id(app, "open", "Open Macally Studio", true, None::<&str>)?)
        .item(&MenuItem::with_id(app, "check-updates", "Check for updates", true, None::<&str>)?)
        .separator()
        .item(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)
        .build()
}

/// The status line and the pause item's text.
fn labels(paused: bool, version: &str, pause_key: &str) -> (String, String) {
    let status = if paused { "Remapping paused" } else { "Remapping on" };
    let pause = match pause_key {
        "" => "Pause remapping".to_string(),
        key => format!("Pause remapping ({key})"),
    };
    (format!("{status} · Version {version}"), pause)
}

fn on_menu(app: &AppHandle, id: &str) {
    let result = match id {
        "open" => {
            crate::show_main(app);
            Ok(())
        }
        "pause" => control::toggle_pause(app),
        "quit" => {
            app.exit(0);
            return;
        }
        "check-updates" => {
            crate::show_main(app);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::updater::check(&app).await {
                    eprintln!("update check: {e:#}");
                }
            });
            Ok(())
        }
        other => match other.strip_prefix("profile:") {
            Some(profile) => control::activate(app, profile),
            None => Ok(()),
        },
    };
    if let Err(e) = result {
        eprintln!("tray {id}: {e:#}");
    }
    refresh(app); // a CheckMenuItem toggles itself on click; redraw from the real state
}

/// Close-to-tray: hide the main window instead of quitting when the setting is on.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        let to_tray = window
            .app_handle()
            .try_state::<AppState>()
            .map(|s| s.settings.lock().unwrap().close_to_tray)
            .unwrap_or(true);
        if window.label() == "main" && to_tray {
            api.prevent_close();
            let _ = window.hide();
            // Monitor ends with the window; the Keyboard view turns it back on when it regains focus
            if let Some(engine) = window.app_handle().try_state::<std::sync::Arc<crate::engine::Engine>>() {
                engine.set_mode(crate::engine::Mode::Normal);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::labels;

    #[test]
    fn status_line_and_pause_item_text() {
        assert_eq!(
            labels(false, "1.0", "Ctrl+Alt+P"),
            ("Remapping on · Version 1.0".to_string(), "Pause remapping (Ctrl+Alt+P)".to_string())
        );
        assert_eq!(labels(true, "1.2", ""), ("Remapping paused · Version 1.2".to_string(), "Pause remapping".to_string()));
    }
}
