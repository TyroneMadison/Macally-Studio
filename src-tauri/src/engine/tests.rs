use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use super::*;
use crate::actions::AppRef;
use crate::profiles::Binding;

#[derive(Default)]
struct Fake {
    sent: Mutex<Vec<(Vec<String>, bool)>>,
    lifts: Mutex<Vec<ModMask>>,
    masks: Mutex<usize>,
}

impl Fake {
    fn masked(&self) -> usize {
        *self.masks.lock().unwrap()
    }
}

impl Platform for Arc<Fake> {
    fn send_combo(&self, keys: &[String], down: bool, lift: ModMask) {
        self.sent.lock().unwrap().push((keys.to_vec(), down));
        self.lifts.lock().unwrap().push(lift);
    }
    fn can_send(&self, key: &str) -> bool {
        key != "BrightnessUp" // like Windows: no key to inject
    }
    fn swallowed_press(&self) {
        *self.masks.lock().unwrap() += 1;
    }
}

fn profile(bindings: &[(&str, &[Modifier], Action)]) -> Profile {
    let mut map = BTreeMap::new();
    for (code, mods, action) in bindings {
        map.insert(code.to_string(), Binding { mods: mods.to_vec(), action: action.clone() });
    }
    Profile { schema: 1, id: "t".into(), name: "T".into(), builtin: false, hotkey: None, auto_apps: vec![], bindings: map, game: None }
}

fn keys(k: &[&str]) -> Vec<String> {
    k.iter().map(|s| s.to_string()).collect()
}

fn combo(k: &[&str]) -> Action {
    Action::KeyCombo { keys: keys(k) }
}

fn calc() -> Action {
    Action::LaunchApp { app: AppRef::Aumid { id: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into() } }
}

fn engine(p: &Profile) -> (Engine, Arc<Fake>, Receiver<EngineMsg>) {
    let fake = Arc::new(Fake::default());
    let (tx, rx) = mpsc::channel();
    let e = Engine::new(Box::new(fake.clone()), tx);
    e.set_profile(p);
    (e, fake, rx)
}

fn sent(f: &Fake) -> Vec<(Vec<String>, bool)> {
    f.sent.lock().unwrap().clone()
}

#[test]
fn table_expands_mods_and_resolves_aliases() {
    let p = profile(&[
        ("F3", &[], combo(&["Win", "Tab"])),
        ("F13", &[Modifier::Shift, Modifier::Ctrl], Action::PassThrough),
    ]);
    let t = Table::from_profile(&p);
    assert_eq!(t.lookup("F3", 0), Some(&combo(&["MetaLeft", "Tab"])));
    assert_eq!(t.lookup("F3", SHIFT), None);
    assert_eq!(t.lookup("F13", CTRL | SHIFT), Some(&Action::PassThrough));
    assert_eq!(t.lookup("F13", SHIFT), None);
    assert_eq!(t.bound_codes(), keys(&["F13", "F3"]));
}

#[test]
fn key_combo_is_pressed_on_down_and_released_on_up() {
    let (e, fake, _rx) = engine(&profile(&[("F3", &[], combo(&["Win", "Tab"]))]));
    assert!(e.handle_key("F3", 0, true, ""));
    assert!(e.handle_key("F3", 0, false, ""));
    assert_eq!(sent(&fake), vec![(keys(&["MetaLeft", "Tab"]), true), (keys(&["MetaLeft", "Tab"]), false)]);
}

#[test]
fn combo_the_os_cannot_send_passes_the_key_through() {
    let (e, fake, _rx) = engine(&profile(&[("F2", &[], combo(&["BrightnessUp"])), ("F7", &[], combo(&["MediaPlayPause"]))]));
    assert!(!e.handle_key("F2", 0, true, ""));
    assert!(!e.handle_key("F2", 0, false, ""));
    assert!(e.handle_key("F7", 0, true, ""));
    assert_eq!(sent(&fake), vec![(keys(&["MediaPlayPause"]), true)]);
}

#[test]
fn unbound_wrong_mods_and_pass_through_are_not_swallowed() {
    let (e, fake, _rx) = engine(&profile(&[("F3", &[], combo(&["KeyA"])), ("F4", &[], Action::PassThrough)]));
    assert!(!e.handle_key("KeyQ", 0, true, ""));
    assert!(!e.handle_key("F3", SHIFT, true, ""));
    assert!(!e.handle_key("F4", 0, true, ""));
    assert!(!e.handle_key("F4", 0, false, ""));
    assert!(sent(&fake).is_empty());
}

#[test]
fn up_releases_what_down_pressed_and_repeat_reuses_it() {
    let (e, fake, _rx) = engine(&profile(&[("MetaLeft", &[], combo(&["ControlLeft"]))]));
    assert!(e.handle_key("MetaLeft", 0, true, ""));
    // auto-repeat now sees our injected Ctrl as held; the key is still ours
    assert!(e.handle_key("MetaLeft", CTRL, true, ""));
    assert!(e.handle_key("MetaLeft", CTRL, false, ""));
    let s = sent(&fake);
    assert_eq!(s.len(), 3);
    assert_eq!(s[2], (keys(&["ControlLeft"]), false));
}

