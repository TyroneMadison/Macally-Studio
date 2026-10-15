//! Foreground-app watcher: emits "foreground-changed" { name } and, in Auto switch mode,
//! activates the first profile whose autoApps lists the app (exe file name on Windows,
//! bundle id on macOS), else the last profile the user picked.

use std::sync::Mutex;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::AppState;
use crate::profiles::{Profile, SwitchMode};

/// First profile whose `auto_apps` contains `name` (case-insensitive), else `last_manual`.
pub fn pick_profile(name: &str, profiles: &[Profile], last_manual: &str) -> String {
    profiles
        .iter()
        .find(|p| p.auto_apps.iter().any(|a| a.eq_ignore_ascii_case(name)))
        .map_or_else(|| last_manual.to_string(), |p| p.id.clone())
}

/// What Auto mode remembers between foreground changes.
#[derive(Default)]
pub struct Auto {
    last_manual: String,
    chosen: String,
}

impl Auto {
    pub const fn new() -> Auto {
        Auto { last_manual: String::new(), chosen: String::new() }
    }

    /// The profile to activate now that `name` is in front, or None to leave `active` alone.
    /// An active profile that Auto mode did not pick was picked by the user: it becomes the fallback.
    pub fn next(&mut self, name: &str, profiles: &[Profile], active: &str) -> Option<String> {
        if active != self.chosen {
            self.last_manual = active.to_string();
        }
        let target = pick_profile(name, profiles, &self.last_manual);
        self.chosen = target.clone();
        (target != active).then_some(target)
    }
}

static AUTO: Mutex<Auto> = Mutex::new(Auto::new());
static LAST_APP: Mutex<String> = Mutex::new(String::new());

/// Our own window coming to the front must not switch away from the profile being edited.
fn own_name(app: &AppHandle) -> String {
    #[cfg(target_os = "macos")]
    return app.config().identifier.clone();
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_default()
    }
}

fn changed(app: &AppHandle, name: String) {
    if name.eq_ignore_ascii_case(&own_name(app)) {
        return;
    }
    {
        let mut last = LAST_APP.lock().unwrap();
        if *last == name {
            return; // another window of the same app
        }
        *last = name.clone();
    }
    let _ = app.emit("foreground-changed", json!({ "name": name }));
    let Some(state) = app.try_state::<AppState>() else { return };
    let (mode, active) = {
        let s = state.settings.lock().unwrap();
        (s.switch_mode, s.active_profile.clone())
    };
    if mode != SwitchMode::Auto {
        return;
    }
    let profiles = state.profiles.lock().unwrap().clone();
    let target = AUTO.lock().unwrap().next(&name, &profiles, &active);
    if let Some(id) = target
        && let Err(e) = crate::control::activate(app, &id)
    {
        eprintln!("auto switch to {id}: {e:#}");
    }
}

/// Call once from setup (on macOS it must run on the main thread, which setup does).
pub fn start(app: AppHandle) {
    #[cfg(target_os = "windows")]
    win::start(app);
    #[cfg(target_os = "macos")]
    mac::start(app);
}

#[cfg(target_os = "windows")]
mod win {
    use std::sync::OnceLock;

    use tauri::AppHandle;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GetMessageW, GetWindowThreadProcessId, MSG, WINEVENT_OUTOFCONTEXT,
    };
    use windows::core::PWSTR;

    static APP: OnceLock<AppHandle> = OnceLock::new();

    pub fn start(app: AppHandle) {
        if APP.set(app).is_err() {
            return;
        }
        std::thread::Builder::new()
            .name("foreground".into())
            .spawn(|| {
                // out-of-context: the callback runs on this thread, through its message loop
                let _hook = unsafe {
                    SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_FOREGROUND,
                        None,
                        Some(on_event),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT,
                    )
                };
                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                    unsafe { DispatchMessageW(&msg) };
                }
            })
            .expect("spawn foreground thread");
    }

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        _event: u32,
        hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        if let (Some(app), Some(name)) = (APP.get(), exe_name(hwnd)) {
            super::changed(app, name);
        }
    }

    // ponytail: Store apps report ApplicationFrameHost.exe; resolve the hosted app if users ask for it.
    fn exe_name(hwnd: HWND) -> Option<String> {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == 0 {
            return None;
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len) };
        let _ = unsafe { CloseHandle(handle) };
        ok.ok()?;
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2_app_kit::{
        NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey, NSWorkspaceDidActivateApplicationNotification,
    };
    use objc2_foundation::NSNotification;
    use tauri::AppHandle;

    pub fn start(app: AppHandle) {
        let block = RcBlock::new(move |n: NonNull<NSNotification>| {
            let bundle_id = unsafe { n.as_ref() }
                .userInfo()
                .and_then(|info| info.objectForKey(unsafe { NSWorkspaceApplicationKey }))
                .and_then(|o| o.downcast::<NSRunningApplication>().ok())
                .and_then(|a| a.bundleIdentifier())
                .map(|s| s.to_string());
            if let Some(b) = bundle_id {
                super::changed(&app, b);
            }
        });
        let token = unsafe {
            NSWorkspace::sharedWorkspace().notificationCenter().addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidActivateApplicationNotification),
                None,
                None,
                &block,
            )
        };
        std::mem::forget(token); // observe for the whole app lifetime
    }
}

#[cfg(test)]
mod tests {
    use super::{Auto, pick_profile};
    use crate::profiles::Profile;

    fn p(id: &str, apps: &[&str]) -> Profile {
        Profile {
            schema: 1,
            id: id.into(),
            name: id.into(),
            builtin: false,
            hotkey: None,
            auto_apps: apps.iter().map(|s| s.to_string()).collect(),
            bindings: Default::default(),
            game: None,
        }
    }

    #[test]
    fn first_matching_profile_wins_case_insensitively() {
        let list = vec![p("photo", &["Photoshop.exe", "com.adobe.Lightroom"]), p("games", &["photoshop.exe", "steam.exe"])];
        assert_eq!(pick_profile("PHOTOSHOP.EXE", &list, "work"), "photo");
        assert_eq!(pick_profile("steam.exe", &list, "work"), "games");
        assert_eq!(pick_profile("com.adobe.lightroom", &list, "work"), "photo");
    }

    #[test]
    fn falls_back_to_last_manual_profile() {
        let list = vec![p("photo", &["Photoshop.exe"])];
        assert_eq!(pick_profile("notepad.exe", &list, "work"), "work");
    }

    #[test]
    fn auto_returns_to_the_profile_the_user_picked() {
        let list = vec![p("work", &[]), p("gaming", &[]), p("photo", &["Photoshop.exe"])];
        let mut auto = Auto::new();
        assert_eq!(auto.next("Photoshop.exe", &list, "work"), Some("photo".into()));
        assert_eq!(auto.next("notepad.exe", &list, "photo"), Some("work".into()));
        // the user picks "gaming" by hand while notepad is in front
        assert_eq!(auto.next("chrome.exe", &list, "gaming"), None);
        assert_eq!(auto.next("Photoshop.exe", &list, "gaming"), Some("photo".into()));
        assert_eq!(auto.next("explorer.exe", &list, "photo"), Some("gaming".into()));
    }
}
