//! What a key can do when pressed. Stored in profiles as `{ "type": "...", ... }`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AppRef {
    /// Windows Start-menu / Store app id, launched via `shell:AppsFolder\<id>`.
    Aumid { id: String },
    Path { path: String, args: String },
    /// macOS bundle identifier.
    Bundle { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    KeyCombo { keys: Vec<String> },
    LaunchApp { app: AppRef },
    SteamGame { appid: u32, name: String },
    Open { target: String },
    Command { program: String, args: Vec<String>, hidden: bool },
    SwitchProfile { profile: String },
    PauseToggle,
    PassThrough,
}

/// Runs every action except KeyCombo and PassThrough (the engine injects those itself).
/// Called on the engine's worker thread, never inside the keyboard hook.
pub fn run(action: &Action, app: &tauri::AppHandle) -> anyhow::Result<()> {
    use tauri_plugin_opener::OpenerExt;
    match action {
        Action::KeyCombo { .. } | Action::PassThrough => Ok(()),
        Action::LaunchApp { app: target } => launch(target),
        Action::SteamGame { appid, .. } => Ok(app.opener().open_url(format!("steam://rungameid/{appid}"), None::<&str>)?),
        Action::Open { target } if has_scheme(target) => Ok(app.opener().open_url(target.as_str(), None::<&str>)?),
        Action::Open { target } => Ok(app.opener().open_path(target.as_str(), None::<&str>)?),
        Action::Command { program, args, hidden } => spawn(program, args, *hidden),
        Action::SwitchProfile { profile } => crate::control::activate(app, profile),
        Action::PauseToggle => crate::control::toggle_pause(app),
    }
}

/// "https://…", "steam://…", "mailto:…", "ms-settings:…" are URLs; "C:\…", "/Users/…" and
/// "notes.txt" are paths. A one-letter scheme is a Windows drive letter.
pub fn has_scheme(target: &str) -> bool {
    match target.split_once(':') {
        Some((scheme, _)) => {
            scheme.len() > 1
                && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        }
        None => false,
    }
}

/// Starts `program args…` without waiting; a background thread reaps it.
fn spawn(program: &str, args: &[String], hidden: bool) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    if hidden {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = hidden;
    let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("start {program}: {e}"))?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch(target: &AppRef) -> anyhow::Result<()> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};

    let (file, params) = match target {
        AppRef::Aumid { id } => (format!("shell:AppsFolder\\{id}"), String::new()),
        AppRef::Path { path, args } => (path.clone(), args.clone()),
        AppRef::Bundle { id } => anyhow::bail!("{id} is a macOS app"),
    };
    let file_w = HSTRING::from(file.as_str());
    let params_w = HSTRING::from(params.as_str());
    let params_ptr = if params.is_empty() { PCWSTR::null() } else { PCWSTR(params_w.as_ptr()) };
    let r = unsafe { ShellExecuteW(None, w!("open"), &file_w, params_ptr, PCWSTR::null(), SW_SHOWNORMAL) };
    // ShellExecuteW reports success as a value greater than 32
    anyhow::ensure!(r.0 as isize > 32, "could not launch {file} (ShellExecute error {})", r.0 as isize);
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch(target: &AppRef) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new("/usr/bin/open");
    match target {
        AppRef::Bundle { id } => cmd.args(["-b", id]),
        AppRef::Path { path, args } if args.trim().is_empty() => cmd.arg(path),
        AppRef::Path { path, args } => cmd.arg(path).arg("--args").args(args.split_whitespace()),
        AppRef::Aumid { id } => anyhow::bail!("{id} is a Windows app"),
    };
    let status = cmd.status()?;
    anyhow::ensure!(status.success(), "open exited with {status}");
    Ok(())
}

#[cfg(test)]
mod run_tests {
    use super::has_scheme;

    #[test]
    fn urls_and_paths_are_told_apart() {
        for url in ["https://github.com", "steam://rungameid/620", "mailto:me@example.com", "file:///C:/x.txt", "ms-settings:display"] {
            assert!(has_scheme(url), "{url} should be a URL");
        }
        for path in [r"C:\Users\me\notes.txt", "D:/games", "/Users/me/Documents", "notes.txt", r"\\server\share", ""] {
            assert!(!has_scheme(path), "{path} should be a path");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn round_trip(action: Action, expected: Value) {
        assert_eq!(serde_json::to_value(&action).unwrap(), expected);
        assert_eq!(serde_json::from_value::<Action>(expected).unwrap(), action);
    }

    #[test]
    fn key_combo() {
        round_trip(
            Action::KeyCombo { keys: vec!["Win".into(), "Tab".into()] },
            json!({ "type": "key_combo", "keys": ["Win", "Tab"] }),
        );
    }

    #[test]
    fn launch_app_aumid() {
        round_trip(
            Action::LaunchApp { app: AppRef::Aumid { id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into() } },
            json!({ "type": "launch_app", "app": { "kind": "aumid", "id": "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App" } }),
        );
    }

    #[test]
    fn launch_app_path() {
        round_trip(
            Action::LaunchApp { app: AppRef::Path { path: "C:\\Apps\\app.exe".into(), args: "--fast".into() } },
            json!({ "type": "launch_app", "app": { "kind": "path", "path": "C:\\Apps\\app.exe", "args": "--fast" } }),
        );
    }

    #[test]
    fn launch_app_bundle() {
        round_trip(
            Action::LaunchApp { app: AppRef::Bundle { id: "com.apple.calculator".into() } },
            json!({ "type": "launch_app", "app": { "kind": "bundle", "id": "com.apple.calculator" } }),
        );
    }

    #[test]
    fn steam_game() {
        round_trip(
            Action::SteamGame { appid: 620, name: "Portal 2".into() },
            json!({ "type": "steam_game", "appid": 620, "name": "Portal 2" }),
        );
    }

    #[test]
    fn open() {
        round_trip(
            Action::Open { target: "https://github.com".into() },
            json!({ "type": "open", "target": "https://github.com" }),
        );
    }

    #[test]
    fn command() {
        round_trip(
            Action::Command { program: "powershell".into(), args: vec!["-File".into(), "C:\\x.ps1".into()], hidden: true },
            json!({ "type": "command", "program": "powershell", "args": ["-File", "C:\\x.ps1"], "hidden": true }),
        );
    }

    #[test]
    fn switch_profile() {
        round_trip(
            Action::SwitchProfile { profile: "gaming".into() },
            json!({ "type": "switch_profile", "profile": "gaming" }),
        );
    }

    #[test]
    fn unit_variants() {
        round_trip(Action::PauseToggle, json!({ "type": "pause_toggle" }));
        round_trip(Action::PassThrough, json!({ "type": "pass_through" }));
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(serde_json::from_value::<Action>(json!({ "type": "macro" })).is_err());
    }
}
