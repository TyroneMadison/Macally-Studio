// macOS app bundles: Info.plist gives id and name, NSWorkspace renders the icon to a 64 px PNG.
// Launch = bundle id (`open -b <id>` in actions.rs).

use std::path::Path;

use super::AppEntry;

/// Apple apps listed first, in this order.
pub const PINNED: &[&str] = &[
    "com.apple.calculator",
    "com.apple.clock",
    "com.apple.AppStore",
    "com.apple.systempreferences",
    "com.apple.screenshot.launcher",
    "com.apple.Notes",
    "com.apple.Terminal",
    "com.apple.finder",
    "com.apple.Safari",
];

/// Built-in = shipped by Apple: everything under /System, plus Safari (it lives in /Applications).
pub fn is_builtin(bundle: &Path, bundle_id: &str) -> bool {
    bundle.starts_with("/System") || bundle_id == "com.apple.Safari"
}

pub fn entry(bundle: &Path, bundle_id: &str, name: String, icon: Option<String>) -> AppEntry {
    let group = if is_builtin(bundle, bundle_id) {
        "builtin"
    } else {
        "app"
    };
    AppEntry {
        id: format!("{group}:{bundle_id}"),
        name,
        kind: "bundle".into(),
        launch: bundle_id.to_string(),
        icon,
        source: "macos".into(),
    }
}

#[cfg(target_os = "macos")]
pub fn scan(icons_dir: &Path) -> anyhow::Result<Vec<AppEntry>> {
    use std::path::PathBuf;

    #[derive(serde::Deserialize)]
    struct Info {
        #[serde(rename = "CFBundleIdentifier")]
        id: String,
        #[serde(rename = "CFBundleDisplayName")]
        display_name: Option<String>,
        #[serde(rename = "CFBundleName")]
        name: Option<String>,
    }

    let home = dirs::home_dir().unwrap_or_default();
    let dirs = [
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        home.join("Applications"),
    ];
    let mut bundles = vec![PathBuf::from("/System/Library/CoreServices/Finder.app")];
    for dir in &dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        bundles.extend(
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "app")),
        );
    }
    let mut out = Vec::new();
    for bundle in bundles {
        // plist reads binary and XML Info.plist files alike
        let Ok(info) = plist::from_file::<_, Info>(bundle.join("Contents/Info.plist")) else {
            continue;
        };
        let stem = bundle
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = info.display_name.or(info.name).unwrap_or(stem);
        let file = icons_dir.join(super::icon_file_name(&info.id, "png"));
        // ponytail: cached icons never refresh; delete <data_dir>/icons to rebuild them.
        let icon = (file.is_file() || save_icon_png(&bundle, &file).is_ok())
            .then(|| file.to_string_lossy().into_owned());
        out.push(entry(&bundle, &info.id, name, icon));
    }
    super::sort_entries(&mut out, PINNED);
    Ok(out)
}

#[cfg(not(target_os = "macos"))]
pub fn scan(_icons_dir: &Path) -> anyhow::Result<Vec<AppEntry>> {
    Ok(Vec::new())
}

/// NSWorkspace icon of a bundle (covers .icns and Assets.car apps) -> 64 px PNG file.
#[cfg(target_os = "macos")]
fn save_icon_png(bundle: &Path, file: &Path) -> anyhow::Result<()> {
    use objc2::AnyThread;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
    use objc2_foundation::{NSDictionary, NSSize, NSString};

    // Scans run on a worker thread with no autorelease pool of its own.
    let png = objc2::rc::autoreleasepool(|_| {
        let img = NSWorkspace::sharedWorkspace()
            .iconForFile(&NSString::from_str(&bundle.to_string_lossy()));
        img.setSize(NSSize::new(64.0, 64.0));
        let cg =
            unsafe { img.CGImageForProposedRect_context_hints(std::ptr::null_mut(), None, None) }?;
        let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
        let data = unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
        }?;
        Some(data.to_vec())
    })
    .ok_or_else(|| anyhow::anyhow!("no icon image for {}", bundle.display()))?;
    // Write then rename, so a failed write never leaves a half-written icon that looks cached.
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, png)?;
    std::fs::rename(&tmp, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_means_system_or_safari() {
        assert!(is_builtin(
            Path::new("/System/Applications/Calculator.app"),
            "com.apple.calculator"
        ));
        assert!(is_builtin(
            Path::new("/Applications/Safari.app"),
            "com.apple.Safari"
        ));
        assert!(!is_builtin(
            Path::new("/Applications/Slack.app"),
            "com.tinyspeck.slackmacgap"
        ));
    }

    #[test]
    fn entry_uses_bundle_id_as_launch() {
        let e = entry(
            Path::new("/System/Applications/Calculator.app"),
            "com.apple.calculator",
            "Calculator".into(),
            None,
        );
        assert_eq!(e.id, "builtin:com.apple.calculator");
        assert_eq!(
            (e.kind.as_str(), e.launch.as_str(), e.source.as_str()),
            ("bundle", "com.apple.calculator", "macos")
        );
        let slack = entry(
            Path::new("/Applications/Slack.app"),
            "com.tinyspeck.slackmacgap",
            "Slack".into(),
            None,
        );
        assert_eq!(slack.id, "app:com.tinyspeck.slackmacgap");
    }
}
