// App catalog: Start-menu apps and installed programs (Windows) or app bundles (macOS), plus the
// Steam library.
// The id prefix carries the picker section: "builtin:" / "app:" / "steam:".

pub mod macos;
pub mod programs;
pub mod steam;
pub mod windows;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::profiles::{Game, Store};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    /// aumid | path | bundle | steam
    pub kind: String,
    pub launch: String,
    /// Absolute path of a cached image (png or jpg) in `<data_dir>/icons`.
    pub icon: Option<String>,
    /// windows | macos | steam
    pub source: String,
}

/// `<hash>.<ext>` for a launch id. FNV-1a 64: std's DefaultHasher may change between toolchains,
/// which would orphan every cached icon.
pub fn icon_file_name(launch: &str, ext: &str) -> String {
    let h = launch.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{h:016x}.{ext}")
}

/// The game an entry starts, for its game profile: the exe it runs (bundle id on macOS), or for a
/// Steam game its install folder. Store apps and links name no exe: those are added while they run.
pub fn game_for(entry: &AppEntry) -> anyhow::Result<Game> {
    let (exe, folder, path) = match entry.kind.as_str() {
        "bundle" => (entry.launch.clone(), None, None),
        "steam" => {
            let appid = entry.launch.strip_prefix("steam://rungameid/").and_then(|id| id.parse().ok());
            (String::new(), appid.and_then(steam::install_dir).map(|d| d.to_string_lossy().into_owned()), None)
        }
        _ => match program_exe(&entry.launch) {
            Some(file) => {
                let name = file.rsplit(['\\', '/']).next().unwrap_or(&file).to_string();
                // a Start-menu id names its exe inside a known folder ({GUID}\...), not a full path
                let path = (!file.starts_with('{')).then_some(file);
                (name, None, path)
            }
            None => (String::new(), None, None),
        },
    };
    anyhow::ensure!(
        !exe.is_empty() || folder.is_some(),
        "cannot tell which program {} runs; start it, then add it from the running apps",
        entry.name
    );
    let icon = entry.icon.clone().filter(|p| Path::new(p).is_file());
    Ok(Game { name: entry.name.clone(), exe, folder, path, icon, auto_switch: true })
}

