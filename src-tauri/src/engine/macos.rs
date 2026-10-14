//! macOS: CGEventTap on its own CFRunLoop thread, CGEventPost for chords, NSEvent for media
//! keys, the Accessibility / Input Monitoring checks, and the USB presence check.

use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_application_services::{AXIsProcessTrustedWithOptions, kAXTrustedCheckOptionPrompt};
use objc2_core_foundation::{CFDictionary, CFMachPort, CFRunLoop, kCFBooleanTrue, kCFRunLoopCommonModes};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventMask, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType, CGPreflightListenEventAccess,
    CGRequestListenEventAccess,
};
use objc2_foundation::NSPoint;

use super::{ALT, CTRL, Engine, HOOK_RUNNING, META, ModMask, SHIFT};
use crate::keymap;

const NX_SYSDEFINED: u32 = 14;
const NX_SUBTYPE_AUX_CONTROL_BUTTONS: i16 = 8;
/// Stamped on every event we post (kCGEventSourceUserData) so the tap lets it through.
const INJECT_TAG: i64 = 0x004D_4143_414C_4C59; // "MACALLY" in ASCII

static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();
/// The tap's mach port, kept for re-enabling after a timeout. Lives as long as the tap thread.
static PORT: AtomicPtr<CFMachPort> = AtomicPtr::new(ptr::null_mut());

pub fn install(engine: Arc<Engine>) {
    if ENGINE.set(engine).is_err() {
        return;
    }
    ensure_permissions();
    std::thread::Builder::new()
        .name("cg-event-tap".into())
        .spawn(|| {
            let mask: CGEventMask = (1 << CGEventType::KeyDown.0)
                | (1 << CGEventType::KeyUp.0)
                | (1 << CGEventType::FlagsChanged.0)
                | (1 << NX_SYSDEFINED);
            // the tap cannot be created until the user grants the permissions; keep trying,
            // so remapping starts by itself (and the permissions screen closes) once they are on.
            // HID level first (sees keys before any app); session level if HID is refused.
            let port = loop {
                let port = [CGEventTapLocation::HIDEventTap, CGEventTapLocation::SessionEventTap]
                    .into_iter()
                    .find_map(|location| unsafe {
                        CGEvent::tap_create(
                            location,
                            CGEventTapPlacement::HeadInsertEventTap,
                            CGEventTapOptions::Default,
                            mask,
                            Some(tap_cb),
                            ptr::null_mut(),
                        )
                    });
                match port {
                    Some(p) => break p,
                    None => std::thread::sleep(Duration::from_secs(2)),
                }
            };
            PORT.store(&*port as *const CFMachPort as *mut CFMachPort, Ordering::Release);
            let Some(source) = CFMachPort::new_run_loop_source(None, Some(&port), 0) else {
                eprintln!("CFMachPortCreateRunLoopSource failed");
                return;
            };
            let Some(run_loop) = CFRunLoop::current() else {
                return;
            };
            run_loop.add_source(Some(&source), unsafe { kCFRunLoopCommonModes });
            CGEvent::tap_enable(&port, true);
            HOOK_RUNNING.store(true, Ordering::SeqCst);
            CFRunLoop::run();
            HOOK_RUNNING.store(false, Ordering::SeqCst);
        })
        .expect("spawn event tap thread");
}

