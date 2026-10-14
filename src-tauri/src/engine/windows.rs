//! Windows: WH_KEYBOARD_LL hook on its own message-loop thread, SendInput for chords,
//! and the Raw Input scan that finds the Macally (VID 0x2222 / PID 0x0114).

use std::sync::atomic::{AtomicU32, Ordering};
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
pub fn send_combo(keys: &[String], down: bool) {
    let mut defs: Vec<&KeyDef> = keys.iter().filter_map(|k| keymap::def(k)).collect();
    if !down {
        defs.reverse();
    }
    let inputs: Vec<INPUT> = defs.into_iter().filter_map(|d| input(d, !down)).collect();
    if !inputs.is_empty() {
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    }
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
    Some(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: d.win_scan, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    })
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
}
