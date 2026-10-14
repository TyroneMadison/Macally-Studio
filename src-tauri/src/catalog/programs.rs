// Installed programs the Start-menu app list can miss: Start-menu shortcuts (.lnk) of the user
// and of all users, the App Paths registry and the Uninstall registry. Windows only; the parsing
// below is pure and tested on every OS.

use std::collections::HashSet;

use super::AppEntry;

/// A program found on disk: its catalog entry and the file it really starts (the shortcut's
/// target, or the exe itself), used to drop the same program found by another source.
pub struct Found {
    pub entry: AppEntry,
    pub target: String,
}

/// `%ProgramFiles%\x` -> `C:\Program Files\x`. Unknown names stay as written.
pub fn expand_env(s: &str, env: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match env(name) {
                    Some(v) => out.push_str(&v),
                    None => out.push_str(&format!("%{name}%")),
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Registry path or icon location -> the exe it names: `"C:\x\app.exe",0`, `C:\x\app.exe,-101`,
/// `%ProgramFiles%\x\app.exe`. None for anything that is not an .exe (an .ico, a .dll, a folder).
pub fn exe_from_location(s: &str, env: impl Fn(&str) -> Option<String>) -> Option<String> {
    let s = s.trim();
    let path = match s.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next().unwrap_or_default(),
        // an unquoted ",<index>" suffix is the icon index, not part of the path
        None => match s.rsplit_once(',') {
            Some((p, idx)) if idx.trim().parse::<i32>().is_ok() => p,
            _ => s,
        },
    };
    let path = expand_env(path.trim(), env);
    path.to_ascii_lowercase().ends_with(".exe").then_some(path)
}

/// Uninstallers, setup programs, read-me files and other shortcuts nobody wants on a key.
pub fn is_noise(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "uninstall", "unins0", "readme", "read me", "release notes", "documentation", "license",
        "website", "setup", "installer", "help file", "manual",
    ]
    .iter()
    .any(|w| n.contains(w))
}

/// Installer caches hold setup exes that only look like programs.
pub fn is_installer_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase().replace('/', "\\");
    ["\\package cache\\", "\\windows\\installer\\", "\\installshield installation information\\", "\\downloaded installations\\"]
        .iter()
        .any(|w| p.contains(w))
}

/// Exes found through the registry that are parts of something else, not programs to open:
/// services, updaters, crash handlers, driver installers, and the insides of Store apps (those
/// come from Get-StartApps as proper app ids).
pub fn is_helper_exe(path: &str) -> bool {
    let p = path.to_ascii_lowercase().replace('/', "\\");
    let stem = p.rsplit('\\').next().unwrap_or(&p).trim_end_matches(".exe");
    p.contains("\\windowsapps\\")
        || ["service", "update", "helper", "crash", "dpinst", "elevat", "unins", "setup", "install"]
            .iter()
            .any(|w| stem.contains(w))
}

pub fn path_entry(name: &str, launch: &str) -> AppEntry {
    AppEntry {
        id: format!("app:{launch}"),
        name: name.trim().to_string(),
        kind: "path".into(),
        launch: launch.into(),
        icon: None,
        source: "windows".into(),
    }
}

/// Keeps the first entry for each program: a later one is dropped when its name or the file it
/// starts was already seen (any case). Earlier lists win, so pass the best source first.
pub fn dedupe(lists: Vec<Vec<Found>>) -> Vec<AppEntry> {
    let mut names = HashSet::new();
    let mut targets = HashSet::new();
    let mut out = Vec::new();
    for f in lists.into_iter().flatten() {
        let name = f.entry.name.trim().to_lowercase();
        let (target, launch) = (f.target.to_lowercase(), f.entry.launch.to_lowercase());
        if name.is_empty() || names.contains(&name) || targets.contains(&target) || targets.contains(&launch) {
            continue;
        }
        names.insert(name);
        targets.insert(target);
        targets.insert(launch);
        out.push(f.entry);
    }
    out
}

#[cfg(target_os = "windows")]
pub use win::scan;

