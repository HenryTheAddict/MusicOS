use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use serde::Serialize;
use tauri::{
    ipc::Channel,
    utils::{config::BundleType, platform::bundle_type},
    AppHandle, Manager, State,
};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};
use tokio::sync::Mutex;

const UNAVAILABLE: &str = "Install a supported musicOS release to check for updates.";
const BUSY: &str = "An update operation is already in progress.";
const MAX_MANIFEST_SIZE: u32 = 1024 * 1024;

#[derive(Default)]
pub struct PendingUpdate {
    checked: Mutex<Option<Update>>,
    cache: OnceLock<PathBuf>,
    instance: std::sync::Mutex<Option<File>>,
    audio_hold: std::sync::Mutex<Option<Arc<()>>>,
    launch_error: std::sync::Mutex<Option<String>>,
}

#[derive(Serialize)]
pub struct UpdateStatus {
    version: String,
    available: bool,
    ready: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum DownloadProgress {
    Downloading { received: u64, total: Option<u64> },
    Verifying,
}

fn supported_bundle(bundle: Option<BundleType>, is_appimage: bool, arch: &str) -> bool {
    matches!(
        (bundle, is_appimage, arch),
        (Some(BundleType::App), _, "aarch64")
            | (Some(BundleType::App), _, "x86_64")
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

pub fn initialize(app: &AppHandle) -> Result<()> {
    if !is_supported(app) {
        return Ok(());
    }
    let state = app.state::<PendingUpdate>();
    let directory = app.path().app_local_data_dir()?.join("desktop-updates");
    fs::create_dir_all(&directory)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(directory.join("instance.lock"))?;
    if let Err(error) = fs2::FileExt::try_lock_exclusive(&lock) {
        // A second packaged process must not replace the app while the first one plays audio.
        if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() {
            std::process::exit(0);
        }
        return Err(error.into());
    }
    *state.instance.lock().unwrap() = Some(lock);
    let _ = state.cache.set(directory.join("ready"));
    Ok(())
}

pub fn release_instance(app: &AppHandle) {
    app.state::<PendingUpdate>().instance.lock().unwrap().take();
}

fn updater(app: &AppHandle) -> tauri_plugin_updater::Result<Updater> {
    app.updater_builder()
        .timeout(Duration::from_secs(20))
        .configure_client(|client| {
            client
                .https_only(true)
                .connect_timeout(Duration::from_secs(15))
        })
        .build()
}

fn write_cache(path: &Path, update: &Update, bytes: &[u8]) -> Result<()> {
    let manifest = serde_json::to_vec(&update.raw_json)?;
    anyhow::ensure!(
        manifest.len() <= MAX_MANIFEST_SIZE as usize,
        "Update manifest is too large"
    );
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing update directory")?)?;
    file.write_all(&(manifest.len() as u32).to_le_bytes())?;
    file.write_all(&manifest)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

fn read_cache(path: &Path) -> Result<Option<(serde_json::Value, File)>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut size = [0; 4];
    file.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size);
    anyhow::ensure!(
        size <= MAX_MANIFEST_SIZE,
        "Invalid cached update manifest size"
    );
    let mut manifest = vec![0; size as usize];
    file.read_exact(&mut manifest)?;
    Ok(Some((serde_json::from_slice(&manifest)?, file)))
}

fn restore(app: &AppHandle, path: &Path) -> Result<Option<(Update, File)>> {
    let Some((manifest, file)) = read_cache(path)? else {
        return Ok(None);
    };
    Ok(updater(app)?
        .restore_update(manifest)?
        .map(|update| (update, file)))
}

#[tauri::command]
pub fn desktop_update_status(app: AppHandle, state: State<'_, PendingUpdate>) -> UpdateStatus {
    let ready = state
        .cache
        .get()
        .and_then(|path| restore(&app, path).ok().flatten())
        .map(|(update, _)| update.version);
    UpdateStatus {
        version: app.package_info().version.to_string(),
        available: state.cache.get().is_some(),
        ready,
        error: state.launch_error.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub async fn desktop_update_check(
    app: AppHandle,
    state: State<'_, PendingUpdate>,
) -> Result<Option<String>, String> {
    let path = state.cache.get().ok_or(UNAVAILABLE)?;
    let mut pending = state.checked.try_lock().map_err(|_| BUSY)?;
    if restore(&app, path).ok().flatten().is_some() {
        return Err("An update is already downloaded. Restart to apply it.".into());
    }
    *pending = None;
    let update = async { updater(&app)?.check().await }
        .await
        .map_err(|error| {
            eprintln!("Could not check for desktop updates: {error}");
            "Couldn't check for updates. Check your connection and try again."
        })?;
    let version = update.as_ref().map(|update| update.version.clone());
    *pending = update;
    Ok(version)
}

#[tauri::command]
pub async fn desktop_update_download(
    state: State<'_, PendingUpdate>,
    version: String,
    progress: Channel<DownloadProgress>,
) -> Result<(), String> {
    let path = state.cache.get().ok_or(UNAVAILABLE)?;
    let mut pending = state.checked.try_lock().map_err(|_| BUSY)?;
    let update = pending
        .as_ref()
        .filter(|update| update.version == version)
        .ok_or("Check for updates again before downloading.")?;

    let mut update = update.clone();
    update.timeout = Some(Duration::from_secs(30 * 60));
    let mut received = 0;
    let mut last_report = None::<Instant>;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                if last_report.is_none_or(|last| last.elapsed() >= Duration::from_millis(150)) {
                    last_report = Some(Instant::now());
                    let _ = progress.send(DownloadProgress::Downloading { received, total });
                }
            },
            || {
                let _ = progress.send(DownloadProgress::Verifying);
            },
        )
        .await
        .map_err(|error| {
            eprintln!("Could not download desktop update: {error}");
            "Could not download or verify the update. Try again."
        })?;
    let path = path.clone();
    tauri::async_runtime::spawn_blocking(move || write_cache(&path, &update, &bytes))
        .await
        .map_err(|_| "Could not save the downloaded update. Try again.")?
        .map_err(|error| {
            eprintln!("Could not save desktop update: {error}");
            "Could not save the downloaded update. Check free disk space and try again."
        })?;
    *pending = None;
    Ok(())
}

fn install_ready(
    app: &AppHandle,
    path: &Path,
    expected: Option<&str>,
    hold: Option<Arc<()>>,
    pause_playback: bool,
) -> Result<bool> {
    let restored = restore(app, path);
    let Some((update, mut file)) = (match restored {
        Ok(update) => update,
        Err(error) => {
            let _ = fs::remove_file(path);
            return Err(
                error.context("The saved update could not be read. Check for updates again.")
            );
        }
    }) else {
        let _ = fs::remove_file(path);
        return Ok(false);
    };
    if let Some(version) = expected {
        anyhow::ensure!(
            version == update.version,
            "The downloaded update has changed."
        );
    }

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    drop(file);
    if let Err(error) = update.verify(&bytes) {
        let _ = fs::remove_file(path);
        return Err(anyhow::Error::new(error)
            .context("The saved update could not be verified. Check for updates again."));
    }

    // Claim the audio worker after verification so a racing Play command cannot interrupt an install.
    let hold = match hold {
        Some(hold) => hold,
        None => crate::app::hold_audio_for_update(app, pause_playback)?,
    };
    crate::app::save_playback_session(app);
    #[cfg(target_os = "windows")]
    crate::app::stop_preview_runtime(app)
        .context("Could not close the device preview. Try again.")?;
    update
        .install(&bytes)
        .context("Could not install the update. Try again.")?;
    // On Windows the updater exits in install(); the new version clears the stale cache on launch.
    let _ = fs::remove_file(path);
    *app.state::<PendingUpdate>().audio_hold.lock().unwrap() = Some(hold);
    app.request_restart();
    Ok(true)
}

pub fn apply_on_launch(app: &AppHandle) {
    let state = app.state::<PendingUpdate>();
    let Some(path) = state.cache.get().filter(|path| path.is_file()).cloned() else {
        return;
    };
    let hold = match crate::app::hold_audio_for_update(app, false) {
        Ok(hold) => hold,
        Err(error) => {
            *state.launch_error.lock().unwrap() = Some(error.to_string());
            return;
        }
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<PendingUpdate>();
        let _operation = state.checked.lock().await;
        let handle = app.clone();
        if let Ok(Err(error)) = tauri::async_runtime::spawn_blocking(move || {
            install_ready(&handle, &path, None, Some(hold), false)
        })
        .await
        {
            eprintln!("Could not apply desktop update on launch: {error:#}");
            *state.launch_error.lock().unwrap() = Some(error.to_string());
        }
    });
}

#[tauri::command]
pub async fn desktop_update_restart(
    app: AppHandle,
    state: State<'_, PendingUpdate>,
    version: String,
    pause_playback: bool,
) -> Result<(), String> {
    let path = state.cache.get().ok_or(UNAVAILABLE)?.clone();
    let _operation = state.checked.try_lock().map_err(|_| BUSY)?;
    match tauri::async_runtime::spawn_blocking(move || {
        install_ready(&app, &path, Some(&version), None, pause_playback)
    })
    .await
    {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err("No downloaded update was found. Check for updates again.".into()),
        Ok(Err(error)) => {
            eprintln!("Could not apply desktop update: {error:#}");
            Err(error.to_string())
        }
        Err(_) => Err("Could not install the update. Try again.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_release_bundles_can_update() {
        assert!(supported_bundle(Some(BundleType::App), false, "aarch64"));
        assert!(supported_bundle(Some(BundleType::Nsis), false, "x86_64"));
        assert!(supported_bundle(Some(BundleType::AppImage), true, "x86_64"));
        assert!(supported_bundle(Some(BundleType::App), false, "x86_64"));
        assert!(!supported_bundle(Some(BundleType::App), false, "arm"));
        assert!(!supported_bundle(
            Some(BundleType::AppImage),
            false,
            "x86_64"
        ));
        assert!(!supported_bundle(Some(BundleType::Msi), false, "x86_64"));
        assert!(!supported_bundle(None, false, "x86_64"));
    }

    #[test]
    fn cache_reads_manifest_and_leaves_package_for_verification() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ready");
        assert!(read_cache(&path).unwrap().is_none());
        let mut file = File::create(&path).unwrap();
        let manifest = br#"{"version":"0.2.0"}"#;
        file.write_all(&(manifest.len() as u32).to_le_bytes())
            .unwrap();
        file.write_all(manifest).unwrap();
        file.write_all(b"signed package").unwrap();
        drop(file);
        let (metadata, mut payload) = read_cache(&path).unwrap().unwrap();
        assert_eq!(metadata["version"], "0.2.0");
        let mut bytes = Vec::new();
        payload.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"signed package");
        drop(payload);
        fs::write(&path, (MAX_MANIFEST_SIZE + 1).to_le_bytes()).unwrap();
        assert!(read_cache(&path).is_err());
    }
}
