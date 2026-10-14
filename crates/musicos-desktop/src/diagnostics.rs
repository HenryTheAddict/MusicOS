use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::models::{ControlCenterConfig, EmulatorStatus};
use crate::storage::atomic_write_json;

#[derive(Debug, Serialize)]
pub struct DiagnosticsBundle<'a> {
    pub generated_at_unix_ms: u128,
    pub application_version: &'static str,
    pub os: &'static str,
    pub architecture: &'static str,
    pub config: &'a ControlCenterConfig,
    pub emulator: &'a EmulatorStatus,
    pub logs: &'a [String],
}

pub fn export(
    destination: &Path,
    config: &ControlCenterConfig,
    emulator: &EmulatorStatus,
    logs: &[String],
) -> Result<()> {
    let generated_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    atomic_write_json(
        destination,
        &DiagnosticsBundle {
            generated_at_unix_ms,
            application_version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            config,
            emulator,
            logs,
        },
    )
}

pub fn clear_cover_cache(cache: &Path) -> Result<usize> {
    if !cache.exists() {
        return Ok(0);
    }
    let count = fs::read_dir(cache)?.filter_map(Result::ok).count();
    fs::remove_dir_all(cache).with_context(|| format!("clearing {}", cache.display()))?;
    fs::create_dir_all(cache)?;
    Ok(count)
}

pub fn reset_database(config: &ControlCenterConfig, explicitly_confirmed: bool) -> Result<PathBuf> {
    if !explicitly_confirmed {
        bail!("database reset requires explicit confirmation");
    }
    if !config.db_path.exists() {
        return Ok(config.db_path.clone());
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup = config
        .db_path
        .with_extension(format!("sqlite3.pre-reset-{timestamp}"));
    fs::rename(&config.db_path, &backup).with_context(|| {
        format!(
            "moving {} to {}",
            config.db_path.display(),
            backup.display()
        )
    })?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_reset_requires_confirmation_and_preserves_the_file() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("library.sqlite3");
        fs::write(&database, b"database").unwrap();
        let config = ControlCenterConfig {
            db_path: database.clone(),
            ..Default::default()
        };

        assert!(reset_database(&config, false).is_err());
        assert!(database.exists());

        let backup = reset_database(&config, true).unwrap();
        assert!(!database.exists());
        assert_eq!(fs::read(backup).unwrap(), b"database");
    }
}
