//! Tauri commands invoked from the webview (lib/ipc.ts).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::catalog::{self, AppEntry};
use crate::profiles::{Profile, Settings, Store, default_profile_id};

/// The 107-key layout, compiled into the binary (no resource file to ship).
pub const LAYOUT_JSON: &str = include_str!("../../src/lib/layout.json");

#[tauri::command]
pub fn get_layout() -> Result<serde_json::Value, String> {
    serde_json::from_str(LAYOUT_JSON).map_err(|e| e.to_string())
}

/// What profile/settings commands need from the remap engine; the engine implements it.
pub trait EngineLike: Send + Sync {
    fn set_profile(&self, p: &Profile);
    fn set_paused(&self, b: bool);
    fn is_paused(&self) -> bool;
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

    /// Engine table + settings.json. Every switch goes through here: the UI, tray, hotkeys,
    /// the switch_profile action and Auto mode (which remembers the last manual pick itself).
    pub fn activate(&self, id: &str) -> anyhow::Result<()> {
        let p = self.profile(id)?;
        self.engine.set_profile(&p);
        let settings = {
            let mut s = self.settings.lock().unwrap();
            s.active_profile = p.id;
            s.clone()
        };
        self.store.lock().unwrap().save_settings(&settings)
    }

    pub fn save(&self, p: Profile) -> anyhow::Result<()> {
        self.store.lock().unwrap().save_profile(&p)?;
        self.reload()?;
        if p.id == self.active_id() {
            self.engine.set_profile(&p);
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
    let login_changed = login(&state.settings.lock().unwrap()) != login(&settings);
    state.save_settings(settings).map_err(err)?;
    crate::control::resync(&app);
    if login_changed {
        crate::control::sync_autostart(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn set_paused(app: AppHandle, state: State<'_, AppState>, paused: bool) {
    state.engine.set_paused(paused);
    let _ = app.emit("paused-changed", json!({ "paused": paused }));
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub paused: bool,
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
        paused: state.engine.is_paused(),
        active_profile: state.active_id(),
        hook_running: crate::engine::hook_running(),
        device_found: crate::engine::device_found(),
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
        permissions_ok: crate::engine::permissions_ok(),
    }
}

type EngineState<'a> = State<'a, Arc<crate::engine::Engine>>;

/// Every key goes to the UI as "key-event" and is swallowed until stop_record (or 15 s pass).
#[tauri::command]
pub fn start_record(engine: EngineState<'_>) {
    engine.set_mode(crate::engine::Mode::Record);
}

#[tauri::command]
pub fn stop_record(engine: EngineState<'_>) {
    engine.set_mode(crate::engine::Mode::Normal);
}

/// Key Monitor: every key goes to the UI as "key-event"; remapping keeps working.
#[tauri::command]
pub fn set_monitor(engine: EngineState<'_>, on: bool) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap;
    use crate::profiles::temp_store;

    /// Records which profile the engine was last given.
    #[derive(Default)]
    struct Spy {
        last: Mutex<String>,
    }

    impl EngineLike for Spy {
        fn set_profile(&self, p: &Profile) {
            *self.last.lock().unwrap() = p.id.clone();
        }
        fn set_paused(&self, _b: bool) {}
        fn is_paused(&self) -> bool {
            false
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

    #[test]
    fn status_serializes_camel_case() {
        let s = Status {
            paused: false,
            active_profile: "windows-default".into(),
            hook_running: true,
            device_found: true,
            version: "1.0.0".into(),
            platform: "windows",
            permissions_ok: true,
        };
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"paused":false,"activeProfile":"windows-default","hookRunning":true,"deviceFound":true,"version":"1.0.0","platform":"windows","permissionsOk":true}"#
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
