//! Key identity: W3C `KeyboardEvent.code` <-> Windows scan/vk <-> macOS keycode / NX media key.

#[derive(Debug, PartialEq, Eq)]
pub struct KeyDef {
    pub code: &'static str,
    /// Set-1 make code (0 = none, e.g. Fn, Eject, brightness).
    pub win_scan: u16,
    /// Arrives with the E0 prefix (LLKHF_EXTENDED); inject with KEYEVENTF_EXTENDEDKEY.
    pub win_ext: bool,
    pub win_vk: u16,
    /// HIToolbox kVK_* keycode.
    pub mac_vk: Option<u16>,
    /// NX_KEYTYPE_* carried by NX_SYSDEFINED (subtype 8) events.
    pub mac_media: Option<u8>,
}

const fn k(
    code: &'static str,
    win_scan: u16,
    win_ext: bool,
    win_vk: u16,
    mac_vk: Option<u16>,
    mac_media: Option<u8>,
) -> KeyDef {
    KeyDef { code, win_scan, win_ext, win_vk, mac_vk, mac_media }
}

pub static KEYS: &[KeyDef] = &[
    k("Escape", 0x01, false, 0x1B, Some(0x35), None),
    k("F1", 0x3B, false, 0x70, Some(0x7A), None),
    k("F2", 0x3C, false, 0x71, Some(0x78), None),
    k("F3", 0x3D, false, 0x72, Some(0x63), None),
    k("F4", 0x3E, false, 0x73, Some(0x76), None),
    k("F5", 0x3F, false, 0x74, Some(0x60), None),
    k("F6", 0x40, false, 0x75, Some(0x61), None),
    k("F7", 0x41, false, 0x76, Some(0x62), None),
    k("F8", 0x42, false, 0x77, Some(0x64), None),
    k("F9", 0x43, false, 0x78, Some(0x65), None),
    k("F10", 0x44, false, 0x79, Some(0x6D), None),
    k("F11", 0x57, false, 0x7A, Some(0x67), None),
    k("F12", 0x58, false, 0x7B, Some(0x6F), None),
    k("F13", 0x64, false, 0x7C, Some(0x69), None),
    k("F14", 0x65, false, 0x7D, Some(0x6B), None),
    k("F15", 0x66, false, 0x7E, Some(0x71), None),
    k("Backquote", 0x29, false, 0xC0, Some(0x32), None),
    k("Digit1", 0x02, false, 0x31, Some(0x12), None),
    k("Digit2", 0x03, false, 0x32, Some(0x13), None),
    k("Digit3", 0x04, false, 0x33, Some(0x14), None),
    k("Digit4", 0x05, false, 0x34, Some(0x15), None),
    k("Digit5", 0x06, false, 0x35, Some(0x17), None),
    k("Digit6", 0x07, false, 0x36, Some(0x16), None),
    k("Digit7", 0x08, false, 0x37, Some(0x1A), None),
    k("Digit8", 0x09, false, 0x38, Some(0x1C), None),
    k("Digit9", 0x0A, false, 0x39, Some(0x19), None),
    k("Digit0", 0x0B, false, 0x30, Some(0x1D), None),
    k("Minus", 0x0C, false, 0xBD, Some(0x1B), None),
    k("Equal", 0x0D, false, 0xBB, Some(0x18), None),
    k("Backspace", 0x0E, false, 0x08, Some(0x33), None),
    k("Tab", 0x0F, false, 0x09, Some(0x30), None),
    k("KeyQ", 0x10, false, 0x51, Some(0x0C), None),
    k("KeyW", 0x11, false, 0x57, Some(0x0D), None),
    k("KeyE", 0x12, false, 0x45, Some(0x0E), None),
    k("KeyR", 0x13, false, 0x52, Some(0x0F), None),
    k("KeyT", 0x14, false, 0x54, Some(0x11), None),
    k("KeyY", 0x15, false, 0x59, Some(0x10), None),
    k("KeyU", 0x16, false, 0x55, Some(0x20), None),
    k("KeyI", 0x17, false, 0x49, Some(0x22), None),
    k("KeyO", 0x18, false, 0x4F, Some(0x1F), None),
    k("KeyP", 0x19, false, 0x50, Some(0x23), None),
    k("BracketLeft", 0x1A, false, 0xDB, Some(0x21), None),
    k("BracketRight", 0x1B, false, 0xDD, Some(0x1E), None),
    k("Backslash", 0x2B, false, 0xDC, Some(0x2A), None),
    k("CapsLock", 0x3A, false, 0x14, Some(0x39), None),
    k("KeyA", 0x1E, false, 0x41, Some(0x00), None),
    k("KeyS", 0x1F, false, 0x53, Some(0x01), None),
    k("KeyD", 0x20, false, 0x44, Some(0x02), None),
    k("KeyF", 0x21, false, 0x46, Some(0x03), None),
    k("KeyG", 0x22, false, 0x47, Some(0x05), None),
    k("KeyH", 0x23, false, 0x48, Some(0x04), None),
    k("KeyJ", 0x24, false, 0x4A, Some(0x26), None),
    k("KeyK", 0x25, false, 0x4B, Some(0x28), None),
    k("KeyL", 0x26, false, 0x4C, Some(0x25), None),
    k("Semicolon", 0x27, false, 0xBA, Some(0x29), None),
    k("Quote", 0x28, false, 0xDE, Some(0x27), None),
    k("Enter", 0x1C, false, 0x0D, Some(0x24), None),
    k("ShiftLeft", 0x2A, false, 0xA0, Some(0x38), None),
    k("KeyZ", 0x2C, false, 0x5A, Some(0x06), None),
    k("KeyX", 0x2D, false, 0x58, Some(0x07), None),
    k("KeyC", 0x2E, false, 0x43, Some(0x08), None),
    k("KeyV", 0x2F, false, 0x56, Some(0x09), None),
    k("KeyB", 0x30, false, 0x42, Some(0x0B), None),
    k("KeyN", 0x31, false, 0x4E, Some(0x2D), None),
    k("KeyM", 0x32, false, 0x4D, Some(0x2E), None),
    k("Comma", 0x33, false, 0xBC, Some(0x2B), None),
    k("Period", 0x34, false, 0xBE, Some(0x2F), None),
    k("Slash", 0x35, false, 0xBF, Some(0x2C), None),
    k("ShiftRight", 0x36, false, 0xA1, Some(0x3C), None),
    k("Fn", 0x00, false, 0x00, Some(0x3F), None), // firmware key, never reaches Windows
    k("ControlLeft", 0x1D, false, 0xA2, Some(0x3B), None),
    k("AltLeft", 0x38, false, 0xA4, Some(0x3A), None),
    k("MetaLeft", 0x5B, true, 0x5B, Some(0x37), None),
    k("Space", 0x39, false, 0x20, Some(0x31), None),
    k("MetaRight", 0x5C, true, 0x5C, Some(0x36), None),
    k("AltRight", 0x38, true, 0xA5, Some(0x3D), None),
    k("ContextMenu", 0x5D, true, 0x5D, Some(0x6E), None),
    k("ControlRight", 0x1D, true, 0xA3, Some(0x3E), None),
    k("Insert", 0x52, true, 0x2D, Some(0x72), None), // mac: kVK_Help
    k("Home", 0x47, true, 0x24, Some(0x73), None),
    k("PageUp", 0x49, true, 0x21, Some(0x74), None),
    k("Delete", 0x53, true, 0x2E, Some(0x75), None),
    k("End", 0x4F, true, 0x23, Some(0x77), None),
    k("PageDown", 0x51, true, 0x22, Some(0x79), None),
    k("ArrowUp", 0x48, true, 0x26, Some(0x7E), None),
    k("ArrowDown", 0x50, true, 0x28, Some(0x7D), None),
    k("ArrowLeft", 0x4B, true, 0x25, Some(0x7B), None),
    k("ArrowRight", 0x4D, true, 0x27, Some(0x7C), None),
    k("NumLock", 0x45, true, 0x90, Some(0x47), None), // mac: kVK_ANSI_KeypadClear
    k("NumpadEqual", 0x59, false, 0x92, Some(0x51), None),
    k("NumpadDivide", 0x35, true, 0x6F, Some(0x4B), None),
    k("NumpadMultiply", 0x37, false, 0x6A, Some(0x43), None),
    k("NumpadSubtract", 0x4A, false, 0x6D, Some(0x4E), None),
    k("NumpadAdd", 0x4E, false, 0x6B, Some(0x45), None),
    k("NumpadEnter", 0x1C, true, 0x0D, Some(0x4C), None),
    k("Numpad0", 0x52, false, 0x60, Some(0x52), None),
    k("Numpad1", 0x4F, false, 0x61, Some(0x53), None),
    k("Numpad2", 0x50, false, 0x62, Some(0x54), None),
    k("Numpad3", 0x51, false, 0x63, Some(0x55), None),
    k("Numpad4", 0x4B, false, 0x64, Some(0x56), None),
    k("Numpad5", 0x4C, false, 0x65, Some(0x57), None),
    k("Numpad6", 0x4D, false, 0x66, Some(0x58), None),
    k("Numpad7", 0x47, false, 0x67, Some(0x59), None),
    k("Numpad8", 0x48, false, 0x68, Some(0x5B), None),
    k("Numpad9", 0x49, false, 0x69, Some(0x5C), None),
    k("NumpadDecimal", 0x53, false, 0x6E, Some(0x41), None),
    // Consumer keys: Windows delivers them by VK (when at all), macOS as NX_SYSDEFINED.
    k("Eject", 0x00, false, 0x00, None, Some(14)),
    k("MediaTrackPrevious", 0x10, true, 0xB1, None, Some(18)),
    k("MediaPlayPause", 0x22, true, 0xB3, None, Some(16)),
    k("MediaTrackNext", 0x19, true, 0xB0, None, Some(17)),
    k("AudioVolumeMute", 0x20, true, 0xAD, Some(0x4A), Some(7)),
    k("AudioVolumeDown", 0x2E, true, 0xAE, Some(0x49), Some(1)),
    k("AudioVolumeUp", 0x30, true, 0xAF, Some(0x48), Some(0)),
    k("BrightnessDown", 0x00, false, 0x00, None, Some(3)),
    k("BrightnessUp", 0x00, false, 0x00, None, Some(2)),
    k("LaunchApp1", 0x6B, true, 0xB6, None, None),
    // PC-only keys: macOS reports a PC keyboard's PrtSc/ScrLk/Pause as F13/F14/F15.
    k("PrintScreen", 0x37, true, 0x2C, None, None),
    k("ScrollLock", 0x46, false, 0x91, None, None),
    k("Pause", 0x45, false, 0x13, None, None), // E1 1D 45 arrives as scan 0x45 without E0
];

