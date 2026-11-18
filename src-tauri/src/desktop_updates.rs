use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{
    ipc::Channel,
    utils::{config::BundleType, platform::bundle_type},
    AppHandle, Manager, State,
};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

const UNAVAILABLE: &str = "Install a supported musicOS release to check for updates.";
const BUSY: &str = "An update operation is already in progress.";

#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Serialize)]
pub struct UpdateStatus {
    version: String,
    available: bool,
}

#[derive(Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum InstallProgress {
    Downloading { received: u64, total: Option<u64> },
    Installing,
}

fn supported_bundle(bundle: Option<BundleType>, is_appimage: bool, arch: &str) -> bool {
    matches!(
        (bundle, is_appimage, arch),
        (Some(BundleType::App), _, "aarch64")
            | (Some(BundleType::Nsis), _, "x86_64")
            | (Some(BundleType::AppImage), true, "x86_64")
    )
}

fn is_supported(app: &AppHandle) -> bool {
    #[cfg(target_os = "linux")]
    let is_appimage = app.env().appimage.is_some();
    #[cfg(not(target_os = "linux"))]
    let is_appimage = false;

    !cfg!(debug_assertions)
        && app.config().plugins.0.contains_key("updater")
        && supported_bundle(bundle_type(), is_appimage, std::env::consts::ARCH)
}

#[tauri::command]
pub fn desktop_update_status(app: AppHandle) -> UpdateStatus {
    UpdateStatus {
        version: app.package_info().version.to_string(),
        available: is_supported(&app),
    }
}

#[tauri::command]
pub async fn desktop_update_check(
    app: AppHandle,
    state: State<'_, PendingUpdate>,
) -> Result<Option<String>, String> {
    if !is_supported(&app) {
        return Err(UNAVAILABLE.into());
    }
    let mut pending = state.0.try_lock().map_err(|_| BUSY)?;
    *pending = None;

    let update = async {
        let builder = app.updater_builder();
        #[cfg(target_os = "windows")]
        let builder = {
            let app = app.clone();
            builder.on_before_exit(move || {
                crate::app::save_playback_session(&app);
                app.cleanup_before_exit();
            })
        };
        builder
            .timeout(Duration::from_secs(20))
            .configure_client(|client| {
                client
                    .https_only(true)
                    .connect_timeout(Duration::from_secs(15))
            })
            .build()?
            .check()
            .await
    }
    .await
    .map_err(|error| {
        eprintln!("Could not check for desktop updates: {error}");
        "Could not check GitHub Releases. Check your connection and try again."
    })?;
    let version = update.as_ref().map(|update| update.version.clone());
    *pending = update;
    Ok(version)
}

#[tauri::command]
pub async fn desktop_update_install(
    app: AppHandle,
    state: State<'_, PendingUpdate>,
    version: String,
    progress: Channel<InstallProgress>,
) -> Result<(), String> {
    if !is_supported(&app) {
        return Err(UNAVAILABLE.into());
    }
    let mut pending = state.0.try_lock().map_err(|_| BUSY)?;
    let update = pending
        .as_mut()
        .filter(|update| update.version == version)
        .ok_or("Check for updates again before installing.")?;
    update.timeout = Some(Duration::from_secs(30 * 60));

    let mut received = 0;
    let mut last_report = None::<Instant>;
    update
        .download_and_install(
            |chunk, total| {
                received += chunk as u64;
                if last_report.is_none_or(|last| last.elapsed() >= Duration::from_millis(150)) {
                    last_report = Some(Instant::now());
                    let _ = progress.send(InstallProgress::Downloading { received, total });
                }
            },
            || {
                let _ = progress.send(InstallProgress::Installing);
            },
        )
        .await
        .map_err(|error| {
            eprintln!("Could not install desktop update: {error}");
            "Could not download, verify, or install the update. The current version is still running."
        })?;

    *pending = None;
    // The Windows installer handles exit and relaunch; elsewhere let the normal exit handler save the session.
    #[cfg(not(target_os = "windows"))]
    app.request_restart();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_release_bundles_can_update() {
        assert!(supported_bundle(Some(BundleType::App), false, "aarch64"));
        assert!(supported_bundle(Some(BundleType::Nsis), false, "x86_64"));
        assert!(supported_bundle(Some(BundleType::AppImage), true, "x86_64"));
        assert!(!supported_bundle(Some(BundleType::App), false, "x86_64"));
        assert!(!supported_bundle(
            Some(BundleType::AppImage),
            false,
            "x86_64"
        ));
        assert!(!supported_bundle(Some(BundleType::Msi), false, "x86_64"));
        assert!(!supported_bundle(None, false, "x86_64"));
    }
}
