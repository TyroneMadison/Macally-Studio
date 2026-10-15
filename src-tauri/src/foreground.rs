//! Foreground-app watcher and automatic profile switching. Emits "foreground-changed" { name } for
//! each app that comes to the front, and once that app has stayed in front for DEBOUNCE, activates
//! the game's own profile while a game is in front (every switch mode), else the first profile
//! whose autoApps lists the app (Auto switch mode), else the profile the user picked last. Apps are
//! named by exe file name on Windows and bundle id on macOS. Each automatic switch also emits
//! "profile-switched" (`Switched`) for the UI.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::AppState;
use crate::profiles::{Game, Profile, Settings, SwitchMode, default_profile_id};

/// How long an app must stay in front before profiles switch for it, so a launcher or a
/// notification that takes focus for a moment never flips profiles back and forth.
pub const DEBOUNCE: Duration = Duration::from_millis(500);
/// How often a game that holds its profile is checked for having quit.
pub const EXIT_POLL: Duration = Duration::from_secs(1);

/// The app in front: exe file name and full path on Windows, bundle id and bundle path on macOS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Front {
    pub name: String,
    pub path: String,
    pub pid: u32,
}

/// `front` is the game: the same exe (bundle id), or anything started from inside the game's folder.
pub fn is_game(g: &Game, front: &Front) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let in_folder = g.folder.as_deref().map(norm).is_some_and(|f| !f.is_empty() && norm(&front.path).starts_with(&format!("{f}/")));
    in_folder || (!g.exe.is_empty() && g.exe.eq_ignore_ascii_case(&front.name))
}

/// What the switcher decides with: the profiles, the active one and which switching is on.
pub struct Ctx<'a> {
    pub profiles: &'a [Profile],
    pub active: &'a str,
    /// Auto switch mode: autoApps rules switch profiles.
    pub rules: bool,
    /// `Settings::game_auto_switch`.
    pub games: bool,
}

impl<'a> Ctx<'a> {
    pub fn new(profiles: &'a [Profile], s: &'a Settings) -> Ctx<'a> {
        Ctx { profiles, active: &s.active_profile, rules: s.switch_mode == SwitchMode::Auto, games: s.game_auto_switch }
    }
}

/// The profile of the game in front, when it switches automatically.
fn game_profile<'a>(front: &Front, ctx: &Ctx<'a>) -> Option<&'a Profile> {
    let switches = |g: &Game| ctx.games && g.auto_switch && is_game(g, front);
    ctx.profiles.iter().find(|p| p.game.as_ref().is_some_and(switches))
}

/// The profile that belongs with `front`: its game's profile, else the first autoApps rule (any case).
pub fn matching<'a>(front: &Front, ctx: &Ctx<'a>) -> Option<&'a Profile> {
    game_profile(front, ctx).or_else(|| {
        let rule = |p: &&Profile| p.auto_apps.iter().any(|a| a.eq_ignore_ascii_case(&front.name));
        ctx.profiles.iter().find(rule).filter(|_| ctx.rules)
    })
}

/// An automatic switch, sent to the UI as "profile-switched".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Switched {
    /// The profile now active, and its name.
    pub id: String,
    pub name: String,
    /// The profile active before.
    pub from: String,
    /// The game that took over; None for an app rule or the way back from a game.
    pub game: Option<Game>,
}

/// Automatic switching as a state machine: the watcher thread feeds it the apps that come to the
/// front and polls it for switches. Time comes in as an argument, so tests never wait.
#[derive(Default)]
pub struct Switcher {
    /// The app in front once it settled; None when its game quit.
    front: Option<Front>,
    /// An app that came to the front, and when; it settles after DEBOUNCE.
    pending: Option<(Front, Instant)>,
    /// Settings or profiles changed since the last poll.
    refresh: bool,
    /// The profile the user picked last (never a game that switches by itself): where switching returns to.
    picked: String,
    /// What the switcher made active last.
    chosen: String,
    /// The process of the game whose profile is active.
    game_pid: Option<u32>,
}

impl Switcher {
    /// An app came to the front (never our own window, so editing a profile never switches it away).
    pub fn seen(&mut self, front: Front, now: Instant) {
        self.pending = Some((front, now));
    }

