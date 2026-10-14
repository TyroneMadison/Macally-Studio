// Steam library: libraryfolders.vdf -> every library -> appmanifest_*.acf -> installed games.

use std::path::{Path, PathBuf};

use keyvalues_parser::{Obj, Value};

use super::AppEntry;

/// Steamworks Common Redistributables: installed everywhere, not a game.
const REDIST_APPID: u32 = 228_980;

/// Library roots from `steamapps/libraryfolders.vdf`: current `"N" { "path" "..." }` objects and
/// pre-2021 `"N" "D:\\SteamLibrary"` strings. Non-numeric keys ("ContentStatsID") are skipped.
pub fn parse_library_folders(text: &str) -> Vec<PathBuf> {
    let Ok(vdf) = keyvalues_parser::parse(text) else {
        return Vec::new();
    };
    let Some(obj) = vdf.value.get_obj() else {
        return Vec::new();
    };
    obj.iter()
        .filter(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_digit()))
        .flat_map(|(_, vals)| vals.iter())
        .filter_map(|v| match v {
            Value::Str(s) => Some(PathBuf::from(s.as_ref())),
            Value::Obj(o) => field(o, "path").map(PathBuf::from),
        })
        .collect()
}

/// `(appid, name)` from `appmanifest_<id>.acf` when StateFlags has the FullyInstalled bit (4);
/// 6 = installed + update queued, which still launches.
pub fn parse_manifest(text: &str) -> Option<(u32, String)> {
    let vdf = keyvalues_parser::parse(text).ok()?;
    let obj = vdf.value.get_obj()?;
    let appid: u32 = field(obj, "appid")?.parse().ok()?;
    let flags: u64 = field(obj, "StateFlags")?.parse().ok()?;
    if flags & 4 == 0 || appid == REDIST_APPID {
        return None;
    }
    let name = field(obj, "name").map_or_else(|| format!("App {appid}"), str::to_string);
    Some((appid, name))
}

/// Case-insensitive string field (old manifests spell it "appID").
fn field<'a>(obj: &'a Obj<'_>, key: &str) -> Option<&'a str> {
    obj.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))?
        .1
        .first()?
        .get_str()
}

/// Best icon in `<steam>/appcache/librarycache`: the current `<appid>/<sha1>.jpg` client icon, then
/// square-ish art, then the hashed subfolders newer apps use, then the pre-2024 flat files.
pub fn icon_source(steam_root: &Path, appid: u32) -> Option<PathBuf> {
    let cache = steam_root.join("appcache").join("librarycache");
    let dir = cache.join(appid.to_string());
    let is_sha1 = |s: &str| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit());
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.sort();
    let client_icon = entries.iter().find(|p| {
        p.extension().is_some_and(|e| e == "jpg")
            && p.file_stem().and_then(|s| s.to_str()).is_some_and(is_sha1)
    });
    if let Some(p) = client_icon {
        return Some(p.clone());
    }
    let nested = entries.iter().filter(|p| p.is_dir()).flat_map(|sub| {
        [
            sub.join("library_capsule.jpg"),
            sub.join("library_header.jpg"),
        ]
    });
    [dir.join("library_600x900.jpg"), dir.join("header.jpg")]
        .into_iter()
        .chain(nested)
        .chain([
            cache.join(format!("{appid}_icon.jpg")),
            cache.join(format!("{appid}_library_600x900.jpg")),
            cache.join(format!("{appid}_header.jpg")),
        ])
        .find(|p| p.is_file())
}

#[cfg(target_os = "windows")]
fn steam_root() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let user: Option<String> = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .and_then(|k| k.get_value("SteamPath"))
        .ok();
    let path = user.or_else(|| {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(r"SOFTWARE\Wow6432Node\Valve\Steam")
            .and_then(|k| k.get_value("InstallPath"))
            .ok()
    })?;
    // SteamPath is stored with forward slashes: c:/program files (x86)/steam
    Some(PathBuf::from(path.replace('/', r"\")))
}

#[cfg(not(target_os = "windows"))]
fn steam_root() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join("Library/Application Support/Steam"))
}

/// Copy the icon into the icon cache as `<hash>.jpg` (kept as jpg: the webview shows any image type).
fn cache_icon(src: &Path, icons_dir: &Path, launch: &str) -> Option<String> {
    let ext = src.extension()?.to_str()?;
    let dest = icons_dir.join(super::icon_file_name(launch, ext));
    if !dest.is_file() {
        std::fs::copy(src, &dest).ok()?;
    }
    Some(dest.to_string_lossy().into_owned())
}

