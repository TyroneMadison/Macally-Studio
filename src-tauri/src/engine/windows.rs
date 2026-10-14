//! Windows: WH_KEYBOARD_LL hook on its own message-loop thread, SendInput for chords,
//! and the Raw Input scan that finds the Macally (VID 0x2222 / PID 0x0114).

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN,
};
use windows::Win32::UI::Input::{
    GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME, RIM_TYPEKEYBOARD,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED,
    LLKHF_UP, MSG, SetWindowsHookExW, TranslateMessage, WH_KEYBOARD_LL,
};

use super::{ALT, CTRL, Engine, HOOK_RUNNING, META, ModMask, SHIFT};
use crate::keymap::{self, KeyDef};

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();

/// Injected events the hook let through without asking the engine (read by hook-selftest).
pub static INJECTED_SEEN: AtomicU32 = AtomicU32::new(0);

/// Left and right modifier keys; a key's index is its bit in LIFTED.
const MOD_KEYS: [(&str, VIRTUAL_KEY, ModMask); 8] = [
    ("ControlLeft", VK_LCONTROL, CTRL),
    ("ControlRight", VK_RCONTROL, CTRL),
    ("AltLeft", VK_LMENU, ALT),
    ("AltRight", VK_RMENU, ALT),
    ("ShiftLeft", VK_LSHIFT, SHIFT),
    ("ShiftRight", VK_RSHIFT, SHIFT),
    ("MetaLeft", VK_LWIN, META),
    ("MetaRight", VK_RWIN, META),
];
/// Alt and Win: down then up with no key between opens the menu bar or the Start menu.
const MENU_KEYS: u8 = 0b1100_1100;
/// Unassigned virtual key, tapped next to Alt or Win so neither opens a menu.
const VK_MASK: u16 = 0xE8;

/// Held modifiers a chord released so they do not leak into it. They are pressed again when
/// the chord ends, unless the user let go of them meanwhile. Only the hook thread uses it.
static LIFTED: AtomicU8 = AtomicU8::new(0);

/// MOD_KEYS bits of the keys for which `f(code, vk, mod)` is true.
fn mod_bits(f: impl Fn(&str, VIRTUAL_KEY, ModMask) -> bool) -> u8 {
    MOD_KEYS.iter().enumerate().filter(|(_, (c, vk, m))| f(c, *vk, *m)).fold(0, |b, (i, _)| b | 1 << i)
}

pub fn install(engine: Arc<Engine>) {
    if ENGINE.set(engine).is_err() {
        return; // already installed
    }
    std::thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(|| {
            // hmod None + thread 0: a global low-level hook, called through this thread's message loop
            if let Err(e) = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) } {
                eprintln!("SetWindowsHookExW failed: {e}");
                return;
            }
            HOOK_RUNNING.store(true, Ordering::SeqCst);
            let mut msg = MSG::default();
            while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            HOOK_RUNNING.store(false, Ordering::SeqCst);
        })
        .expect("spawn keyboard hook thread");
}