    /// Settings or profiles changed: decide again, unless the user picked a profile by hand since
    /// the last switch (that pick holds until the next app change).
    pub fn refresh(&mut self) {
        self.refresh = true;
    }

    /// How long the watcher thread may sleep before `poll` has work; None = until the next message.
    pub fn wake_in(&self, now: Instant) -> Option<Duration> {
        if self.refresh {
            return Some(Duration::ZERO);
        }
        let settle = self.pending.as_ref().map(|(_, at)| (*at + DEBOUNCE).saturating_duration_since(now));
        settle.or(self.game_pid.map(|_| EXIT_POLL))
    }

    /// The switch to make now, if any. `alive(pid)`: that process still runs.
    pub fn poll(&mut self, now: Instant, ctx: &Ctx, alive: impl Fn(u32) -> bool) -> Option<Switched> {
        let mut changed = false;
        if let Some((front, _)) = self.pending.take_if(|(_, at)| now >= *at + DEBOUNCE) {
            changed = self.front.as_ref() != Some(&front);
            self.front = Some(front);
        }
        if self.game_pid.is_some_and(|pid| !alive(pid)) {
            // the game quit; whatever is in front now has not been reported (it may be our own window)
            (self.front, self.game_pid, changed) = (None, None, true);
        }
        let by_hand = !self.chosen.is_empty() && ctx.active != self.chosen;
        let refresh = std::mem::take(&mut self.refresh) && !by_hand;
        if !changed && !refresh {
            return None;
        }
        let profile = |id: &str| ctx.profiles.iter().find(|p| p.id == id);
        let switches_itself = |p: &Profile| ctx.games && p.game.as_ref().is_some_and(|g| g.auto_switch);
        if ctx.active != self.chosen && !profile(ctx.active).is_some_and(switches_itself) {
            self.picked = ctx.active.to_string(); // picked by hand: the profile to come back to
        }
        let front = self.front.as_ref();
        let game = front.and_then(|f| game_profile(f, ctx));
        let target = match front.and_then(|f| matching(f, ctx)) {
            Some(p) => p.id.clone(),
            None if profile(&self.picked).is_some() => self.picked.clone(),
            None => default_profile_id().to_string(),
        };
        self.game_pid = game.and(front).map(|f| f.pid);
        self.chosen = target.clone();
        if target == ctx.active {
            return None;
        }
        Some(Switched {
            name: profile(&target).map_or_else(|| target.clone(), |p| p.name.clone()),
            id: target,
            from: ctx.active.to_string(),
            game: game.and_then(|p| p.game.clone()),
        })
    }
}

enum Msg {
    Front(Front),
    Refresh,
}

static SWITCHER: OnceLock<Sender<Msg>> = OnceLock::new();
/// Name and process of the last app reported, so other windows of the same app are not.
static LAST_APP: Mutex<(String, u32)> = Mutex::new((String::new(), 0));

/// Settings or profiles changed in a way that may change which profile belongs in front.
pub fn refresh() {
    if let Some(tx) = SWITCHER.get() {
        let _ = tx.send(Msg::Refresh);
    }
}

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

fn changed(app: &AppHandle, front: Front) {
    if front.name.eq_ignore_ascii_case(&own_name(app)) {
        return;
    }
    {
        let mut last = LAST_APP.lock().unwrap();
        if last.0 == front.name && last.1 == front.pid {
            return; // another window of the same app
        }
        *last = (front.name.clone(), front.pid);
    }
    let _ = app.emit("foreground-changed", json!({ "name": front.name }));
    if let Some(tx) = SWITCHER.get() {
        let _ = tx.send(Msg::Front(front));
    }
}

/// Call once from setup (on macOS it must run on the main thread, which setup does).
pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel();
    if SWITCHER.set(tx).is_err() {
        return;
    }
    let worker = app.clone();
    std::thread::Builder::new()
        .name("auto-switch".into())
        .spawn(move || {
            let state = || {
                let s = worker.try_state::<AppState>()?;
                let settings = s.settings.lock().unwrap().clone();
                let profiles = s.profiles.lock().unwrap().clone();
                Some((profiles, settings))
            };
            run(&rx, state, |s| match crate::control::activate(&worker, &s.id) {
                Ok(()) => {
                    let _ = worker.emit("profile-switched", s);
                }
                Err(e) => eprintln!("auto switch to {}: {e:#}", s.id),
            });
        })
        .expect("spawn auto-switch thread");
    #[cfg(target_os = "windows")]
    win::watch(move |front| changed(&app, front));
    #[cfg(target_os = "macos")]
    mac::start(app);
}

