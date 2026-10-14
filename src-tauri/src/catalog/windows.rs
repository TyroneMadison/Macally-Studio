// Windows Start-menu apps via Get-StartApps: one list covering Win32, Store and system apps.
// Launch = `shell:AppsFolder\<AppID>`, which ShellExecute opens for every AppID shape.

use std::path::Path;

use serde::Deserialize;

use super::AppEntry;

/// Well-known system apps: marked built-in and listed first, in this order.
pub const BUILTIN: &[&str] = &[
    "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
    "Microsoft.WindowsAlarms_8wekyb3d8bbwe!App",
    "Microsoft.WindowsStore_8wekyb3d8bbwe!App",
    "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel",
    "Microsoft.ScreenSketch_8wekyb3d8bbwe!App",
    "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App",
    "Microsoft.Windows.Explorer",
    "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
    "Microsoft.Paint_8wekyb3d8bbwe!App",
    "Microsoft.Windows.ControlPanel",
];

#[derive(Deserialize)]
struct StartApp {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AppID")]
    app_id: String,
}

/// `Get-StartApps | ConvertTo-Json` output -> entries, built-ins first. Pure, so it is tested on every OS.
pub fn parse_start_apps(json: &str) -> Vec<AppEntry> {
    let json = json.trim_start_matches('\u{feff}').trim(); // Windows PowerShell 5.1 may emit a BOM
    // ConvertTo-Json unwraps a one-element array into a bare object.
    let apps: Vec<StartApp> = match serde_json::from_str::<serde_json::Value>(json) {
        Ok(serde_json::Value::Array(a)) => a
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect(),
        Ok(v @ serde_json::Value::Object(_)) => serde_json::from_value(v).into_iter().collect(),
        _ => Vec::new(),
    };
    let mut out: Vec<AppEntry> = apps
        .into_iter()
        // URL shortcuts (support links, steam://rungameid/...) are not apps; steam.rs lists the games.
        .filter(|a| !a.app_id.contains("://"))
        .map(|a| {
            let group = if BUILTIN.contains(&a.app_id.as_str()) {
                "builtin"
            } else {
                "app"
            };
            let launch = format!(r"shell:AppsFolder\{}", a.app_id);
            AppEntry {
                id: format!("{group}:{launch}"),
                name: a.name,
                kind: "aumid".into(),
                launch,
                icon: None,
                source: "windows".into(),
            }
        })
        .collect();
    super::sort_entries(&mut out, BUILTIN);
    out
}

#[cfg(target_os = "windows")]
pub fn scan(icons_dir: &Path) -> anyhow::Result<Vec<AppEntry>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding = [Text.Encoding]::UTF8; Get-StartApps | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    anyhow::ensure!(
        out.status.success(),
        "Get-StartApps failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut apps = parse_start_apps(&String::from_utf8_lossy(&out.stdout));
    for app in &mut apps {
        let file = icons_dir.join(super::icon_file_name(&app.launch, "png"));
        // ponytail: cached icons never refresh; delete <data_dir>/icons to rebuild them.
        if file.is_file() || save_icon_png(&app.launch, &file).is_ok() {
            app.icon = Some(file.to_string_lossy().into_owned());
        }
    }
    Ok(apps)
}

#[cfg(not(target_os = "windows"))]
pub fn scan(_icons_dir: &Path) -> anyhow::Result<Vec<AppEntry>> {
    Ok(Vec::new())
}

/// Shell icon of a parsing name (`shell:AppsFolder\<AppID>` or an exe path) -> 64 px PNG file.
#[cfg(target_os = "windows")]
fn save_icon_png(parsing_name: &str, file: &Path) -> anyhow::Result<()> {
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
        GetDIBits, GetObjectW, HGDIOBJ, ReleaseDC,
    };
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
    };
    use windows::core::HSTRING;

    let (w, h, mut px) = unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED); // S_FALSE when already initialised
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(parsing_name), None)?;
        let hbmp = factory.GetImage(SIZE { cx: 64, cy: 64 }, SIIGBF_ICONONLY)?;
        let obj = HGDIOBJ::from(hbmp);
        let mut bm = BITMAP::default();
        GetObjectW(
            obj,
            std::mem::size_of::<BITMAP>() as i32,
            Some((&raw mut bm).cast()),
        );
        let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // negative = top-down rows
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        let mut px = vec![0u8; (w * h * 4) as usize];
        let hdc = GetDC(None);
        let lines = GetDIBits(
            hdc,
            hbmp,
            0,
            h as u32,
            Some(px.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);
        let _ = DeleteObject(obj);
        anyhow::ensure!(w > 0 && lines == h, "GetDIBits copied {lines} of {h} rows");
        (w as u32, h as u32, px)
    };
    for c in px.as_chunks_mut::<4>().0 {
        c.swap(0, 2); // BGRA -> RGBA
    }
    // ponytail: the shell bitmap may be premultiplied; semi-transparent edges come out a little dark,
    // invisible on the dark UI. Un-premultiply here if icons ever show on a light background.
    // Write then rename, so a failed encode never leaves a half-written icon that looks cached.
    let tmp = file.with_extension("tmp");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(&tmp)?), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header()?;
    writer.write_image_data(&px)?;
    writer.finish()?;
    std::fs::rename(&tmp, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_start_apps_with_bom_builtins_first_and_skips_urls() {
        let json = "\u{feff}[\
            {\"Name\":\"7-Zip File Manager\",\"AppID\":\"{6D809377-6AF0-444B-8957-A3773F02200E}\\\\7-Zip\\\\7zFM.exe\"},\
            {\"Name\":\"Clock\",\"AppID\":\"Microsoft.WindowsAlarms_8wekyb3d8bbwe!App\"},\
            {\"Name\":\"Steam Support Center\",\"AppID\":\"http://support.steampowered.com/\"},\
            {\"Name\":\"Black Myth: Wukong\",\"AppID\":\"steam://rungameid/2358720\"},\
            {\"Name\":\"Calculator\",\"AppID\":\"Microsoft.WindowsCalculator_8wekyb3d8bbwe!App\"}\
        ]";
        let apps = parse_start_apps(json);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Calculator", "Clock", "7-Zip File Manager"]);
        assert_eq!(
            apps[0].launch,
            r"shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"
        );
        assert_eq!(
            apps[0].id,
            r"builtin:shell:AppsFolder\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"
        );
        assert_eq!(
            (apps[0].kind.as_str(), apps[0].source.as_str()),
            ("aumid", "windows")
        );
        assert_eq!(
            apps[2].launch,
            r"shell:AppsFolder\{6D809377-6AF0-444B-8957-A3773F02200E}\7-Zip\7zFM.exe"
        );
        assert!(apps[2].id.starts_with("app:"));
    }

    #[test]
    fn parses_single_object_and_garbage() {
        let one = r#"{"Name":"Notepad","AppID":"Microsoft.WindowsNotepad_8wekyb3d8bbwe!App"}"#;
        assert_eq!(parse_start_apps(one).len(), 1);
        assert!(parse_start_apps("").is_empty());
    }
}
