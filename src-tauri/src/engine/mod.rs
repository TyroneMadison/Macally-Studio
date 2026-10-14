//! Remap engine: the lookup table built from the active profile, the modes, and the one
//! decision every platform hook asks for (`handle_key`: swallow or pass through).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::actions::Action;
use crate::commands::EngineLike;
use crate::keymap;
use crate::profiles::{Modifier, Profile};

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
/// Record mode ends by itself after this long, so a UI that never calls stop_record
/// cannot leave the whole keyboard swallowed.
pub const RECORD_TIMEOUT: Duration = Duration::from_secs(15);

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
    Record,
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
    fn send_combo(&self, keys: &[String], down: bool);
    /// False for a key this OS has no way to inject (brightness on Windows).
    fn can_send(&self, _key: &str) -> bool {
        true
    }
}

/// Per-key state; one lock, never held while injecting.
#[derive(Default)]
struct Live {
    /// Keys whose down we acted on: their auto-repeat and key-up go to the same action.
    held: HashMap<String, Action>,
    last_fire: HashMap<String, Instant>,
    /// Record swallows modifiers, so the OS never sees them held; Record tracks them itself.
    record_mods: ModMask,
    record_since: Option<Instant>,
    /// App hotkey chords; they always pass through so the hotkey still fires (spec section 11).
    reserved: HashSet<(String, ModMask)>,
}

pub struct Engine {
    platform: Box<dyn Platform>,
    tx: Sender<EngineMsg>,
    table: RwLock<Table>,
    paused: AtomicBool,
    mode: AtomicU8,
    live: Mutex<Live>,
}

impl Engine {
    pub fn new(platform: Box<dyn Platform>, tx: Sender<EngineMsg>) -> Engine {
        Engine {
            platform,
            tx,
            table: RwLock::new(Table::default()),
            paused: AtomicBool::new(false),
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

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn set_mode(&self, m: Mode) {
        let mut live = self.live.lock().unwrap();
        live.record_mods = 0;
        live.record_since = (m == Mode::Record).then(Instant::now);
        self.mode.store(m as u8, Ordering::SeqCst);
    }

    pub fn mode(&self) -> Mode {
        match self.mode.load(Ordering::SeqCst) {
            1 => Mode::Record,
            2 => Mode::Monitor,
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
        let mut mode = self.mode();
        if mode == Mode::Record && live.record_since.is_some_and(|t| now.duration_since(t) >= RECORD_TIMEOUT) {
            live.record_since = None;
            self.mode.store(Mode::Normal as u8, Ordering::SeqCst);
            mode = Mode::Normal;
        }
        match mode {
            Mode::Record => {
                let before = live.record_mods;
                live.record_mods = if down { before | bit } else { before & !bit };
                self.emit(code, (mods | before) & !bit, down, raw);
            }
            Mode::Monitor => self.emit(code, mods, down, raw),
            Mode::Normal => {}
        }
        if !down {
            // release exactly what the matching key-down did, whatever changed since
            let released = live.held.remove(code);
            drop(live);
            if let Some(Action::KeyCombo { keys }) = &released {
                self.platform.send_combo(keys, false);
            }
            return released.is_some() || mode == Mode::Record;
        }
        if mode == Mode::Record {
            return true;
        }
        let action = match live.held.get(code) {
            Some(a) => a.clone(), // auto-repeat of a key we already own
            None => {
                if live.reserved.contains(&(code.to_string(), mods)) {
                    return false;
                }
                let table = self.table.read().unwrap();
                // a remapped modifier (Cmd -> Ctrl) applies whatever else is held
                let hit = table.lookup(code, mods).or_else(|| if bit != 0 { table.lookup(code, 0) } else { None });
                match hit {
                    None | Some(Action::PassThrough) => return false,
                    // paused: only a PauseToggle key still works, or nothing could unpause
                    Some(a) if self.is_paused() && !matches!(a, Action::PauseToggle) => return false,
                    Some(a) => a.clone(),
                }
            }
        };
        let fire = matches!(action, Action::KeyCombo { .. }) || debounce(&mut live.last_fire, code, now);
        live.held.insert(code.to_string(), action.clone());
        drop(live);
        match action {
            Action::KeyCombo { keys } => self.platform.send_combo(&keys, true),
            other if fire => {
                let _ = self.tx.send(EngineMsg::Run(other));
            }
            _ => {}
        }
        true
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
    fn set_paused(&self, b: bool) {
        Engine::set_paused(self, b)
    }
    fn is_paused(&self) -> bool {
        Engine::is_paused(self)
    }
}
