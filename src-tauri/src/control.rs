//! App-wide operations shared by the tray, hotkeys, key actions and the foreground watcher.
//! Profile and pause changes go through the same commands the UI calls, so every trigger
//! saves, updates the engine and emits "profile-changed" / "paused-changed" the same way.

use tauri::{AppHandle, Manager};

use crate::commands::{self, AppState};

pub fn activate(app: &AppHandle, id: &str) -> anyhow::Result<()> {
    let state = app.try_state::<AppState>().ok_or_else(|| anyhow::anyhow!("app state not ready"))?;
    commands::set_active_profile(app.clone(), state, id.to_string()).map_err(anyhow::Error::msg)
}

pub fn toggle_pause(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.try_state::<AppState>().ok_or_else(|| anyhow::anyhow!("app state not ready"))?;
    let paused = !state.engine.is_paused();
    commands::set_paused(app.clone(), state, paused);
    Ok(())
}

/// The id `step` places after `current` in `ids`, wrapping (step -1 = previous).
pub fn step_id(ids: &[String], current: &str, step: i32) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let n = ids.len() as i32;
    let i = ids.iter().position(|id| id == current).map_or(0, |i| i as i32);
    Some(ids[(i + step).rem_euclid(n) as usize].clone())
}

/// Next (1) or previous (-1) profile in list order.
pub fn step(app: &AppHandle, step: i32) -> anyhow::Result<()> {
    let Some(state) = app.try_state::<AppState>() else { return Ok(()) };
    let ids: Vec<String> = state.profiles.lock().unwrap().iter().map(|p| p.id.clone()).collect();
    match step_id(&ids, &state.active_id(), step) {
        Some(id) => activate(app, &id),
        None => Ok(()),
    }
}

/// After startup and after any profile or settings change: re-register hotkeys,
/// rebuild the tray, and apply "start at login" / "run as administrator".
pub fn resync(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let settings = state.settings.lock().unwrap().clone();
    let profiles = state.profiles.lock().unwrap().clone();
    if let Err(e) = crate::hotkeys::register_all(app, &settings, &profiles) {
        eprintln!("hotkeys: {e:#}");
    }
    crate::tray::refresh(app);
    sync_autostart(app, &settings);
}

fn sync_autostart(app: &AppHandle, s: &crate::profiles::Settings) {
    use tauri_plugin_autostart::ManagerExt;
    if cfg!(debug_assertions) {
        return; // `tauri dev` must not register the dev binary as a login item
    }
    // elevated starts at login come from a scheduled task instead of the Run key / LaunchAgent
    let admin = cfg!(target_os = "windows") && s.run_as_admin;
    let want = s.start_at_login && !admin;
    let launcher = app.autolaunch();
    if launcher.is_enabled().unwrap_or(!want) != want
        && let Err(e) = if want { launcher.enable() } else { launcher.disable() }
    {
        eprintln!("start at login: {e}");
    }
    #[cfg(target_os = "windows")]
    elevation::sync(s.start_at_login && admin, admin);
}

#[cfg(target_os = "windows")]
mod elevation {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};

    const TASK: &str = "Macally Studio";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// `logon_task`: keep an ONLOGON task at highest privileges. `admin`: this process must be elevated.
    pub fn sync(logon_task: bool, admin: bool) {
        let elevated = unsafe { IsUserAnAdmin() }.as_bool();
        if admin && !elevated {
            relaunch_elevated();
        }
        if !elevated {
            return; // creating or deleting a highest-privilege task needs elevation
        }
        let Ok(exe) = std::env::current_exe() else { return };
        let action = format!("\"{}\" --minimized", exe.display());
        let args: Vec<&str> = if logon_task {
            vec!["/Create", "/F", "/SC", "ONLOGON", "/RL", "HIGHEST", "/TN", TASK, "/TR", &action]
        } else {
            vec!["/Delete", "/F", "/TN", TASK]
        };
        let _ = Command::new("schtasks").args(&args).creation_flags(CREATE_NO_WINDOW).output();
    }

    fn relaunch_elevated() {
        let Ok(exe) = std::env::current_exe() else { return };
        let exe = HSTRING::from(exe.as_os_str());
        let r = unsafe { ShellExecuteW(None, w!("runas"), &exe, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
        // exit at once so the elevated copy wins the single-instance check;
        // a declined UAC prompt (r <= 32) keeps this copy running unelevated
        if r.0 as isize > 32 {
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::step_id;

    #[test]
    fn step_wraps_both_ways() {
        let ids: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert_eq!(step_id(&ids, "b", 1).as_deref(), Some("c"));
        assert_eq!(step_id(&ids, "c", 1).as_deref(), Some("a"));
        assert_eq!(step_id(&ids, "a", -1).as_deref(), Some("c"));
        assert_eq!(step_id(&ids, "gone", 1).as_deref(), Some("b"));
        assert_eq!(step_id(&[], "a", 1), None);
    }
}