#[test]
fn remapped_modifier_applies_with_other_mods_held() {
    let (e, fake, _rx) = engine(&profile(&[("MetaLeft", &[], combo(&["ControlLeft"]))]));
    assert!(e.handle_key("MetaLeft", SHIFT | META, true, ""));
    assert_eq!(sent(&fake), vec![(keys(&["ControlLeft"]), true)]);
}

#[test]
fn held_with_modifiers_the_chord_does_not_press_are_lifted() {
    let (e, fake, _rx) = engine(&profile(&[
        ("F13", &[Modifier::Shift], combo(&["Ctrl", "KeyZ"])),
        ("F14", &[Modifier::Shift, Modifier::Ctrl], combo(&["Ctrl", "KeyY"])),
        ("MetaLeft", &[], combo(&["ControlLeft"])),
    ]));
    assert!(e.handle_key("F13", SHIFT, true, ""));
    assert!(e.handle_key("F13", 0, true, "")); // auto-repeat: Shift now reads as up
    assert!(e.handle_key("F13", 0, false, ""));
    assert!(e.handle_key("F14", SHIFT | CTRL, true, ""));
    assert!(e.handle_key("MetaLeft", SHIFT, true, "")); // a remapped modifier keeps other mods
    assert_eq!(*fake.lifts.lock().unwrap(), vec![SHIFT, SHIFT, SHIFT, SHIFT, 0]);
}

#[test]
fn remapped_mods_reports_taken_modifiers_and_chord_modifiers() {
    let (e, _fake, _rx) = engine(&profile(&[("MetaLeft", &[], combo(&["ControlLeft"]))]));
    assert_eq!(e.remapped_mods(), (0, 0));
    assert!(e.handle_key("MetaLeft", 0, true, ""));
    assert_eq!(e.remapped_mods(), (META, CTRL));
    assert!(e.handle_key("MetaLeft", CTRL, false, ""));
    assert_eq!(e.remapped_mods(), (0, 0));
}

#[test]
fn paused_passes_everything_except_pause_toggle() {
    let (e, fake, rx) = engine(&profile(&[("F3", &[], combo(&["KeyA"])), ("F15", &[], Action::PauseToggle)]));
    e.set_paused(true);
    assert!(e.is_paused());
    assert!(!e.handle_key("F3", 0, true, ""));
    assert!(!e.handle_key("F3", 0, false, ""));
    assert!(sent(&fake).is_empty());
    assert!(e.handle_key("F15", 0, true, ""));
    assert_eq!(rx.try_recv(), Ok(EngineMsg::Run(Action::PauseToggle)));
}

#[test]
fn reserved_hotkey_chords_pass_through() {
    let (e, _fake, _rx) = engine(&profile(&[("KeyP", &[Modifier::Ctrl, Modifier::Alt], combo(&["KeyA"]))]));
    e.set_reserved(vec![("KeyP".into(), CTRL | ALT)]);
    assert!(!e.handle_key("KeyP", CTRL | ALT, true, ""));
}

#[test]
fn other_actions_go_to_the_worker_once_per_press() {
    let (e, fake, rx) = engine(&profile(&[("F14", &[], calc())]));
    assert!(e.handle_key("F14", 0, true, ""));
    assert!(e.handle_key("F14", 0, true, "")); // auto-repeat
    assert!(e.handle_key("F14", 0, false, ""));
    assert_eq!(rx.try_recv(), Ok(EngineMsg::Run(calc())));
    assert!(rx.try_recv().is_err());
    assert!(sent(&fake).is_empty());
}

#[test]
fn debounce_window_restarts_on_every_down() {
    let (e, _fake, rx) = engine(&profile(&[("F14", &[], calc())]));
    let t0 = Instant::now();
    let press = |at: Duration| {
        assert!(e.handle_at("F14", 0, true, "", t0 + at));
        assert!(e.handle_at("F14", 0, false, "", t0 + at));
    };
    press(Duration::ZERO);
    press(Duration::from_millis(300)); // inside 400 ms: dropped, window restarts
    press(Duration::from_millis(650)); // 350 ms after the last press: still dropped
    press(Duration::from_millis(1100)); // 450 ms after the last press: fires
    assert_eq!(rx.try_iter().count(), 2);
}

#[test]
fn monitor_reports_and_keeps_remapping() {
    let (e, fake, rx) = engine(&profile(&[("F3", &[], combo(&["KeyA"]))]));
    e.set_mode(Mode::Monitor);
    assert!(!e.handle_key("KeyQ", SHIFT, true, "sc 0x10"));
    assert_eq!(
        rx.try_recv(),
        Ok(EngineMsg::Key(KeyEvent { code: "KeyQ".into(), mods: keys(&["Shift"]), down: true, raw: "sc 0x10".into() }))
    );
    assert!(e.handle_key("F3", 0, true, "sc 0x3D"));
    assert_eq!(sent(&fake).len(), 1);
}

