use super::*;
use crate::actions::{Action, AppRef};
use serde_json::json;

fn profile(id: &str) -> Profile {
    let binding = Binding {
        mods: vec![Modifier::Shift],
        action: Action::LaunchApp { app: AppRef::Bundle { id: "com.apple.calculator".into() } },
    };
    Profile {
        schema: 1,
        id: id.into(),
        name: "Photo editing".into(),
        builtin: false,
        hotkey: Some("Ctrl+Alt+2".into()),
        auto_apps: vec!["Photoshop.exe".into()],
        bindings: BTreeMap::from([("F13".to_string(), binding)]),
    }
}

#[test]
fn profile_json_is_camel_case() {
    let v = serde_json::to_value(profile("photo")).unwrap();
    assert_eq!(v["autoApps"], json!(["Photoshop.exe"]));
    assert_eq!(v["hotkey"], "Ctrl+Alt+2");
    assert_eq!(v["bindings"]["F13"]["mods"], json!(["Shift"]));
    assert_eq!(v["bindings"]["F13"]["action"]["type"], "launch_app");
}

#[test]
fn modifiers_serialize_as_variant_names() {
    let mods = [Modifier::Ctrl, Modifier::Alt, Modifier::Shift, Modifier::Meta];
    assert_eq!(serde_json::to_value(mods).unwrap(), json!(["Ctrl", "Alt", "Shift", "Meta"]));
}

#[test]
fn settings_defaults_match_spec() {
    let s = Settings::default();
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["schema"], 1);
    assert_eq!(v["activeProfile"], default_profile_id());
    assert_eq!(v["switchMode"], "auto");
    assert_eq!(
        v["hotkeys"],
        json!({ "pause": "Ctrl+Alt+P", "nextProfile": "Ctrl+Alt+]", "prevProfile": "Ctrl+Alt+[", "show": "Ctrl+Alt+M" })
    );
    assert!(s.start_at_login && s.close_to_tray && s.windows_legends && s.auto_check_updates);
    assert!(!s.run_as_admin);
}