// ponytail: linear scans over ~120 entries (well under a microsecond); add a HashMap if profiling ever says so.

/// Scan code + extended flag first (layout independent, tells ArrowUp from Numpad8),
/// then the virtual key for events that carry no scan code (media keys sent by VK).
pub fn code_from_win(scan: u16, ext: bool, vk: u16) -> Option<&'static str> {
    KEYS.iter()
        .find(|d| scan != 0 && d.win_scan == scan && d.win_ext == ext)
        .or_else(|| KEYS.iter().find(|d| vk != 0 && d.win_vk == vk))
        .map(|d| d.code)
}

pub fn code_from_mac(vk: u16) -> Option<&'static str> {
    KEYS.iter().find(|d| d.mac_vk == Some(vk)).map(|d| d.code)
}

pub fn code_from_mac_media(key_type: u8) -> Option<&'static str> {
    KEYS.iter().find(|d| d.mac_media == Some(key_type)).map(|d| d.code)
}

/// Keys the keyboard's own firmware handles and never reports to the OS, so no app can see them
/// or use them as a remap source. On the Macally this is the Fn key (confirmed by a HID probe:
/// the keyboard exposes only standard keyboard, consumer and system-control collections, no vendor
/// collection, and Fn appears on none of them).
pub const FIRMWARE_ONLY: &[&str] = &["Fn"];

