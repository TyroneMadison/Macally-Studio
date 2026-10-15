//! Tauri commands invoked from the webview (lib/ipc.ts).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::catalog::{self, AppEntry};
use crate::profiles::{Game, Profile, Settings, Store, default_profile_id};

/// The 107-key layout, compiled into the binary (no resource file to ship).
pub const LAYOUT_JSON: &str = include_str!("../../src/lib/layout.json");

#[tauri::command]
pub fn get_layout() -> Result<serde_json::Value, String> {
    serde_json::from_str(LAYOUT_JSON).map_err(|e| e.to_string())
}

/// What profile/settings commands need from the remap engine; the engine implements it.
pub trait EngineLike: Send + Sync {
    fn set_profile(&self, p: &Profile);
}

/// Shared app state. Locks are taken one at a time, never nested, so lock order cannot deadlock.
pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<Settings>,
    pub profiles: Mutex<Vec<Profile>>,
    pub engine: Arc<dyn EngineLike>,
}

impl AppState {
    /// Installs built-ins, loads everything and hands the active profile to the engine.
    pub fn load(store: Store, engine: Arc<dyn EngineLike>) -> anyhow::Result<AppState> {
        store.ensure_builtins()?;
        let profiles = store.list_profiles()?;
        let mut settings = store.load_settings();
        if !profiles.iter().any(|p| p.id == settings.active_profile) {
            settings.active_profile = default_profile_id().into();
        }
        let active = settings.active_profile.clone();
        let state = AppState {
            store: Mutex::new(store),
            settings: Mutex::new(settings),
            profiles: Mutex::new(profiles),
            engine,
        };
        state.activate(&active)?;
        Ok(state)
    }

    pub fn profile(&self, id: &str) -> anyhow::Result<Profile> {
        self.profiles
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .with_context(|| format!("no profile with id {id:?}"))
    }

    pub fn active_id(&self) -> String {
        self.settings.lock().unwrap().active_profile.clone()
    }

    fn reload(&self) -> anyhow::Result<()> {
        let list = self.store.lock().unwrap().list_profiles()?;
        *self.profiles.lock().unwrap() = list;
        Ok(())
    }

    /// What the engine runs for `p`: a game's own keys on top of the Default profile's, so a game
    /// changes only the keys it binds. Any other profile runs exactly its own keys.
    fn engine_profile(&self, p: &Profile) -> Profile {
        match (&p.game, self.profile(default_profile_id())) {
            (Some(_), Ok(base)) => {
                let mut bindings = base.bindings;
                bindings.extend(p.bindings.clone());
                Profile { bindings, ..p.clone() }
            }
            _ => p.clone(),
        }
    }

    /// Engine table + settings.json. Every switch goes through here: the UI, tray, hotkeys,
    /// the switch_profile action and Auto mode (which remembers the last manual pick itself).
    pub fn activate(&self, id: &str) -> anyhow::Result<()> {
        let p = self.profile(id)?;
        self.engine.set_profile(&self.engine_profile(&p));
        let settings = {
            let mut s = self.settings.lock().unwrap();
            s.active_profile = p.id;
            s.clone()
        };
        self.store.lock().unwrap().save_settings(&settings)
    }

    pub fn save(&self, p: Profile) -> anyhow::Result<()> {
        self.store.lock().unwrap().save_profile(&p)?;
        self.written(&p)
    }

    /// A built-in back the way it shipped.
    pub fn reset(&self, id: &str) -> anyhow::Result<()> {
        let p = self.store.lock().unwrap().reset_profile(id)?;
        self.written(&p)
    }

    /// After a profile file changed: reload the list, and the engine when it runs that profile (the
    /// active one, or the Default under an active game's).
    fn written(&self, p: &Profile) -> anyhow::Result<()> {
        self.reload()?;
        if let Ok(active) = self.profile(&self.active_id())
            && (p.id == active.id || (p.id == default_profile_id() && active.game.is_some()))
        {
            self.engine.set_profile(&self.engine_profile(&active));
        }
        Ok(())
    }

    /// Returns the id that became active when the active profile was the one deleted.
    pub fn delete(&self, id: &str) -> anyhow::Result<Option<String>> {
        self.store.lock().unwrap().delete_profile(id)?;
        self.reload()?;
        if id != self.active_id() {
            return Ok(None);
        }
        self.activate(default_profile_id())?;
        Ok(Some(default_profile_id().into()))
    }

