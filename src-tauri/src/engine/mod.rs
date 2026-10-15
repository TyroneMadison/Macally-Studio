//! Remap engine: the lookup table built from the active profile, the modes, and the one
//! decision every platform hook asks for (`handle_key`: swallow or pass through).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::actions::Action;
use crate::commands::EngineLike;
use crate::keymap;
use crate::profiles::{Modifier, Profile};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
use self::macos as os;
#[cfg(target_os = "windows")]
use self::windows as os;

#[cfg(test)]
mod tests;

pub type ModMask = u8; // bit0 Ctrl, bit1 Alt, bit2 Shift, bit3 Meta
pub const CTRL: ModMask = 1;
pub const ALT: ModMask = 2;
pub const SHIFT: ModMask = 4;
pub const META: ModMask = 8;

/// Launches and other side effects fire at most once per key inside this window; every
/// key-down restarts it, so a held key with auto-repeat fires once.
pub const DEBOUNCE: Duration = Duration::from_millis(400);

/// Set by the platform `install` once the OS hook is live; read by `get_status`.
pub static HOOK_RUNNING: AtomicBool = AtomicBool::new(false);

/// USB ids of the Macally BLSLIMKEYPROSG ("USB Backlit Keyboard"), observed on hardware.
pub const MACALLY_VID: u16 = 0x2222;
pub const MACALLY_PID: u16 = 0x0114;

pub fn mask_of(mods: &[Modifier]) -> ModMask {
    mods.iter().fold(0, |m, x| {
        m | match x {
            Modifier::Ctrl => CTRL,
            Modifier::Alt => ALT,
            Modifier::Shift => SHIFT,
            Modifier::Meta => META,
        }
    })
}

/// The ModMask bit a modifier key sets; 0 for every other key.
pub fn mod_bit(code: &str) -> ModMask {
    match code {
        "ControlLeft" | "ControlRight" => CTRL,
        "AltLeft" | "AltRight" => ALT,
        "ShiftLeft" | "ShiftRight" => SHIFT,
        "MetaLeft" | "MetaRight" => META,
        _ => 0,
    }
}

/// The modifiers a chord presses itself.
fn combo_mask(keys: &[String]) -> ModMask {
    keys.iter().fold(0, |m, k| m | mod_bit(k))
}

/// A KeyCombo of modifiers only (F13 -> Win, Cmd -> Ctrl): the key becomes that modifier.
fn is_modifier_chord(a: &Action) -> bool {
    matches!(a, Action::KeyCombo { keys } if !keys.is_empty() && keys.iter().all(|k| mod_bit(k) != 0))
}

/// "Ctrl", "Alt", "Shift", "Meta": the Modifier names, also valid KeyCombo aliases.
pub fn mod_names(m: ModMask) -> Vec<String> {
    [(CTRL, "Ctrl"), (ALT, "Alt"), (SHIFT, "Shift"), (META, "Meta")]
        .iter()
        .filter(|(bit, _)| m & bit != 0)
        .map(|(_, name)| name.to_string())
        .collect()
}

/// Windows Raw Input device path check, e.g. `\\?\HID#VID_2222&PID_0114&MI_00#...`.
pub fn is_macally_path(path: &str) -> bool {
    path.to_ascii_uppercase().contains(&format!("VID_{MACALLY_VID:04X}&PID_{MACALLY_PID:04X}"))
}

/// `ioreg -p IOUSB -l -w0` output check: both ids (printed in decimal) inside one device block.
pub fn ioreg_has_macally(text: &str) -> bool {
    let vid = format!("\"idVendor\" = {MACALLY_VID}");
    let pid = format!("\"idProduct\" = {MACALLY_PID}");
    text.split("+-o ").any(|dev| dev.contains(&vid) && dev.contains(&pid))
}

#[derive(Default)]
pub struct Table {
    map: HashMap<(String, ModMask), Action>,
}

impl Table {
    /// Modifier lists become masks; KeyCombo aliases ("Ctrl", "Win") become key codes now,
    /// so the hook never has to resolve them.
    pub fn from_profile(p: &Profile) -> Table {
        let map = p
            .bindings
            .iter()
            .map(|(code, b)| {
                let action = match &b.action {
                    Action::KeyCombo { keys } => Action::KeyCombo {
                        keys: keys.iter().map(|k| keymap::resolve_alias(k).to_string()).collect(),
                    },
                    other => other.clone(),
                };
                ((keymap::resolve_alias(code).to_string(), mask_of(&b.mods)), action)
            })
            .collect();
        Table { map }
    }

