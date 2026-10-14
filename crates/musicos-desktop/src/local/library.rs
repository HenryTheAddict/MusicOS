use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result};
use musicos_init::database::{Db, LibraryTrack};
use musicos_init::media::{
    scan_library_with_progress, ScanMode, ScanProgress, ScanRequest, ScanResult,
};

use crate::api_client::ApiClient;
use crate::local::enrichment::{
    accept_metadata_review, dismiss_metadata_review, load_metadata_review_items, MetadataReviewItem,
};
use crate::local::source_audit::{
    audit_sources, cleanup_hidden_records, SourceAuditReport, SourceCleanupReport,
};
use crate::models::{LibrarySnapshot, LibrarySource, Mix, Playlist, Track};

#[derive(Debug, Clone, Default)]
pub struct LibraryQuery {
    pub search: String,
}

pub trait LibraryRepository {
    fn snapshot(&self, request: &LibraryQuery) -> Result<LibrarySnapshot>;
    fn playlists(&self) -> Result<Vec<Playlist>>;
    fn playlist_tracks(&self, id: i64) -> Result<Vec<Track>>;
    fn create_playlist(&self, name: &str) -> Result<Playlist>;
    fn rename_playlist(&self, id: i64, name: &str) -> Result<Playlist>;
    fn delete_playlist(&self, id: i64) -> Result<()>;
    fn add_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()>;
    fn remove_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()>;
    fn reorder_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct LocalLibraryRepository {
    db_path: PathBuf,
    media_dir: PathBuf,
}

impl LocalLibraryRepository {
    pub fn new(db_path: impl Into<PathBuf>, media_dir: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
            media_dir: media_dir.into(),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn media_dir(&self) -> &Path {
        &self.media_dir
    }

    pub fn ensure_ready(&self) -> Result<()> {
        std::fs::create_dir_all(&self.media_dir)?;
        let db = self.open()?;
        db.add_library_source(
            "managed",
            &self.media_dir.display().to_string(),
            "MusicOS Library",
        )?;
        Ok(())
    }

    pub fn scan(
        &self,
        mode: ScanMode,
        cancel: &AtomicBool,
        progress: impl FnMut(ScanProgress),
    ) -> Result<ScanResult> {
        self.scan_sources(mode, None, cancel, progress)
    }

    pub fn scan_sources(
        &self,
        mode: ScanMode,
        reference_source_ids: Option<Vec<i64>>,
        cancel: &AtomicBool,
        progress: impl FnMut(ScanProgress),
    ) -> Result<ScanResult> {
        scan_library_with_progress(
            &self.db_path,
            &ScanRequest {
                managed_root: self.media_dir.clone(),
                reference_source_ids,
                mode,
            },
            cancel,
            progress,
        )
    }

    pub fn add_reference_source(&self, path: &Path) -> Result<LibrarySource> {
        anyhow::ensure!(
            path.is_dir(),
            "the selected library source is not a readable folder"
        );
        let path = path
            .canonicalize()
            .with_context(|| format!("resolving library source {}", path.display()))?;
        let managed = self
            .media_dir
            .canonicalize()
            .unwrap_or_else(|_| self.media_dir.clone());
        anyhow::ensure!(
            path != managed,
            "the managed MusicOS folder is already part of the library"
        );
        anyhow::ensure!(
            path.parent().is_some() && path.components().count() > 1,
            "choose a music folder rather than an entire filesystem"
        );
        let db = self.open()?;
        anyhow::ensure!(
            !db.library_sources()?.iter().any(|source| {
                Path::new(&source.path)
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from(&source.path))
                    == path
            }),
            "this folder is already a library source"
        );
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Referenced music");
        Ok(source_from_init(db.add_library_source(
            "reference",
            &path.display().to_string(),
            name,
        )?))
    }

    pub fn update_source(&self, id: i64, enabled: bool) -> Result<LibrarySource> {
        let db = self.open()?;
        db.update_library_source(id, None, Some(enabled))?
            .map(source_from_init)
            .context("library source was not found")
    }