#[test]
fn partial_settings_fill_in_defaults() {
    let s: Settings = serde_json::from_str(r#"{ "switchMode": "manual" }"#).unwrap();
    assert_eq!(s.switch_mode, SwitchMode::Manual);
    assert_eq!(s.hotkeys.pause, "Ctrl+Alt+P");
    assert!(s.close_to_tray);
}

#[test]
fn slug_rules() {
    assert_eq!(slug("Photo Editing!"), "photo-editing");
    assert_eq!(slug("../Evil  Name"), "evil-name");
    assert_eq!(slug("windows-default"), "windows-default");
    assert_eq!(slug("--"), "profile");
}

#[test]
fn save_get_list_delete() {
    let st = temp_store("crud");
    st.save_profile(&profile("photo")).unwrap();
    assert_eq!(st.get_profile("photo").unwrap(), profile("photo"));
    assert_eq!(st.list_profiles().unwrap(), vec![profile("photo")]);
    st.delete_profile("photo").unwrap();
    assert!(st.get_profile("photo").is_err());
}

#[test]
fn rejects_path_like_ids() {
    let st = temp_store("ids");
    assert!(st.get_profile("../settings").is_err());
    assert!(st.delete_profile("..\\settings").is_err());
    assert!(st.save_profile(&profile("a/b")).is_err());
    assert!(st.save_profile(&profile("")).is_err());
}

#[test]
fn builtins_are_editable_but_not_deletable() {
    let st = temp_store("edit");
    st.ensure_builtins().unwrap();
    st.save_profile(&profile("windows-default")).unwrap(); // arrives with builtin: false
    let saved = st.get_profile("windows-default").unwrap();
    assert!(saved.builtin);
    assert_eq!(saved.bindings, profile("photo").bindings);
    assert!(st.delete_profile("windows-default").is_err());
    let mut p = profile("photo");
    p.builtin = true; // a user profile cannot lock itself
    st.save_profile(&p).unwrap();
    st.delete_profile("photo").unwrap();
}

#[test]
fn edits_to_a_builtin_survive_restarts_until_reset() {
    let st = temp_store("reset");
    st.ensure_builtins().unwrap();
    let shipped = st.get_profile("windows-default").unwrap();
    let mut edited = shipped.clone();
    edited.bindings.clear();
    st.save_profile(&edited).unwrap();
    st.ensure_builtins().unwrap(); // next launch
    assert!(st.get_profile("windows-default").unwrap().bindings.is_empty());
    assert_eq!(st.reset_profile("windows-default").unwrap(), shipped);
    assert_eq!(st.get_profile("windows-default").unwrap(), shipped);
}

#[test]
fn reset_refuses_user_profiles() {
    let st = temp_store("reset-mine");
    st.save_profile(&profile("photo")).unwrap();
    assert!(st.reset_profile("photo").is_err());
    assert_eq!(st.get_profile("photo").unwrap(), profile("photo"));
}

#[test]
fn duplicate_appends_copy_and_counts_up() {
    let st = temp_store("dup");
    st.save_profile(&profile("photo")).unwrap();
    let a = st.duplicate_profile("photo").unwrap();
    let b = st.duplicate_profile("photo").unwrap();
    assert_eq!((a.id.as_str(), b.id.as_str()), ("photo-copy", "photo-copy-2"));
    assert_eq!(a.name, "Photo editing copy");
    assert_eq!(a.hotkey, None);
    assert_eq!(a.bindings, profile("photo").bindings);
    assert_eq!(st.list_profiles().unwrap().len(), 3);
}

#[test]
fn export_then_import_gets_a_new_id() {
    let st = temp_store("io");
    st.save_profile(&profile("photo")).unwrap();
    let file = st.data_dir.join("exported.json");
    st.export_profile("photo", &file).unwrap();
    let imported = st.import_profile(&file).unwrap();
    assert_eq!(imported.id, "photo-2");
    assert_eq!(imported.bindings, profile("photo").bindings);
    assert_eq!(imported.hotkey, None); // "photo" already uses Ctrl+Alt+2
    assert_eq!(st.get_profile("photo-2").unwrap(), imported);
}

#[test]
fn import_sanitizes_id_and_clears_builtin() {
    let st = temp_store("import");
    let file = st.data_dir.join("in.json");
    fs::write(&file, r#"{ "schema": 1, "id": "../Evil Name", "name": "X", "builtin": true }"#).unwrap();
    let p = st.import_profile(&file).unwrap();
    assert_eq!(p.id, "evil-name");
    assert!(!p.builtin);
    assert!(p.bindings.is_empty());
}

#[test]
fn newer_schema_is_refused() {
    let st = temp_store("schema");
    let file = st.data_dir.join("new.json");
    fs::write(&file, r#"{ "schema": 2, "id": "x", "name": "X" }"#).unwrap();
    assert!(st.import_profile(&file).is_err());
}

#[test]
fn settings_round_trip_and_default_when_missing() {
    let st = temp_store("settings");
    assert_eq!(st.load_settings(), Settings::default());
    let s = Settings { switch_mode: SwitchMode::Hotkey, close_to_tray: false, ..Settings::default() };
    st.save_settings(&s).unwrap();
    assert_eq!(st.load_settings(), s);
}

#[test]
fn unreadable_settings_are_backed_up_not_lost() {
    let st = temp_store("settings-bak");
    let path = st.data_dir.join("settings.json");
    for text in ["{ not json", r#"{ "schema": 2 }"#] {
        fs::write(&path, text).unwrap();
        assert_eq!(st.load_settings(), Settings::default());
        assert!(!path.exists());
        assert_eq!(fs::read_to_string(st.data_dir.join("settings.json.bak")).unwrap(), text);
    }
}

#[test]
fn builtins_install_with_spec_bindings() {
    let st = temp_store("builtins");
    st.ensure_builtins().unwrap();
    st.ensure_builtins().unwrap(); // second launch never duplicates
    assert_eq!(st.list_profiles().unwrap().len(), 2);

    let win = st.get_profile("windows-default").unwrap();
    assert!(win.builtin);
    assert_eq!(win.name, "Windows Default");
    let keys: Vec<&str> = win.bindings.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["F10", "F11", "F12", "F13", "F14", "F15", "F3", "F4", "F7", "F8", "F9", "MetaLeft", "MetaRight"]
    );
    let combo = |keys: &[&str]| Action::KeyCombo { keys: keys.iter().map(|k| k.to_string()).collect() };
    let aumid = |id: &str| Action::LaunchApp { app: AppRef::Aumid { id: id.into() } };
    assert_eq!(win.bindings["F3"].action, combo(&["Win", "Tab"]));
    assert_eq!(win.bindings["F4"].action, combo(&["Win"]));
    // with Fn-Lock on (fn+esc) the top row sends plain F7-F12; keep the printed media functions
    assert_eq!(win.bindings["F7"].action, combo(&["MediaTrackPrevious"]));
    assert_eq!(win.bindings["F8"].action, combo(&["MediaPlayPause"]));
    assert_eq!(win.bindings["F9"].action, combo(&["MediaTrackNext"]));
    assert_eq!(win.bindings["F10"].action, combo(&["AudioVolumeMute"]));
    assert_eq!(win.bindings["F11"].action, combo(&["AudioVolumeDown"]));
    assert_eq!(win.bindings["F12"].action, combo(&["AudioVolumeUp"]));
    assert_eq!(win.bindings["F13"].action, combo(&["Win", "Shift", "KeyS"]));
    assert_eq!(win.bindings["F14"].action, aumid("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"));
    assert_eq!(win.bindings["F15"].action, aumid("Microsoft.WindowsAlarms_8wekyb3d8bbwe!App"));
    assert_eq!(win.bindings["MetaLeft"].action, combo(&["ControlLeft"]));
    assert_eq!(win.bindings["MetaRight"].action, combo(&["ControlRight"]));
    assert!(win.bindings.values().all(|b| b.mods.is_empty()));

    let mac = st.get_profile("macos-default").unwrap();
    assert!(mac.builtin);
    assert!(mac.bindings.is_empty());
}

#[test]
fn builtin_key_names_exist_in_keymap() {
    for text in BUILTINS {
        let p: Profile = serde_json::from_str(text).unwrap();
        for (code, b) in &p.bindings {
            let sent = match &b.action {
                Action::KeyCombo { keys } => keys.clone(),
                _ => vec![],
            };
            for name in std::iter::once(code).chain(&sent) {
                assert!(crate::keymap::def(name).is_some(), "{}: unknown key {name}", p.id);
            }
        }
    }
}

#[test]
fn community_example_profile_loads() {
    let p: Profile = serde_json::from_str(include_str!("../../../community/example-profile.json"))
        .expect("community/example-profile.json must match the Profile format");
    assert_eq!(p.schema, 1);
    assert!(!p.builtin);
    assert!(!p.bindings.is_empty());
    for (code, b) in &p.bindings {
        assert!(crate::keymap::def(code).is_some(), "unknown key {code}");
        if let Action::KeyCombo { keys } = &b.action {
            for k in keys {
                assert!(crate::keymap::def(k).is_some(), "unknown combo key {k}");
            }
        }
    }
}
