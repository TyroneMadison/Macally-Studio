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
        game: None,
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

/// `profile("photo")` exactly as Version 1.3 wrote it to disk.
const PHOTO_1_3: &str = r#"{
  "schema": 1,
  "id": "photo",
  "name": "Photo editing",
  "builtin": false,
  "hotkey": "Ctrl+Alt+2",
  "autoApps": [
    "Photoshop.exe"
  ],
  "bindings": {
    "F13": {
      "mods": [
        "Shift"
      ],
      "action": {
        "type": "launch_app",
        "app": {
          "kind": "bundle",
          "id": "com.apple.calculator"
        }
      }
    }
  }
}"#;

#[test]
fn profile_files_keep_the_1_3_format() {
    assert_eq!(serde_json::to_string_pretty(&profile("photo")).unwrap(), PHOTO_1_3);
    assert_eq!(serde_json::from_str::<Profile>(PHOTO_1_3).unwrap(), profile("photo"));
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
        ["F10", "F11", "F12", "F13", "F14", "F15", "F3", "F4", "F7", "F8", "F9"]
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
    // Command is the Windows key on Windows: left alone, a tap opens Start and Win+V, Win+C work
    assert!(!win.bindings.contains_key("MetaLeft") && !win.bindings.contains_key("MetaRight"));
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

fn game(name: &str, exe: &str) -> Game {
    Game { name: name.into(), exe: exe.into(), folder: None, icon: None, auto_switch: true }
}

#[test]
fn game_json_is_camel_case_and_switches_by_default() {
    assert!(serde_json::to_value(profile("photo")).unwrap().get("game").is_none());
    let p = Profile { game: Some(game("Portal 2", "portal2.exe")), ..profile("portal-2") };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["game"], json!({ "name": "Portal 2", "exe": "portal2.exe", "autoSwitch": true }));
    let g: Game = serde_json::from_value(json!({ "name": "Portal 2", "folder": r"G:\SteamLibrary\steamapps\common\Portal 2" })).unwrap();
    assert!(g.auto_switch && g.exe.is_empty());
}

#[test]
fn games_switch_unless_settings_say_otherwise() {
    assert!(Settings::default().game_auto_switch);
    assert_eq!(serde_json::to_value(Settings::default()).unwrap()["gameAutoSwitch"], true);
    let s: Settings = serde_json::from_str(r#"{ "gameAutoSwitch": false }"#).unwrap();
    assert!(!s.game_auto_switch);
}

#[test]
fn a_game_gets_its_own_profile_with_a_copy_of_the_base_bindings() {
    let st = temp_store("game-add");
    st.ensure_builtins().unwrap();
    let base = st.get_profile("windows-default").unwrap();
    let p = st.add_game(game("Portal 2", "portal2.exe"), "windows-default").unwrap();
    assert_eq!((p.id.as_str(), p.name.as_str()), ("portal-2", "Portal 2"));
    assert!(!p.builtin && p.hotkey.is_none() && p.auto_apps.is_empty());
    assert_eq!(p.bindings, base.bindings);
    assert_eq!(p.game, Some(game("Portal 2", "portal2.exe")));
    assert_eq!(st.get_profile("portal-2").unwrap(), p);
    // the same game again, by exe in any case: the profile it already has
    assert_eq!(st.add_game(game("Portal 2 again", "PORTAL2.EXE"), "macos-default").unwrap(), p);
    // a different game with the same name gets its own id
    let beta = st.add_game(game("Portal 2", "portal2_beta.exe"), "macos-default").unwrap();
    assert_eq!(beta.id, "portal-2-2");
    assert!(beta.bindings.is_empty());
    assert!(st.add_game(game("X", "x.exe"), "nope").is_err());
    assert_eq!(st.list_profiles().unwrap().len(), 4);
}

#[test]
fn a_folder_game_is_the_same_game_by_folder() {
    let st = temp_store("game-folder");
    st.ensure_builtins().unwrap();
    let folder = |dir: &str| Game { folder: Some(dir.into()), ..game("Portal 2", "") };
    let p = st.add_game(folder(r"G:\Steam\steamapps\common\Portal 2"), "windows-default").unwrap();
    assert_eq!(st.add_game(folder(r"g:\steam\steamapps\common\portal 2"), "windows-default").unwrap(), p);
    // two folder games never count as the same game through their empty exe
    assert_ne!(st.add_game(folder(r"G:\Steam\steamapps\common\Portal"), "windows-default").unwrap().id, p.id);
}

#[test]
fn only_the_game_commands_change_a_profiles_game() {
    let st = temp_store("game-keep");
    st.ensure_builtins().unwrap();
    let p = st.add_game(game("Portal 2", "portal2.exe"), "windows-default").unwrap();
    // the UI saves profiles without knowing about games
    st.save_profile(&Profile { game: None, name: "Portal".into(), ..p.clone() }).unwrap();
    let saved = st.get_profile("portal-2").unwrap();
    assert_eq!((saved.name.as_str(), &saved.game), ("Portal", &p.game));
    // a new profile cannot claim a game by being saved
    st.save_profile(&Profile { game: Some(game("Y", "y.exe")), ..profile("plain") }).unwrap();
    assert_eq!(st.get_profile("plain").unwrap().game, None);
    // a copy is a plain profile: two profiles never share a game
    assert_eq!(st.duplicate_profile("portal-2").unwrap().game, None);
}

#[test]
fn auto_switch_is_stored_per_game() {
    let st = temp_store("game-toggle");
    st.ensure_builtins().unwrap();
    st.add_game(game("Portal 2", "portal2.exe"), "windows-default").unwrap();
    let off = st.set_game_auto_switch("portal-2", false).unwrap();
    assert!(!off.game.unwrap().auto_switch);
    assert!(!st.get_profile("portal-2").unwrap().game.unwrap().auto_switch);
    assert!(st.set_game_auto_switch("portal-2", true).unwrap().game.unwrap().auto_switch);
    assert!(st.set_game_auto_switch("windows-default", false).is_err(), "not a game");
}

/// Every profile file, byte for byte.
fn snapshot(st: &Store) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for e in fs::read_dir(st.data_dir.join("profiles")).unwrap().flatten() {
        files.insert(format!("profiles/{}", e.file_name().to_string_lossy()), fs::read(e.path()).unwrap());
    }
    files
}