    pub fn remove_source(&self, id: i64) -> Result<()> {
        let db = self.open()?;
        anyhow::ensure!(
            db.remove_library_source(id)?,
            "library source was not found"
        );
        Ok(())
    }

    pub fn repair(&self) -> Result<musicos_init::database::DbRepairReport> {
        self.open()?.repair_library()
    }

    pub fn clear_cover_cache(&self) -> Result<usize> {
        self.open()?.clear_cover_art_cache_metadata()
    }

    pub fn toggle_favorite(&self, id: i64) -> Result<bool> {
        self.open()?.toggle_favorite(id)
    }

    pub fn source_audit(&self) -> Result<SourceAuditReport> {
        self.ensure_ready()?;
        audit_sources(&self.open()?, &self.media_dir)
    }

    pub fn cleanup_indexed_leftovers(&self, expected_revision: u64) -> Result<SourceCleanupReport> {
        cleanup_hidden_records(&self.db_path, &self.media_dir, expected_revision)
    }

    pub fn metadata_enrichment_counts(&self) -> Result<std::collections::BTreeMap<String, usize>> {
        self.open()?.metadata_enrichment_counts()
    }

    pub fn metadata_review_items(&self, limit: usize) -> Result<Vec<MetadataReviewItem>> {
        load_metadata_review_items(&self.db_path, limit)
    }

    pub fn accept_metadata_review(&self, track_id: i64) -> Result<bool> {
        accept_metadata_review(&self.db_path, track_id)
    }

    pub fn dismiss_metadata_review(&self, track_id: i64) -> Result<bool> {
        dismiss_metadata_review(&self.db_path, track_id)
    }

    pub(crate) fn open(&self) -> Result<Db> {
        Db::open(
            self.db_path
                .to_str()
                .context("library database path is not valid UTF-8")?,
        )
    }

    pub fn snapshot_with_audit(
        &self,
        request: &LibraryQuery,
    ) -> Result<(LibrarySnapshot, SourceAuditReport)> {
        self.ensure_ready()?;
        let db = self.open()?;
        let audit = audit_sources(&db, &self.media_dir)?;
        let trusted_source_ids = audit
            .trusted_source_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let enabled_source_ids = db
            .library_sources()?
            .into_iter()
            .filter(|source| source.enabled)
            .map(|source| source.id)
            .collect::<HashSet<_>>();
        let mut tracks = Vec::new();
        let mut offset = 0;
        loop {
            let page =
                db.query_tracks_page(&request.search, "", "", "", "title", false, 500, offset)?;
            tracks.extend(
                page.items
                    .into_iter()
                    .filter(|track| {
                        track.source_id.is_some_and(|source_id| {
                            trusted_source_ids.contains(&source_id)
                                && enabled_source_ids.contains(&source_id)
                        })
                    })
                    .map(track_from_library),
            );
            match page.next_offset {
                Some(next) => offset = next,
                None => break,
            }
        }
        let playlists: Vec<Playlist> = db
            .get_playlists()?
            .into_iter()
            .map(|playlist| {
                let track_count = db.playlist_track_count(playlist.id).unwrap_or_default() as usize;
                Playlist {
                    id: playlist.id,
                    name: playlist.name,
                    track_count,
                }
            })
            .collect();
        let mixes = db
            .dynamic_mixes()?
            .into_iter()
            .map(|mix| Mix {
                id: mix.id,
                name: mix.name,
                kind: mix.kind,
                track_count: mix.track_count,
            })
            .collect();
        let sources = db
            .library_sources()?
            .into_iter()
            .map(source_from_init)
            .collect();
        let album_count = tracks
            .iter()
            .map(|track| {
                (
                    track.album.to_ascii_lowercase(),
                    track.artist.to_ascii_lowercase(),
                )
            })
            .collect::<HashSet<_>>()
            .len() as i64;
        let artist_count = tracks
            .iter()
            .map(|track| track.artist.to_ascii_lowercase())
            .collect::<HashSet<_>>()
            .len() as i64;
        let favorite_count = tracks.iter().filter(|track| track.favorited).count() as i64;
        let track_count = tracks.len() as i64;
        let playlist_count = playlists.len() as i64;
        let snapshot = LibrarySnapshot {
            tracks,
            playlists,
            mixes,
            sources,
            counts: [
                ("tracks".to_string(), track_count),
                ("albums".to_string(), album_count),
                ("artists".to_string(), artist_count),
                ("playlists".to_string(), playlist_count),
                ("favorites".to_string(), favorite_count),
            ]
            .into_iter()
            .collect(),
        };
        Ok((snapshot, audit))
    }
}

impl LibraryRepository for LocalLibraryRepository {
    fn snapshot(&self, request: &LibraryQuery) -> Result<LibrarySnapshot> {
        Ok(self.snapshot_with_audit(request)?.0)
    }

