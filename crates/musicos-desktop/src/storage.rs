use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Serialize};

use crate::models::{ControlCenterConfig, SyncProgress, DESKTOP_CONFIG_VERSION};

pub fn desktop_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("MUSICOS_APP_DATA") {
        return absolute_from_current_dir(PathBuf::from(path));
    }
    ProjectDirs::from("com", "MusicOS", "MusicOS Control Center")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("out/control-center"))
}

pub fn config_path() -> PathBuf {
    std::env::var_os("MUSICOS_DESKTOP_CONFIG")
        .map(PathBuf::from)
        .map(absolute_from_current_dir)
        .unwrap_or_else(|| desktop_data_dir().join("control-center.json"))
}

pub fn load_config() -> Result<ControlCenterConfig> {
    let path = config_path();
    if path.exists() {
        match read_json::<ControlCenterConfig>(&path) {
            Ok(mut config) => {
                let previous_version = config.version;
                if previous_version < 4 {
                    // Native releases before v4 always synchronized by copying
                    // the source file, so preserve that behavior on upgrade.
                    config.selected_sync_profile = "original".to_string();
                }
                if previous_version < 5 {
                    config.last_section = crate::models::DesktopSection::Albums;
                    config.library_view = crate::models::LibraryViewMode::Albums;
                    config.library_ui.last_scope = crate::models::LibraryScopeKey::Albums;
                    config.library_ui.presentation_by_scope.insert(
                        crate::models::LibraryScopeKey::Albums,
                        crate::models::LibraryPresentation::Grid,
                    );
                    config.library_ui.presentation_by_scope.insert(
                        crate::models::LibraryScopeKey::Artists,
                        crate::models::LibraryPresentation::Grid,
                    );
                }
                if config.format_profiles.is_empty() {
                    config.format_profiles = crate::models::built_in_format_profiles();
                }
                config.version = DESKTOP_CONFIG_VERSION;
                apply_env_overrides(&mut config);
                normalize_persisted_paths(&mut config);
                save_config(&config)?;
                return Ok(config);
            }
            Err(_) => {
                let corrupt = path.with_extension(format!(
                    "corrupt-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                ));
                fs::copy(&path, &corrupt).with_context(|| {
                    format!("preserving corrupt configuration as {}", corrupt.display())
                })?;
            }
        }
    }

    let mut config = migrate_electron_config()?.unwrap_or_else(fresh_config);
    apply_env_overrides(&mut config);
    normalize_persisted_paths(&mut config);
    save_config(&config)?;
    Ok(config)
}

fn fresh_config() -> ControlCenterConfig {
    let storage:serde_json::Value=serde_json::from_str(include_str!("../../../product.config.json")).expect("Validated product configuration");
    let storage=&storage["storage"];let data=desktop_data_dir();
    ControlCenterConfig {media_dir:data.join(storage["mediaDirectory"].as_str().unwrap()),db_path:data.join(storage["databaseFile"].as_str().unwrap()),sync_dir:data.join(storage["syncDirectory"].as_str().unwrap()),..Default::default()}
}

pub fn save_config(config: &ControlCenterConfig) -> Result<()> {
    atomic_write_json(&config_path(), config)
}

pub fn append_sync_history(progress: &SyncProgress) -> Result<()> {
    let path = desktop_data_dir().join("control-center-sync-history.json");
    let mut history: Vec<SyncProgress> = if path.exists() {
        read_json(&path).unwrap_or_default()
    } else {
        Vec::new()
    };
    history.insert(0, progress.clone());
    history.truncate(50);
    atomic_write_json(&path, &history)
}

fn apply_env_overrides(config: &mut ControlCenterConfig) {
    if let Ok(port) = std::env::var("MUSICOS_COMPANION_PORT") {
        config.library_target = format!("http://127.0.0.1:{port}");
    }
    if let Ok(target) = std::env::var("MUSICOS_SYNC_TARGET") {
        config.target = target.clone();
        config.device_target = target;
    }
    if let Some(path) = std::env::var_os("MUSICOS_SYNC_DIR") {
        config.sync_dir = absolute_from_current_dir(path.into());
    }
    if let Some(path) = std::env::var_os("MUSICOS_MEDIA_DIR") {
        config.media_dir = absolute_from_current_dir(path.into());
    }
    if let Some(path) = std::env::var_os("MUSICOS_MEDIA_DB") {
        config.db_path = absolute_from_current_dir(path.into());
    }
}

fn normalize_persisted_paths(config: &mut ControlCenterConfig) {
    let defaults = fresh_config();
    config.sync_dir = resolve_persisted_path(&config.sync_dir, defaults.sync_dir);
    config.media_dir = resolve_persisted_path(&config.media_dir, defaults.media_dir);
    config.db_path = resolve_persisted_path(&config.db_path, defaults.db_path);
}

fn resolve_persisted_path(path: &Path, fallback: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let from_current_dir = absolute_from_current_dir(path.to_path_buf());
    if from_current_dir.exists() {
        from_current_dir
    } else {
        fallback
    }
}

fn absolute_from_current_dir(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn migrate_electron_config() -> Result<Option<ControlCenterConfig>> {
    for legacy_root in legacy_electron_roots() {
        if !legacy_root.exists() {
            continue;
        }
        let mut config = ControlCenterConfig::default();
        let legacy_db = legacy_root.join("desktop-media.sqlite3");
        let legacy_media = legacy_root.join("MusicOS Media");
        if legacy_db.exists() {
            config.db_path = legacy_db;
        }
        if legacy_media.exists() {
            config.media_dir = legacy_media;
        }
        let legacy_settings = legacy_root.join("settings.json");
        if legacy_settings.exists() {
            let settings: serde_json::Value = read_json(&legacy_settings)?;
            if let Some(value) = settings
                .get("deviceTarget")
                .and_then(|value| value.as_str())
            {
                config.device_target = value.to_string();
            }
            if let Some(value) = settings.get("syncDir").and_then(|value| value.as_str()) {
                config.sync_dir = value.into();
            }
            if let Some(value) = settings.get("mediaDir").and_then(|value| value.as_str()) {
                config.media_dir = value.into();
            }
            if let Some(value) = settings.get("dbPath").and_then(|value| value.as_str()) {
                config.db_path = value.into();
            }
        }
        migrate_legacy_document(
            &legacy_root.join("control-center-sync-history.json"),
            &desktop_data_dir().join("control-center-sync-history.json"),
        )?;
        migrate_legacy_document(
            &legacy_root.join("playlist-sync-snapshots.json"),
            &desktop_data_dir().join("playlist-sync-snapshots.json"),
        )?;
        return Ok(Some(config));
    }
    Ok(None)
}

fn migrate_legacy_document(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() || destination.exists() || source == destination {
        return Ok(());
    }
    let value: serde_json::Value = read_json(source)?;
    atomic_write_json(destination, &value)
}

pub fn legacy_electron_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.push(
            home.join("Library/Application Support")
                .join("musicos-control-center"),
        );
        roots.push(home.join(".config").join("musicos-control-center"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(app_data).join("musicos-control-center"));
    }
    roots
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().unwrap_or_else(|| Path::new(".")))?;
    serde_json::to_writer_pretty(&mut temporary, value)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("persisting {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_install_uses_profile_paths_instead_of_the_launch_folder(){let config=fresh_config();let data=desktop_data_dir();assert!(config.media_dir.starts_with(&data));assert!(config.db_path.starts_with(&data));assert!(config.sync_dir.starts_with(&data));assert!(config.media_dir.is_absolute());}

    #[test]
    fn atomic_json_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.json");
        let value = vec!["Music", "Albums", "Songs"];
        atomic_write_json(&path, &value).unwrap();
        assert_eq!(read_json::<Vec<String>>(&path).unwrap(), value);
    }

    #[test]
    fn unresolved_relative_library_paths_fall_back_to_app_data() {
        let fallback = PathBuf::from("/stable/MusicOS Media");
        let resolved = resolve_persisted_path(
            Path::new("definitely-missing-musicos-path"),
            fallback.clone(),
        );
        assert_eq!(resolved, fallback);
    }
}
