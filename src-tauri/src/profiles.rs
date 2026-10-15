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

/// Built-in defaults earlier versions shipped. A built-in on disk whose bindings still equal one
/// of these was never edited, so it moves to the current default; an edited one is left alone.
/// 1.0 and 1.1 mapped Command (the Win key) to Ctrl, which left the keyboard with no Win key.
const RETIRED_DEFAULTS: [&str; 1] = [include_str!("../profiles/retired/windows-default-1.0.json")];

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
    /// Set on a game's own profile; only `add_game` and `set_game_auto_switch` change it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<Game>,
}

/// The game a game profile belongs to: the profile is active while the game is in front.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// The game's own name, for the switch notice.
    pub name: String,
    /// Exe file name on Windows ("portal2.exe"), bundle id on macOS. Empty when `folder` names the game.
    #[serde(default)]
    pub exe: String,
    /// Every program inside this folder counts as the game (a Steam game's install folder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The exe's full path when it is known, to show where the game lives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Cached icon file in `<data_dir>/icons`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Switch to this profile while the game is in front; `Settings::game_auto_switch` stops every game.
    #[serde(default = "yes")]
    pub auto_switch: bool,
}

fn yes() -> bool {
    true
}

impl Game {
    /// The same game: the same exe or the same folder, in any case.
    fn same(&self, other: &Game) -> bool {
        let eq = |a: &str, b: &str| !a.is_empty() && a.to_lowercase() == b.to_lowercase();
        eq(&self.exe, &other.exe) || eq(self.folder.as_deref().unwrap_or(""), other.folder.as_deref().unwrap_or(""))
    }
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
    pub next_profile: String,
    pub prev_profile: String,
    pub show: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Hotkeys {
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
    /// Games switch to their own profiles while in front.
    pub game_auto_switch: bool,
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
            game_auto_switch: true,
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

fn retired_defaults() -> impl Iterator<Item = Profile> {
    RETIRED_DEFAULTS.iter().map(|text| serde_json::from_str(text).expect("retired built-in profiles parse"))
}

/// The shipped built-ins, flagged `builtin`.
pub(crate) fn builtins() -> impl Iterator<Item = Profile> {
    BUILTINS.iter().map(|text| {
        let p: Profile = serde_json::from_str(text).expect("shipped built-in profiles parse");
        Profile { builtin: true, ..p }
    })
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

    /// `builtin` follows the id and `game` stays as it is on disk: a save can neither lock a profile,
    /// lift a built-in's delete guard nor add or drop a game.
    pub fn save_profile(&self, p: &Profile) -> anyhow::Result<()> {
        let game = self.get_profile(&p.id).ok().and_then(|old| old.game);
        self.write(&Profile { builtin: builtins().any(|b| b.id == p.id), game, ..p.clone() })
    }

    /// A new profile for `game`, or the profile it already has. It starts with a copy of `base`'s
    /// bindings, or with none: a game's profile runs on top of the Default profile (AppState::activate).
    pub fn add_game(&self, game: Game, base: Option<&str>) -> anyhow::Result<Profile> {
        if let Some(p) = self.list_profiles()?.into_iter().find(|p| p.game.as_ref().is_some_and(|g| g.same(&game))) {
            return Ok(p);
        }
        let p = Profile {
            schema: SCHEMA,
            id: self.free_id(&slug(&game.name)),
            name: game.name.clone(),
            builtin: false,
            hotkey: None,
            auto_apps: Vec::new(),
            bindings: match base {
                Some(id) => self.get_profile(id)?.bindings,
                None => BTreeMap::new(),
            },
            game: Some(game),
        };
        self.write(&p)?;
        Ok(p)
    }

    pub fn set_game_auto_switch(&self, id: &str, on: bool) -> anyhow::Result<Profile> {
        let mut p = self.get_profile(id)?;
        p.game.as_mut().with_context(|| format!("{id:?} is not a game profile"))?.auto_switch = on;
        self.write(&p)?;
        Ok(p)
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
        p.game = None; // or a game
        self.write(&p)?;
        Ok(p)
    }

    /// Copies a profile file in. The id is sanitized and renumbered on collision.
    pub fn import_profile(&self, path: &Path) -> anyhow::Result<Profile> {
        let mut p = read_profile(path)?;
        p.builtin = false;
        p.id = self.free_id(&slug(&p.id));
        if p.hotkey.is_some() && self.list_profiles()?.iter().any(|o| o.hotkey == p.hotkey) {
            p.hotkey = None; // two profiles cannot share a hotkey
        }
        self.write(&p)?;
        Ok(p)
    }

    pub fn export_profile(&self, id: &str, path: &Path) -> anyhow::Result<()> {
        fs::copy(self.path(id)?, path).with_context(|| format!("exporting {id} to {}", path.display()))?;
        Ok(())
    }

    /// Removes bindings on firmware-only keys (Fn) from every profile. Such a key never reaches the
    /// OS, so a binding on it does nothing; this clears ones saved before the UI stopped offering
    /// them. Returns how many profiles changed, and is idempotent. Run once on load.
    pub fn strip_firmware_only(&self) -> anyhow::Result<usize> {
        let mut changed = 0;
        for mut p in self.list_profiles()? {
            let before = p.bindings.len();
            p.bindings.retain(|code, _| !crate::keymap::is_firmware_only(code));
            if p.bindings.len() != before {
                self.write(&p)?; // p came from disk, so builtin/game are already right
                changed += 1;
            }
        }
        Ok(changed)
    }

    /// Writes a shipped built-in only when its file is missing or unreadable, so edits survive updates.
    pub fn ensure_builtins(&self) -> anyhow::Result<()> {
        for p in builtins() {
            match self.get_profile(&p.id) {
                Err(_) => self.write(&p)?,
                // an untouched copy of an older default: take the new bindings, keep the rest
                Ok(mut have) if retired_defaults().any(|old| old.id == have.id && old.bindings == have.bindings) => {
                    have.bindings = p.bindings;
                    self.write(&have)?;
                }
                Ok(_) => {}
            }
        }
        Ok(())
    }

    /// Puts a built-in back the way it shipped; refuses every other profile.
    pub fn reset_profile(&self, id: &str) -> anyhow::Result<Profile> {
        let p = builtins().find(|p| p.id == id).with_context(|| format!("{id:?} is not a built-in profile"))?;
        self.write(&p)?;
        Ok(p)
    }

    /// A file we cannot read back (typo, newer schema) moves to settings.json.bak
    /// so saving the defaults does not destroy it.
    pub fn load_settings(&self) -> Settings {
        let path = self.data_dir.join("settings.json");
        let Ok(text) = fs::read_to_string(&path) else { return Settings::default() };
        match serde_json::from_str::<Settings>(&text) {
            Ok(s) if s.schema <= SCHEMA => s,
            _ => {
                let _ = fs::rename(&path, path.with_extension("json.bak"));
                Settings::default()
            }
        }
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