pub fn is_firmware_only(code: &str) -> bool {
    FIRMWARE_ONLY.contains(&code)
}

/// Looks a key up by code; modifier aliases ("Ctrl", "Win", ...) are accepted.
pub fn def(code: &str) -> Option<&'static KeyDef> {
    let code = resolve_alias(code);
    KEYS.iter().find(|d| d.code == code)
}

/// Modifier shorthands accepted in actions and "held with" lists.
pub fn resolve_alias(name: &str) -> &str {
    match name {
        "Ctrl" => "ControlLeft",
        "Alt" => "AltLeft",
        "Shift" => "ShiftLeft",
        "Win" | "Cmd" | "Meta" => "MetaLeft",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fn_is_the_only_firmware_only_key() {
        assert!(is_firmware_only("Fn"));
        assert!(!is_firmware_only("KeyA"));
        assert!(!is_firmware_only("ControlLeft"));
        assert!(!is_firmware_only("F5")); // the backlight keys still send F5/F6 with Fn-Lock on
        // every firmware-only key is a real key in the map, and none can be injected
        for code in FIRMWARE_ONLY {
            let d = def(code).unwrap();
            assert_eq!((d.win_scan, d.win_vk), (0, 0), "{code} is injectable");
        }
    }

    #[test]
    fn codes_are_unique() {
        let mut codes: Vec<&str> = KEYS.iter().map(|d| d.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), KEYS.len());
    }

    #[test]
    fn windows_round_trip() {
        for d in KEYS.iter().filter(|d| d.win_scan != 0) {
            assert_eq!(code_from_win(d.win_scan, d.win_ext, d.win_vk), Some(d.code), "{}", d.code);
        }
    }

    #[test]
    fn mac_round_trip() {
        for d in KEYS {
            if let Some(vk) = d.mac_vk {
                assert_eq!(code_from_mac(vk), Some(d.code), "{}", d.code);
            }
            if let Some(m) = d.mac_media {
                assert_eq!(code_from_mac_media(m), Some(d.code), "{}", d.code);
            }
        }
    }

    #[test]
    fn extended_keys_are_not_numpad_keys() {
        assert_eq!(code_from_win(0x48, true, 0x26), Some("ArrowUp"));
        assert_eq!(code_from_win(0x48, false, 0x68), Some("Numpad8"));
        // NumLock off: numpad 8 reports VK_UP but keeps its non-extended scan code
        assert_eq!(code_from_win(0x48, false, 0x26), Some("Numpad8"));
        assert_eq!(code_from_win(0x1C, true, 0x0D), Some("NumpadEnter"));
        assert_eq!(code_from_win(0x1C, false, 0x0D), Some("Enter"));
        assert_eq!(code_from_win(0x45, true, 0x90), Some("NumLock"));
        assert_eq!(code_from_win(0x45, false, 0x13), Some("Pause"));
    }

    #[test]
    fn scanless_events_fall_back_to_vk() {
        assert_eq!(code_from_win(0, false, 0xAF), Some("AudioVolumeUp"));
        assert_eq!(code_from_win(0, false, 0), None);
        assert_eq!(code_from_win(0x7F, false, 0), None);
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(resolve_alias("Ctrl"), "ControlLeft");
        assert_eq!(resolve_alias("Alt"), "AltLeft");
        assert_eq!(resolve_alias("Shift"), "ShiftLeft");
        for a in ["Win", "Cmd", "Meta"] {
            assert_eq!(resolve_alias(a), "MetaLeft");
        }
        assert_eq!(resolve_alias("KeyA"), "KeyA");
        assert_eq!(def("Win").map(|d| d.win_vk), Some(0x5B));
        assert!(def("Nope").is_none());
    }

    #[test]
    fn keycodes_ts_mirrors_keys() {
        let ts = include_str!("../../src/lib/keycodes.ts");
        let mut mirrored: Vec<&str> = ts.split('"').skip(1).step_by(2).collect();
        let mut rust: Vec<&str> = KEYS.iter().map(|d| d.code).collect();
        mirrored.sort();
        rust.sort();
        assert_eq!(mirrored, rust);
    }

    #[test]
    fn actions_ts_hides_exactly_the_unsendable_keys() {
        #[cfg(target_os = "macos")]
        use crate::engine::macos::can_send;
        #[cfg(target_os = "windows")]
        use crate::engine::windows::can_send;
        let ts = include_str!("../../src/lib/actions.ts");
        let line = ts.lines().find(|l| l.starts_with("const UNSENDABLE")).expect("UNSENDABLE in actions.ts");
        let (mac, win) = line.split_once(" : ").expect("isMac ? [mac] : [windows]");
        let list = if cfg!(target_os = "macos") { mac } else { win };
        let hidden: Vec<&str> = list.split('"').skip(1).step_by(2).collect();
        let unsendable: Vec<&str> = KEYS.iter().map(|d| d.code).filter(|c| !can_send(c)).collect();
        assert_eq!(hidden, unsendable);
    }
}
