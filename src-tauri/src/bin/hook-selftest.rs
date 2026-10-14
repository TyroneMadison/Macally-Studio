//! Windows keyboard-hook self-test (no UI, no Tauri):
//!   cargo run --manifest-path src-tauri/Cargo.toml --features selftest --bin hook-selftest
//! Installs the real hook with F13 -> KeyA, injects F13 with SendInput, and checks that the hook
//! saw it as injected and passed it on untouched. Exit code 0 = pass, 1 = fail.

#[cfg(target_os = "windows")]
fn main() {
    std::process::exit(win::run());
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("hook-selftest only runs on Windows");
}

#[cfg(target_os = "windows")]
mod win {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, mpsc};
    use std::time::{Duration, Instant};

    use macally_studio_lib::actions::Action;
    use macally_studio_lib::engine::{self, Engine, Platform};
    use macally_studio_lib::profiles::{Binding, Profile};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_REMOVE, PeekMessageW, SetWindowsHookExW,
        UnhookWindowsHookEx, WH_KEYBOARD_LL,
    };

    const VK_F13: u32 = 0x7C;
    const SCAN_F13: u16 = 0x64;

    static COMBOS: AtomicU32 = AtomicU32::new(0);
    static F13_AFTER_ENGINE: AtomicU32 = AtomicU32::new(0);

    /// Counts would-be injections instead of typing anything.
    struct Counter;
    impl Platform for Counter {
        fn send_combo(&self, _keys: &[String], _down: bool) {
            COMBOS.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Installed before the engine hook, so Windows calls it after the engine hook:
    /// it only sees what the engine passed on.
    unsafe extern "system" fn observer(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && unsafe { (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode } == VK_F13 {
            F13_AFTER_ENGINE.fetch_add(1, Ordering::SeqCst);
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    /// Low-level hooks are called through the installing thread's message queue.
    fn pump(ms: u64) {
        let end = Instant::now() + Duration::from_millis(ms);
        let mut msg = MSG::default();
        while Instant::now() < end {
            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                unsafe { DispatchMessageW(&msg) };
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn f13(up: bool) -> INPUT {
        let flags = if up { KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP } else { KEYEVENTF_SCANCODE };
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: SCAN_F13, dwFlags: flags, time: 0, dwExtraInfo: 0 },
            },
        }
    }

    pub fn run() -> i32 {
        let observer = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(observer), None, 0) }.expect("observer hook");

        let (tx, _rx) = mpsc::channel();
        let engine = Arc::new(Engine::new(Box::new(Counter), tx));
        let mut bindings = BTreeMap::new();
        bindings.insert("F13".to_string(), Binding { mods: vec![], action: Action::KeyCombo { keys: vec!["KeyA".into()] } });
        engine.set_profile(&Profile {
            schema: 1,
            id: "selftest".into(),
            name: "Self-test".into(),
            builtin: false,
            hotkey: None,
            auto_apps: vec![],
            bindings,
        });
        engine::windows::install(engine.clone());
        let started = Instant::now();
        while !engine::HOOK_RUNNING.load(Ordering::SeqCst) && started.elapsed() < Duration::from_secs(2) {
            pump(10);
        }

        unsafe { SendInput(&[f13(false), f13(true)], std::mem::size_of::<INPUT>() as i32) };
        pump(300);

        let injected = engine::windows::INJECTED_SEEN.load(Ordering::SeqCst);
        let passed_on = F13_AFTER_ENGINE.load(Ordering::SeqCst);
        let combos_from_hook = COMBOS.load(Ordering::SeqCst);
        // software cannot produce a non-injected F13, so a physical press is simulated on the engine
        let swallowed = engine.handle_key("F13", 0, true, "selftest") && engine.handle_key("F13", 0, false, "selftest");
        let combos = COMBOS.load(Ordering::SeqCst);
        let _ = unsafe { UnhookWindowsHookEx(observer) };

        let checks = [
            ("hook installed", engine::HOOK_RUNNING.load(Ordering::SeqCst)),
            ("hook saw the injected F13 down+up as LLKHF_INJECTED", injected >= 2),
            ("injected F13 was passed through to the next hook", passed_on >= 2),
            ("injected F13 did not trigger the F13 -> KeyA remap", combos_from_hook == 0),
            ("a physical F13 is swallowed and remapped (down + up)", swallowed && combos == 2),
        ];
        let mut ok = true;
        for (name, pass) in checks {
            println!("{} {name}", if pass { "PASS" } else { "FAIL" });
            ok &= pass;
        }
        println!("info: Macally keyboard connected: {}", engine::windows::device_found());
        println!("hook-selftest: {}", if ok { "OK" } else { "FAILED" });
        if ok { 0 } else { 1 }
    }
}
