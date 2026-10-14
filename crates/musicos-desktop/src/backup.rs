use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::api_client::ApiClient;
use crate::local::LibraryRepository;
use crate::models::{Playlist, Track};
use crate::storage::{atomic_write_json, read_json};

const BACKUP_KIND: &str = "musicos-library-backup";
const BACKUP_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistBackup {
    pub name: String,
    pub track_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryBackup {
    pub kind: String,
    pub version: u32,
    pub created_at_unix_ms: u128,
    pub settings: serde_json::Value,
    pub playlists: Vec<PlaylistBackup>,
    pub diagnostics: serde_json::Value,
    pub history: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestoreResult {
    pub restored_settings: bool,
    pub restored_playlists: usize,
    pub restored_playlist_tracks: usize,
}

pub fn collect(client: &ApiClient) -> Result<LibraryBackup> {
    let settings = client.get("/api/settings")?;
    let playlists: Vec<Playlist> = client.get("/api/playlists")?;
    let mut playlist_backups = Vec::with_capacity(playlists.len());
    for playlist in playlists {
        let tracks: Vec<Track> = client
            .get(&format!("/api/playlists/{}/tracks", playlist.id))
            .unwrap_or_default();
        playlist_backups.push(PlaylistBackup {
            name: playlist.name,
            track_ids: tracks.into_iter().map(|track| track.id).collect(),
        });
    }
    Ok(LibraryBackup {
        kind: BACKUP_KIND.to_string(),
        version: BACKUP_VERSION,
        created_at_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        settings,
        playlists: playlist_backups,
        diagnostics: client.get("/api/diagnostics").unwrap_or_default(),
        history: client
            .get("/api/history?limit=500")
            .unwrap_or_else(|_| serde_json::json!([])),
    })
}

pub fn collect_repository(
    repository: &impl LibraryRepository,
    settings: serde_json::Value,
) -> Result<LibraryBackup> {
    let playlists = repository.playlists()?;
    let mut playlist_backups = Vec::with_capacity(playlists.len());
    for playlist in playlists {
        let tracks = repository.playlist_tracks(playlist.id)?;
        playlist_backups.push(PlaylistBackup {
            name: playlist.name,
            track_ids: tracks.into_iter().map(|track| track.id).collect(),
        });
    }
    Ok(LibraryBackup {
        kind: BACKUP_KIND.to_string(),
        version: BACKUP_VERSION,
        created_at_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        settings,
        playlists: playlist_backups,
        diagnostics: serde_json::json!({"source": "desktop-library"}),
        history: serde_json::json!([]),
    })
}

pub fn save(path: &Path, backup: &LibraryBackup) -> Result<()> {
    validate(backup)?;
    atomic_write_json(path, backup)
}

pub fn load(path: &Path) -> Result<LibraryBackup> {
    let backup: LibraryBackup = read_json(path)?;
    validate(&backup)?;
    Ok(backup)
}

pub fn restore(client: &ApiClient, backup: &LibraryBackup) -> Result<RestoreResult> {
    validate(backup)?;
    let _: serde_json::Value = client.post("/api/settings", &backup.settings)?;
    let mut result = RestoreResult {
        restored_settings: true,
        ..Default::default()
    };
    for playlist in &backup.playlists {
        let response: serde_json::Value = client.post(
            "/api/playlists",
            &serde_json::json!({ "name": playlist.name }),
        )?;
        let id = response
            .pointer("/playlist/id")
            .and_then(serde_json::Value::as_i64)
            .with_context(|| format!("created playlist {} had no id", playlist.name))?;
        result.restored_playlists += 1;
        for track_id in &playlist.track_ids {
            let _: serde_json::Value = client.post(
                &format!("/api/playlists/{id}/tracks"),
                &serde_json::json!({ "track_id": track_id }),
            )?;
            result.restored_playlist_tracks += 1;
        }
    }
    Ok(result)
}

pub fn restore_repository(
    repository: &impl LibraryRepository,
    backup: &LibraryBackup,
) -> Result<RestoreResult> {
    validate(backup)?;
    let mut result = RestoreResult::default();
    for playlist in &backup.playlists {
        let created = repository.create_playlist(&playlist.name)?;
        result.restored_playlists += 1;
        for track_id in &playlist.track_ids {
            repository.add_tracks(created.id, &[*track_id])?;
            result.restored_playlist_tracks += 1;
        }
    }
    Ok(result)
}

fn validate(backup: &LibraryBackup) -> Result<()> {
    if backup.kind != BACKUP_KIND {
        bail!("not a MusicOS library backup");
    }
    if backup.version != BACKUP_VERSION {
        bail!("unsupported MusicOS backup version {}", backup.version);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_round_trips_backup_documents() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("backup.json");
        let backup = LibraryBackup {
            kind: BACKUP_KIND.to_string(),
            version: BACKUP_VERSION,
            created_at_unix_ms: 42,
            settings: serde_json::json!({"theme": "blue"}),
            playlists: vec![PlaylistBackup {
                name: "Night drive".to_string(),
                track_ids: vec![7, 9],
            }],
            diagnostics: serde_json::json!({}),
            history: serde_json::json!([]),
        };
        save(&path, &backup).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(restored.playlists[0].track_ids, vec![7, 9]);
    }

    #[test]
    fn rejects_unrelated_json_documents() {
        let backup = LibraryBackup {
            kind: "other".to_string(),
            version: BACKUP_VERSION,
            created_at_unix_ms: 0,
            settings: serde_json::json!({}),
            playlists: Vec::new(),
            diagnostics: serde_json::json!({}),
            history: serde_json::json!([]),
        };
        assert!(validate(&backup).is_err());
    }
}
