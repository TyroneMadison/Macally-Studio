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
    /// Also what a "pause_toggle" binding from before pausing was removed loads as.
    #[serde(alias = "pause_toggle")]
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

/// What ShellExecute is given for a Windows launch: the file to open, its arguments and the folder
/// to start in.
#[derive(Debug, PartialEq, Eq)]
pub struct LaunchSpec {
    pub file: String,
    pub params: String,
    pub dir: Option<String>,
}

/// Store and Start-menu apps open through `shell:AppsFolder\<AUMID>`. A shortcut (.lnk) opens as is:
/// it carries its own arguments and start folder. A program starts in its own folder, as it does
/// from Explorer, because many look for their files next to the exe.
pub fn launch_spec(target: &AppRef) -> anyhow::Result<LaunchSpec> {
    match target {
        AppRef::Aumid { id } => {
            anyhow::ensure!(!id.trim().is_empty(), "no app id to launch");
            Ok(LaunchSpec { file: format!("shell:AppsFolder\\{}", id.trim()), params: String::new(), dir: None })
        }
        AppRef::Path { path, args } => {
            let path = path.trim().trim_matches('"').to_string();
            anyhow::ensure!(!path.is_empty(), "no program to launch");
            let lnk = path.to_ascii_lowercase().ends_with(".lnk");
            // string-based so Windows paths split the same on every OS the tests run on
            let dir = path.rfind(['\\', '/']).filter(|_| !lnk).map(|i| path[..i].to_string()).filter(|d| !d.is_empty());
            Ok(LaunchSpec { file: path, params: args.trim().to_string(), dir })
        }
        AppRef::Bundle { id } => anyhow::bail!("{id} is a macOS app"),
    }
}

#[cfg(target_os = "windows")]
fn launch(target: &AppRef) -> anyhow::Result<()> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};

    let spec = launch_spec(target)?;
    let file = HSTRING::from(spec.file.as_str());
    let params = HSTRING::from(spec.params.as_str());
    let dir = HSTRING::from(spec.dir.as_deref().unwrap_or_default());
    let opt = |s: &HSTRING| if s.is_empty() { PCWSTR::null() } else { PCWSTR(s.as_ptr()) };
    let r = unsafe { ShellExecuteW(None, w!("open"), &file, opt(&params), opt(&dir), SW_SHOWNORMAL) };
    // ShellExecuteW reports success as a value greater than 32
    anyhow::ensure!(r.0 as isize > 32, "could not launch {} (ShellExecute error {})", spec.file, r.0 as isize);
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
    use super::{AppRef, LaunchSpec, has_scheme, launch_spec};

    fn path(p: &str, args: &str) -> AppRef {
        AppRef::Path { path: p.into(), args: args.into() }
    }

    #[test]
    fn exe_starts_in_its_own_folder_with_its_arguments() {
        assert_eq!(
            launch_spec(&path(r"C:\Program Files\Tool\tool.exe", " --fast ")).unwrap(),
            LaunchSpec {
                file: r"C:\Program Files\Tool\tool.exe".into(),
                params: "--fast".into(),
                dir: Some(r"C:\Program Files\Tool".into()),
            }
        );
        // a quoted path from a registry value loses its quotes
        assert_eq!(launch_spec(&path(r#""C:\Apps\a.exe""#, "")).unwrap().file, r"C:\Apps\a.exe");
    }

    #[test]
    fn shortcut_opens_as_is() {
        let lnk = r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Tool\Tool.lnk";
        assert_eq!(
            launch_spec(&path(lnk, "")).unwrap(),
            LaunchSpec { file: lnk.into(), params: String::new(), dir: None }
        );
    }

    #[test]
    fn store_app_opens_through_apps_folder() {
        let calc = AppRef::Aumid { id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into() };
        assert_eq!(
            launch_spec(&calc).unwrap(),
            LaunchSpec {
                file: r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into(),
                params: String::new(),
                dir: None,
            }
        );
        // Get-StartApps ids for desktop programs carry a known-folder path; they open the same way
        let zip = AppRef::Aumid { id: r"{6D809377-6AF0-444B-8957-A3773F02200E}\7-Zip\7zFM.exe".into() };
        assert_eq!(launch_spec(&zip).unwrap().file, r"shell:AppsFolder\{6D809377-6AF0-444B-8957-A3773F02200E}\7-Zip\7zFM.exe");
    }

    #[test]
    fn nothing_to_launch_is_an_error() {
        assert!(launch_spec(&AppRef::Aumid { id: " ".into() }).is_err());
        assert!(launch_spec(&path("", "")).is_err());
        assert!(launch_spec(&AppRef::Bundle { id: "com.apple.calculator".into() }).is_err());
    }

    /// Starts Notepad through the real launch path, then closes it. Opt-in, desktop only:
    /// `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored launches_notepad`.
    /// Skipped while Notepad is already open, so the user's window and tabs are never touched.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "opens and closes Notepad"]
    fn launches_notepad_through_the_launch_path() {
        fn notepad_pids() -> Vec<u32> {
            let out = std::process::Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq notepad.exe", "/FO", "CSV", "/NH"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| l.split("\",\"").nth(1)?.trim_matches('"').parse().ok())
                .collect()
        }
        if !notepad_pids().is_empty() {
            eprintln!("Notepad is already open; skipped so it is not disturbed");
            return;
        }
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        super::launch(&path(&format!(r"{windir}\System32\notepad.exe"), "")).unwrap();
        let mut pids = Vec::new();
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            pids = notepad_pids();
            if !pids.is_empty() {
                break;
            }
        }
        assert!(!pids.is_empty(), "Notepad did not start");
        for pid in &pids {
            let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output();
        }
        eprintln!("launched and closed Notepad (pid {pids:?})");
    }

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
        round_trip(Action::PassThrough, json!({ "type": "pass_through" }));
    }

    #[test]
    fn a_pause_binding_from_before_loads_as_pass_through() {
        // pausing is gone; a key bound to it in an older profile goes back to being itself
        assert_eq!(serde_json::from_value::<Action>(json!({ "type": "pause_toggle" })).unwrap(), Action::PassThrough);
        assert_eq!(serde_json::to_value(Action::PassThrough).unwrap(), json!({ "type": "pass_through" }));
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(serde_json::from_value::<Action>(json!({ "type": "macro" })).is_err());
    }
}
