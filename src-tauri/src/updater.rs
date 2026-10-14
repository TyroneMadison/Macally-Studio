//! Update check and install through tauri-plugin-updater (GitHub Releases latest.json).
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

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

/// Downloads and installs the newer version, then relaunches. Fails when there is none (e.g. the release was pulled).
/// On Windows the installer closes the app itself before `restart` is reached.
pub async fn install(app: &AppHandle) -> anyhow::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        anyhow::bail!("no update available");
    };
    update.download_and_install(|_, _| {}, || {}).await?;
    app.restart()
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