/// The exe a Windows launch string runs: an .exe, a shortcut's target, or a Start-menu app id that
/// is a path (`shell:AppsFolder\{known folder}\Tool\tool.exe`).
fn program_exe(launch: &str) -> Option<String> {
    let file = launch.strip_prefix(r"shell:AppsFolder\").unwrap_or(launch);
    let file = if file.to_ascii_lowercase().ends_with(".lnk") { shortcut_target(file)? } else { file.to_string() };
    file.to_ascii_lowercase().ends_with(".exe").then_some(file)
}

#[cfg(target_os = "windows")]
fn shortcut_target(lnk: &str) -> Option<String> {
    programs::shortcut_target(Path::new(lnk))
}

#[cfg(not(target_os = "windows"))]
fn shortcut_target(_lnk: &str) -> Option<String> {
    None
}

/// Built-ins first (in `pinned` order, then by name), then everything else by name.
pub fn sort_entries(v: &mut [AppEntry], pinned: &[&str]) {
    v.sort_by_cached_key(|e| {
        let pin = pinned
            .iter()
            .position(|p| e.launch.ends_with(p))
            .unwrap_or(usize::MAX);
        (!e.id.starts_with("builtin:"), pin, e.name.to_lowercase())
    });
}

/// Flatten per-source lists, keeping the first entry for each launch string.
pub fn merge(lists: Vec<Vec<AppEntry>>) -> Vec<AppEntry> {
    let mut seen = HashSet::new();
    lists
        .into_iter()
        .flatten()
        .filter(|e| seen.insert(e.launch.clone()))
        .collect()
}

// Cache + refresh: catalog-cache.json in the data dir, rebuilt by scanning every source.

#[derive(Serialize, Clone)]
struct Progress {
    done: usize,
    total: usize,
    source: &'static str,
}

fn cache_path(store: &Store) -> PathBuf {
    store.data_dir.join("catalog-cache.json")
}

/// Last scan result; empty when there is no cache yet or it cannot be read.
pub fn load_cached(store: &Store) -> Vec<AppEntry> {
    std::fs::read(cache_path(store))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_cache(store: &Store, entries: &[AppEntry]) -> anyhow::Result<()> {
    let tmp = cache_path(store).with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(entries)?)?;
    std::fs::rename(tmp, cache_path(store))?;
    Ok(())
}

type Scan = fn(&Path) -> anyhow::Result<Vec<AppEntry>>;

/// One scan at a time: the launch-time background refresh and the Refresh button write the same icon files.
static SCANNING: Mutex<()> = Mutex::new(());

/// Rescan every source, rewrite the cache, emit "catalog-progress" { done, total, source } after each source.
/// The last event (done == total) goes out only once the cache is written: the UI refetches on it.
pub fn refresh(store: &Store, app: &AppHandle) -> anyhow::Result<Vec<AppEntry>> {
    let _scanning = SCANNING.lock().unwrap_or_else(|e| e.into_inner());
    let icons_dir = store.data_dir.join("icons");
    std::fs::create_dir_all(&icons_dir)?;
    let os: (&'static str, Scan) = if cfg!(target_os = "macos") {
        ("macos", macos::scan)
    } else {
        ("windows", windows::scan)
    };
    let sources = [os, ("steam", steam::scan)];
    let mut lists = Vec::new();
    for (i, (source, scan)) in sources.iter().enumerate() {
        // One broken source (no Steam, PowerShell blocked by policy) must not empty the whole catalog.
        lists.push(scan(&icons_dir).unwrap_or_else(|e| {
            eprintln!("catalog: {source} scan failed: {e:#}");
            Vec::new()
        }));
        if i + 1 < sources.len() {
            let _ = app.emit("catalog-progress", Progress { done: i + 1, total: sources.len(), source });
        }
    }
    // programs picked with Browse are not found by any scan; keep them while they exist
    let added = load_cached(store).into_iter().filter(|e| e.kind == "path" && Path::new(&e.launch).exists());
    lists.push(added.collect());
    let all = merge(lists);
    save_cache(store, &all)?;
    let (total, source) = (sources.len(), sources[sources.len() - 1].0);
    let _ = app.emit("catalog-progress", Progress { done: total, total, source });
    Ok(all)
}

/// Browse for .exe/.app: adds a `path` entry to the cached catalog (no-op when it is already there).
pub fn add_path(store: &Store, path: &str) -> anyhow::Result<Vec<AppEntry>> {
    let _scanning = SCANNING.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = load_cached(store);
    if !all.iter().any(|e| e.launch == path) {
        let name = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path);
        all.push(AppEntry {
            id: format!("app:{path}"),
            name: name.into(),
            kind: "path".into(),
            launch: path.into(),
            icon: None,
            source: if cfg!(target_os = "macos") { "macos" } else { "windows" }.into(),
        });
        save_cache(store, &all)?;
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, name: &str, launch: &str) -> AppEntry {
        AppEntry {
            id: id.into(),
            name: name.into(),
            kind: "aumid".into(),
            launch: launch.into(),
            icon: None,
            source: "windows".into(),
        }
    }

    #[test]
    fn icon_file_name_is_stable_fnv1a() {
        assert_eq!(icon_file_name("", "png"), "cbf29ce484222325.png");
        assert_eq!(icon_file_name("a", "jpg"), "af63dc4c8601ec8c.jpg");
    }

    #[test]
    fn sort_puts_pinned_builtins_first_then_names() {
        let mut v = vec![
            entry("app:z", "Zed", "z"),
            entry("builtin:b", "Beta", "x!b"),
            entry("app:a", "alpha", "a"),
            entry("builtin:c", "Calc", "x!c"),
        ];
        sort_entries(&mut v, &["x!c"]);
        let names: Vec<_> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Calc", "Beta", "alpha", "Zed"]);
    }

    #[test]
    fn merge_dedupes_by_launch_keeping_first() {
        let v = merge(vec![
            vec![
                entry("builtin:1", "First", "same"),
                entry("app:2", "Other", "other"),
            ],
            vec![entry("steam:3", "Dupe", "same")],
        ]);
        let names: Vec<_> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["First", "Other"]);
    }

    #[test]
    fn add_path_appends_one_named_entry() {
        let store = crate::profiles::temp_store("catalog-add");
        let exe = if cfg!(windows) { r"C:\Games\Tool.exe" } else { "/Applications/Tool.app" };
        let v = add_path(&store, exe).unwrap();
        assert_eq!(v[0].id, format!("app:{exe}"));
        assert_eq!((v[0].name.as_str(), v[0].kind.as_str()), ("Tool", "path"));
        assert_eq!(add_path(&store, exe).unwrap().len(), 1);
        assert_eq!(load_cached(&store), v);
        std::fs::remove_dir_all(&store.data_dir).unwrap();
    }

    fn program(kind: &str, launch: &str) -> AppEntry {
        AppEntry { kind: kind.into(), ..entry("app:x", "Tool", launch) }
    }

    #[test]
    fn a_game_from_a_program_is_its_exe() {
        let g = game_for(&program("path", r"D:\Games\Tool\Tool.EXE")).unwrap();
        assert_eq!((g.name.as_str(), g.exe.as_str(), g.folder.as_deref(), g.auto_switch), ("Tool", "Tool.EXE", None, true));
        assert_eq!(g.path.as_deref(), Some(r"D:\Games\Tool\Tool.EXE"), "where it lives, to show");
        let start_menu = program("aumid", r"shell:AppsFolder\{7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E}\Tool\tool.exe");
        let g = game_for(&start_menu).unwrap();
        assert_eq!((g.exe.as_str(), g.path.as_deref()), ("tool.exe", None), "a known-folder id is no path");
        let mac = game_for(&program("bundle", "com.valvesoftware.portal2")).unwrap();
        assert_eq!((mac.exe.as_str(), mac.path.as_deref()), ("com.valvesoftware.portal2", None));
    }

    #[test]
    fn a_program_that_names_no_exe_is_added_while_it_runs_instead() {
        for e in [
            program("aumid", r"shell:AppsFolder\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App"),
            program("path", r"D:\Docs\readme.txt"),
            program("steam", "steam://rungameid/4294967295"),
            program("steam", "steam://rungameid/x"),
        ] {
            let err = game_for(&e).unwrap_err().to_string();
            assert!(err.contains("running apps"), "{}: {err}", e.launch);
        }
    }

    #[test]
    fn a_game_keeps_its_icon_only_when_the_file_exists() {
        let store = crate::profiles::temp_store("catalog-game-icon");
        let icon = store.data_dir.join("tool.png");
        std::fs::write(&icon, b"png").unwrap();
        let with = |icon: &Path| AppEntry { icon: Some(icon.to_string_lossy().into_owned()), ..program("path", r"C:\Tool\tool.exe") };
        assert_eq!(game_for(&with(&icon)).unwrap().icon, Some(icon.to_string_lossy().into_owned()));
        assert_eq!(game_for(&with(&store.data_dir.join("gone.png"))).unwrap().icon, None);
        std::fs::remove_dir_all(&store.data_dir).unwrap();
    }

    #[test]
    fn cache_round_trips_and_missing_cache_is_empty() {
        let store = crate::profiles::temp_store("catalog-cache");
        assert!(load_cached(&store).is_empty());
        let v = vec![AppEntry {
            id: "steam:620".into(),
            name: "Portal 2".into(),
            kind: "steam".into(),
            launch: "steam://rungameid/620".into(),
            icon: None,
            source: "steam".into(),
        }];
        save_cache(&store, &v).unwrap();
        assert_eq!(load_cached(&store), v);
        std::fs::write(cache_path(&store), b"{ not json").unwrap();
        assert!(load_cached(&store).is_empty());
        std::fs::remove_dir_all(&store.data_dir).unwrap();
    }
}
