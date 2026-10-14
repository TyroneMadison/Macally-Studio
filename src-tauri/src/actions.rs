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