    fn playlists(&self) -> Result<Vec<Playlist>> {
        Ok(self.snapshot(&LibraryQuery::default())?.playlists)
    }

    fn playlist_tracks(&self, id: i64) -> Result<Vec<Track>> {
        let db = self.open()?;
        let visible = self
            .snapshot(&LibraryQuery::default())?
            .tracks
            .into_iter()
            .map(|track| track.id)
            .collect::<HashSet<_>>();
        Ok(db
            .get_playlist_tracks(id)?
            .into_iter()
            .filter(|track| visible.contains(&track.id))
            .map(track_from_init)
            .collect())
    }

    fn create_playlist(&self, name: &str) -> Result<Playlist> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "playlist name is required");
        let db = self.open()?;
        let id = db.create_playlist(name)?;
        Ok(Playlist {
            id,
            name: name.to_string(),
            track_count: 0,
        })
    }

    fn rename_playlist(&self, id: i64, name: &str) -> Result<Playlist> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "playlist name is required");
        let db = self.open()?;
        db.rename_playlist(id, name)?;
        Ok(Playlist {
            id,
            name: name.to_string(),
            track_count: db.playlist_track_count(id).unwrap_or_default() as usize,
        })
    }

    fn delete_playlist(&self, id: i64) -> Result<()> {
        self.open()?.delete_playlist(id)
    }

    fn add_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()> {
        self.open()?.add_tracks_to_playlist(id, track_ids)?;
        Ok(())
    }

    fn remove_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()> {
        self.open()?.remove_tracks_from_playlist(id, track_ids)?;
        Ok(())
    }

    fn reorder_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()> {
        self.open()?.reorder_playlist_tracks(id, track_ids)
    }
}

#[derive(Clone)]
pub struct RemoteLibraryRepository {
    client: ApiClient,
}

impl RemoteLibraryRepository {
    pub fn new(client: ApiClient) -> Self {
        Self { client }
    }
}

impl LibraryRepository for RemoteLibraryRepository {
    fn snapshot(&self, _request: &LibraryQuery) -> Result<LibrarySnapshot> {
        self.client.library_snapshot()
    }

    fn playlists(&self) -> Result<Vec<Playlist>> {
        self.client.get("/api/playlists")
    }

    fn playlist_tracks(&self, id: i64) -> Result<Vec<Track>> {
        self.client.get(&format!("/api/playlists/{id}/tracks"))
    }

    fn create_playlist(&self, name: &str) -> Result<Playlist> {
        let value: serde_json::Value = self
            .client
            .post("/api/playlists", &serde_json::json!({ "name": name }))?;
        serde_json::from_value(
            value
                .get("playlist")
                .cloned()
                .context("created playlist response was missing playlist")?,
        )
        .context("decoding created playlist")
    }

    fn rename_playlist(&self, id: i64, name: &str) -> Result<Playlist> {
        let value: serde_json::Value = self.client.patch(
            &format!("/api/playlists/{id}"),
            &serde_json::json!({ "name": name }),
        )?;
        serde_json::from_value(
            value
                .get("playlist")
                .cloned()
                .context("renamed playlist response was missing playlist")?,
        )
        .context("decoding renamed playlist")
    }