/// Must return fast: Windows silently removes a hook that blocks (LowLevelHooksTimeout).
/// No I/O here; actions go to the engine's worker thread.
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if kb.flags.0 & LLKHF_INJECTED.0 != 0 {
            // our own SendInput (or another tool's): never re-handle, that is what breaks the loop
            INJECTED_SEEN.fetch_add(1, Ordering::Relaxed);
        } else if let Some(engine) = ENGINE.get() {
            let ext = kb.flags.0 & LLKHF_EXTENDED.0 != 0;
            let down = kb.flags.0 & LLKHF_UP.0 == 0;
            let (scan, vk) = (kb.scanCode as u16, kb.vkCode as u16);
            if !down {
                // the user let go of a modifier: a chord ending later must not press it again
                LIFTED.fetch_and(!mod_bits(|_, v, _| v.0 == vk), Ordering::SeqCst);
            }
            let code = keymap::code_from_win(scan, ext, vk).unwrap_or("Unidentified");
            let raw = format!("sc 0x{scan:02X}{} vk 0x{vk:02X}", if ext { " e0" } else { "" });
            if engine.handle_key(code, read_mods(), down, &raw) {
                return LRESULT(1); // swallow
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn is_down(vk: VIRTUAL_KEY) -> bool {
    (unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000) != 0
}

/// Modifiers held right now. The key that fired the hook is not updated yet; the engine
/// masks a modifier key's own bit anyway.
fn read_mods() -> ModMask {
    let mut m = 0;
    if is_down(VK_LCONTROL) || is_down(VK_RCONTROL) {
        m |= CTRL;
    }
    if is_down(VK_LMENU) || is_down(VK_RMENU) {
        m |= ALT;
    }
    if is_down(VK_LSHIFT) || is_down(VK_RSHIFT) {
        m |= SHIFT;
    }
    if is_down(VK_LWIN) || is_down(VK_RWIN) {
        m |= META;
    }
    m
}

/// Down: keys pressed in order. Up: released in reverse order. Aliases are already resolved.
pub fn send_combo(keys: &[String], down: bool, lift: ModMask) {
    let held = if down { mod_bits(|_, vk, m| lift & m != 0 && is_down(vk)) } else { 0 };
    let (inputs, lifted) = chord(keys, down, lift, held, LIFTED.load(Ordering::SeqCst));
    LIFTED.store(lifted, Ordering::SeqCst);
    if !inputs.is_empty() {
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    }
}

/// One chord press or release, and the new LIFTED bits. Down: the `held` modifiers go up
/// first, so Shift+F13 -> Ctrl+Z reaches the app as Ctrl+Z, not Ctrl+Shift+Z.
/// Up: the lifted modifiers of `lift` the user still holds go down again.
fn chord(keys: &[String], down: bool, lift: ModMask, held: u8, lifted: u8) -> (Vec<INPUT>, u8) {
    let mut defs: Vec<&KeyDef> = keys.iter().filter_map(|k| keymap::def(k)).collect();
    if down {
        let mut inputs = mod_inputs(held, true);
        inputs.extend(defs.into_iter().filter_map(|d| input(d, false)));
        return (inputs, lifted | held);
    }
    defs.reverse();
    let mut inputs: Vec<INPUT> = defs.into_iter().filter_map(|d| input(d, true)).collect();
    // a chord releasing its own Ctrl (a key remapped to Ctrl) settles any lift of that key
    let lifted = lifted & !mod_bits(|c, _, _| keys.iter().any(|k| k == c));
    let back = lifted & mod_bits(|_, _, m| lift & m != 0);
    inputs.extend(mod_inputs(back, false));
    (inputs, lifted & !back)
}

/// Releases (`up`) or presses the MOD_KEYS in `bits`, tapping VK_MASK while Alt or Win is down.
fn mod_inputs(bits: u8, up: bool) -> Vec<INPUT> {
    let mut v: Vec<INPUT> = MOD_KEYS
        .iter()
        .enumerate()
        .filter(|(i, _)| bits & (1 << i) != 0)
        .filter_map(|(_, (code, _, _))| input(keymap::def(code)?, up))
        .collect();
    if bits & MENU_KEYS != 0 {
        let tap = [key_input(VK_MASK, 0, KEYBD_EVENT_FLAGS(0)), key_input(VK_MASK, 0, KEYEVENTF_KEYUP)];
        if up {
            v.splice(0..0, tap);
        } else {
            v.extend(tap);
        }
    }
    v
}

/// False for keys Windows has no way to send (brightness, Eject, Fn).
pub fn can_send(key: &str) -> bool {
    keymap::def(key).is_some_and(|d| input(d, false).is_some())
}

fn input(d: &KeyDef, up: bool) -> Option<INPUT> {
    // consumer keys (volume, media, launch) only work as virtual keys; Pause and NumLock
    // share scan 0x45 and only the virtual key tells them apart
    let by_vk = d.win_scan == 0 || (0xA6..=0xB7).contains(&d.win_vk) || matches!(d.code, "Pause" | "NumLock");
    if by_vk && d.win_vk == 0 {
        return None; // BrightnessUp/Down, Eject, Fn: Windows has no key to send
    }
    let mut flags = if d.win_ext { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) };
    let vk = if by_vk {
        d.win_vk
    } else {
        flags |= KEYEVENTF_SCANCODE;
        0
    };
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    Some(key_input(vk, d.win_scan, flags))
}

fn key_input(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

/// Windows needs no permission to hook the keyboard.
pub fn permissions_granted() -> bool {
    true
}

/// True when a keyboard whose device path carries the Macally VID/PID is connected.
pub fn device_found() -> bool {
    let size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
    let mut count = 0u32;
    unsafe { GetRawInputDeviceList(None, &mut count, size) };
    let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
    let n = unsafe { GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, size) };
    if n == u32::MAX {
        return false;
    }
    list.iter().take(n as usize).filter(|d| d.dwType == RIM_TYPEKEYBOARD).any(|d| {
        let mut len = 0u32;
        unsafe { GetRawInputDeviceInfoW(Some(d.hDevice), RIDI_DEVICENAME, None, &mut len) };
        let mut buf = vec![0u16; len as usize];
        let got = unsafe { GetRawInputDeviceInfoW(Some(d.hDevice), RIDI_DEVICENAME, Some(buf.as_mut_ptr().cast()), &mut len) };
        got != u32::MAX && super::is_macally_path(&String::from_utf16_lossy(&buf[..(got as usize).min(buf.len())]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(d: &str, up: bool) -> (u16, u16, u32) {
        let i = input(keymap::def(d).unwrap(), up).unwrap();
        let ki = unsafe { i.Anonymous.ki };
        (ki.wVk.0, ki.wScan, ki.dwFlags.0)
    }

    #[test]
    fn letters_and_extended_keys_go_by_scan_code() {
        assert_eq!(flags("KeyA", false), (0, 0x1E, KEYEVENTF_SCANCODE.0));
        assert_eq!(flags("MetaLeft", true), (0, 0x5B, (KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP).0));
    }

    #[test]
    fn media_pause_and_numlock_go_by_virtual_key() {
        // the built-in profile's F7-F12: VK_MEDIA_* / VK_VOLUME_* with KEYEVENTF_EXTENDEDKEY
        let ext = KEYEVENTF_EXTENDEDKEY.0;
        assert_eq!(flags("MediaTrackPrevious", false), (0xB1, 0x10, ext));
        assert_eq!(flags("MediaPlayPause", false), (0xB3, 0x22, ext));
        assert_eq!(flags("MediaTrackNext", false), (0xB0, 0x19, ext));
        assert_eq!(flags("AudioVolumeMute", false), (0xAD, 0x20, ext));
        assert_eq!(flags("AudioVolumeDown", false), (0xAE, 0x2E, ext));
        assert_eq!(flags("AudioVolumeUp", true), (0xAF, 0x30, (KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP).0));
        assert_eq!(flags("Pause", false), (0x13, 0x45, 0));
        assert_eq!(flags("NumLock", true), (0x90, 0x45, (KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP).0));
        assert!(input(keymap::def("BrightnessUp").unwrap(), false).is_none());
        assert!(can_send("MediaPlayPause") && can_send("Win"));
        assert!(!can_send("BrightnessDown") && !can_send("Eject") && !can_send("Nope"));
    }

    /// (virtual key, scan code, is key-up) per input.
    fn keys_of(v: &[INPUT]) -> Vec<(u16, u16, bool)> {
        v.iter()
            .map(|i| {
                let ki = unsafe { i.Anonymous.ki };
                (ki.wVk.0, ki.wScan, ki.dwFlags.0 & KEYEVENTF_KEYUP.0 != 0)
            })
            .collect()
    }

    #[test]
    fn chord_lifts_held_with_modifiers_and_presses_them_back() {
        let ctrl_z: Vec<String> = ["ControlLeft", "KeyZ"].iter().map(|s| s.to_string()).collect();
        let (lshift, lwin) = (1 << 4, 1 << 6);
        // Shift+F13 -> Ctrl+Z: Shift up, then Ctrl and Z down
        let (down, lifted) = chord(&ctrl_z, true, SHIFT, lshift, 0);
        assert_eq!(keys_of(&down), vec![(0, 0x2A, true), (0, 0x1D, false), (0, 0x2C, false)]);
        assert_eq!(lifted, lshift);
        // auto-repeat: Shift already reads as up, nothing more is lifted
        assert_eq!(chord(&ctrl_z, true, SHIFT, 0, lifted).1, lshift);
        // release: Z and Ctrl up, then Shift down again
        let (up, lifted) = chord(&ctrl_z, false, SHIFT, 0, lifted);
        assert_eq!(keys_of(&up), vec![(0, 0x2C, true), (0, 0x1D, true), (0, 0x2A, false)]);
        assert_eq!(lifted, 0);
        // the user let go of Shift first (the hook cleared its bit): nothing is pressed back
        assert_eq!(chord(&ctrl_z, false, SHIFT, 0, 0).0.len(), 2);
        // Win goes up behind a VK_MASK tap and comes back followed by one: no Start menu
        let (down, _) = chord(&ctrl_z[1..], true, META, lwin, 0);
        assert_eq!(keys_of(&down)[..3], [(0xE8, 0, false), (0xE8, 0, true), (0, 0x5B, true)]);
        let (up, _) = chord(&ctrl_z[1..], false, META, 0, lwin);
        assert_eq!(keys_of(&up)[1..], [(0, 0x5B, false), (0xE8, 0, false), (0xE8, 0, true)]);
    }
}