    // ponytail: allocates a String per lookup; fine at keystroke rates.
    pub fn lookup(&self, code: &str, mods: ModMask) -> Option<&Action> {
        self.map.get(&(code.to_string(), mods))
    }

    pub fn bound_codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = self.map.keys().map(|(c, _)| c.clone()).collect();
        codes.sort();
        codes.dedup();
        codes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    /// Key Monitor: every key also goes to the UI as "key-event"; remapping keeps working.
    Monitor,
}

/// Emitted to the UI as the Tauri event "key-event".
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KeyEvent {
    pub code: String,
    pub mods: Vec<String>,
    pub down: bool,
    pub raw: String,
}

/// Work the hook hands to the worker thread, so the hook itself returns in microseconds.
#[derive(Debug, PartialEq)]
pub enum EngineMsg {
    Key(KeyEvent),
    Run(Action),
}

/// What the engine needs from the OS: inject a chord. Tests pass a fake.
pub trait Platform: Send + Sync {
    /// `lift`: modifiers held for the binding ("Only when held with") that the chord does not
    /// press itself; the OS layer keeps them out of the chord if they would otherwise leak in.
    fn send_combo(&self, keys: &[String], down: bool, lift: ModMask);
    /// False for a key this OS has no way to inject (brightness on Windows).
    fn can_send(&self, _key: &str) -> bool {
        true
    }
    /// A key press was swallowed. Windows opens Start (Win) or the menu bar (Alt) when one of
    /// those goes down and up with no key the OS saw in between, so it must see one.
    fn swallowed_press(&self) {}
}

/// Per-key state; one lock, never held while injecting.
#[derive(Default)]
struct Live {
    /// Keys whose down we acted on, with their lift: auto-repeat and key-up reuse both.
    held: HashMap<String, (Action, ModMask)>,
    last_fire: HashMap<String, Instant>,
    /// App hotkey chords; they always pass through so the hotkey still fires (spec section 11).
    reserved: HashSet<(String, ModMask)>,
}

pub struct Engine {
    platform: Box<dyn Platform>,
    tx: Sender<EngineMsg>,
    table: RwLock<Table>,
    mode: AtomicU8,
    live: Mutex<Live>,
}

impl Engine {
    pub fn new(platform: Box<dyn Platform>, tx: Sender<EngineMsg>) -> Engine {
        Engine {
            platform,
            tx,
            table: RwLock::new(Table::default()),
            mode: AtomicU8::new(Mode::Normal as u8),
            live: Mutex::new(Live::default()),
        }
    }

    /// A KeyCombo this OS cannot inject is left out, so its key keeps working as itself
    /// instead of going dead. Logged here, once per profile load, never per key press.
    pub fn set_profile(&self, p: &Profile) {
        let mut t = Table::from_profile(p);
        t.map.retain(|(code, _), action| {
            let Action::KeyCombo { keys } = action else { return true };
            match keys.iter().find(|k| !self.platform.can_send(k)) {
                Some(k) => {
                    eprintln!("{}: {code} -> {k} cannot be sent on this OS; the key passes through", p.id);
                    false
                }
                None => true,
            }
        });
        *self.table.write().unwrap() = t;
    }

    pub fn set_mode(&self, m: Mode) {
        self.mode.store(m as u8, Ordering::SeqCst);
    }

    pub fn mode(&self) -> Mode {
        match self.mode.load(Ordering::SeqCst) {
            1 => Mode::Monitor,
            _ => Mode::Normal,
        }
    }

    pub fn set_reserved(&self, chords: Vec<(String, ModMask)>) {
        self.live.lock().unwrap().reserved = chords.into_iter().collect();
    }

    /// Called by the platform hook for every physical key event. Returns true to swallow it.
    pub fn handle_key(&self, code: &str, mods: ModMask, down: bool, raw: &str) -> bool {
        self.handle_at(code, mods, down, raw, Instant::now())
    }