    fn delete_playlist(&self, id: i64) -> Result<()> {
        let _: serde_json::Value = self.client.delete(&format!("/api/playlists/{id}"))?;
        Ok(())
    }

    fn add_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()> {
        for track_id in track_ids {
            let _: serde_json::Value = self.client.post(
                &format!("/api/playlists/{id}/tracks"),
                &serde_json::json!({ "track_id": track_id }),
            )?;
        }
        Ok(())
    }

    fn remove_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()> {
        for track_id in track_ids {
            let _: serde_json::Value = self
                .client
                .delete(&format!("/api/playlists/{id}/tracks/{track_id}"))?;
        }
        Ok(())
    }

    fn reorder_tracks(&self, id: i64, track_ids: &[i64]) -> Result<()> {
        let _: serde_json::Value = self.client.put(
            &format!("/api/playlists/{id}/tracks/reorder"),
            &serde_json::json!({ "track_ids": track_ids }),
        )?;
        Ok(())
    }
}

pub fn track_from_library(value: LibraryTrack) -> Track {
    let path = value.track.path;
    let available = value.source_available && Path::new(&path).is_file();
    Track {
        id: value.track.id,
        path,
        title: value.track.title,
        artist: value.track.artist,
        album: value.track.album,
        track_number: value.track.track_number,
        disc_number: value.disc_number,
        duration: value.track.duration,
        indexed_at: value.track.indexed_at,
        favorited: value.track.favorited,
        play_count: value.track.play_count,
        rating: value.track.rating,
        genre: value.genre,
        year: value.year,
        stable_id: value.stable_id,
        content_hash: value.content_hash,
        cover_art_path: value.track.cover_art_path.map(PathBuf::from),
        available,
        ..Default::default()
    }
}

pub fn track_from_init(value: musicos_init::database::Track) -> Track {
    Track {
        id: value.id,
        path: value.path,
        title: value.title,
        artist: value.artist,
        album: value.album,
        track_number: value.track_number,
        disc_number: 0,
        duration: value.duration,
        indexed_at: value.indexed_at,
        favorited: value.favorited,
        play_count: value.play_count,
        rating: value.rating,
        genre: String::new(),
        year: 0,
        stable_id: String::new(),
        content_hash: None,
        cover_art_path: value.cover_art_path.map(PathBuf::from),
        available: true,
        ..Default::default()
    }
}

fn source_from_init(value: musicos_init::database::LibrarySource) -> LibrarySource {
    LibrarySource {
        id: value.id,
        kind: value.kind,
        path: PathBuf::from(value.path),
        display_name: value.display_name,
        enabled: value.enabled,
        available: value.available,
        last_scan_at: value.last_scan_at,
        last_error: value.last_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_repository_reads_and_edits_sqlite_without_http() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("library.sqlite3");
        let media_dir = temp.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();
        let track_path = media_dir.join("Local Artist - Local Song.mp3");
        std::fs::write(&track_path, b"fixture").unwrap();

        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let track_id = db
            .insert_track(
                track_path.to_str().unwrap(),
                "Local Song",
                "Local Artist",
                "Local Album",
                183,
                1,
                7,
            )
            .unwrap();
        drop(db);

        let repository = LocalLibraryRepository::new(&db_path, &media_dir);
        repository.ensure_ready().unwrap();
        let snapshot = repository.snapshot(&LibraryQuery::default()).unwrap();
        assert_eq!(snapshot.tracks.len(), 1);
        assert_eq!(snapshot.tracks[0].title, "Local Song");

        let playlist = repository.create_playlist("Offline favorites").unwrap();
        repository.add_tracks(playlist.id, &[track_id]).unwrap();
        assert_eq!(repository.playlist_tracks(playlist.id).unwrap().len(), 1);
        repository
            .rename_playlist(playlist.id, "Local favorites")
            .unwrap();
        repository.remove_tracks(playlist.id, &[track_id]).unwrap();
        assert!(repository.playlist_tracks(playlist.id).unwrap().is_empty());
        repository.delete_playlist(playlist.id).unwrap();
    }
}