/// Must return fast or macOS disables the tap; actions go to the engine's worker thread.
unsafe extern "C-unwind" fn tap_cb(
    _proxy: CGEventTapProxy,
    ty: CGEventType,
    event: NonNull<CGEvent>,
    _user: *mut c_void,
) -> *mut CGEvent {
    if ty == CGEventType::TapDisabledByTimeout || ty == CGEventType::TapDisabledByUserInput {
        let port = PORT.load(Ordering::Acquire);
        if !port.is_null() {
            CGEvent::tap_enable(unsafe { &*port }, true);
        }
        return event.as_ptr();
    }
    let ev = unsafe { event.as_ref() };
    if CGEvent::integer_value_field(Some(ev), CGEventField::EventSourceUserData) == INJECT_TAG {
        return event.as_ptr(); // our own injection
    }
    let (Some(engine), Some((code, down, raw))) = (ENGINE.get(), decode(ty, ev)) else {
        return event.as_ptr();
    };
    // the HID system sets flags from the physical modifiers, before the tap: give each event the
    // ones the remaps stand for, so with Cmd remapped to Ctrl, Cmd+C reaches the app as Ctrl+C
    let mut flags = CGEvent::flags(Some(ev));
    let (taken, pressed) = engine.remapped_mods();
    if taken | pressed != 0 {
        flags.remove(flags_of(taken));
        flags |= flags_of(pressed);
        if ty.0 != NX_SYSDEFINED {
            CGEvent::set_flags(Some(ev), flags);
        }
    }
    let mods = mods_from_flags(flags);
    if engine.handle_key(code, mods, down, &raw) { ptr::null_mut() } else { event.as_ptr() }
}

/// (code, is_down, raw description) for key, modifier and media events.
fn decode(ty: CGEventType, ev: &CGEvent) -> Option<(&'static str, bool, String)> {
    if ty.0 == NX_SYSDEFINED {
        let ns = NSEvent::eventWithCGEvent(ev)?;
        if ns.subtype().0 != NX_SUBTYPE_AUX_CONTROL_BUTTONS {
            return None; // not a media key (e.g. aux mouse buttons)
        }
        let data1 = ns.data1();
        let key = ((data1 & 0xFFFF_0000) >> 16) as u8;
        let down = ((data1 & 0xFF00) >> 8) == 0xA;
        return Some((keymap::code_from_mac_media(key).unwrap_or("Unidentified"), down, format!("media {key}")));
    }
    let vk = CGEvent::integer_value_field(Some(ev), CGEventField::KeyboardEventKeycode) as u16;
    let code = keymap::code_from_mac(vk).unwrap_or("Unidentified");
    let raw = format!("vk 0x{vk:02X}");
    if ty == CGEventType::FlagsChanged {
        // ponytail: generic masks; pressing both Shifts then releasing one reads as still down
        let flags = CGEvent::flags(Some(ev));
        let mask = match code {
            "CapsLock" => CGEventFlags::MaskAlphaShift,
            other if super::mod_bit(other) != 0 => flag_of(other),
            _ => CGEventFlags::MaskSecondaryFn,
        };
        return Some((code, flags.contains(mask), raw));
    }
    Some((code, ty == CGEventType::KeyDown, raw))
}

fn mods_from_flags(f: CGEventFlags) -> ModMask {
    let mut m = 0;
    if f.contains(CGEventFlags::MaskControl) {
        m |= CTRL;
    }
    if f.contains(CGEventFlags::MaskAlternate) {
        m |= ALT;
    }
    if f.contains(CGEventFlags::MaskShift) {
        m |= SHIFT;
    }
    if f.contains(CGEventFlags::MaskCommand) {
        m |= META;
    }
    m
}

fn flags_of(m: ModMask) -> CGEventFlags {
    use CGEventFlags as F;
    [(CTRL, F::MaskControl), (ALT, F::MaskAlternate), (SHIFT, F::MaskShift), (META, F::MaskCommand)]
        .into_iter()
        .filter(|(bit, _)| m & bit != 0)
        .fold(CGEventFlags::empty(), |f, (_, flag)| f | flag)
}

fn flag_of(code: &str) -> CGEventFlags {
    match code {
        "ShiftLeft" | "ShiftRight" => CGEventFlags::MaskShift,
        "ControlLeft" | "ControlRight" => CGEventFlags::MaskControl,
        "AltLeft" | "AltRight" => CGEventFlags::MaskAlternate,
        "MetaLeft" | "MetaRight" => CGEventFlags::MaskCommand,
        _ => CGEventFlags::empty(),
    }
}

/// False for keys a Mac has no keycode or media key for (PrintScreen, ScrollLock, Pause).
pub fn can_send(key: &str) -> bool {
    keymap::def(key).is_some_and(|d| d.mac_vk.is_some() || d.mac_media.is_some())
}

