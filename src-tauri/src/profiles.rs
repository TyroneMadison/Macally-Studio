//! Profiles and settings: types, defaults and the on-disk store.
//! Layout: <config dir>/Macally Studio/{settings.json, profiles/<id>.json}

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};

use crate::actions::Action;

pub const SCHEMA: u32 = 1;

/// Shipped built-in profiles, compiled in (no resource file to ship).
const BUILTINS: [&str; 2] = [
    include_str!("../profiles/windows-default.json"),
    include_str!("../profiles/macos-default.json"),
];

/// Serialized as "Ctrl" | "Alt" | "Shift" | "Meta".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    #[serde(default)]
    pub mods: Vec<Modifier>,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub schema: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub auto_apps: Vec<String>,
    /// Key code (W3C `KeyboardEvent.code`) -> binding.
    #[serde(default)]
    pub bindings: BTreeMap<String, Binding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwitchMode {
    Manual,
    Hotkey,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Hotkeys {
    pub pause: String,
    pub next_profile: String,
    pub prev_profile: String,
    pub show: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Hotkeys {
            pause: "Ctrl+Alt+P".into(),
            next_profile: "Ctrl+Alt+]".into(),
            prev_profile: "Ctrl+Alt+[".into(),
            show: "Ctrl+Alt+M".into(),
        }
    }
}

/// Missing fields in settings.json fall back to `Settings::default()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema: u32,
    pub active_profile: String,
    pub switch_mode: SwitchMode,
    pub hotkeys: Hotkeys,
    pub start_at_login: bool,
    pub close_to_tray: bool,
    pub run_as_admin: bool,
    pub windows_legends: bool,
    pub auto_check_updates: bool,
}

/// The built-in profile for the OS we run on.
pub fn default_profile_id() -> &'static str {
    if cfg!(target_os = "macos") { "macos-default" } else { "windows-default" }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema: SCHEMA,
            active_profile: default_profile_id().into(),
            switch_mode: SwitchMode::Auto,
            hotkeys: Hotkeys::default(),
            start_at_login: true,
            close_to_tray: true,
            run_as_admin: false,
            windows_legends: true,
            auto_check_updates: true,
        }
    }
}

/// Lowercase ASCII letters and digits joined by single dashes; never empty.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() { "profile".into() } else { out.into() }
}

fn read_profile(path: &Path) -> anyhow::Result<Profile> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let p: Profile = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    ensure!(p.schema <= SCHEMA, "{} needs a newer Macally Studio (schema {})", path.display(), p.schema);
    Ok(p)
}

pub struct Store {
    pub data_dir: PathBuf,
}

impl Store {
    /// %APPDATA%\Macally Studio on Windows, ~/Library/Application Support/Macally Studio on macOS.
    pub fn open() -> anyhow::Result<Store> {
        let base = dirs::config_dir().context("this system has no config directory")?;
        Store::at(base.join("Macally Studio"))
    }

    pub fn at(data_dir: PathBuf) -> anyhow::Result<Store> {
        fs::create_dir_all(data_dir.join("profiles"))?;
        Ok(Store { data_dir })
    }

    fn profiles_dir(&self) -> PathBuf {
        self.data_dir.join("profiles")
    }

    /// The only way ids become paths, so "../settings" can never escape profiles/.
    fn path(&self, id: &str) -> anyhow::Result<PathBuf> {
        ensure!(slug(id) == id, "invalid profile id {id:?}");
        Ok(self.profiles_dir().join(format!("{id}.json")))
    }

    fn write(&self, p: &Profile) -> anyhow::Result<()> {
        fs::write(self.path(&p.id)?, serde_json::to_string_pretty(p)?)?;
        Ok(())
    }

    /// `base`, else `base-2`, `base-3`, ...
    fn free_id(&self, base: &str) -> String {
        let free = |id: &String| !self.profiles_dir().join(format!("{id}.json")).exists();
        std::iter::once(base.to_string())
            .chain((2..).map(|n| format!("{base}-{n}")))
            .find(free)
            .expect("an unbounded search always finds a free id")
    }

    /// Built-ins first, then by name.
    pub fn list_profiles(&self) -> anyhow::Result<Vec<Profile>> {
        let mut out: Vec<Profile> = fs::read_dir(self.profiles_dir())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            // a hand-broken file must not hide every other profile
            .filter_map(|p| read_profile(&p).ok())
            .collect();
        out.sort_by_key(|p| (!p.builtin, p.name.to_lowercase()));
        Ok(out)
    }

    pub fn get_profile(&self, id: &str) -> anyhow::Result<Profile> {
        read_profile(&self.path(id)?)
    }

    pub fn save_profile(&self, p: &Profile) -> anyhow::Result<()> {
        let read_only = p.builtin || self.get_profile(&p.id).is_ok_and(|old| old.builtin);
        ensure!(!read_only, "built-in profiles are read-only; duplicate it to edit");
        self.write(p)
    }

    pub fn delete_profile(&self, id: &str) -> anyhow::Result<()> {
        ensure!(!self.get_profile(id)?.builtin, "built-in profiles cannot be deleted");
        fs::remove_file(self.path(id)?)?;
        Ok(())
    }

    pub fn duplicate_profile(&self, id: &str) -> anyhow::Result<Profile> {
        let mut p = self.get_profile(id)?;
        p.id = self.free_id(&format!("{id}-copy"));
        p.name = format!("{} copy", p.name);
        p.builtin = false;
        p.hotkey = None; // two profiles cannot share a hotkey
        self.write(&p)?;
        Ok(p)
    }

    /// Copies a profile file in. The id is sanitized and renumbered on collision.
    pub fn import_profile(&self, path: &Path) -> anyhow::Result<Profile> {
        let mut p = read_profile(path)?;
        p.builtin = false;
        p.id = self.free_id(&slug(&p.id));
        self.write(&p)?;
        Ok(p)
    }

    pub fn export_profile(&self, id: &str, path: &Path) -> anyhow::Result<()> {
        fs::copy(self.path(id)?, path).with_context(|| format!("exporting {id} to {}", path.display()))?;
        Ok(())
    }

    /// Writes the shipped built-ins on every launch so app updates refresh them.
    /// They are read-only, so nothing a user made is overwritten.
    pub fn ensure_builtins(&self) -> anyhow::Result<()> {
        for text in BUILTINS {
            let mut p: Profile = serde_json::from_str(text)?;
            p.builtin = true;
            self.write(&p)?;
        }
        Ok(())
    }

    pub fn load_settings(&self) -> Settings {
        fs::read_to_string(self.data_dir.join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, s: &Settings) -> anyhow::Result<()> {
        fs::write(self.data_dir.join("settings.json"), serde_json::to_string_pretty(s)?)?;
        Ok(())
    }
}

/// A fresh store in the OS temp dir; `tag` keeps parallel tests apart.
#[cfg(test)]
pub(crate) fn temp_store(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("macally-studio-test-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    Store::at(dir).unwrap()
}

#[cfg(test)]
mod tests;
