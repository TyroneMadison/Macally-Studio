// App catalog: Start-menu apps (Windows) or app bundles (macOS) plus the Steam library.
// The id prefix carries the picker section: "builtin:" / "app:" / "steam:".

pub mod macos;
pub mod steam;
pub mod windows;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::profiles::Store;

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
        let progress = Progress { done: i + 1, total: sources.len(), source };
        let _ = app.emit("catalog-progress", progress);
    }
    let all = merge(lists);
    save_cache(store, &all)?;
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