/// The auto-switch thread: feeds `rx` into a Switcher, applies its switches, sleeps in between.
/// `state` gives the current profiles and settings.
fn run(rx: &Receiver<Msg>, state: impl Fn() -> Option<(Vec<Profile>, Settings)>, apply: impl Fn(&Switched)) {
    let mut sw = Switcher::default();
    loop {
        let msg = match sw.wake_in(Instant::now()) {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match msg {
            Ok(Msg::Front(front)) => sw.seen(front, Instant::now()),
            Ok(Msg::Refresh) => sw.refresh(),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some((profiles, settings)) = state() else { continue };
        if let Some(s) = sw.poll(Instant::now(), &Ctx::new(&profiles, &settings), alive) {
            apply(&s);
        }
    }
}

#[cfg(target_os = "windows")]
use win::alive;

#[cfg(target_os = "macos")]
use mac::alive;

#[cfg(target_os = "windows")]
pub use win::running_apps;

#[cfg(target_os = "macos")]
pub use mac::running_apps;

/// Windows shell processes that own windows but are no programs to bind (File Explorer included).
pub fn is_shell(path: &str) -> bool {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_ascii_lowercase();
    [
        "explorer.exe",
        "applicationframehost.exe",
        "textinputhost.exe",
        "shellexperiencehost.exe",
        "startmenuexperiencehost.exe",
        "searchhost.exe",
        "lockapp.exe",
    ]
    .contains(&name.as_str())
}

#[cfg(target_os = "windows")]
mod win {
    use std::path::Path;
    use std::sync::OnceLock;

    use windows::Win32::Foundation::{CloseHandle, E_ACCESSDENIED, HWND, LPARAM, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EVENT_SYSTEM_FOREGROUND, EnumWindows, GW_OWNER, GWL_EXSTYLE, GetMessageW, GetWindow,
        GetWindowLongW, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, MSG, WINEVENT_OUTOFCONTEXT,
        WS_EX_TOOLWINDOW,
    };
    use windows::core::{BOOL, PWSTR};

    use super::Front;
    use crate::catalog::AppEntry;

    type OnFront = Box<dyn Fn(Front) + Send + Sync>;
    static ON_FRONT: OnceLock<OnFront> = OnceLock::new();

    /// Calls `on_front`, on a thread of its own, each time a window comes to the front.
    pub fn watch(on_front: impl Fn(Front) + Send + Sync + 'static) {
        if ON_FRONT.set(Box::new(on_front)).is_err() {
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
        if let (Some(on_front), Some(front)) = (ON_FRONT.get(), front_of(hwnd)) {
            on_front(front);
        }
    }

    // ponytail: Store apps report ApplicationFrameHost.exe; resolve the hosted app if users ask for it.
    pub fn front_of(hwnd: HWND) -> Option<Front> {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let path = exe_path(pid)?;
        let name = path.rsplit(['\\', '/']).next()?.to_string();
        Some(Front { name, path, pid })
    }

    /// Full path of the exe a process runs.
    pub fn exe_path(pid: u32) -> Option<String> {
        if pid == 0 {
            return None;
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len) };
        let _ = unsafe { CloseHandle(handle) };
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }

    /// The process still runs. One we may not look at counts as running: the next app change decides.
    pub fn alive(pid: u32) -> bool {
        let handle = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            Ok(h) => h,
            Err(e) => return e.code() == E_ACCESSDENIED,
        };
        let mut code = 0u32;
        let read = unsafe { GetExitCodeProcess(handle, &mut code) }.is_ok();
        let _ = unsafe { CloseHandle(handle) };
        !read || code == STILL_ACTIVE.0 as u32
    }

    /// Programs with a window open right now (one entry each, with its icon), for adding a running
    /// game. Leaves out this app and the parts of the Windows shell.
    pub fn running_apps(icons_dir: &Path) -> Vec<AppEntry> {
        let me = std::process::id();
        let mut seen = std::collections::HashSet::new();
        let mut out: Vec<AppEntry> = app_windows()
            .into_iter()
            .filter(|&(_, pid)| pid != me)
            .filter_map(|(_, pid)| exe_path(pid))
            .filter(|path| !super::is_shell(path) && seen.insert(path.to_lowercase()))
            .map(|path| {
                let stem = || Path::new(&path).file_stem().map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
                AppEntry {
                    id: format!("running:{path}"),
                    name: crate::catalog::programs::file_description(&path).unwrap_or_else(stem),
                    kind: "path".into(),
                    icon: crate::catalog::windows::cached_icon(&path, icons_dir),
                    launch: path,
                    source: "running".into(),
                }
            })
            .collect();
        out.sort_by_cached_key(|e| e.name.to_lowercase());
        out
    }

    /// Windows people switch between: visible, titled, top-level, unowned and not tool windows.
    pub fn app_windows() -> Vec<(HWND, u32)> {
        unsafe extern "system" fn each(hwnd: HWND, out: LPARAM) -> BOOL {
            let out = unsafe { &mut *(out.0 as *mut Vec<(HWND, u32)>) };
            let tool = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOOLWINDOW.0 != 0;
            let owned = unsafe { GetWindow(hwnd, GW_OWNER) }.is_ok_and(|o| !o.is_invalid());
            if unsafe { IsWindowVisible(hwnd) }.as_bool() && !tool && !owned && unsafe { GetWindowTextLengthW(hwnd) } > 0 {
                let mut pid = 0u32;
                unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
                out.push((hwnd, pid));
            }
            true.into()
        }
        let mut out: Vec<(HWND, u32)> = Vec::new();
        let _ = unsafe { EnumWindows(Some(each), LPARAM(&mut out as *mut Vec<(HWND, u32)> as isize)) };
        out
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::path::Path;
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2_app_kit::{
        NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
        NSWorkspaceDidActivateApplicationNotification,
    };
    use objc2_foundation::NSNotification;
    use tauri::AppHandle;

    use super::Front;
    use crate::catalog::AppEntry;

    pub fn start(app: AppHandle) {
        let block = RcBlock::new(move |n: NonNull<NSNotification>| {
            let front = unsafe { n.as_ref() }
                .userInfo()
                .and_then(|info| info.objectForKey(unsafe { NSWorkspaceApplicationKey }))
                .and_then(|o| o.downcast::<NSRunningApplication>().ok())
                .and_then(|a| front_of(&a));
            if let Some(front) = front {
                super::changed(&app, front);
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

    fn front_of(a: &NSRunningApplication) -> Option<Front> {
        let name = a.bundleIdentifier()?.to_string();
        let path = a.bundleURL().and_then(|u| u.path()).map(|p| p.to_string()).unwrap_or_default();
        Some(Front { name, path, pid: a.processIdentifier() as u32 })
    }

    pub fn alive(pid: u32) -> bool {
        NSRunningApplication::runningApplicationWithProcessIdentifier(pid as _).is_some_and(|a| !a.isTerminated())
    }

    /// Apps with a Dock icon that run now (one entry each, with its icon), leaving out this app.
    pub fn running_apps(icons_dir: &Path) -> Vec<AppEntry> {
        let me = std::process::id();
        let mut out: Vec<AppEntry> = NSWorkspace::sharedWorkspace()
            .runningApplications()
            .iter()
            .filter(|a| a.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .filter_map(|a| {
                let f = front_of(&a).filter(|f| f.pid != me)?;
                Some(AppEntry {
                    id: format!("running:{}", f.name),
                    name: a.localizedName().map_or_else(|| f.name.clone(), |n| n.to_string()),
                    kind: "bundle".into(),
                    icon: crate::catalog::macos::cached_icon(Path::new(&f.path), &f.name, icons_dir),
                    launch: f.name,
                    source: "running".into(),
                })
            })
            .collect();
        out.sort_by_cached_key(|e| e.name.to_lowercase());
        out
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::profiles::{Game, default_profile_id};

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

    fn game(exe: &str) -> Game {
        Game { name: format!("Game {exe}"), exe: exe.into(), folder: None, icon: Some(format!("{exe}.png")), auto_switch: true }
    }

    /// The profile of the game `exe`.
    fn g(id: &str, exe: &str) -> Profile {
        Profile { game: Some(game(exe)), ..p(id, &[]) }
    }

    fn front(name: &str, path: &str) -> Front {
        Front { name: name.into(), path: path.into(), pid: 1 }
    }

    #[test]
    fn a_game_is_its_exe_in_any_case() {
        let portal = game("Portal2.exe");
        assert!(is_game(&portal, &front("portal2.exe", r"D:\Games\portal2.exe")));
        assert!(!is_game(&portal, &front("portal.exe", r"D:\Games\portal.exe")));
        let nameless = Game { exe: String::new(), ..portal };
        assert!(!is_game(&nameless, &front("", "")), "an empty exe matches nothing");
    }

    #[test]
    fn a_folder_game_is_anything_started_inside_its_folder() {
        let folder = |dir: &str| Game { exe: String::new(), folder: Some(dir.into()), ..game("") };
        let steam = folder(r"G:\SteamLibrary\steamapps\common\Portal 2");
        assert!(is_game(&steam, &front("portal2.exe", r"g:\steamlibrary\steamapps\common\portal 2\bin\portal2.exe")));
        assert!(is_game(&steam, &front("launcher.exe", r"G:\SteamLibrary\steamapps\common\Portal 2\launcher.exe")));
        assert!(!is_game(&steam, &front("tool.exe", r"G:\SteamLibrary\steamapps\common\Portal 2 Tools\tool.exe")));
        assert!(!is_game(&steam, &front("steam.exe", r"G:\SteamLibrary\steam.exe")));
        let mac = folder("/Users/me/Library/Application Support/Steam/steamapps/common/Portal 2/");
        assert!(is_game(&mac, &front("com.valve.portal2", "/Users/me/Library/Application Support/Steam/steamapps/common/Portal 2/Portal 2.app")));
    }

    #[test]
    fn games_win_over_app_rules_and_rules_need_auto_mode() {
        let list = vec![p("photo", &["Photoshop.exe", "portal2.exe"]), p("other", &["photoshop.exe"]), g("portal", "portal2.exe")];
        let mut ctx = Ctx { profiles: &list, active: "work", rules: true, games: true };
        let id = |p: Option<&Profile>| p.map(|p| p.id.clone());
        assert_eq!(id(matching(&front("PHOTOSHOP.EXE", ""), &ctx)), Some("photo".into()), "first rule, any case");
        assert_eq!(id(matching(&front("portal2.exe", ""), &ctx)), Some("portal".into()), "the game before a rule");
        assert_eq!(id(matching(&front("notepad.exe", ""), &ctx)), None);
        ctx.rules = false; // Manual or Hotkey mode
        assert_eq!(id(matching(&front("Photoshop.exe", ""), &ctx)), None);
        assert_eq!(id(matching(&front("portal2.exe", ""), &ctx)), Some("portal".into()), "games switch in every mode");
        ctx.games = false;
        assert_eq!(id(matching(&front("portal2.exe", ""), &ctx)), None);
    }

    /// Plays the watcher thread: polls every 50 ms of fake time and applies each switch to `active`.
    struct Sim {
        sw: Switcher,
        now: Instant,
        profiles: Vec<Profile>,
        active: String,
        rules: bool,
        games: bool,
        exited: Vec<u32>,
        switched: Vec<Switched>,
    }

    impl Sim {
        fn new(profiles: Vec<Profile>, active: &str) -> Sim {
            Sim {
                sw: Switcher::default(),
                now: Instant::now(),
                profiles,
                active: active.into(),
                rules: true,
                games: true,
                exited: Vec::new(),
                switched: Vec::new(),
            }
        }

        fn focus(&mut self, exe: &str, pid: u32) {
            self.sw.seen(Front { name: exe.into(), path: format!(r"C:\Apps\{exe}"), pid }, self.now);
        }

        /// Lets `ms` pass and returns the active profile.
        fn wait(&mut self, ms: u64) -> String {
            for _ in 0..ms / 50 {
                self.now += Duration::from_millis(50);
                let ctx = Ctx { profiles: &self.profiles, active: &self.active, rules: self.rules, games: self.games };
                let exited = &self.exited;
                if let Some(s) = self.sw.poll(self.now, &ctx, |pid| !exited.contains(&pid)) {
                    self.active = s.id.clone();
                    self.switched.push(s);
                }
            }
            self.active.clone()
        }

        fn game_mut(&mut self, id: &str) -> &mut Game {
            self.profiles.iter_mut().find(|p| p.id == id).and_then(|p| p.game.as_mut()).unwrap()
        }
    }

    const SETTLE: u64 = DEBOUNCE.as_millis() as u64 + 100;

    fn world() -> Vec<Profile> {
        vec![p(default_profile_id(), &[]), p("work", &[]), g("portal", "portal2.exe")]
    }

    #[test]
    fn a_game_in_front_switches_to_its_profile_after_the_debounce() {
        let mut s = Sim::new(world(), "work");
        s.focus("portal2.exe", 7);
        assert_eq!(s.wait(DEBOUNCE.as_millis() as u64 - 100), "work", "not before the debounce");
        assert_eq!(s.wait(200), "portal");
        let to_game = Switched { id: "portal".into(), name: "portal".into(), from: "work".into(), game: Some(game("portal2.exe")) };
        assert_eq!(s.switched, [to_game]);
    }

    #[test]
    fn leaving_the_game_goes_back_to_the_profile_from_before() {
        let mut s = Sim::new(world(), "work");
        s.focus("portal2.exe", 7);
        s.wait(SETTLE);
        s.focus("chrome.exe", 8);
        assert_eq!(s.wait(SETTLE), "work");
        let back = Switched { id: "work".into(), name: "work".into(), from: "portal".into(), game: None };
        assert_eq!(s.switched.last(), Some(&back));
    }

    #[test]
    fn quick_focus_changes_never_switch() {
        let mut s = Sim::new(world(), "work");
        for _ in 0..3 {
            s.focus("portal2.exe", 7);
            s.wait(200);
            s.focus("chrome.exe", 8);
            s.wait(200);
        }
        assert_eq!(s.wait(SETTLE), "work");
        assert!(s.switched.is_empty());
        // in the game, a notification that takes focus for a moment
        s.focus("portal2.exe", 7);
        s.wait(SETTLE);
        s.focus("ShellExperienceHost.exe", 9);
        s.wait(150);
        s.focus("portal2.exe", 7);
        assert_eq!(s.wait(SETTLE * 2), "portal");
        assert_eq!(s.switched.len(), 1);
    }

    #[test]
    fn a_game_that_exits_gives_back_its_profile() {
        let mut s = Sim::new(world(), "work");
        s.focus("portal2.exe", 7);
        s.wait(SETTLE);
        // the game quits while our own window is in front, which the watcher never reports
        s.exited.push(7);
        assert_eq!(s.wait(EXIT_POLL.as_millis() as u64 + 100), "work");
        assert_eq!(s.switched.last().map(|e| e.game.is_none()), Some(true));
    }

    #[test]
    fn turning_auto_switch_off_stops_the_game_and_hands_back_its_profile() {
        let mut s = Sim::new(world(), "work");
        s.game_mut("portal").auto_switch = false;
        s.focus("portal2.exe", 7);
        assert_eq!(s.wait(SETTLE), "work", "per game");
        s.game_mut("portal").auto_switch = true;
        s.sw.refresh();
        assert_eq!(s.wait(100), "portal", "on again while the game is in front");
        s.games = false;
        s.sw.refresh();
        assert_eq!(s.wait(100), "work", "every game");
        s.games = true;
        s.sw.refresh();
        assert_eq!(s.wait(100), "portal");
        s.game_mut("portal").auto_switch = false;
        s.sw.refresh();
        assert_eq!(s.wait(100), "work");
    }

    #[test]
    fn the_default_profile_when_the_one_from_before_is_gone() {
        let mut s = Sim::new(world(), "work");
        s.focus("portal2.exe", 7);
        s.wait(SETTLE);
        s.profiles.retain(|p| p.id != "work");
        s.focus("chrome.exe", 8);
        assert_eq!(s.wait(SETTLE), default_profile_id());
    }

    #[test]
    fn a_game_profile_is_never_the_one_to_come_back_to() {
        // left active by the last session: there is nothing to come back to but the default
        let mut s = Sim::new(world(), "portal");
        s.focus("chrome.exe", 8);
        assert_eq!(s.wait(SETTLE), default_profile_id());
        // picked by hand (to edit it): the next app change still returns to the profile from before
        s.active = "work".into();
        s.focus("notepad.exe", 9);
        s.wait(SETTLE);
        s.active = "portal".into();
        s.focus("explorer.exe", 10);
        assert_eq!(s.wait(SETTLE), "work");
    }

    #[test]
    fn a_profile_picked_by_hand_holds_until_the_next_app_change() {
        let mut s = Sim::new(world(), "work");
        s.profiles.push(p("racing", &[]));
        s.focus("portal2.exe", 7);
        s.wait(SETTLE);
        s.active = "racing".into(); // a hotkey or the tray, while the game is in front
        s.sw.refresh(); // say, a setting saved meanwhile
        assert_eq!(s.wait(SETTLE), "racing");
        s.focus("chrome.exe", 8);
        assert_eq!(s.wait(SETTLE), "racing", "now the profile to come back to");
        s.focus("portal2.exe", 7);
        assert_eq!(s.wait(SETTLE), "portal");
        s.focus("chrome.exe", 8);
        assert_eq!(s.wait(SETTLE), "racing");
    }

    #[test]
    fn app_rules_return_to_the_profile_the_user_picked() {
        let mut s = Sim::new(vec![p("work", &[]), p("gaming", &[]), p("photo", &["Photoshop.exe"])], "work");
        s.focus("Photoshop.exe", 1);
        assert_eq!(s.wait(SETTLE), "photo");
        s.focus("notepad.exe", 2);
        assert_eq!(s.wait(SETTLE), "work");
        s.active = "gaming".into(); // the user picks "gaming" by hand while notepad is in front
        s.focus("chrome.exe", 3);
        assert_eq!(s.wait(SETTLE), "gaming");
        s.focus("Photoshop.exe", 1);
        assert_eq!(s.wait(SETTLE), "photo");
        s.focus("explorer.exe", 4);
        assert_eq!(s.wait(SETTLE), "gaming");
        s.rules = false; // Manual mode
        s.focus("Photoshop.exe", 1);
        assert_eq!(s.wait(SETTLE), "gaming");
    }

    #[test]
    fn the_watcher_sleeps_until_something_is_due() {
        let list = world();
        let t = Instant::now();
        let ctx = Ctx { profiles: &list, active: "work", rules: true, games: true };
        let mut sw = Switcher::default();
        assert_eq!(sw.wake_in(t), None, "nothing to do: wait for the next app change");
        sw.seen(front("portal2.exe", r"C:\portal2.exe"), t);
        assert_eq!(sw.wake_in(t + Duration::from_millis(100)), Some(DEBOUNCE - Duration::from_millis(100)));
        assert!(sw.poll(t + DEBOUNCE, &ctx, |_| true).is_some());
        assert_eq!(sw.wake_in(t + DEBOUNCE), Some(EXIT_POLL), "checks that the game still runs");
        sw.refresh();
        assert_eq!(sw.wake_in(t + DEBOUNCE), Some(Duration::ZERO));
    }

    #[test]
    fn the_windows_shell_is_no_running_app() {
        for p in [r"C:\Windows\explorer.exe", r"C:\Windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy\ShellExperienceHost.exe"] {
            assert!(is_shell(p), "{p}");
        }
        for p in [r"C:\Windows\System32\notepad.exe", r"G:\SteamLibrary\steamapps\common\Portal 2\portal2.exe"] {
            assert!(!is_shell(p), "{p}");
        }
    }

    /// The apps with a window on this desktop: real programs with their icons, none of them twice.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "reads the windows on this desktop and writes icons to a temp folder"]
    fn running_apps_on_this_machine() {
        let dir = std::env::temp_dir().join(format!("macally-running-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let apps = running_apps(&dir);
        eprintln!("{} running apps, {} with an icon", apps.len(), apps.iter().filter(|a| a.icon.is_some()).count());
        for a in &apps {
            assert!(a.launch.to_ascii_lowercase().ends_with(".exe") && std::path::Path::new(&a.launch).is_file(), "{}", a.launch);
            assert!(!is_shell(&a.launch) && !a.name.is_empty());
        }
        let mut paths: Vec<String> = apps.iter().map(|a| a.launch.to_lowercase()).collect();
        paths.dedup();
        assert_eq!(paths.len(), apps.len());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn alive_tells_a_running_process_from_one_that_quit() {
        assert!(win::alive(std::process::id()));
        let mut child = std::process::Command::new("cmd").args(["/c", "exit"]).spawn().unwrap();
        child.wait().unwrap();
        assert!(!win::alive(child.id()), "quit, though its handle is still open");
    }

    /// The whole watcher on this desktop with Notepad as a stand-in game: the real foreground hook,
    /// the Switcher and the exit check. Notepad must not be open already; the test never touches
    /// windows it did not open. Run with --nocapture to see the timeline. Windows 11 Notepad
    /// reopens its last session and saves it again on close: back up
    /// %LOCALAPPDATA%\Packages\Microsoft.WindowsNotepad_8wekyb3d8bbwe\LocalState first if that matters.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "opens Notepad, brings it to the front and closes it again"]
    fn notepad_as_a_stand_in_game_switches_in_and_back() {
        use std::sync::{Arc, mpsc};

        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, SetForegroundWindow, WM_CLOSE,
        };

        let notepads = || -> Vec<HWND> {
            let is_notepad = |pid| win::exe_path(pid).is_some_and(|p| p.to_lowercase().ends_with(r"\notepad.exe"));
            win::app_windows().into_iter().filter(|&(_, pid)| is_notepad(pid)).map(|(h, _)| h).collect()
        };
        if !notepads().is_empty() {
            eprintln!("Notepad is already open: skipped, so no window of yours is touched");
            return;
        }
        let t0 = Instant::now();
        let log = move |what: String| eprintln!("{:>6} ms  {what}", t0.elapsed().as_millis());

        let profiles = vec![p(default_profile_id(), &[]), g("notepad", "notepad.exe")];
        let active = Arc::new(Mutex::new(default_profile_id().to_string()));
        let (tx, rx) = mpsc::channel();
        win::watch(move |f| {
            log(format!("in front: {} (pid {})", f.name, f.pid));
            let _ = tx.send(Msg::Front(f));
        });
        let (switched_tx, switched) = mpsc::channel();
        let (read, write) = (active.clone(), active.clone());
        std::thread::spawn(move || {
            let state = || Some((profiles.clone(), Settings { active_profile: read.lock().unwrap().clone(), ..Settings::default() }));
            run(&rx, state, |s| {
                *write.lock().unwrap() = s.id.clone();
                let _ = switched_tx.send(s.clone());
            });
        });

        let mut child = std::process::Command::new("notepad.exe").spawn().expect("start Notepad");
        let hwnd = (0..100)
            .find_map(|_| notepads().first().copied().or_else(|| (std::thread::sleep(Duration::from_millis(100)), None).1))
            .expect("Notepad opened a window");
        unsafe {
            // a background process may only take the focus while attached to the one in front
            let front_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let me = GetCurrentThreadId();
            let _ = AttachThreadInput(me, front_thread, true);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = AttachThreadInput(me, front_thread, false);
        }
        let to_game = switched.recv_timeout(Duration::from_secs(10)).expect("a switch to the Notepad profile");
        log(format!("switched to {:?} for {:?}", to_game.id, to_game.game.as_ref().map(|g| &g.name)));
        assert_eq!((to_game.id.as_str(), to_game.from.as_str()), ("notepad", default_profile_id()));
        assert_eq!(to_game.game.map(|g| g.exe).as_deref(), Some("notepad.exe"));

        for h in notepads() {
            let _ = unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        log("closed Notepad".into());
        let back = switched.recv_timeout(Duration::from_secs(10)).expect("a switch back after Notepad closed");
        log(format!("switched back to {:?}", back.id));
        assert_eq!((back.id.as_str(), back.from.as_str()), (default_profile_id(), "notepad"));
        assert!(back.game.is_none());
        assert_eq!(*active.lock().unwrap(), default_profile_id());
        assert!(notepads().is_empty(), "Notepad closed");
        let _ = child.wait();
    }

    #[test]
    fn switched_serializes_for_the_ui() {
        let s = Switched { id: "portal".into(), name: "Portal 2".into(), from: "work".into(), game: Some(game("portal2.exe")) };
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::json!({ "id": "portal", "name": "Portal 2", "from": "work",
                "game": { "name": "Game portal2.exe", "exe": "portal2.exe", "icon": "portal2.exe.png", "autoSwitch": true } })
        );
    }
}