pub fn scan(icons_dir: &Path) -> anyhow::Result<Vec<AppEntry>> {
    let Some(root) = steam_root().filter(|r| r.join("steamapps").is_dir()) else {
        return Ok(Vec::new()); // Steam not installed
    };
    let folders = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf"))
        .unwrap_or_default();
    // The pre-2021 vdf lists only the extra libraries, so the root is always scanned.
    let mut libraries = parse_library_folders(&folders);
    let same = |l: &PathBuf| l.to_string_lossy().eq_ignore_ascii_case(&root.to_string_lossy());
    if !libraries.iter().any(same) {
        libraries.insert(0, root.clone());
    }
    let mut out = Vec::new();
    for lib in libraries {
        let Ok(rd) = std::fs::read_dir(lib.join("steamapps")) else {
            continue;
        };
        for path in rd.flatten().map(|e| e.path()) {
            let is_manifest = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"));
            if !is_manifest {
                continue;
            }
            let Some((appid, name)) = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| parse_manifest(&t))
            else {
                continue;
            };
            let launch = format!("steam://rungameid/{appid}");
            let icon =
                icon_source(&root, appid).and_then(|src| cache_icon(&src, icons_dir, &launch));
            out.push(AppEntry {
                id: format!("steam:{appid}"),
                name,
                kind: "steam".into(),
                launch,
                icon,
                source: "steam".into(),
            });
        }
    }
    out.sort_by_cached_key(|e| e.name.to_lowercase());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARY_FOLDERS: &str = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"contentid"		"3061845179700256361"
		"totalsize"		"0"
		"update_clean_bytes_tally"		"25674273"
		"time_last_update_verified"		"1784349545"
		"apps"
		{
			"228980"		"336736433"
		}
	}
	"1"
	{
		"path"		"G:\\SteamLibrary"
		"label"		""
		"contentid"		"1185843405823734451"
		"totalsize"		"2000381014016"
		"apps"
		{
			"220"		"5770565299"
			"2358720"		"149864365377"
		}
	}
}
"#;

    const LIBRARY_FOLDERS_OLD: &str = r#""LibraryFolders"
{
	"TimeNextStatsReport"		"1612345678"
	"ContentStatsID"		"-4507115197466390000"
	"1"		"D:\\SteamLibrary"
}
"#;

    const MANIFEST: &str = r#""AppState"
{
	"appid"		"APPID"
	"universe"		"1"
	"LauncherPath"		"C:\\Program Files (x86)\\Steam\\steam.exe"
	"name"		"Portal 2"
	"StateFlags"		"FLAGS"
	"installdir"		"Portal 2"
	"LastUpdated"		"1784349553"
	"SizeOnDisk"		"12884901888"
	"buildid"		"24098430"
	"BytesToDownload"		"0"
	"BytesDownloaded"		"0"
	"InstalledDepots"
	{
		"621"
		{
			"manifest"		"8124929965194586177"
			"size"		"19265607"
		}
	}
}
"#;

    #[test]
    fn library_folders_current_format() {
        assert_eq!(
            parse_library_folders(LIBRARY_FOLDERS),
            [
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"G:\SteamLibrary")
            ]
        );
    }

    #[test]
    fn library_folders_old_format_and_garbage() {
        assert_eq!(
            parse_library_folders(LIBRARY_FOLDERS_OLD),
            [PathBuf::from(r"D:\SteamLibrary")]
        );
        assert!(parse_library_folders("not vdf {").is_empty());
    }

    /// The sample manifest with its appid and StateFlags filled in.
    fn manifest(appid: &str, flags: &str) -> String {
        MANIFEST.replace("APPID", appid).replace("FLAGS", flags)
    }

    #[test]
    fn manifest_installed_game() {
        let installed = Some((620, "Portal 2".to_string()));
        assert_eq!(parse_manifest(&manifest("620", "4")), installed);
        // 6 = installed + update queued: still launches
        assert_eq!(parse_manifest(&manifest("620", "6")), installed);
    }

    #[test]
    fn manifest_skips_not_installed_and_redist() {
        // 1026 = UpdateRequired | UpdateStarted, FullyInstalled bit clear
        assert_eq!(parse_manifest(&manifest("620", "1026")), None);
        assert_eq!(parse_manifest(&manifest("228980", "4")), None);
        assert_eq!(parse_manifest("garbage"), None);
    }

    #[test]
    fn icon_source_handles_both_cache_layouts() {
        let root = std::env::temp_dir().join(format!("macally-steam-test-{}", std::process::id()));
        let cache = root.join("appcache").join("librarycache");
        let current = cache.join("620");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(current.join("header.jpg"), b"h").unwrap();
        std::fs::write(
            current.join("0123456789abcdef0123456789abcdef01234567.jpg"),
            b"i",
        )
        .unwrap();
        let nested = cache
            .join("730")
            .join("fedcba9876543210fedcba9876543210fedcba98");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("library_capsule.jpg"), b"c").unwrap();
        std::fs::write(cache.join("440_icon.jpg"), b"l").unwrap();

        let client_icon = icon_source(&root, 620).unwrap();
        assert_eq!(
            client_icon.file_name().unwrap(),
            "0123456789abcdef0123456789abcdef01234567.jpg"
        );
        assert_eq!(
            icon_source(&root, 730),
            Some(nested.join("library_capsule.jpg"))
        );
        assert_eq!(icon_source(&root, 440), Some(cache.join("440_icon.jpg")));
        assert_eq!(icon_source(&root, 999), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