/// A data folder exactly as Version 1.3 left it: a Windows Default the user edited, the macOS
/// Default, a profile of their own with an app rule and a hotkey, and settings without games.
#[test]
fn upgrading_from_1_3_keeps_every_profile_and_binding() {
    let st = temp_store("upgrade-1-3");
    let calculator = Action::LaunchApp { app: AppRef::Aumid { id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into() } };
    let mut win = builtins().find(|p| p.id == "windows-default").unwrap();
    win.bindings.insert("F13".into(), Binding { mods: vec![], action: calculator });
    win.bindings.insert("MetaLeft".into(), Binding { mods: vec![], action: Action::KeyCombo { keys: vec!["Meta".into()] } });
    let mac = builtins().find(|p| p.id == "macos-default").unwrap();
    for p in [&win, &mac] {
        st.write(p).unwrap();
    }
    fs::write(st.data_dir.join("profiles").join("photo.json"), PHOTO_1_3).unwrap();
    let settings_1_3 = r#"{ "schema": 1, "activeProfile": "photo", "switchMode": "hotkey",
        "hotkeys": { "pause": "Ctrl+Alt+P", "nextProfile": "Ctrl+Alt+N", "prevProfile": "Ctrl+Alt+B", "show": "Ctrl+Alt+M" },
        "startAtLogin": false, "closeToTray": true, "runAsAdmin": false, "windowsLegends": false, "autoCheckUpdates": true }"#;
    fs::write(st.data_dir.join("settings.json"), settings_1_3).unwrap();
    let before = snapshot(&st);

    // first start of the new version
    st.ensure_builtins().unwrap();
    let profiles = st.list_profiles().unwrap();
    let settings = st.load_settings();

    assert_eq!(snapshot(&st), before, "no profile file is rewritten");
    assert_eq!(profiles, vec![mac, win.clone(), profile("photo")]);
    assert_eq!(profiles[1].bindings.len(), 12);
    assert!(profiles.iter().all(|p| p.game.is_none()));
    assert_eq!((settings.active_profile.as_str(), settings.switch_mode), ("photo", SwitchMode::Hotkey));
    assert_eq!(settings.hotkeys.next_profile, "Ctrl+Alt+N");
    assert!(!settings.start_at_login && !settings.windows_legends && settings.close_to_tray);
    assert!(settings.game_auto_switch, "games start out switching");

    // adding and toggling a game leaves every other profile alone
    st.add_game(game("Portal 2", "portal2.exe"), "windows-default").unwrap();
    st.set_game_auto_switch("portal-2", false).unwrap();
    let mut after = snapshot(&st);
    assert!(after.remove("profiles/portal-2.json").is_some());
    assert_eq!(after, before);
    assert_eq!(st.get_profile("windows-default").unwrap(), win);
}

#[test]
fn untouched_old_windows_default_moves_to_the_new_one() {
    let st = temp_store("migrate");
    let old: Profile = serde_json::from_str(include_str!("../../profiles/retired/windows-default-1.0.json")).unwrap();
    assert_eq!(old.bindings["MetaLeft"].action, Action::KeyCombo { keys: vec!["ControlLeft".into()] });
    st.write(&Profile { hotkey: Some("Ctrl+Alt+1".into()), ..old }).unwrap();
    st.ensure_builtins().unwrap();
    let win = st.get_profile("windows-default").unwrap();
    assert!(!win.bindings.contains_key("MetaLeft"), "Command is the Win key again");
    assert_eq!(win.bindings.len(), 11);
    assert_eq!(win.hotkey.as_deref(), Some("Ctrl+Alt+1"), "only the bindings change");
}

#[test]
fn edited_old_windows_default_is_left_alone() {
    let st = temp_store("migrate-edited");
    let mut old: Profile = serde_json::from_str(include_str!("../../profiles/retired/windows-default-1.0.json")).unwrap();
    old.bindings.remove("F14");
    st.write(&old).unwrap();
    st.ensure_builtins().unwrap();
    let win = st.get_profile("windows-default").unwrap();
    assert!(win.bindings.contains_key("MetaLeft"), "the user's own edits win over a new default");
    assert!(!win.bindings.contains_key("F14"));
}
