// App catalog: Start-menu apps (Windows) or app bundles (macOS) plus the Steam library.
// The id prefix carries the picker section: "builtin:" / "app:" / "steam:".

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

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
}
