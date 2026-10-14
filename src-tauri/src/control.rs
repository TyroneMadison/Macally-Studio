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