    pub fn duplicate(&self, id: &str) -> anyhow::Result<Profile> {
        let p = self.store.lock().unwrap().duplicate_profile(id)?;
        self.reload()?;
        Ok(p)
    }

    pub fn import(&self, path: &Path) -> anyhow::Result<Profile> {
        let p = self.store.lock().unwrap().import_profile(path)?;
        self.reload()?;
        Ok(p)
    }

    /// The profiles that belong to a game.
    pub fn games(&self) -> Vec<Profile> {
        self.profiles.lock().unwrap().iter().filter(|p| p.game.is_some()).cloned().collect()
    }

    /// A game's own profile, starting with a copy of `base`'s bindings, or with none (the Default
    /// profile's keys then work in the game until it changes them).
    pub fn add_game(&self, game: Game, base: Option<&str>) -> anyhow::Result<Profile> {
        let p = self.store.lock().unwrap().add_game(game, base)?;
        self.reload()?;
        Ok(p)
    }

    /// Auto-switch for one game, or for every game when `id` is None.
    pub fn set_game_auto_switch(&self, id: Option<&str>, on: bool) -> anyhow::Result<()> {
        let Some(id) = id else {
            let s = Settings { game_auto_switch: on, ..self.settings.lock().unwrap().clone() };
            return self.save_settings(s);
        };
        let p = self.store.lock().unwrap().set_game_auto_switch(id, on)?;
        self.written(&p)
    }

    /// Deletes a game's profile. Returns the id that became active when it was the active one.
    pub fn remove_game(&self, id: &str) -> anyhow::Result<Option<String>> {
        anyhow::ensure!(self.profile(id)?.game.is_some(), "{id:?} is not a game profile");
        self.delete(id)
    }

    /// The UI may hold a stale activeProfile; only `activate` changes it.
    pub fn save_settings(&self, mut s: Settings) -> anyhow::Result<()> {
        s.active_profile = self.active_id();
        self.store.lock().unwrap().save_settings(&s)?;
        *self.settings.lock().unwrap() = s;
        Ok(())
    }
}

type CmdResult<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn emit_profile_changed(app: &AppHandle, id: &str) {
    let _ = app.emit("profile-changed", json!({ "id": id }));
}