#[test]
fn macally_device_matchers() {
    assert!(is_macally_path(r"\\?\hid#vid_2222&pid_0114&mi_00#7&1b2c3d4&0&0000#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}"));
    assert!(!is_macally_path(r"\\?\HID#VID_05AC&PID_0250&MI_00#8&abc#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}"));
    let ioreg = "+-o USB Backlit Keyboard@14100000\n  \"idProduct\" = 276\n  \"idVendor\" = 8738\n+-o Magic Mouse@1\n  \"idProduct\" = 597\n";
    assert!(ioreg_has_macally(ioreg));
    assert!(!ioreg_has_macally("+-o A@1\n  \"idVendor\" = 8738\n+-o B@2\n  \"idProduct\" = 276\n"));
}

// --- the Windows key -------------------------------------------------------------------

#[test]
fn shipped_windows_default_leaves_command_as_the_win_key() {
    // the 1.0/1.1 default mapped Command (LWin/RWin) to Ctrl, so the hook swallowed every Win press
    let win = crate::profiles::builtins().find(|p| p.id == "windows-default").unwrap();
    let (e, fake, _rx) = engine(&win);
    for code in ["MetaLeft", "MetaRight"] {
        assert!(!e.handle_key(code, 0, true, ""), "{code} down must reach Windows");
        assert!(!e.handle_key(code, 0, false, ""), "{code} up must reach Windows");
    }
    // Win+V on the real Win key: V is unbound, so Windows gets its own chord
    assert!(!e.handle_key("KeyV", META, true, ""));
    assert!(!e.handle_key("KeyV", META, false, ""));
    assert!(sent(&fake).is_empty());
}

#[test]
fn key_remapped_to_win_holds_win_so_chords_work() {
    let (e, fake, _rx) = engine(&profile(&[("ContextMenu", &[], combo(&["Win"]))]));
    assert!(e.handle_key("ContextMenu", 0, true, "")); // Win down
    assert!(e.handle_key("ContextMenu", META, true, "")); // auto-repeat: still ours
    assert!(!e.handle_key("KeyV", META, true, "")); // V reaches Windows: Win+V
    assert!(!e.handle_key("KeyV", META, false, ""));
    assert!(e.handle_key("ContextMenu", META, false, "")); // Win up, once
    assert_eq!(
        sent(&fake),
        vec![(keys(&["MetaLeft"]), true), (keys(&["MetaLeft"]), true), (keys(&["MetaLeft"]), false)]
    );
    assert_eq!(*fake.lifts.lock().unwrap(), vec![0, 0, 0]);
    assert_eq!(fake.masked(), 0, "a tap of the Win remap on its own must still open Start");
}

#[test]
fn key_remapped_to_win_works_with_other_modifiers_already_held() {
    let (e, fake, _rx) = engine(&profile(&[("F13", &[], combo(&["Win"])), ("AltLeft", &[], combo(&["Win"]))]));
    assert!(e.handle_key("F13", SHIFT, true, "")); // Shift first, then F13: Win+Shift
    assert!(e.handle_key("AltLeft", CTRL, true, "")); // option remapped to Win, Ctrl held
    assert_eq!(sent(&fake), vec![(keys(&["MetaLeft"]), true), (keys(&["MetaLeft"]), true)]);
    assert_eq!(*fake.lifts.lock().unwrap(), vec![0, 0], "held Shift and Ctrl stay in the chord");
}

#[test]
fn win_chord_binding_and_identity_remap() {
    let (e, fake, _rx) =
        engine(&profile(&[("F13", &[], combo(&["Win", "KeyV"])), ("MetaLeft", &[], combo(&["Win"]))]));
    assert!(e.handle_key("F13", 0, true, ""));
    assert!(e.handle_key("F13", 0, false, ""));
    assert!(e.handle_key("MetaLeft", 0, true, ""));
    assert!(e.handle_key("MetaLeft", META, false, ""));
    assert_eq!(
        sent(&fake),
        vec![
            (keys(&["MetaLeft", "KeyV"]), true),
            (keys(&["MetaLeft", "KeyV"]), false),
            (keys(&["MetaLeft"]), true),
            (keys(&["MetaLeft"]), false),
        ]
    );
}

#[test]
fn swallowed_presses_are_reported_once_per_press() {
    // Win held + F14 -> Calculator: Windows never sees F14, so Start would open on Win up
    let (e, fake, _rx) = engine(&profile(&[("F14", &[Modifier::Meta], calc()), ("F3", &[], combo(&["Win", "Tab"]))]));
    assert!(e.handle_key("F14", META, true, ""));
    assert!(e.handle_key("F14", META, true, "")); // auto-repeat
    assert!(e.handle_key("F14", META, false, ""));
    assert!(e.handle_key("F3", 0, true, ""));
    assert_eq!(fake.masked(), 2);
}