#[cfg(target_os = "windows")]
mod win {
    use std::path::{Path, PathBuf};

    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface, w};
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    use super::{Found, exe_from_location, is_helper_exe, is_installer_path, is_noise, path_entry};

    const APP_PATHS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
    const APP_PATHS_32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths";
    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    fn env(name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    /// Every source, best first: Start-menu shortcuts, then App Paths, then the Uninstall list.
    pub fn scan() -> Vec<Vec<Found>> {
        vec![start_menu_shortcuts(), app_paths(), uninstall_programs()]
    }

    /// `...\Start Menu\Programs` of the user (Roaming) and of all users (ProgramData).
    fn start_menu_dirs() -> Vec<PathBuf> {
        let tail = Path::new("Microsoft").join("Windows").join("Start Menu").join("Programs");
        let mut dirs = Vec::new();
        if let Some(roaming) = dirs::data_dir() {
            dirs.push(roaming.join(&tail));
        }
        dirs.push(PathBuf::from(env("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into())).join(&tail));
        dirs
    }

    fn lnk_files(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            // Startup holds autostart shortcuts, often with "start hidden" arguments
            let startup = p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("Startup"));
            if p.is_dir() && depth > 0 && !startup {
                lnk_files(&p, depth - 1, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
                out.push(p);
            }
        }
    }

    /// Shortcuts to programs. Ones pointing at documents, folders or web pages are left out; an
    /// advertised (MSI) shortcut has no plain target but still opens, so it stays.
    pub fn start_menu_shortcuts() -> Vec<Found> {
        let mut files = Vec::new();
        for d in start_menu_dirs() {
            lnk_files(&d, 6, &mut files);
        }
        files
            .into_iter()
            .filter_map(|lnk| {
                let name = lnk.file_stem()?.to_str()?.to_string();
                if is_noise(&name) {
                    return None;
                }
                let target = shortcut_target(&lnk).unwrap_or_default();
                let launch = lnk.to_string_lossy().into_owned();
                if target.is_empty() {
                    Some(Found { entry: path_entry(&name, &launch), target: launch })
                } else if target.to_ascii_lowercase().ends_with(".exe")
                    && Path::new(&target).is_file()
                    && !is_noise(&stem(&target))
                {
                    Some(Found { entry: path_entry(&name, &launch), target })
                } else {
                    None
                }
            })
            .collect()
    }

    /// The file a .lnk starts, or None for advertised shortcuts and unreadable files.
    pub fn shortcut_target(lnk: &Path) -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED); // S_FALSE when already initialised
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
            link.cast::<IPersistFile>().ok()?.Load(&HSTRING::from(lnk.as_os_str()), STGM_READ).ok()?;
            let mut buf = [0u16; 1024];
            link.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            (len > 0).then(|| String::from_utf16_lossy(&buf[..len]))
        }
    }

    /// App Paths: `<exe name>` keys whose default value is the program's full path.
    pub fn app_paths() -> Vec<Found> {
        let roots = [(HKEY_CURRENT_USER, APP_PATHS), (HKEY_LOCAL_MACHINE, APP_PATHS), (HKEY_LOCAL_MACHINE, APP_PATHS_32)];
        let mut out = Vec::new();
        for (hive, path) in roots {
            let Ok(root) = RegKey::predef(hive).open_subkey(path) else { continue };
            for key in root.enum_keys().flatten() {
                let Ok(sub) = root.open_subkey(&key) else { continue };
                let Ok(raw) = sub.get_value::<String, _>("") else { continue };
                let Some(exe) = exe_from_location(&raw, env) else { continue };
                if !Path::new(&exe).is_file() || is_installer_path(&exe) || is_helper_exe(&exe) {
                    continue;
                }
                let name = file_description(&exe).unwrap_or_else(|| stem(&exe));
                if !is_noise(&name) && !is_noise(&stem(&exe)) {
                    out.push(Found { entry: path_entry(&name, &exe), target: exe });
                }
            }
        }
        out
    }

    /// Installed programs (Apps & features) whose DisplayIcon is their own exe. Windows parts,
    /// updates and installer-cache entries are left out.
    pub fn uninstall_programs() -> Vec<Found> {
        let roots = [(HKEY_CURRENT_USER, UNINSTALL), (HKEY_LOCAL_MACHINE, UNINSTALL), (HKEY_LOCAL_MACHINE, UNINSTALL_32)];
        let mut out = Vec::new();
        for (hive, path) in roots {
            let Ok(root) = RegKey::predef(hive).open_subkey(path) else { continue };
            for key in root.enum_keys().flatten() {
                let Ok(sub) = root.open_subkey(&key) else { continue };
                let hidden = sub.get_value::<u32, _>("SystemComponent").is_ok_and(|v| v == 1)
                    || sub.get_value::<String, _>("ParentKeyName").is_ok()
                    || sub.get_value::<String, _>("ReleaseType").is_ok();
                let Ok(name) = sub.get_value::<String, _>("DisplayName") else { continue };
                if hidden || name.trim().is_empty() || is_noise(&name) || name.starts_with("Windows Driver Package") {
                    continue;
                }
                let Ok(icon) = sub.get_value::<String, _>("DisplayIcon") else { continue };
                let Some(exe) = exe_from_location(&icon, env) else { continue };
                if Path::new(&exe).is_file() && !is_installer_path(&exe) && !is_helper_exe(&exe) {
                    out.push(Found { entry: path_entry(&name, &exe), target: exe });
                }
            }
        }
        out
    }

    fn stem(path: &str) -> String {
        Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.into())
    }

    /// The exe's own name for itself: FileDescription, else ProductName, from its version info.
    pub fn file_description(exe: &str) -> Option<String> {
        unsafe {
            let file = HSTRING::from(exe);
            let size = GetFileVersionInfoSizeW(&file, None);
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            GetFileVersionInfoW(&file, None, size, data.as_mut_ptr().cast()).ok()?;
            let (mut ptr, mut len) = (std::ptr::null_mut(), 0u32);
            let lang = if VerQueryValueW(data.as_ptr().cast(), w!("\\VarFileInfo\\Translation"), &mut ptr, &mut len).as_bool()
                && len >= 4
            {
                let pair = std::slice::from_raw_parts(ptr as *const u16, 2);
                format!("{:04x}{:04x}", pair[0], pair[1])
            } else {
                "040904b0".to_string()
            };
            for field in ["FileDescription", "ProductName"] {
                let query = HSTRING::from(format!("\\StringFileInfo\\{lang}\\{field}"));
                if VerQueryValueW(data.as_ptr().cast(), &query, &mut ptr, &mut len).as_bool() && len > 1 {
                    let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr as *const u16, len as usize - 1));
                    let text = text.trim_matches(char::from(0)).trim();
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        match name {
            "ProgramFiles" => Some(r"C:\Program Files".into()),
            "LOCALAPPDATA" => Some(r"C:\Users\me\AppData\Local".into()),
            _ => None,
        }
    }

    #[test]
    fn env_variables_expand_and_unknown_ones_stay() {
        assert_eq!(expand_env(r"%ProgramFiles%\Tool\tool.exe", env), r"C:\Program Files\Tool\tool.exe");
        assert_eq!(expand_env(r"%NOPE%\x.exe", env), r"%NOPE%\x.exe");
        assert_eq!(expand_env("100% sure", env), "100% sure");
    }

    #[test]
    fn locations_become_exe_paths() {
        assert_eq!(exe_from_location(r#""C:\Apps\Tool\tool.exe",0"#, env).as_deref(), Some(r"C:\Apps\Tool\tool.exe"));
        assert_eq!(exe_from_location(r"C:\Apps\Tool\tool.exe,-101", env).as_deref(), Some(r"C:\Apps\Tool\tool.exe"));
        assert_eq!(
            exe_from_location(r"%LOCALAPPDATA%\Programs\Editor\Editor.EXE", env).as_deref(),
            Some(r"C:\Users\me\AppData\Local\Programs\Editor\Editor.EXE")
        );
        assert_eq!(exe_from_location(r#""C:\Apps\Tool, Inc\tool.exe""#, env).as_deref(), Some(r"C:\Apps\Tool, Inc\tool.exe"));
        assert_eq!(exe_from_location(r"C:\Apps\Tool\tool.ico", env), None);
        assert_eq!(exe_from_location(r"C:\Windows\system32\shell32.dll,-16769", env), None);
        assert_eq!(exe_from_location("", env), None);
    }

    #[test]
    fn noise_and_installer_caches_are_recognised() {
        for n in ["Uninstall Discord", "unins000", "Read Me", "Tool Setup", "Online Documentation", "Tool Website"] {
            assert!(is_noise(n), "{n}");
        }
        for n in ["Discord", "Visual Studio Code", "Steam", "Blender 5.2"] {
            assert!(!is_noise(n), "{n}");
        }
        assert!(is_installer_path(r"C:\ProgramData\Package Cache\{abc}\setup.exe"));
        assert!(is_installer_path(r"C:\Windows\Installer\{abc}\icon.exe"));
        assert!(!is_installer_path(r"C:\Program Files\Tool\tool.exe"));
        for p in [
            r"C:\Program Files\WindowsApps\Microsoft.WindowsStore_1_x64__8wekyb3d8bbwe\store.exe",
            r"C:\Program Files\Denuvo\denuvo-anti-cheat-update-service.exe",
            r"C:\PROGRA~1\DIFX\B39520~1\DPInst.exe",
            r"C:\Program Files\Tool\CrashHandler.exe",
        ] {
            assert!(is_helper_exe(p), "{p}");
        }
        for p in [r"C:\Program Files (x86)\Overwolf\Overwolf.exe", r"C:\Program Files\CLIP STUDIO\CLIPStudioPaint.exe"] {
            assert!(!is_helper_exe(p), "{p}");
        }
    }

    fn found(name: &str, launch: &str, target: &str) -> Found {
        Found { entry: path_entry(name, launch), target: target.into() }
    }

    #[test]
    fn dedupe_keeps_the_first_by_name_or_target() {
        let start_menu = vec![found("Tool", r"C:\SM\Tool.lnk", r"C:\Apps\tool.exe"), found("Editor", r"C:\SM\Editor.lnk", r"C:\SM\Editor.lnk")];
        let app_paths = vec![
            found("Tool (64-bit)", r"C:\APPS\TOOL.EXE", r"C:\APPS\TOOL.EXE"), // same exe as the shortcut
            found("editor", r"C:\Other\editor.exe", r"C:\Other\editor.exe"), // same name, any case
            found("Player", r"C:\Apps\player.exe", r"C:\Apps\player.exe"),
        ];
        let names: Vec<String> = dedupe(vec![start_menu, app_paths]).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["Tool", "Editor", "Player"]);
    }

    #[test]
    fn path_entries_launch_their_file() {
        let e = path_entry(" Tool ", r"C:\SM\Tool.lnk");
        assert_eq!((e.id.as_str(), e.name.as_str(), e.kind.as_str()), (r"app:C:\SM\Tool.lnk", "Tool", "path"));
    }

    /// Reads this machine's real Start menu and registry: every program found must exist and launch
    /// through a file the shell can open.
    #[cfg(target_os = "windows")]
    #[test]
    fn finds_installed_programs_on_this_machine() {
        let lists = scan();
        let counts: Vec<usize> = lists.iter().map(Vec::len).collect();
        let shortcuts = counts[0];
        let all = dedupe(lists);
        eprintln!("shortcuts {}, app paths {}, uninstall {}, after dedupe {}", counts[0], counts[1], counts[2], all.len());
        assert!(shortcuts > 0, "no Start-menu shortcuts found");
        assert!(all.len() >= 5, "only {} programs found", all.len());
        for e in &all {
            let lower = e.launch.to_ascii_lowercase();
            assert!(lower.ends_with(".lnk") || lower.ends_with(".exe"), "{} -> {}", e.name, e.launch);
            assert!(std::path::Path::new(&e.launch).is_file(), "{} -> {} is missing", e.name, e.launch);
            assert!(!is_noise(&e.name), "{} should have been filtered", e.name);
        }
        assert!(all.iter().any(|e| e.launch.to_ascii_lowercase().ends_with(".lnk")), "no shortcut entries");
        let mut names: Vec<String> = all.iter().map(|e| e.name.to_lowercase()).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate program names");
    }
}