#[tauri::command]
pub fn get_profiles(state: State<'_, AppState>) -> Vec<Profile> {
    state.profiles.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_profile(state: State<'_, AppState>, id: String) -> CmdResult<Profile> {
    state.profile(&id).map_err(err)
}

#[tauri::command]
pub fn save_profile(app: AppHandle, state: State<'_, AppState>, profile: Profile) -> CmdResult<()> {
    state.save(profile).map_err(err)?;
    crate::control::resync(&app);
    Ok(())
}

#[tauri::command]
pub fn reset_profile(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.reset(&id).map_err(err)?;
    crate::control::resync(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    if let Some(active) = state.delete(&id).map_err(err)? {
        emit_profile_changed(&app, &active);
    }
    crate::control::resync(&app);
    Ok(())
}

#[tauri::command]
pub fn duplicate_profile(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<Profile> {
    let p = state.duplicate(&id).map_err(err)?;
    crate::control::resync(&app);
    Ok(p)
}

#[tauri::command]
pub fn import_profile(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<Profile> {
    let p = state.import(Path::new(&path)).map_err(err)?;
    crate::control::resync(&app);
    Ok(p)
}

#[tauri::command]
pub fn export_profile(state: State<'_, AppState>, id: String, path: String) -> CmdResult<()> {
    state.store.lock().unwrap().export_profile(&id, Path::new(&path)).map_err(err)
}

#[tauri::command]
pub fn set_active_profile(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.activate(&id).map_err(err)?;
    emit_profile_changed(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> CmdResult<()> {
    let login = |s: &Settings| (s.start_at_login, s.run_as_admin);
    let switching = |s: &Settings| (s.switch_mode, s.game_auto_switch);
    let (login_changed, switching_changed) = {
        let old = state.settings.lock().unwrap();
        (login(&old) != login(&settings), switching(&old) != switching(&settings))
    };
    state.save_settings(settings).map_err(err)?;
    crate::control::resync(&app);
    if login_changed {
        crate::control::sync_autostart(&app);
    }
    if switching_changed {
        crate::foreground::refresh();
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub active_profile: String,
    pub hook_running: bool,
    pub device_found: bool,
    pub version: String,
    /// "windows" or "macos".
    pub platform: &'static str,
    /// macOS: Accessibility and Input Monitoring are both on. Always true on Windows.
    pub permissions_ok: bool,
}

/// Async: runs off the main thread, because device_found runs `ioreg` on macOS.
#[tauri::command(async)]
pub fn get_status(app: AppHandle, state: State<'_, AppState>) -> Status {
    Status {
        active_profile: state.active_id(),
        hook_running: crate::engine::hook_running(),
        device_found: crate::engine::device_found(),
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
        permissions_ok: crate::engine::permissions_ok(),
    }
}

type EngineState<'a> = State<'a, Arc<crate::engine::Engine>>;

/// Key Monitor: every key goes to the UI as "key-event"; remapping keeps working.
/// Never while the window is hidden (close-to-tray, login start): keys typed in other apps stay out of the UI.
#[tauri::command]
pub fn set_monitor(window: tauri::Window, engine: EngineState<'_>, on: bool) {
    let on = on && window.is_visible().unwrap_or(false);
    engine.set_mode(if on { crate::engine::Mode::Monitor } else { crate::engine::Mode::Normal });
}

/// The cached catalog, returned at once (empty on the very first run). The first call per launch
/// also starts a background rescan; its "catalog-progress" events tell the UI to fetch again.
#[tauri::command]
pub async fn get_catalog(app: AppHandle) -> CmdResult<Vec<AppEntry>> {
    static STARTED: AtomicBool = AtomicBool::new(false);
    let store = Store::open().map_err(err)?;
    let cached = catalog::load_cached(&store);
    if !STARTED.swap(true, Ordering::Relaxed) {
        std::thread::spawn(move || {
            if let Err(e) = catalog::refresh(&store, &app) {
                eprintln!("catalog: background refresh failed: {e:#}");
            }
        });
    }
    Ok(cached)
}

/// Full rescan, waited for (the picker's Refresh button).
#[tauri::command]
pub async fn refresh_catalog(app: AppHandle) -> CmdResult<Vec<AppEntry>> {
    tauri::async_runtime::spawn_blocking(move || catalog::refresh(&Store::open()?, &app))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

/// Browse for .exe/.app: keeps the program in the catalog. The "catalog-progress" event makes the UI refetch.
#[tauri::command]
pub async fn add_catalog_entry(app: AppHandle, path: String) -> CmdResult<Vec<AppEntry>> {
    let all = tauri::async_runtime::spawn_blocking(move || catalog::add_path(&Store::open()?, &path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    let _ = app.emit("catalog-progress", json!({ "done": 1, "total": 1, "source": "browse" }));
    Ok(all)
}

// Games: each game has its own profile (`Profile::game`), active while the game is in front
// (foreground.rs). Every automatic switch emits "profile-switched" (foreground::Switched).

/// Programs with a window open now, with their icons: the running apps a game can be added from.
#[tauri::command]
pub async fn get_running_apps() -> CmdResult<Vec<AppEntry>> {
    tauri::async_runtime::spawn_blocking(|| -> anyhow::Result<Vec<AppEntry>> {
        let icons = Store::open()?.data_dir.join("icons");
        std::fs::create_dir_all(&icons)?;
        Ok(crate::foreground::running_apps(&icons))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

#[tauri::command]
pub fn get_games(state: State<'_, AppState>) -> Vec<Profile> {
    state.games()
}

/// Adds the game an entry of get_running_apps or get_catalog starts and returns its profile (the one
/// it already has when added before). `base`: a profile whose bindings it starts with; none starts
/// empty, on top of the Default profile.
/// Async: reading a shortcut or the Steam libraries never holds up the window.
#[tauri::command(async)]
pub fn add_game(app: AppHandle, state: State<'_, AppState>, entry: AppEntry, base: Option<String>) -> CmdResult<Profile> {
    let game = catalog::game_for(&entry).map_err(err)?;
    let p = state.add_game(game, base.as_deref()).map_err(err)?;
    crate::control::resync(&app);
    crate::foreground::refresh();
    Ok(p)
}

/// Deletes a game and its profile.
#[tauri::command]
pub fn remove_game(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    if let Some(active) = state.remove_game(&id).map_err(err)? {
        emit_profile_changed(&app, &active);
    }
    crate::control::resync(&app);
    crate::foreground::refresh();
    Ok(())
}

/// Auto-switch on or off for one game (`id`), or for every game when `id` is null.
#[tauri::command]
pub fn set_game_auto_switch(state: State<'_, AppState>, id: Option<String>, on: bool) -> CmdResult<()> {
    state.set_game_auto_switch(id.as_deref(), on).map_err(err)?;
    crate::foreground::refresh();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap;
    use crate::profiles::{Binding, temp_store};
    use std::collections::BTreeMap;

    /// Records which profile the engine was last given.
    #[derive(Default)]
    struct Spy {
        last: Mutex<String>,
        ran: Mutex<Option<Profile>>,
    }

    impl Spy {
        fn ran(&self) -> Profile {
            self.ran.lock().unwrap().clone().expect("the engine got a profile")
        }
    }

    impl EngineLike for Spy {
        fn set_profile(&self, p: &Profile) {
            *self.last.lock().unwrap() = p.id.clone();
            *self.ran.lock().unwrap() = Some(p.clone());
        }
    }

    fn setup(tag: &str) -> (AppState, Arc<Spy>) {
        let spy = Arc::new(Spy::default());
        let state = AppState::load(temp_store(tag), spy.clone()).unwrap();
        (state, spy)
    }

    #[test]
    fn load_activates_the_os_default() {
        let (state, spy) = setup("cmd-load");
        assert_eq!(*spy.last.lock().unwrap(), default_profile_id());
        assert_eq!(state.profiles.lock().unwrap().len(), 2);
        assert_eq!(state.store.lock().unwrap().load_settings().active_profile, default_profile_id());
    }

    #[test]
    fn activate_switches_engine_and_persists() {
        let (state, spy) = setup("cmd-activate");
        state.activate("macos-default").unwrap();
        assert_eq!(*spy.last.lock().unwrap(), "macos-default");
        assert_eq!(state.store.lock().unwrap().load_settings().active_profile, "macos-default");
        assert!(state.activate("nope").is_err());
    }

    #[test]
    fn saving_the_active_profile_refreshes_the_engine() {
        let (state, spy) = setup("cmd-save");
        let mut p = state.duplicate("windows-default").unwrap();
        state.activate(&p.id).unwrap();
        spy.last.lock().unwrap().clear();
        p.name = "Renamed".into();
        state.save(p.clone()).unwrap();
        assert_eq!(*spy.last.lock().unwrap(), p.id);
        assert_eq!(state.profile(&p.id).unwrap().name, "Renamed");
    }

    #[test]
    fn resetting_the_active_builtin_refreshes_the_engine() {
        let (state, spy) = setup("cmd-reset");
        let mut p = state.profile("windows-default").unwrap();
        let shipped = p.bindings.clone();
        p.bindings.clear();
        state.save(p).unwrap();
        state.activate("windows-default").unwrap();
        spy.last.lock().unwrap().clear();
        state.reset("windows-default").unwrap();
        assert_eq!(*spy.last.lock().unwrap(), "windows-default");
        assert_eq!(state.profile("windows-default").unwrap().bindings, shipped);
    }

    #[test]
    fn deleting_the_active_profile_falls_back_to_default() {
        let (state, spy) = setup("cmd-delete");
        let p = state.duplicate("windows-default").unwrap();
        state.activate(&p.id).unwrap();
        assert_eq!(state.delete(&p.id).unwrap().as_deref(), Some(default_profile_id()));
        assert_eq!(state.active_id(), default_profile_id());
        assert_eq!(*spy.last.lock().unwrap(), default_profile_id());
        assert!(state.profile(&p.id).is_err());
    }

    #[test]
    fn save_settings_keeps_the_active_profile() {
        let (state, _spy) = setup("cmd-settings");
        state.activate("macos-default").unwrap();
        let stale = Settings { active_profile: "windows-default".into(), close_to_tray: false, ..Settings::default() };
        state.save_settings(stale).unwrap();
        let saved = state.store.lock().unwrap().load_settings();
        assert_eq!(saved.active_profile, "macos-default");
        assert!(!saved.close_to_tray);
    }

    fn portal() -> Game {
        Game { name: "Portal 2".into(), exe: "portal2.exe".into(), folder: None, path: None, icon: None, auto_switch: true }
    }

    #[test]
    fn a_game_profile_runs_on_top_of_the_default_profile() {
        let (state, spy) = setup("cmd-layers");
        let p = state.add_game(portal(), None).unwrap();
        assert!(p.bindings.is_empty(), "a new game starts with no keys of its own");
        let off = Binding { mods: vec![], action: crate::actions::Action::Block };
        state.save(Profile { bindings: BTreeMap::from([("F3".to_string(), off.clone())]), ..p.clone() }).unwrap();
        state.activate(&p.id).unwrap();
        let default = state.profile(default_profile_id()).unwrap();
        let ran = spy.ran();
        assert_eq!(ran.id, p.id);
        assert_eq!(ran.bindings["F3"], off, "the game's own key wins");
        for (code, b) in default.bindings.iter().filter(|(c, _)| c.as_str() != "F3") {
            assert_eq!(ran.bindings.get(code), Some(b), "{code} comes from the Default profile");
        }
        // a change to the Default reaches the engine while the game runs
        let f20 = Binding { mods: vec![], action: crate::actions::Action::KeyCombo { keys: vec!["KeyA".into()] } };
        let mut edited = default.clone();
        edited.bindings.insert("F20".into(), f20.clone());
        state.save(edited).unwrap();
        assert_eq!(spy.ran().bindings.get("F20"), Some(&f20));
        assert!(!state.profile(&p.id).unwrap().bindings.contains_key("F20"), "only the engine merges; the game's file stays its own");
        // every other profile runs exactly its own keys
        state.activate("macos-default").unwrap();
        assert_eq!(spy.ran().bindings, state.profile("macos-default").unwrap().bindings);
    }

    #[test]
    fn games_are_added_listed_toggled_and_removed() {
        let (state, spy) = setup("cmd-games");
        let p = state.add_game(portal(), None).unwrap();
        assert!(p.bindings.is_empty(), "starts with no keys of its own; the Default profile covers the rest");
        assert_eq!(state.games(), vec![p.clone()]);
        assert_eq!(state.profiles.lock().unwrap().len(), 3);

        state.set_game_auto_switch(Some("portal-2"), false).unwrap();
        assert!(!state.profile("portal-2").unwrap().game.unwrap().auto_switch);
        state.set_game_auto_switch(None, false).unwrap();
        assert!(!state.settings.lock().unwrap().game_auto_switch);
        assert!(!state.store.lock().unwrap().load_settings().game_auto_switch);
        assert!(state.set_game_auto_switch(Some("macos-default"), true).is_err(), "not a game");

        state.activate("portal-2").unwrap();
        assert!(state.remove_game("macos-default").is_err(), "only game profiles go this way");
        assert_eq!(state.remove_game("portal-2").unwrap().as_deref(), Some(default_profile_id()));
        assert!(state.games().is_empty());
        assert_eq!(*spy.last.lock().unwrap(), default_profile_id());
    }

    #[test]
    fn status_serializes_camel_case() {
        let s = Status {
            active_profile: "windows-default".into(),
            hook_running: true,
            device_found: true,
            version: "1.0.0".into(),
            platform: "windows",
            permissions_ok: true,
        };
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"activeProfile":"windows-default","hookRunning":true,"deviceFound":true,"version":"1.0.0","platform":"windows","permissionsOk":true}"#
        );
    }

    #[test]
    fn every_layout_code_is_in_keymap() {
        let layout = get_layout().unwrap();
        let keys = layout["keys"].as_array().unwrap();
        assert_eq!(keys.len(), 107);
        let missing: Vec<&str> = keys
            .iter()
            .map(|k| k["code"].as_str().unwrap())
            .filter(|c| keymap::def(c).is_none())
            .collect();
        assert!(missing.is_empty(), "codes missing from keymap::KEYS: {missing:?}");
    }
}
