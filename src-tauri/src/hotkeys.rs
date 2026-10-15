//! Global hotkeys: pause and show always; next/prev and each profile's own hotkey in Hotkey mode.
//! Every registered chord is also reserved in the engine, so a remap can never eat it.

use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::control;
use crate::engine::{ALT, CTRL, Engine, META, ModMask, SHIFT};
use crate::profiles::{Profile, Settings, SwitchMode};

/// "Win+Shift+S" -> "Super+Shift+S": the shortcut parser knows Super/Cmd but not Win/Meta.
pub fn normalize(accel: &str) -> String {
    accel
        .split('+')
        .map(|t| match t.trim().to_ascii_lowercase().as_str() {
            "win" | "meta" | "windows" => "Super".to_string(),
            _ => t.trim().to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// The physical chord a hotkey string stands for, as the engine sees it ("Ctrl+Alt+P" -> ("KeyP", Ctrl|Alt)).
pub fn chord(accel: &str) -> Option<(String, ModMask)> {
    let mut mods = 0;
    let mut key = None;
    for token in accel.split('+').map(str::trim) {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= CTRL,
            "alt" | "option" => mods |= ALT,
            "shift" => mods |= SHIFT,
            "win" | "meta" | "windows" | "cmd" | "command" | "super" => mods |= META,
            _ => key = Some(code_of(token)?),
        }
    }
    key.map(|k| (k, mods))
}

fn code_of(token: &str) -> Option<String> {
    let mut chars = token.chars();
    let code = match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => format!("Key{}", c.to_ascii_uppercase()),
        (Some(c), None) if c.is_ascii_digit() => format!("Digit{c}"),
        _ => match token {
            "[" => "BracketLeft".to_string(),
            "]" => "BracketRight".to_string(),
            other => other.to_string(),
        },
    };
    crate::keymap::def(&code).map(|d| d.code.to_string())
}

/// Unregisters everything, then registers the app hotkeys for the current settings and profiles.
/// A hotkey that fails (bad string, already taken by another app) is logged and skipped.
pub fn register_all(app: &AppHandle, settings: &Settings, profiles: &[Profile]) -> anyhow::Result<()> {
    app.global_shortcut().unregister_all()?;
    let mut reserved = Vec::new();
    bind(app, &settings.hotkeys.pause, &mut reserved, |app| log(control::toggle_pause(app)));
    bind(app, &settings.hotkeys.show, &mut reserved, crate::show_main);
    if settings.switch_mode == SwitchMode::Hotkey {
        bind(app, &settings.hotkeys.next_profile, &mut reserved, |app| log(control::step(app, 1)));
        bind(app, &settings.hotkeys.prev_profile, &mut reserved, |app| log(control::step(app, -1)));
        for p in profiles {
            if let Some(accel) = &p.hotkey {
                let id = p.id.clone();
                bind(app, accel, &mut reserved, move |app| log(control::activate(app, &id)));
            }
        }
    }
    if let Some(engine) = app.try_state::<Arc<Engine>>() {
        engine.set_reserved(reserved);
    }
    Ok(())
}

fn bind(
    app: &AppHandle,
    accel: &str,
    reserved: &mut Vec<(String, ModMask)>,
    action: impl Fn(&AppHandle) + Send + Sync + 'static,
) {
    if accel.trim().is_empty() {
        return;
    }
    let result = app.global_shortcut().on_shortcut(normalize(accel).as_str(), move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            action(app);
        }
    });
    match result {
        Ok(()) => reserved.extend(chord(accel)),
        Err(e) => eprintln!("hotkey {accel:?} not registered: {e}"),
    }
}

fn log(r: anyhow::Result<()>) {
    if let Err(e) = r {
        eprintln!("hotkey action failed: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_global_shortcut::Shortcut;

    #[test]
    fn normalizes_win_and_meta_for_the_parser() {
        assert_eq!(normalize("Win+Shift+S"), "Super+Shift+S");
        assert_eq!(normalize("Meta+F13"), "Super+F13");
        assert_eq!(normalize("Ctrl+Alt+P"), "Ctrl+Alt+P");
    }

    #[test]
    fn chords_match_engine_codes() {
        assert_eq!(chord("Ctrl+Alt+P"), Some(("KeyP".into(), CTRL | ALT)));
        assert_eq!(chord("Ctrl+Alt+]"), Some(("BracketRight".into(), CTRL | ALT)));
        assert_eq!(chord("Ctrl+Alt+["), Some(("BracketLeft".into(), CTRL | ALT)));
        assert_eq!(chord("Win+Shift+F13"), Some(("F13".into(), META | SHIFT)));
        assert_eq!(chord("Ctrl+Alt+2"), Some(("Digit2".into(), CTRL | ALT)));
        assert_eq!(chord("Cmd+KeyM"), Some(("KeyM".into(), META)));
        assert_eq!(chord("Ctrl+Alt"), None);
        assert_eq!(chord("Ctrl+Nope"), None);
    }

    #[test]
    fn default_and_typed_hotkeys_parse_for_the_plugin() {
        let h = crate::profiles::Hotkeys::default();
        for accel in [h.pause.as_str(), &h.next_profile, &h.prev_profile, &h.show, "Ctrl+Alt+Cmd+F13", "Meta+Shift+2"] {
            assert!(normalize(accel).parse::<Shortcut>().is_ok(), "{accel}");
            assert!(chord(accel).is_some(), "{accel}");
        }
    }
}
