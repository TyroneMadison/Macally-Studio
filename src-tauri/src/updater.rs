//! Update check and install through tauri-plugin-updater (GitHub Releases latest.json).
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::commands::AppState;

/// Payload of the "update-available" event and the result of `check_update`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpdateInfo {
    /// Display form: "1.1", never "1.1.0".
    pub version: String,
    /// Release notes (the GitHub release body); empty when there are none.
    pub notes: String,
}

pub fn info(semver: &str, body: Option<&str>) -> UpdateInfo {
    UpdateInfo { version: crate::version::display(semver), notes: body.unwrap_or_default().to_string() }
}

/// Asks GitHub Releases for a newer version and emits "update-available" when there is one.
pub async fn check(app: &AppHandle) -> anyhow::Result<Option<UpdateInfo>> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(None);
    };
    let found = info(&update.version, update.body.as_deref());
    app.emit("update-available", found.clone())?;
    Ok(Some(found))
}

/// Downloads and installs the newer version, then relaunches. Does nothing when already current.
/// On Windows the installer closes the app itself before `restart` is reached.
pub async fn install(app: &AppHandle) -> anyhow::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    update.download_and_install(|_, _| {}, || {}).await?;
    app.restart()
}

/// Background check at startup when "Check for updates automatically" is on.
pub fn check_on_launch(app: &AppHandle) {
    let enabled = app.try_state::<AppState>().is_some_and(|s| s.settings.lock().unwrap().auto_check_updates);
    if !enabled {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        // ponytail: fixed delay so the UI has attached its "update-available" listener;
        // swap for a UI-ready handshake if the banner is ever missed.
        std::thread::sleep(Duration::from_secs(5));
        if let Err(e) = tauri::async_runtime::block_on(check(&app)) {
            eprintln!("update check failed: {e:#}");
        }
    });
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    check(&app).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    install(&app).await.map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_version_is_shown_without_patch() {
        assert_eq!(
            info("1.1.0", Some("Faster app scan")),
            UpdateInfo { version: "1.1".into(), notes: "Faster app scan".into() }
        );
        assert_eq!(info("2.0.0", None), UpdateInfo { version: "2.0".into(), notes: String::new() });
    }
}