/// Down: keys pressed in order, each carrying the modifiers pressed so far.
/// Up: released in reverse order. Media keys go out as NX_SYSDEFINED events.
/// No lift needed: every posted event carries its own flags, so held modifiers cannot leak in.
pub fn send_combo(keys: &[String], down: bool, _lift: ModMask) {
    let Some(source) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) else {
        return;
    };
    CGEventSource::set_user_data(Some(&source), INJECT_TAG);
    let mut flags = if down {
        CGEventFlags::empty()
    } else {
        keys.iter().fold(CGEventFlags::empty(), |f, k| f | flag_of(k))
    };
    let ordered: Vec<&String> = if down { keys.iter().collect() } else { keys.iter().rev().collect() };
    for key in ordered {
        let Some(def) = keymap::def(key) else { continue };
        if let Some(media) = def.mac_media {
            post_media(media, down);
            continue;
        }
        let Some(vk) = def.mac_vk else { continue };
        if down {
            flags |= flag_of(key);
        } else {
            flags.remove(flag_of(key));
        }
        if let Some(ev) = CGEvent::new_keyboard_event(Some(&source), vk, down) {
            CGEvent::set_flags(Some(&ev), flags);
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&ev));
        }
    }
}

fn post_media(key: u8, down: bool) {
    let state: isize = if down { 0xA } else { 0xB };
    let Some(ns) = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::SystemDefined,
        NSPoint::new(0.0, 0.0),
        NSEventModifierFlags::from_bits_retain((state as usize) << 8),
        0.0,
        0,
        None,
        NX_SUBTYPE_AUX_CONTROL_BUTTONS,
        ((key as isize) << 16) | (state << 8),
        -1,
    ) else {
        return;
    };
    if let Some(cg) = ns.CGEvent() {
        CGEvent::set_integer_value_field(Some(&cg), CGEventField::EventSourceUserData, INJECT_TAG);
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&cg));
    }
}

fn accessibility(prompt: bool) -> bool {
    unsafe {
        let true_value = kCFBooleanTrue.expect("kCFBooleanTrue");
        let opts = CFDictionary::from_slices(&[kAXTrustedCheckOptionPrompt], &[true_value]);
        AXIsProcessTrustedWithOptions(prompt.then(|| opts.as_opaque()))
    }
}

/// Asks for Accessibility (posting events, active tap) and Input Monitoring (reading keys).
/// macOS shows each system prompt once; afterwards the user flips the switch in System Settings.
pub fn ensure_permissions() -> bool {
    let ax = accessibility(true);
    let input = CGPreflightListenEventAccess() || CGRequestListenEventAccess();
    ax && input
}

/// Same check without prompting (used by get_status and the permissions screen).
pub fn permissions_granted() -> bool {
    accessibility(false) && CGPreflightListenEventAccess()
}

/// `ioreg` lists USB devices with their decimal VID/PID (8738 / 276 for the Macally).
pub fn device_found() -> bool {
    std::process::Command::new("/usr/sbin/ioreg")
        .args(["-p", "IOUSB", "-l", "-w0"])
        .output()
        .map(|o| super::ioreg_has_macally(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_flags_map_to_mod_mask_and_back() {
        assert_eq!(mods_from_flags(CGEventFlags::MaskCommand | CGEventFlags::MaskShift), META | SHIFT);
        assert_eq!(mods_from_flags(CGEventFlags::MaskSecondaryFn), 0);
        assert_eq!(flag_of("AltRight"), CGEventFlags::MaskAlternate);
        assert_eq!(flag_of("KeyA"), CGEventFlags::empty());
        assert_eq!(flags_of(CTRL | META), CGEventFlags::MaskControl | CGEventFlags::MaskCommand);
        assert_eq!(mods_from_flags(flags_of(ALT | SHIFT)), ALT | SHIFT);
        assert!(can_send("BrightnessUp") && can_send("Cmd"));
        assert!(!can_send("PrintScreen"));
    }
}