    fn handle_at(&self, code: &str, mods: ModMask, down: bool, raw: &str, now: Instant) -> bool {
        let bit = mod_bit(code);
        let mods = mods & !bit; // a modifier never counts as "held with" itself
        let mut live = self.live.lock().unwrap();
        if self.mode() == Mode::Monitor {
            self.emit(code, mods, down, raw);
        }
        if !down {
            // release exactly what the matching key-down did, whatever changed since
            let released = live.held.remove(code);
            drop(live);
            if let Some((Action::KeyCombo { keys }, lift)) = &released {
                self.platform.send_combo(keys, false, *lift);
            }
            return released.is_some();
        }
        let (action, lift) = match live.held.get(code) {
            Some(h) => h.clone(), // auto-repeat of a key we already own
            None => {
                if live.reserved.contains(&(code.to_string(), mods)) {
                    return false;
                }
                let table = self.table.read().unwrap();
                // a remapped modifier (Cmd -> Ctrl), or a key remapped to one (F13 -> Win), works
                // whatever else is held, in any order, so it lifts nothing
                let (hit, with) = match table.lookup(code, mods) {
                    None => match table.lookup(code, 0) {
                        Some(a) if bit != 0 || is_modifier_chord(a) => (Some(a), 0),
                        _ => (None, mods),
                    },
                    hit => (hit, mods),
                };
                let action = match hit {
                    None | Some(Action::PassThrough) => return false,
                    Some(a) => a.clone(),
                };
                let lift = match &action {
                    Action::KeyCombo { keys } => with & !combo_mask(keys),
                    _ => 0,
                };
                (action, lift)
            }
        };
        let first = !live.held.contains_key(code);
        let fire = matches!(action, Action::KeyCombo { .. }) || debounce(&mut live.last_fire, code, now);
        live.held.insert(code.to_string(), (action.clone(), lift));
        drop(live);
        // a key that becomes a modifier is not a key in between for its own Win or Alt
        if first && !is_modifier_chord(&action) {
            self.platform.swallowed_press();
        }
        match action {
            Action::KeyCombo { keys } => self.platform.send_combo(&keys, true, lift),
            other if fire => {
                let _ = self.tx.send(EngineMsg::Run(other));
            }
            _ => {}
        }
        true
    }

    /// What the held remaps stand for: (modifier keys the engine took over, modifiers its held
    /// chords press). The macOS tap fixes event flags with it; there they follow the physical keys.
    pub fn remapped_mods(&self) -> (ModMask, ModMask) {
        let live = self.live.lock().unwrap();
        live.held.iter().fold((0, 0), |(taken, pressed), (code, (action, _))| {
            let chord = if let Action::KeyCombo { keys } = action { combo_mask(keys) } else { 0 };
            (taken | mod_bit(code), pressed | chord)
        })
    }

    fn emit(&self, code: &str, mods: ModMask, down: bool, raw: &str) {
        let ev = KeyEvent { code: code.to_string(), mods: mod_names(mods), down, raw: raw.to_string() };
        let _ = self.tx.send(EngineMsg::Key(ev));
    }
}

/// True when `code` may fire at `now`; every call restarts that key's window.
fn debounce(last: &mut HashMap<String, Instant>, code: &str, now: Instant) -> bool {
    let fire = last.get(code).is_none_or(|t| now.duration_since(*t) >= DEBOUNCE);
    last.insert(code.to_string(), now);
    fire
}

impl EngineLike for Engine {
    fn set_profile(&self, p: &Profile) {
        Engine::set_profile(self, p)
    }
}

/// Injects through the real OS API.
struct OsPlatform;

impl Platform for OsPlatform {
    fn send_combo(&self, keys: &[String], down: bool, lift: ModMask) {
        os::send_combo(keys, down, lift);
    }
    fn can_send(&self, key: &str) -> bool {
        os::can_send(key)
    }
    fn swallowed_press(&self) {
        os::swallowed_press();
    }
}

/// Creates the engine, its worker thread (UI events + actions) and the platform hook.
pub fn start(app: AppHandle) -> Arc<Engine> {
    let (tx, rx) = mpsc::channel();
    let engine = Arc::new(Engine::new(Box::new(OsPlatform), tx));
    std::thread::Builder::new()
        .name("engine-worker".into())
        .spawn(move || {
            for msg in rx {
                match msg {
                    EngineMsg::Key(ev) => {
                        let _ = app.emit("key-event", ev);
                    }
                    EngineMsg::Run(action) => {
                        if let Err(e) = crate::actions::run(&action, &app) {
                            eprintln!("action failed: {e:#}");
                        }
                    }
                }
            }
        })
        .expect("spawn engine worker");
    os::install(engine.clone());
    engine
}

/// macOS: Accessibility and Input Monitoring are both on. Always true on Windows.
pub fn permissions_ok() -> bool {
    os::permissions_granted()
}

pub fn hook_running() -> bool {
    HOOK_RUNNING.load(Ordering::SeqCst) && permissions_ok()
}

/// The Macally (USB VID 0x2222, PID 0x0114) is plugged in.
pub fn device_found() -> bool {
    os::device_found()
}
