pub use crate::database_access::Connection;
use anyhow::Result;
use rusqlite::{params, params_from_iter, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: i32,
    pub duration: i32, // in seconds
    pub file_size: i64,
    pub modified_at: i64,
    pub indexed_at: i64,
    pub cover_art_path: Option<String>,
    pub cover_art_updated_at: Option<i64>,
    pub cover_art_source_modified_at: Option<i64>,
    pub favorited: bool,
    pub play_count: i32,
    pub last_played: Option<i64>,
    pub rating: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrack {
    #[serde(flatten)]
    pub track: Track,
    pub stable_id: String,
    pub content_hash: Option<String>,
    pub genre: String,
    pub year: i32,
    pub disc_number: i32,
    pub revision: i64,
    pub source_id: Option<i64>,
    pub source_name: Option<String>,
    pub source_kind: Option<String>,
    pub source_available: bool,
    pub metadata_overridden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrarySource {
    pub id: i64,
    pub kind: String,
    pub path: String,
    pub display_name: String,
    pub enabled: bool,
    pub available: bool,
    pub last_scan_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSourceRecord {
    pub id: i64,
    pub path: String,
    pub source_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPage {
    pub items: Vec<LibraryTrack>,
    pub total: i64,
    pub limit: usize,
    pub offset: usize,
    pub next_offset: Option<usize>,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub name: String,
    pub artist: String,
    pub genre: String,
    pub year: i32,
    pub track_count: i64,
    pub duration: i64,
    pub artwork_track_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub name: String,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistManifest {
    pub id: i64,
    pub stable_id: String,
    pub name: String,
    pub revision: i64,
    pub updated_at: i64,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTombstone {
    pub stable_id: String,
    pub deleted_at: i64,
    pub revision: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub artwork_path: Option<String>,
    pub album_artist: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataEnrichmentStatus {
    pub track_id: i64,
    pub state: String,
    pub provider: String,
    pub confidence: f32,
    pub candidate_json: Option<String>,
    pub recording_mbid: Option<String>,
    pub release_mbid: Option<String>,
    pub release_group_mbid: Option<String>,
    pub error: Option<String>,
    pub checked_at: i64,
    pub source_modified_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixDescriptor {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub track_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbRepairReport {
    pub integrity_ok: bool,
    pub integrity_message: String,
    pub missing_tracks_removed: usize,
    pub orphan_playlist_rows_removed: usize,
    pub playlist_positions_repaired: usize,
    pub wal_checkpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: i64,
    pub pin_type: String, // "track" or "playlist"
    pub target_id: i64,
    pub name: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LibraryCounts {
    pub tracks: i64,
    pub albums: i64,
    pub artists: i64,
    pub playlists: i64,
    pub pins: i64,
    pub favorites: i64,
}

pub struct Db {
    conn: Connection,
    path: PathBuf,
}

impl Db {
    pub fn open(path:&str)->Result<Self>{Self::open_priority(path,false)}
    pub fn open_background(path:&str)->Result<Self>{Self::open_priority(path,true)}
    fn open_priority(path: &str,background:bool) -> Result<Self> {
        let path_buf = PathBuf::from(path);
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = if background{Connection::open_background(&path_buf)?}else{Connection::open(&path_buf)?};
        #[cfg(all(target_os = "linux", feature = "pid1"))]
        conn.pragma_update(None, "synchronous", "FULL")?;
        Self::backup_before_v2_migration(&conn, &path_buf)?;
        let db = Self {
            conn,
            path: path_buf,
        };
        crate::database_access::initialize_schema(&db.path,"core-v3",||{db.apply_pragmas()?;db.init_schema()})?;
        Ok(db)
    }

    fn backup_before_v2_migration(conn: &Connection, path: &Path) -> Result<()> {
        if !path.is_file() {
            return Ok(());
        }
        let has_tracks: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tracks')",
            [],
            |row| row.get(0),
        )?;
        if !has_tracks {
            return Ok(());
        }
        let mut columns = conn.prepare("PRAGMA table_info(tracks)")?;
        let rows = columns.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_stable_id = false;
        for row in rows {
            if row? == "stable_id" {
                has_stable_id = true;
                break;
            }
        }
        if has_stable_id {
            return Ok(());
        }
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL);");
        let backup = path.with_extension("pre-v2-migration.sqlite3.bak");
        if !backup.exists() {
            std::fs::copy(path, backup)?;
        }
        Ok(())
    }

    fn apply_pragmas(&self) -> Result<()> {
        self.conn.busy_timeout(Duration::from_secs(5))?;
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(())
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album TEXT NOT NULL,
                track_number INTEGER NOT NULL DEFAULT 0,
                duration INTEGER NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                modified_at INTEGER NOT NULL DEFAULT 0,
                indexed_at INTEGER NOT NULL DEFAULT 0,
                favorited INTEGER DEFAULT 0,
                play_count INTEGER DEFAULT 0,
                last_played INTEGER,
                rating INTEGER NOT NULL DEFAULT 0 CHECK(rating BETWEEN 0 AND 5)
            );
            CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL
            );
            CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id INTEGER,
                track_id INTEGER,
                position INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (playlist_id, track_id),
                FOREIGN KEY (playlist_id) REFERENCES playlists (id) ON DELETE CASCADE,
                FOREIGN KEY (track_id) REFERENCES tracks (id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS pins (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pin_type TEXT NOT NULL,
                target_id INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id INTEGER NOT NULL,
                played_at INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES tracks (id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS playlist_tombstones (
                stable_id TEXT PRIMARY KEY,
                deleted_at INTEGER NOT NULL,
                revision INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS library_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                path TEXT UNIQUE NOT NULL,
                display_name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                available INTEGER NOT NULL DEFAULT 1,
                last_scan_at INTEGER,
                last_error TEXT
            );
            CREATE TABLE IF NOT EXISTS track_metadata_overrides (
                track_id INTEGER PRIMARY KEY,
                title TEXT,
                artist TEXT,
                album TEXT,
                track_number INTEGER,
                disc_number INTEGER,
                genre TEXT,
                year INTEGER,
                artwork_path TEXT,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES tracks (id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS metadata_enrichment_status (
                track_id INTEGER PRIMARY KEY,
                state TEXT NOT NULL,
                provider TEXT NOT NULL DEFAULT '',
                confidence REAL NOT NULL DEFAULT 0,
                candidate_json TEXT,
                recording_mbid TEXT,
                release_mbid TEXT,
                release_group_mbid TEXT,
                error TEXT,
                checked_at INTEGER NOT NULL,
                source_modified_at INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (track_id) REFERENCES tracks (id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS metadata_ignored_candidates (
                track_id INTEGER NOT NULL,fingerprint TEXT NOT NULL,candidate_json TEXT NOT NULL,
                ignored_at INTEGER NOT NULL,PRIMARY KEY(track_id,fingerprint),
                FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS metadata_provider_cache (
                cache_key TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                response_json TEXT NOT NULL,
                fetched_at INTEGER NOT NULL
            );",
        )?;
        self.ensure_column("tracks", "file_size", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "modified_at", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "indexed_at", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "track_number", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "cover_art_path", "TEXT")?;
        self.ensure_column("tracks", "cover_art_updated_at", "INTEGER")?;
        self.ensure_column("tracks", "cover_art_source_modified_at", "INTEGER")?;
        self.ensure_column("tracks", "rating", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "stable_id", "TEXT")?;
        self.ensure_column("tracks", "content_hash", "TEXT")?;
        self.ensure_column("tracks", "genre", "TEXT NOT NULL DEFAULT ''")?;
        self.ensure_column("tracks", "year", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "disc_number", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "revision", "INTEGER NOT NULL DEFAULT 1")?;
        self.ensure_column("tracks", "deleted_at", "INTEGER")?;
        self.ensure_column("tracks", "source_id", "INTEGER")?;
        self.ensure_column(
            "track_metadata_overrides",
            "origin",
            "TEXT NOT NULL DEFAULT 'user'",
        )?;
        self.ensure_column("track_metadata_overrides", "provider", "TEXT")?;
        self.ensure_column("track_metadata_overrides", "album_artist", "TEXT")?;
        self.ensure_column("track_metadata_overrides", "confidence", "REAL")?;
        self.ensure_column("playlists", "stable_id", "TEXT")?;
        self.ensure_column("playlists", "revision", "INTEGER NOT NULL DEFAULT 1")?;
        self.ensure_column("playlists", "updated_at", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("playlist_tracks", "position", "INTEGER NOT NULL DEFAULT 0")?;
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_album_title ON tracks(album COLLATE NOCASE, track_number, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_artist_title ON tracks(artist COLLATE NOCASE, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_favorited ON tracks(favorited, title COLLATE NOCASE);
             CREATE UNIQUE INDEX IF NOT EXISTS idx_tracks_stable_id ON tracks(stable_id) WHERE stable_id IS NOT NULL;
             CREATE INDEX IF NOT EXISTS idx_tracks_content_hash ON tracks(content_hash) WHERE content_hash IS NOT NULL;
             CREATE INDEX IF NOT EXISTS idx_tracks_genre_title ON tracks(genre COLLATE NOCASE, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_source ON tracks(source_id, deleted_at);
             CREATE INDEX IF NOT EXISTS idx_tracks_live_title ON tracks(title COLLATE NOCASE) WHERE deleted_at IS NULL;
             CREATE INDEX IF NOT EXISTS idx_tracks_live_revision ON tracks(revision DESC) WHERE deleted_at IS NULL;
             CREATE INDEX IF NOT EXISTS idx_library_sources_enabled ON library_sources(enabled, kind);
             CREATE UNIQUE INDEX IF NOT EXISTS idx_playlists_stable_id ON playlists(stable_id) WHERE stable_id IS NOT NULL;
             CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist ON playlist_tracks(playlist_id, position, track_id);
             CREATE INDEX IF NOT EXISTS idx_playlist_tracks_track ON playlist_tracks(track_id);
             CREATE INDEX IF NOT EXISTS idx_pins_lookup ON pins(pin_type, target_id);
             CREATE INDEX IF NOT EXISTS idx_play_history_recent ON play_history(played_at DESC, track_id);",
        )?;
        self.backfill_stable_ids()?;
        self.ensure_default_library_source()?;
        Ok(())
    }

    fn ensure_default_library_source(&self) -> Result<()> {
        let path = std::env::var("MUSICOS_UPLOAD_DIR").unwrap_or_else(|_| "./music".to_string());
        let normalized = normalize_source_path(&path);
        let source_id = self
            .conn
            .query_row(
                "SELECT id FROM library_sources WHERE path = ?1",
                params![normalized],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let source_id = if let Some(id) = source_id {
            id
        } else {
            self.conn.execute(
                "INSERT INTO library_sources (kind, path, display_name, enabled, available)
                 VALUES ('managed', ?1, 'MusicOS Library', 1, ?2)",
                params![normalized, i32::from(Path::new(&path).exists())],
            )?;
            self.conn.last_insert_rowid()
        };
        let needs_assignment: bool = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tracks
                WHERE source_id IS NULL AND (path = ?1 OR path LIKE ?2)
             )",
            params![
                normalized,
                format!("{normalized}{}%", std::path::MAIN_SEPARATOR)
            ],
            |row| row.get(0),
        )?;
        if needs_assignment {
            self.conn.execute(
                "UPDATE tracks SET source_id = ?1
                 WHERE source_id IS NULL AND (path = ?2 OR path LIKE ?3)",
                params![
                    source_id,
                    normalized,
                    format!("{normalized}{}%", std::path::MAIN_SEPARATOR)
                ],
            )?;
        }
        Ok(())
    }

    fn backfill_stable_ids(&self) -> Result<()> {
        let mut tracks = self
            .conn
            .prepare("SELECT id, path FROM tracks WHERE stable_id IS NULL OR stable_id = ''")?;
        let rows = tracks.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut pending_tracks = Vec::new();
        for row in rows {
            pending_tracks.push(row?);
        }
        drop(tracks);
        for (id, path) in pending_tracks {
            let stable_id = stable_id("track", &format!("{id}:{path}"));
            self.conn.execute(
                "UPDATE tracks SET stable_id = ?1 WHERE id = ?2",
                params![stable_id, id],
            )?;
        }

        let mut playlists = self
            .conn
            .prepare("SELECT id, name FROM playlists WHERE stable_id IS NULL OR stable_id = ''")?;
        let rows = playlists.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut pending_playlists = Vec::new();
        for row in rows {
            pending_playlists.push(row?);
        }
        drop(playlists);
        let now = unix_seconds_i64();
        for (id, name) in pending_playlists {
            let stable_id = stable_id("playlist", &format!("{id}:{name}"));
            self.conn.execute(
                "UPDATE playlists SET stable_id = ?1, updated_at = CASE WHEN updated_at = 0 THEN ?2 ELSE updated_at END WHERE id = ?3",
                params![stable_id, now, id],
            )?;
        }
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, definition: &str) -> Result<()> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for existing in columns {
            if existing? == column {
                return Ok(());
            }
        }
        self.conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
        Ok(())
    }

    pub fn cover_art_dir(&self) -> PathBuf {
        self.path
            .parent()
            .map(|parent| parent.join("cover-art"))
            .unwrap_or_else(|| PathBuf::from("cover-art"))
    }

    pub fn set_track_cover_art(
        &self,
        id: i64,
        cover_art_path: Option<&str>,
        source_modified_at: i64,
    ) -> Result<()> {
        let updated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        self.conn.execute(
            "UPDATE tracks
             SET cover_art_path = ?1,
                 cover_art_updated_at = ?2,
                 cover_art_source_modified_at = ?3
             WHERE id = ?4",
            params![cover_art_path, updated_at, source_modified_at, id],
        )?;
        Ok(())
    }

    pub fn cover_art_cache_count(&self) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE cover_art_path IS NOT NULL AND cover_art_path != ''",
            [],
            |row| row.get(0),
        )?)
    }

    pub fn clear_cover_art_cache_metadata(&self) -> Result<usize> {
        let count = self.cover_art_cache_count()? as usize;
        self.conn.execute(
            "UPDATE tracks
             SET cover_art_path = NULL,
                 cover_art_updated_at = NULL,
                 cover_art_source_modified_at = NULL",
            [],
        )?;
        Ok(count)
    }

    pub fn clear_cover_art_path(&self, path: &Path) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE tracks
             SET cover_art_path = NULL,
                 cover_art_updated_at = NULL,
                 cover_art_source_modified_at = NULL
             WHERE cover_art_path = ?1",
            params![path.display().to_string()],
        )?)
    }

    pub fn library_sources(&self) -> Result<Vec<LibrarySource>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, path, display_name, enabled, available, last_scan_at, last_error
             FROM library_sources ORDER BY CASE kind WHEN 'managed' THEN 0 ELSE 1 END, display_name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], source_from_row)?;
        let mut sources = Vec::new();
        for row in rows {
            sources.push(row?);
        }
        Ok(sources)
    }

    pub fn track_source_records(&self) -> Result<Vec<TrackSourceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, source_id
             FROM tracks
             WHERE deleted_at IS NULL
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TrackSourceRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                source_id: row.get(2)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn library_source(&self, id: i64) -> Result<Option<LibrarySource>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, kind, path, display_name, enabled, available, last_scan_at, last_error
             FROM library_sources WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        Ok(rows.next()?.map(source_from_row).transpose()?)
    }

    pub fn add_library_source(
        &self,
        kind: &str,
        source_path: &str,
        display_name: &str,
    ) -> Result<LibrarySource> {
        let kind = if kind.eq_ignore_ascii_case("managed") {
            "managed"
        } else {
            "reference"
        };
        let path = normalize_source_path(source_path);
        anyhow::ensure!(!path.trim().is_empty(), "source path is required");
        let fallback_name = Path::new(&path)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Music folder");
        let name = if display_name.trim().is_empty() {
            fallback_name
        } else {
            display_name.trim()
        };
        self.conn.execute(
            "INSERT INTO library_sources (kind, path, display_name, enabled, available)
             VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT(path) DO UPDATE SET
                display_name = excluded.display_name,
                enabled = 1,
                available = excluded.available,
                last_error = NULL",
            params![kind, path, name, i32::from(Path::new(&path).is_dir())],
        )?;
        let id: i64 = self.conn.query_row(
            "SELECT id FROM library_sources WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        self.library_source(id)?
            .ok_or_else(|| anyhow::anyhow!("library source could not be created"))
    }

    pub fn update_library_source(
        &self,
        id: i64,
        display_name: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<Option<LibrarySource>> {
        if enabled==Some(false){anyhow::ensure!(self.library_source(id)?.is_none_or(|s|s.kind!="managed"),"The managed MusicOS library cannot be disabled");}
        if let Some(name) = display_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            self.conn.execute(
                "UPDATE library_sources SET display_name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
        }
        if let Some(enabled) = enabled {
            self.conn.execute(
                "UPDATE library_sources SET enabled = ?1 WHERE id = ?2",
                params![i32::from(enabled), id],
            )?;
        }
        self.library_source(id)
    }

    pub fn mark_library_source(&self, id: i64, available: bool, error: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE library_sources
             SET available = ?1, last_scan_at = ?2, last_error = ?3
             WHERE id = ?4",
            params![i32::from(available), unix_seconds_i64(), error, id],
        )?;
        Ok(())
    }

    pub fn remove_library_source(&self, id: i64) -> Result<bool> {
        let Some(source) = self.library_source(id)? else {
            return Ok(false);
        };
        anyhow::ensure!(
            source.kind != "managed",
            "the managed MusicOS library cannot be removed"
        );
        self.update_library_source(id,None,Some(false))?;
        Ok(true)
    }

    /// Relocation verifies bytes off the playback thread and preserves track identities.
    pub fn library_source_track_count(&self,id:i64)->Result<usize>{Ok(self.conn.query_row("SELECT count(*) FROM tracks WHERE source_id=? AND deleted_at IS NULL",[id],|r|r.get::<_,i64>(0))? as usize)}
    pub fn relocate_library_source(&self,id:i64,target:&Path)->Result<(usize,usize)>{
        use std::io::Read;
        let source=self.library_source(id)?.ok_or_else(||anyhow::anyhow!("Folder no longer exists"))?;
        anyhow::ensure!(source.kind!="managed","The managed MusicOS library cannot be relocated");
        let root=std::fs::canonicalize(target)?;anyhow::ensure!(root.is_dir(),"Choose a music folder");
        let hash=|path:&Path|->Result<String>{let mut file=std::fs::File::open(path)?;let mut digest=Sha256::new();let mut buffer=[0_u8;65536];loop{let n=file.read(&mut buffer)?;if n==0{break;}digest.update(&buffer[..n]);}Ok(format!("{:x}",digest.finalize()))};
        let mut q=self.conn.prepare("SELECT id,path,content_hash,file_size FROM tracks WHERE source_id=?")?;
        let records=q.query_map([id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,i64>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;drop(q);
        let mut matches=Vec::new();for (track,old,stored,size) in &records{
            let Ok(relative)=Path::new(old).strip_prefix(&source.path) else {continue};let path=root.join(relative);
            if relative.components().any(|part|matches!(part,std::path::Component::ParentDir)) {continue;}
            let Ok(path)=std::fs::canonicalize(path)else{continue};if !path.starts_with(&root){continue;}
            let Ok(meta)=std::fs::metadata(&path)else{continue};if !meta.is_file()||meta.len()as i64!=*size {continue;}
            let Some(expected)=stored.clone().or_else(||hash(Path::new(old)).ok())else{continue};
            if hash(&path)?!=expected {continue;}
            let collision:bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM tracks WHERE path=? AND id!=?)",params![path.to_string_lossy(),track],|r|r.get(0))?;if collision{continue;}
            matches.push((*track,path.to_string_lossy().into_owned(),meta.modified()?.duration_since(UNIX_EPOCH)?.as_secs()as i64));
        }
        anyhow::ensure!(records.is_empty()||!matches.is_empty(),"No matching files were verified. Your folder and songs are unchanged.");
        let tx=self.conn.unchecked_transaction()?;for (track,path,modified) in &matches{tx.execute("UPDATE tracks SET path=?,modified_at=?,revision=revision+1 WHERE id=?",params![path,modified,track])?;}
        tx.execute("UPDATE library_sources SET path=?,available=1,last_error=NULL WHERE id=?",params![root.to_string_lossy(),id])?;tx.commit()?;
        Ok((matches.len(),records.len()-matches.len()))
    }

    pub fn assign_track_source(&self, track_id: i64, source_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET source_id = ?1 WHERE id = ?2",
            params![source_id, track_id],
        )?;
        Ok(())
    }

    pub fn assign_track_source_by_path(&self, path: &str, source_id: i64) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE tracks SET source_id = ?1 WHERE path = ?2 AND source_id IS NOT ?1",
            params![source_id, path],
        )?)
    }

    pub fn playlist_reference_count_for_tracks(&self, track_ids: &[i64]) -> Result<usize> {
        let mut total = 0usize;
        for chunk in track_ids.chunks(400) {
            if chunk.is_empty() {
                continue;
            }
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(",");
            total += self.conn.query_row(
                &format!("SELECT COUNT(*) FROM playlist_tracks WHERE track_id IN ({placeholders})"),
                params_from_iter(chunk.iter()),
                |row| row.get::<_, i64>(0),
            )? as usize;
        }
        Ok(total)
    }

    pub fn backup_to(&self, destination: &Path) -> Result<()> {
        anyhow::ensure!(
            !destination.exists(),
            "backup destination already exists: {}",
            destination.display()
        );
        self.conn
            .execute("VACUUM INTO ?1", params![destination.display().to_string()])?;
        Ok(())
    }

    pub fn delete_track_records(&self, track_ids: &[i64]) -> Result<usize> {
        if track_ids.is_empty() {
            return Ok(0);
        }
        self.begin_transaction()?;
        let result = (|| {
            let mut removed = 0usize;
            for chunk in track_ids.chunks(400) {
                let placeholders = std::iter::repeat_n("?", chunk.len())
                    .collect::<Vec<_>>()
                    .join(",");
                removed += self.conn.execute(
                    &format!("DELETE FROM tracks WHERE id IN ({placeholders})"),
                    params_from_iter(chunk.iter()),
                )?;
            }
            Ok(removed)
        })();
        match result {
            Ok(removed) => {
                self.commit_transaction()?;
                Ok(removed)
            }
            Err(error) => {
                let _ = self.rollback_transaction();
                Err(error)
            }
        }
    }

    pub fn remove_missing_tracks_for_source(
        &self,
        source_id: i64,
        existing_paths: &[String],
    ) -> Result<usize> {
        let existing = existing_paths
            .iter()
            .collect::<std::collections::HashSet<_>>();
        let mut stmt = self
            .conn
            .prepare("SELECT id, path FROM tracks WHERE source_id = ?1 AND deleted_at IS NULL")?;
        let rows = stmt.query_map(params![source_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut missing = Vec::new();
        for row in rows {
            let (id, path) = row?;
            if !existing.contains(&path) {
                missing.push(id);
            }
        }
        drop(stmt);
        let mut removed = 0;
        for id in missing {
            removed += self
                .conn
                .execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        }
        Ok(removed)
    }

    pub fn track_source_kind(&self, track_id: i64) -> Result<String> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(s.kind, 'managed')
             FROM tracks t LEFT JOIN library_sources s ON s.id = t.source_id
             WHERE t.id = ?1",
            params![track_id],
            |row| row.get(0),
        )?)
    }

    pub fn update_track_metadata_override(
        &self,
        ids: &[i64],
        update: &MetadataUpdate,
    ) -> Result<usize> {
        self.update_metadata_recorded(ids, update, None)
    }

    pub fn update_metadata_review(&self, id:i64, update:&MetadataUpdate, revision:i64, candidate:&str) -> Result<usize> {
        self.update_metadata_recorded(&[id], update, Some((revision,candidate)))
    }

    fn update_metadata_recorded(&self,ids:&[i64],update:&MetadataUpdate,review:Option<(i64,&str)>)->Result<usize>{
        let changes=ids.iter().map(|id|(*id,update.clone())).collect::<Vec<_>>();let reviews=review.map(|(revision,candidate)|vec![(ids[0],revision,candidate.to_owned())]).unwrap_or_default();
        self.update_metadata_batch(&changes,&reviews).map(|(count,_)|count)
    }
    pub fn reserve_writer(&self)->rusqlite::Result<crate::database_access::Permit>{self.conn.reserve_writer()}
    pub fn update_metadata_batch(&self,changes:&[(i64,MetadataUpdate)],reviews:&[(i64,i64,String)])->Result<(usize,i64)>{
        self.update_metadata_batch_with_file_job(changes,reviews,None)
    }
    pub fn update_metadata_batch_with_file_job(&self,changes:&[(i64,MetadataUpdate)],reviews:&[(i64,i64,String)],file_job:Option<&str>)->Result<(usize,i64)>{
        anyhow::ensure!(!changes.is_empty(), "at least one track is required");
        let now = unix_seconds_i64();
        self.ensure_metadata_history()?;
        self.begin_transaction()?;
        let result = (|| {
            for (id,revision,candidate) in reviews {
                if candidate.is_empty(){let actual:i64=self.conn.query_row("SELECT revision FROM tracks WHERE id=?",[id],|r|r.get(0))?;anyhow::ensure!(actual==*revision,"A selected song changed. Review again.");continue;}
                let actual:(i64,String,String)=self.conn.query_row("SELECT t.revision,s.candidate_json,s.state FROM tracks t JOIN metadata_enrichment_status s ON s.track_id=t.id WHERE t.id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
                anyhow::ensure!(actual==(*revision,candidate.to_owned(),"review".into()),"This suggestion changed. Refresh the comparison before applying it.");
            }
            self.conn.execute("INSERT INTO desktop_metadata_changes_v1(created_at,undone,canonical_json) VALUES (?,0,?)",params![now,self.canonical_history_snapshot()?])?;
            let batch=self.conn.last_insert_rowid();
            if let Some(job)=file_job{self.conn.execute("UPDATE desktop_metadata_changes_v1 SET file_job=? WHERE id=?",params![job,batch])?;}
            let mut changed = 0;
            for (id,update) in changes {
                let before=self.metadata_history_snapshot(*id)?;
                changed += self.conn.execute(
                    "INSERT INTO track_metadata_overrides (
                        track_id, title, artist, album, track_number, disc_number, genre, year, artwork_path, updated_at, origin, album_artist
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'user', ?11)
                     ON CONFLICT(track_id) DO UPDATE SET
                        title = COALESCE(excluded.title, title),
                        artist = COALESCE(excluded.artist, artist),
                        album = COALESCE(excluded.album, album),
                        track_number = COALESCE(excluded.track_number, track_number),
                        disc_number = COALESCE(excluded.disc_number, disc_number),
                        genre = COALESCE(excluded.genre, genre),
                        year = COALESCE(excluded.year, year),
                        artwork_path = COALESCE(excluded.artwork_path, artwork_path),
                        album_artist = COALESCE(excluded.album_artist, album_artist),
                        updated_at = excluded.updated_at,
                        origin = 'user',
                        provider = NULL,
                        confidence = NULL",
                    params![
                        id,
                        clean_text(update.title.as_deref()),
                        clean_text(update.artist.as_deref()),
                        clean_text(update.album.as_deref()),
                        update.track_number.map(|value| value.max(0)),
                        update.disc_number.map(|value| value.max(0)),
                        update.genre.as_deref().map(str::trim),
                        update.year.map(|value| value.clamp(0, 9999)),
                        clean_text(update.artwork_path.as_deref()),
                        now,
                        update.album_artist.as_deref().map(str::trim)
                    ],
                )?;
                self.conn.execute(
                    "UPDATE tracks SET revision = revision + 1 WHERE id = ?1",
                    params![id],
                )?;
                let after=self.metadata_history_snapshot(*id)?;
                self.conn.execute("INSERT INTO desktop_metadata_change_entries_v1(change_id,track_id,before_json,after_json) VALUES (?,?,?,?)",params![batch,id,before,after])?;
            }
            for (id,_,candidate) in reviews {if candidate.is_empty(){continue;}self.conn.execute("UPDATE metadata_enrichment_status SET state='applied' WHERE track_id=?",[id])?;}
            Ok((changed,batch))
        })();
        match result {
            Ok((changed,batch)) => {
                self.commit_transaction()?;
                Ok((changed,batch))
            }
            Err(err) => {
                let _ = self.rollback_transaction();
                Err(err)
            }
        }
    }

    pub fn metadata_override_origin(&self, track_id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT origin FROM track_metadata_overrides WHERE track_id = ?1",
                params![track_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Applies a provider match without replacing any track that has a user
    /// authored override. Automatic matches may refresh earlier automatic
    /// values as providers improve their catalogue.
    pub fn apply_automatic_metadata_override(
        &self,
        track_id: i64,
        update: &MetadataUpdate,
        provider: &str,
        confidence: f32,
    ) -> Result<bool> {
        if self.metadata_override_origin(track_id)?.as_deref() == Some("user") {
            return Ok(false);
        }
        let changed = self.conn.execute(
            "INSERT INTO track_metadata_overrides (
                track_id, title, artist, album, track_number, disc_number, genre, year,
                artwork_path, updated_at, origin, provider, confidence
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'automatic', ?11, ?12)
             ON CONFLICT(track_id) DO UPDATE SET
                title = COALESCE(excluded.title, title),
                artist = COALESCE(excluded.artist, artist),
                album = COALESCE(excluded.album, album),
                track_number = COALESCE(excluded.track_number, track_number),
                disc_number = COALESCE(excluded.disc_number, disc_number),
                genre = COALESCE(excluded.genre, genre),
                year = COALESCE(excluded.year, year),
                artwork_path = COALESCE(excluded.artwork_path, artwork_path),
                updated_at = excluded.updated_at,
                origin = 'automatic',
                provider = excluded.provider,
                confidence = excluded.confidence",
            params![
                track_id,
                clean_text(update.title.as_deref()),
                clean_text(update.artist.as_deref()),
                clean_text(update.album.as_deref()),
                update.track_number.map(|value| value.max(0)),
                update.disc_number.map(|value| value.max(0)),
                update.genre.as_deref().map(str::trim),
                update.year.map(|value| value.clamp(0, 9999)),
                clean_text(update.artwork_path.as_deref()),
                unix_seconds_i64(),
                provider.trim(),
                confidence.clamp(0.0, 1.0),
            ],
        )?;
        if changed > 0 {
            self.conn.execute(
                "UPDATE tracks SET revision = revision + 1 WHERE id = ?1",
                params![track_id],
            )?;
        }
        Ok(changed > 0)
    }

    pub fn metadata_candidate_ignored(&self,track_id:i64,fingerprint:&str)->Result<bool>{
        Ok(self.conn.query_row("SELECT EXISTS(SELECT 1 FROM metadata_ignored_candidates WHERE track_id=? AND fingerprint=?)",params![track_id,fingerprint],|r|r.get(0))?)
    }
    pub fn record_metadata_enrichment_status(
        &self,
        status: &MetadataEnrichmentStatus,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO metadata_enrichment_status (
                track_id, state, provider, confidence, candidate_json, recording_mbid,
                release_mbid, release_group_mbid, error, checked_at, source_modified_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(track_id) DO UPDATE SET
                state = excluded.state,
                provider = excluded.provider,
                confidence = excluded.confidence,
                candidate_json = excluded.candidate_json,
                recording_mbid = excluded.recording_mbid,
                release_mbid = excluded.release_mbid,
                release_group_mbid = excluded.release_group_mbid,
                error = excluded.error,
                checked_at = excluded.checked_at,
                source_modified_at = excluded.source_modified_at",
            params![
                status.track_id,
                status.state,
                status.provider,
                status.confidence,
                status.candidate_json,
                status.recording_mbid,
                status.release_mbid,
                status.release_group_mbid,
                status.error,
                status.checked_at,
                status.source_modified_at,
            ],
        )?;
        Ok(())
    }

    pub fn metadata_enrichment_counts(&self) -> Result<std::collections::BTreeMap<String, usize>> {
        let mut stmt = self.conn.prepare(
            "SELECT state, COUNT(*) FROM metadata_enrichment_status GROUP BY state ORDER BY state",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut counts = std::collections::BTreeMap::new();
        for row in rows {
            let (state, count) = row?;
            counts.insert(state, count.max(0) as usize);
        }
        Ok(counts)
    }

    pub fn metadata_enrichment_statuses(
        &self,
        state: &str,
        limit: usize,
    ) -> Result<Vec<MetadataEnrichmentStatus>> {
        let mut stmt = self.conn.prepare(
            "SELECT track_id, state, provider, confidence, candidate_json, recording_mbid,
                    release_mbid, release_group_mbid, error, checked_at, source_modified_at
             FROM metadata_enrichment_status
             WHERE (?1 = '' OR state = ?1)
             ORDER BY checked_at DESC, track_id ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![state, limit.clamp(1, 500) as i64], |row| {
            Ok(MetadataEnrichmentStatus {
                track_id: row.get(0)?,
                state: row.get(1)?,
                provider: row.get(2)?,
                confidence: row.get::<_, f64>(3)? as f32,
                candidate_json: row.get(4)?,
                recording_mbid: row.get(5)?,
                release_mbid: row.get(6)?,
                release_group_mbid: row.get(7)?,
                error: row.get(8)?,
                checked_at: row.get(9)?,
                source_modified_at: row.get(10)?,
            })
        })?;
        let mut statuses = Vec::new();
        for row in rows {
            statuses.push(row?);
        }
        Ok(statuses)
    }

    pub fn set_metadata_enrichment_state(&self, track_id: i64, state: &str) -> Result<bool> {
        Ok(self.conn.execute(
            "UPDATE metadata_enrichment_status SET state = ?1, checked_at = ?2 WHERE track_id = ?3",
            params![state.trim(), unix_seconds_i64(), track_id],
        )? > 0)
    }

    pub fn cached_provider_response(
        &self,
        key: &str,
        max_age_seconds: i64,
    ) -> Result<Option<String>> {
        let minimum = unix_seconds_i64().saturating_sub(max_age_seconds.max(0));
        Ok(self
            .conn
            .query_row(
                "SELECT response_json FROM metadata_provider_cache
                 WHERE cache_key = ?1 AND fetched_at >= ?2",
                params![key, minimum],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn cache_provider_response(
        &self,
        key: &str,
        provider: &str,
        response_json: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO metadata_provider_cache (cache_key, provider, response_json, fetched_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(cache_key) DO UPDATE SET
                provider = excluded.provider,
                response_json = excluded.response_json,
                fetched_at = excluded.fetched_at",
            params![key, provider, response_json, unix_seconds_i64()],
        )?;
        Ok(())
    }

    pub fn insert_track(
        &self,
        path: &str,
        title: &str,
        artist: &str,
        album: &str,
        duration: i32,
        file_size: i64,
        modified_at: i64,
    ) -> Result<i64> {
        self.insert_track_with_number(
            path,
            title,
            artist,
            album,
            0,
            duration,
            file_size,
            modified_at,
        )
    }

    pub fn insert_track_with_number(
        &self,
        path: &str,
        title: &str,
        artist: &str,
        album: &str,
        track_number: i32,
        duration: i32,
        file_size: i64,
        modified_at: i64,
    ) -> Result<i64> {
        let indexed_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        self.conn.execute(
            "INSERT INTO tracks (path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                track_number = excluded.track_number,
                duration = excluded.duration,
                file_size = excluded.file_size,
                modified_at = excluded.modified_at,
                indexed_at = excluded.indexed_at",
            params![
                path,
                title,
                artist,
                album,
                track_number.max(0),
                duration,
                file_size,
                modified_at,
                indexed_at
            ],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM tracks WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "UPDATE tracks
             SET stable_id = COALESCE(NULLIF(stable_id, ''), ?1),
                 revision = CASE WHEN revision < 1 THEN 1 ELSE revision END
             WHERE id = ?2",
            params![stable_id("track", &format!("{id}:{path}")), id],
        )?;
        Ok(id)
    }

    pub fn begin_transaction(&self) -> Result<()> {
        self.conn.execute("BEGIN TRANSACTION", [])?;
        Ok(())
    }

    pub fn commit_transaction(&self) -> Result<()> {
        self.conn.execute("COMMIT TRANSACTION", [])?;
        Ok(())
    }

    pub fn rollback_transaction(&self) -> Result<()> {
        let _ = self.conn.execute("ROLLBACK TRANSACTION", []);
        Ok(())
    }

    pub fn track_is_fresh(&self, path: &str, file_size: i64, modified_at: i64) -> Result<bool> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT file_size, modified_at, content_hash FROM tracks WHERE path = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![path])?;
        if let Some(row) = rows.next()? {
            let saved_size: i64 = row.get(0)?;
            let saved_modified: i64 = row.get(1)?;
            let content_hash: Option<String> = row.get(2)?;
            return Ok(saved_size == file_size
                && saved_modified == modified_at
                && content_hash
                    .as_deref()
                    .is_some_and(|value| !value.is_empty()));
        }
        Ok(false)
    }

    pub(crate) fn has_tracks_in_collection(
        &self,
        album: Option<&str>,
        artist: Option<&str>,
    ) -> Result<bool> {
        Ok(self.conn
            .prepare_cached(
                "SELECT EXISTS (
                SELECT 1 FROM tracks t
                LEFT JOIN track_metadata_overrides o ON o.track_id = t.id
                WHERE (?1 IS NULL OR COALESCE(o.album, t.album) = ?1)
                  AND (?2 IS NULL OR COALESCE(o.artist, t.artist) = ?2)
            )",
            )?
            .query_row(params![album, artist], |row| row.get(0))?)
    }

    pub fn get_tracks(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.path, COALESCE(o.title, t.title), COALESCE(o.artist, t.artist),
                    COALESCE(o.album, t.album), COALESCE(o.track_number, t.track_number),
                    t.duration, t.file_size, t.modified_at, t.indexed_at,
                    COALESCE(o.artwork_path, t.cover_art_path), t.cover_art_updated_at,
                    t.cover_art_source_modified_at, t.favorited, t.play_count, t.last_played, t.rating
             FROM tracks t
             LEFT JOIN track_metadata_overrides o ON o.track_id = t.id
             ORDER BY COALESCE(o.title, t.title) COLLATE NOCASE ASC",
        )?;
        let track_iter = stmt.query_map([], track_from_row)?;

        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn query_tracks_page(
        &self,
        query: &str,
        filter: &str,
        artist: &str,
        album: &str,
        sort: &str,
        descending: bool,
        limit: usize,
        offset: usize,
    ) -> Result<TrackPage> {
        let limit = limit.clamp(1, 500);
        let query_pattern = format!("%{}%", query.trim().to_ascii_lowercase());
        let favorites_only = i32::from(filter.eq_ignore_ascii_case("favorites"));
        let direction = if descending { "DESC" } else { "ASC" };
        let title = "COALESCE(o.title, t.title) COLLATE NOCASE";
        let artist_order = "COALESCE(o.artist, t.artist) COLLATE NOCASE";
        let album_order = "COALESCE(o.album, t.album) COLLATE NOCASE";
        let track_number = "COALESCE(o.track_number, t.track_number)";
        let disc_number = "COALESCE(o.disc_number, t.disc_number)";
        let order = match sort {
            "artist" => format!(
                "{artist_order} {direction}, {album_order} {direction}, {track_number} {direction}, {title} {direction}"
            ),
            "album" => format!(
                "{album_order} {direction}, {disc_number} {direction}, {track_number} {direction}, {title} {direction}"
            ),
            "duration" => format!("t.duration {direction}, {title} ASC"),
            "added" | "recent" => format!("t.indexed_at {direction}, {title} ASC"),
            "plays" | "play_count" => {
                format!("t.play_count {direction}, {title} ASC")
            }
            _ => format!("{title} {direction}"),
        };
        let where_sql = "
            t.deleted_at IS NULL
            AND (?1 = 0 OR t.favorited = 1)
            AND (?2 = '' OR COALESCE(o.artist, t.artist) = ?2 COLLATE NOCASE)
            AND (?3 = '' OR COALESCE(o.album, t.album) = ?3 COLLATE NOCASE)
            AND (?4 = '%%'
                OR lower(COALESCE(o.title, t.title)) LIKE ?4
                OR lower(COALESCE(o.artist, t.artist)) LIKE ?4
                OR lower(COALESCE(o.album, t.album)) LIKE ?4
                OR lower(COALESCE(o.genre, t.genre)) LIKE ?4)";
        let unfiltered = query.trim().is_empty()
            && favorites_only == 0
            && artist.is_empty()
            && album.is_empty();
        let total: i64 = if unfiltered {
            self.conn.query_row(
                "SELECT COUNT(*) FROM tracks WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )?
        } else {
            self.conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM tracks t
                     LEFT JOIN track_metadata_overrides o ON o.track_id = t.id
                     WHERE {where_sql}"
                ),
                params![favorites_only, artist, album, query_pattern],
                |row| row.get(0),
            )?
        };
        let page = if unfiltered && !matches!(sort, "artist" | "album" | "duration" | "added" | "recent" | "plays" | "play_count") {
            format!(
                "SELECT t.id, t.title COLLATE NOCASE AS sort_title
                 FROM tracks t
                 WHERE t.deleted_at IS NULL AND NOT EXISTS (
                     SELECT 1 FROM track_metadata_overrides o
                     WHERE o.track_id = t.id AND o.title IS NOT NULL
                 )
                 UNION ALL
                 SELECT t.id, o.title COLLATE NOCASE AS sort_title
                 FROM track_metadata_overrides o
                 CROSS JOIN tracks t ON t.id = o.track_id
                 WHERE t.deleted_at IS NULL AND o.title IS NOT NULL
                 ORDER BY sort_title {direction}
                 LIMIT ?5 OFFSET ?6"
            )
        } else {
            format!(
                "SELECT t.id
                 FROM tracks t
                 LEFT JOIN track_metadata_overrides o ON o.track_id = t.id
                 WHERE {where_sql}
                 ORDER BY {order}
                 LIMIT ?5 OFFSET ?6"
            )
        };
        let sql = format!(
            "WITH page AS MATERIALIZED (
                 {page}
             )
             SELECT t.id, t.path,
                    COALESCE(o.title, t.title) AS title,
                    COALESCE(o.artist, t.artist) AS artist,
                    COALESCE(o.album, t.album) AS album,
                    COALESCE(o.track_number, t.track_number) AS track_number,
                    t.duration, t.file_size, t.modified_at, t.indexed_at,
                    COALESCE(o.artwork_path, t.cover_art_path), t.cover_art_updated_at, t.cover_art_source_modified_at,
                    t.favorited, t.play_count, t.last_played, t.rating,
                    COALESCE(t.stable_id, ''), t.content_hash,
                    COALESCE(o.genre, t.genre) AS genre,
                    COALESCE(o.year, t.year) AS year,
                    COALESCE(o.disc_number, t.disc_number) AS disc_number,
                    t.revision,
                    t.source_id, s.display_name, s.kind, COALESCE(s.available, 1),
                    CASE WHEN o.track_id IS NULL THEN 0 ELSE 1 END
             FROM page
             JOIN tracks t ON t.id = page.id
             LEFT JOIN track_metadata_overrides o ON o.track_id = t.id
             LEFT JOIN library_sources s ON s.id = t.source_id
             ORDER BY {order}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![
                favorites_only,
                artist,
                album,
                query_pattern,
                limit as i64,
                offset as i64
            ],
            library_track_from_row,
        )?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        let revision = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(revision), 0) FROM tracks WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let consumed = offset.saturating_add(items.len());
        Ok(TrackPage {
            items,
            total,
            limit,
            offset,
            next_offset: (consumed < total as usize).then_some(consumed),
            revision,
        })
    }

    pub fn album_summaries(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AlbumSummary>> {
        let pattern = format!("%{}%", query.trim().to_ascii_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT album, artist, COALESCE(MAX(NULLIF(genre, '')), ''), MAX(year),
                    COUNT(*), COALESCE(SUM(duration), 0), MIN(id)
             FROM tracks
             WHERE deleted_at IS NULL
               AND (?1 = '%%' OR lower(album) LIKE ?1 OR lower(artist) LIKE ?1 OR lower(genre) LIKE ?1)
             GROUP BY artist COLLATE NOCASE, album COLLATE NOCASE
             ORDER BY album COLLATE NOCASE ASC, artist COLLATE NOCASE ASC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(
            params![pattern, limit.clamp(1, 500) as i64, offset as i64],
            |row| {
                Ok(AlbumSummary {
                    name: row.get(0)?,
                    artist: row.get(1)?,
                    genre: row.get(2)?,
                    year: row.get(3)?,
                    track_count: row.get(4)?,
                    duration: row.get(5)?,
                    artwork_track_id: row.get(6)?,
                })
            },
        )?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn artist_summaries(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ArtistSummary>> {
        let pattern = format!("%{}%", query.trim().to_ascii_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT artist, COUNT(DISTINCT album), COUNT(*)
             FROM tracks
             WHERE deleted_at IS NULL
               AND (?1 = '%%' OR lower(artist) LIKE ?1)
             GROUP BY artist COLLATE NOCASE
             ORDER BY artist COLLATE NOCASE ASC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(
            params![pattern, limit.clamp(1, 500) as i64, offset as i64],
            |row| {
                Ok(ArtistSummary {
                    name: row.get(0)?,
                    album_count: row.get(1)?,
                    track_count: row.get(2)?,
                })
            },
        )?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn genres(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT CASE WHEN trim(genre) = '' THEN 'Unknown genre' ELSE genre END, COUNT(*)
             FROM tracks WHERE deleted_at IS NULL
             GROUP BY CASE WHEN trim(genre) = '' THEN 'Unknown genre' ELSE genre END
             ORDER BY 1 COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn update_track_metadata(&self, ids: &[i64], update: &MetadataUpdate) -> Result<usize> {
        anyhow::ensure!(!ids.is_empty(), "at least one track is required");
        let now = unix_seconds_i64();
        self.begin_transaction()?;
        let result = (|| {
            let mut changed = 0;
            for id in ids {
                changed += self.conn.execute(
                    "UPDATE tracks SET
                        title = COALESCE(?1, title),
                        artist = COALESCE(?2, artist),
                        album = COALESCE(?3, album),
                        track_number = COALESCE(?4, track_number),
                        disc_number = COALESCE(?5, disc_number),
                        genre = COALESCE(?6, genre),
                        year = COALESCE(?7, year),
                        revision = revision + 1,
                        indexed_at = ?8
                     WHERE id = ?9 AND deleted_at IS NULL",
                    params![
                        update
                            .title
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty()),
                        update
                            .artist
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty()),
                        update
                            .album
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty()),
                        update.track_number.map(|value| value.max(0)),
                        update.disc_number.map(|value| value.max(0)),
                        update.genre.as_deref().map(str::trim),
                        update.year.map(|value| value.clamp(0, 9999)),
                        now,
                        id
                    ],
                )?;
            }
            Ok(changed)
        })();
        match result {
            Ok(changed) => {
                self.commit_transaction()?;
                Ok(changed)
            }
            Err(err) => {
                let _ = self.rollback_transaction();
                Err(err)
            }
        }
    }

    pub fn set_track_content_hash(&self, id: i64, content_hash: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET content_hash = ?1, revision = revision + 1 WHERE id = ?2",
            params![content_hash, id],
        )?;
        Ok(())
    }

    pub fn get_track_by_id(&self, id: i64) -> Result<Option<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.path, COALESCE(o.title, t.title), COALESCE(o.artist, t.artist),
                    COALESCE(o.album, t.album), COALESCE(o.track_number, t.track_number),
                    t.duration, t.file_size, t.modified_at, t.indexed_at,
                    COALESCE(o.artwork_path, t.cover_art_path), t.cover_art_updated_at,
                    t.cover_art_source_modified_at, t.favorited, t.play_count, t.last_played, t.rating
             FROM tracks t
             LEFT JOIN track_metadata_overrides o ON o.track_id = t.id
             WHERE t.id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(track_from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_albums(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT album FROM tracks ORDER BY album COLLATE NOCASE ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_artists(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT artist FROM tracks ORDER BY artist COLLATE NOCASE ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_artist_albums(&self, artist: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT album FROM tracks WHERE artist = ?1 ORDER BY album COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map(params![artist], |row| row.get(0))?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn search_tracks(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        let terms = query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let mut tracks = self.get_tracks()?;
        tracks.retain(|track| {
            let haystack =
                format!("{}\n{}\n{}", track.title, track.artist, track.album).to_ascii_lowercase();
            terms.iter().all(|term| haystack.contains(term))
        });
        tracks.truncate(limit.clamp(1, 500));
        Ok(tracks)
    }

    pub fn toggle_favorite(&self, id: i64) -> Result<bool> {
        let current: i32 = self.conn.query_row(
            "SELECT favorited FROM tracks WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let next = if current == 0 { 1 } else { 0 };
        self.conn.execute(
            "UPDATE tracks SET favorited = ?1 WHERE id = ?2",
            params![next, id],
        )?;
        Ok(next != 0)
    }

    pub fn set_track_rating(&self, id: i64, rating: u8) -> Result<()> {
        anyhow::ensure!(rating <= 5, "rating must be between 0 and 5");
        self.conn.execute(
            "UPDATE tracks SET rating = ?1 WHERE id = ?2",
            params![rating, id],
        )?;
        Ok(())
    }

    pub fn get_favorites(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played, rating
             FROM tracks WHERE favorited = 1 ORDER BY title COLLATE NOCASE ASC",
        )?;
        let track_iter = stmt.query_map([], track_from_row)?;
        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn increment_play_count(&self, id: i64) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        self.conn.execute(
            "UPDATE tracks SET play_count = play_count + 1, last_played = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        self.conn.execute(
            "INSERT INTO play_history (track_id, played_at) VALUES (?1, ?2)",
            params![id, now],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: i32) -> Result<Vec<Track>> {
        self.get_history_paged(limit, 0)
    }

    pub fn get_history_paged(&self, limit: i32, offset: i32) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.path, t.title, t.artist, t.album, t.track_number, t.duration,
                    t.file_size, t.modified_at, t.indexed_at,
                    t.cover_art_path, t.cover_art_updated_at, t.cover_art_source_modified_at,
                    t.favorited, t.play_count, t.last_played, t.rating
             FROM play_history h
             JOIN tracks t ON h.track_id = t.id
             ORDER BY h.played_at DESC LIMIT ?1 OFFSET ?2",
        )?;
        let track_iter = stmt.query_map(params![limit, offset.max(0)], track_from_row)?;
        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn dynamic_mixes(&self) -> Result<Vec<MixDescriptor>> {
        let tracks = self.get_tracks()?;
        let long_count = tracks
            .iter()
            .filter(|track| track.duration >= 8 * 60)
            .count();
        let never_count = tracks.iter().filter(|track| track.play_count == 0).count();
        let recent_played_count = tracks
            .iter()
            .filter(|track| track.last_played.unwrap_or_default() > 0)
            .count();
        let most_played_count = tracks.iter().filter(|track| track.play_count > 0).count();
        let favorites_count = tracks.iter().filter(|track| track.favorited).count();
        let all_count = tracks.len();

        Ok(vec![
            MixDescriptor {
                id: "favorites".to_string(),
                name: "Favorites".to_string(),
                kind: "favorites".to_string(),
                track_count: favorites_count,
            },
            MixDescriptor {
                id: "recently_added".to_string(),
                name: "Recently Added".to_string(),
                kind: "recently_added".to_string(),
                track_count: all_count,
            },
            MixDescriptor {
                id: "recently_played".to_string(),
                name: "Recently Played".to_string(),
                kind: "recently_played".to_string(),
                track_count: recent_played_count,
            },
            MixDescriptor {
                id: "most_played".to_string(),
                name: "Most Played".to_string(),
                kind: "most_played".to_string(),
                track_count: most_played_count,
            },
            MixDescriptor {
                id: "never_played".to_string(),
                name: "Never Played".to_string(),
                kind: "never_played".to_string(),
                track_count: never_count,
            },
            MixDescriptor {
                id: "long_tracks".to_string(),
                name: "Long Tracks".to_string(),
                kind: "long_tracks".to_string(),
                track_count: long_count,
            },
            MixDescriptor {
                id: "shuffle_all".to_string(),
                name: "Shuffle All".to_string(),
                kind: "shuffle_all".to_string(),
                track_count: all_count,
            },
            MixDescriptor {
                id: "random_album".to_string(),
                name: "Random Album".to_string(),
                kind: "random_album".to_string(),
                track_count: all_count,
            },
        ])
    }

    pub fn get_mix_tracks(&self, mix_id: &str, limit: usize) -> Result<Vec<Track>> {
        let limit = limit.clamp(1, 500);
        let mut tracks = self.get_tracks()?;
        match mix_id {
            "favorites" => {
                tracks.retain(|track| track.favorited);
                tracks.sort_by_key(|track| track.title.to_ascii_lowercase());
            }
            "recently_added" => tracks.sort_by_key(|track| {
                (
                    std::cmp::Reverse(track.indexed_at),
                    track.title.to_ascii_lowercase(),
                )
            }),
            "recently_played" => tracks.sort_by_key(|track| {
                (
                    std::cmp::Reverse(track.last_played.unwrap_or_default()),
                    track.title.to_ascii_lowercase(),
                )
            }),
            "most_played" => {
                tracks.retain(|track| track.play_count > 0);
                tracks.sort_by_key(|track| {
                    (
                        std::cmp::Reverse(track.play_count),
                        track.title.to_ascii_lowercase(),
                    )
                });
            }
            "never_played" => {
                tracks.retain(|track| track.play_count == 0);
                tracks.sort_by_key(|track| track.title.to_ascii_lowercase());
            }
            "long_tracks" => {
                tracks.retain(|track| track.duration >= 8 * 60);
                tracks.sort_by_key(|track| {
                    (
                        std::cmp::Reverse(track.duration),
                        track.title.to_ascii_lowercase(),
                    )
                });
            }
            "shuffle_all" => {
                let seed = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
                tracks.sort_by_key(|track| pseudo_shuffle_key(track.id, seed));
            }
            "random_album" => {
                let albums = self.get_albums()?;
                if let Some(album) = albums.get(
                    (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize)
                        % albums.len().max(1),
                ) {
                    tracks.retain(|track| track.album == *album);
                    tracks.sort_by_key(|track| {
                        (
                            if track.track_number > 0 { 0 } else { 1 },
                            track.track_number,
                            track.title.to_ascii_lowercase(),
                        )
                    });
                }
            }
            _ => tracks.clear(),
        }
        Ok(tracks.into_iter().take(limit).collect())
    }

    pub fn create_playlist(&self, name: &str) -> Result<i64> {
        let now = unix_seconds_i64();
        let stable_id = stable_id("playlist", &format!("{}:{now}", name.trim()));
        self.conn.execute(
            "INSERT OR IGNORE INTO playlists (name, stable_id, revision, updated_at) VALUES (?1, ?2, 1, ?3)",
            params![name, stable_id, now],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM playlists WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn get_playlists(&self) -> Result<Vec<Playlist>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM playlists ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn playlist_manifests(&self) -> Result<Vec<PlaylistManifest>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, COALESCE(stable_id, ''), name, revision, updated_at
             FROM playlists ORDER BY name COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        let mut manifests = Vec::new();
        for row in rows {
            let (id, stable_id, name, revision, updated_at) = row?;
            let mut track_stmt = self.conn.prepare_cached(
                "SELECT COALESCE(t.content_hash, t.stable_id, printf('track-%d', t.id))
                 FROM playlist_tracks pt
                 JOIN tracks t ON t.id = pt.track_id
                 WHERE pt.playlist_id = ?1 AND t.deleted_at IS NULL
                 ORDER BY pt.position ASC, pt.rowid ASC",
            )?;
            let track_rows = track_stmt.query_map(params![id], |track_row| track_row.get(0))?;
            let mut track_ids = Vec::new();
            for track_row in track_rows {
                track_ids.push(track_row?);
            }
            manifests.push(PlaylistManifest {
                id,
                stable_id,
                name,
                revision,
                updated_at,
                track_ids,
            });
        }
        Ok(manifests)
    }

    pub fn playlist_tombstones(&self) -> Result<Vec<PlaylistTombstone>> {
        let mut stmt = self.conn.prepare(
            "SELECT stable_id, deleted_at, revision
             FROM playlist_tombstones ORDER BY deleted_at ASC, stable_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PlaylistTombstone {
                stable_id: row.get(0)?,
                deleted_at: row.get(1)?,
                revision: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        let identity = self
            .conn
            .query_row(
                "SELECT stable_id, revision FROM playlists WHERE id = ?1",
                params![id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
            )
            .ok();
        if let Some((Some(stable_id), revision)) = identity {
            self.conn.execute(
                "INSERT INTO playlist_tombstones (stable_id, deleted_at, revision)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(stable_id) DO UPDATE SET deleted_at = excluded.deleted_at, revision = excluded.revision",
                params![stable_id, unix_seconds_i64(), revision + 1],
            )?;
        }
        // Deleting from playlists automatically deletes from playlist_tracks via foreign key ON DELETE CASCADE.
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        self.conn.execute(
            "DELETE FROM pins WHERE pin_type = 'playlist' AND target_id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE playlists SET name = ?1, revision = revision + 1, updated_at = ?2 WHERE id = ?3",
            params![name.trim(), unix_seconds_i64(), id],
        )?;
        Ok(())
    }

    pub fn cleanup_missing_files(&self) -> Result<usize> {
        let mut missing = Vec::new();
        {
            let mut stmt = self.conn.prepare_cached("SELECT id, path FROM tracks")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (id, path) = row?;
                if !std::path::Path::new(&path).exists() {
                    missing.push(id);
                }
            }
        }
        for id in &missing {
            self.conn
                .execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
            self.conn.execute(
                "DELETE FROM pins WHERE pin_type = 'track' AND target_id = ?1",
                params![id],
            )?;
        }
        let removed = missing.len();
        Ok(removed)
    }

    pub fn repair_library(&self) -> Result<DbRepairReport> {
        let integrity_message: String =
            self.conn
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        let integrity_ok = integrity_message.eq_ignore_ascii_case("ok");

        let missing_tracks_removed = self.cleanup_missing_files()?;
        let orphan_playlist_rows_removed = self.conn.execute(
            "DELETE FROM playlist_tracks
             WHERE playlist_id NOT IN (SELECT id FROM playlists)
                OR track_id NOT IN (SELECT id FROM tracks)",
            [],
        )?;

        let playlist_ids = self.playlist_ids()?;
        let mut playlist_positions_repaired = 0;
        for playlist_id in playlist_ids {
            playlist_positions_repaired += self.compact_playlist_positions(playlist_id)?;
        }

        let wal_checkpoint = self
            .conn
            .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                let busy: i64 = row.get(0)?;
                let log: i64 = row.get(1)?;
                let checkpointed: i64 = row.get(2)?;
                Ok(format!(
                    "busy={busy} log_frames={log} checkpointed={checkpointed}"
                ))
            })
            .unwrap_or_else(|err| format!("unavailable: {err}"));

        Ok(DbRepairReport {
            integrity_ok,
            integrity_message,
            missing_tracks_removed,
            orphan_playlist_rows_removed,
            playlist_positions_repaired,
            wal_checkpoint,
        })
    }

    fn playlist_ids(&self) -> Result<Vec<i64>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id FROM playlists ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    pub fn add_track_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        let next_position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )?;
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, next_position],
        )?;
        if changed > 0 {
            self.bump_playlist_revision(playlist_id)?;
        }
        Ok(())
    }

    pub fn add_tracks_to_playlist(&self, playlist_id: i64, track_ids: &[i64]) -> Result<usize> {
        if track_ids.is_empty() {
            return Ok(0);
        }
        self.begin_transaction()?;
        let result = (|| {
            let mut next_position: i64 = self.conn.query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |row| row.get(0),
            )?;
            let mut added = 0;
            for track_id in track_ids {
                let changed = self.conn.execute(
                    "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
                    params![playlist_id, track_id, next_position],
                )?;
                if changed > 0 {
                    added += 1;
                    next_position += 1;
                }
            }
            if added > 0 {
                self.bump_playlist_revision(playlist_id)?;
            }
            Ok(added)
        })();
        match result {
            Ok(added) => {
                self.commit_transaction()?;
                Ok(added)
            }
            Err(error) => {
                let _ = self.rollback_transaction();
                Err(error)
            }
        }
    }

    pub fn playlist_track_count(&self, playlist_id: i64) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )?)
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        let changed = self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        self.compact_playlist_positions(playlist_id)?;
        if changed > 0 {
            self.bump_playlist_revision(playlist_id)?;
        }
        Ok(())
    }

    pub fn remove_tracks_from_playlist(
        &self,
        playlist_id: i64,
        track_ids: &[i64],
    ) -> Result<usize> {
        if track_ids.is_empty() {
            return Ok(0);
        }
        self.begin_transaction()?;
        let result = (|| {
            let mut removed = 0;
            for track_id in track_ids {
                removed += self.conn.execute(
                    "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                    params![playlist_id, track_id],
                )?;
            }
            if removed > 0 {
                self.compact_playlist_positions(playlist_id)?;
                self.bump_playlist_revision(playlist_id)?;
            }
            Ok(removed)
        })();
        match result {
            Ok(removed) => {
                self.commit_transaction()?;
                Ok(removed)
            }
            Err(error) => {
                let _ = self.rollback_transaction();
                Err(error)
            }
        }
    }

    pub fn reorder_playlist_tracks(&self, playlist_id: i64, track_ids: &[i64]) -> Result<()> {
        self.begin_transaction()?;
        let result = (|| {
            for (position, track_id) in track_ids.iter().enumerate() {
                self.conn.execute(
                    "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3",
                    params![position as i64, playlist_id, track_id],
                )?;
            }
            self.compact_playlist_positions(playlist_id)?;
            self.bump_playlist_revision(playlist_id)?;
            Ok(())
        })();
        match result {
            Ok(()) => self.commit_transaction(),
            Err(err) => {
                let _ = self.rollback_transaction();
                Err(err)
            }
        }
    }

    fn compact_playlist_positions(&self, playlist_id: i64) -> Result<usize> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT track_id FROM playlist_tracks
             WHERE playlist_id = ?1
             ORDER BY position ASC, rowid ASC",
        )?;
        let rows = stmt.query_map(params![playlist_id], |row| row.get::<_, i64>(0))?;
        let mut track_ids = Vec::new();
        for row in rows {
            track_ids.push(row?);
        }
        drop(stmt);

        let mut updated = 0;
        for (position, track_id) in track_ids.iter().enumerate() {
            let changed = self.conn.execute(
                "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3 AND position != ?1",
                params![position as i64, playlist_id, track_id],
            )?;
            updated += changed;
        }
        Ok(updated)
    }

    fn bump_playlist_revision(&self, playlist_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE playlists SET revision = revision + 1, updated_at = ?1 WHERE id = ?2",
            params![unix_seconds_i64(), playlist_id],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(&self, playlist_id: i64) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.path, t.title, t.artist, t.album, t.track_number, t.duration,
                    t.file_size, t.modified_at, t.indexed_at,
                    t.cover_art_path, t.cover_art_updated_at, t.cover_art_source_modified_at,
                    t.favorited, t.play_count, t.last_played, t.rating
             FROM playlist_tracks pt
             JOIN tracks t ON pt.track_id = t.id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position ASC, CASE WHEN t.track_number > 0 THEN 0 ELSE 1 END, t.track_number, t.title COLLATE NOCASE ASC"
        )?;
        let track_iter = stmt.query_map(params![playlist_id], track_from_row)?;
        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn get_album_tracks(&self, album: &str) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played, rating
             FROM tracks
             WHERE album = ?1
             ORDER BY CASE WHEN track_number > 0 THEN 0 ELSE 1 END, track_number, title COLLATE NOCASE ASC",
        )?;
        let track_iter = stmt.query_map(params![album], track_from_row)?;
        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn get_artist_tracks(&self, artist: &str) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played, rating
             FROM tracks
             WHERE artist = ?1
             ORDER BY album COLLATE NOCASE ASC, CASE WHEN track_number > 0 THEN 0 ELSE 1 END, track_number, title COLLATE NOCASE ASC",
        )?;
        let track_iter = stmt.query_map(params![artist], track_from_row)?;
        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn get_artist_album_tracks(&self, artist: &str, album: &str) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played, rating
             FROM tracks
             WHERE artist = ?1 AND album = ?2
             ORDER BY CASE WHEN track_number > 0 THEN 0 ELSE 1 END, track_number, title COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map(params![artist, album], track_from_row)?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    pub fn pin_item(&self, pin_type: &str, target_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO pins (pin_type, target_id) VALUES (?1, ?2)",
            params![pin_type, target_id],
        )?;
        Ok(())
    }

    pub fn unpin_item(&self, pin_type: &str, target_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM pins WHERE pin_type = ?1 AND target_id = ?2",
            params![pin_type, target_id],
        )?;
        Ok(())
    }

    pub fn is_pinned(&self, pin_type: &str, target_id: i64) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pins WHERE pin_type = ?1 AND target_id = ?2",
            params![pin_type, target_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn get_pins(&self) -> Result<Vec<Pin>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, pin_type, target_id FROM pins")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let pin_type: String = row.get(1)?;
            let target_id: i64 = row.get(2)?;
            Ok((id, pin_type, target_id))
        })?;

        let mut list = Vec::new();
        for r in rows {
            let (id, pin_type, target_id) = r?;
            let (name, subtitle) = if pin_type == "track" {
                if let Ok(Some(track)) = self.get_track_by_id(target_id) {
                    (track.title, track.artist)
                } else {
                    continue;
                }
            } else {
                let name: String = self
                    .conn
                    .query_row(
                        "SELECT name FROM playlists WHERE id = ?1",
                        params![target_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_else(|_| "Unknown Playlist".to_string());
                (name, "Playlist".to_string())
            };

            list.push(Pin {
                id,
                pin_type,
                target_id,
                name,
                subtitle,
            });
        }
        Ok(list)
    }

    pub fn library_counts(&self) -> Result<LibraryCounts> {
        Ok(LibraryCounts {
            tracks: self
                .conn
                .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))?,
            albums: self
                .conn
                .query_row("SELECT COUNT(DISTINCT album) FROM tracks", [], |row| {
                    row.get(0)
                })?,
            artists: self.conn.query_row(
                "SELECT COUNT(DISTINCT artist) FROM tracks",
                [],
                |row| row.get(0),
            )?,
            playlists: self
                .conn
                .query_row("SELECT COUNT(*) FROM playlists", [], |row| row.get(0))?,
            pins: self
                .conn
                .query_row("SELECT COUNT(*) FROM pins", [], |row| row.get(0))?,
            favorites: self.conn.query_row(
                "SELECT COUNT(*) FROM tracks WHERE favorited = 1",
                [],
                |row| row.get(0),
            )?,
        })
    }
}

fn pseudo_shuffle_key(id: i64, seed: i64) -> i64 {
    let mut value = id.wrapping_mul(1_103_515_245).wrapping_add(seed);
    value ^= value.rotate_left(13);
    value
}

fn track_from_row(row: &Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        track_number: row.get(5)?,
        duration: row.get(6)?,
        file_size: row.get(7)?,
        modified_at: row.get(8)?,
        indexed_at: row.get(9)?,
        cover_art_path: row.get(10)?,
        cover_art_updated_at: row.get(11)?,
        cover_art_source_modified_at: row.get(12)?,
        favorited: row.get::<_, i32>(13)? != 0,
        play_count: row.get(14)?,
        last_played: row.get(15)?,
        rating: row.get::<_, i32>(16)?.clamp(0, 5) as u8,
    })
}

fn library_track_from_row(row: &Row<'_>) -> rusqlite::Result<LibraryTrack> {
    Ok(LibraryTrack {
        track: track_from_row(row)?,
        stable_id: row.get(17)?,
        content_hash: row.get(18)?,
        genre: row.get(19)?,
        year: row.get(20)?,
        disc_number: row.get(21)?,
        revision: row.get(22)?,
        source_id: row.get(23)?,
        source_name: row.get(24)?,
        source_kind: row.get(25)?,
        source_available: row.get::<_, i32>(26)? != 0,
        metadata_overridden: row.get::<_, i32>(27)? != 0,
    })
}

fn source_from_row(row: &Row<'_>) -> rusqlite::Result<LibrarySource> {
    Ok(LibrarySource {
        id: row.get(0)?,
        kind: row.get(1)?,
        path: row.get(2)?,
        display_name: row.get(3)?,
        enabled: row.get::<_, i32>(4)? != 0,
        available: row.get::<_, i32>(5)? != 0,
        last_scan_at: row.get(6)?,
        last_error: row.get(7)?,
    })
}

fn normalize_source_path(value: &str) -> String {
    let path = Path::new(value);
    path.canonicalize()
        .unwrap_or_else(|_| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(path)
            }
        })
        .display()
        .to_string()
}

fn clean_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn unix_seconds_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn stable_id(kind: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{kind}-{}", &digest[..24])
}

#[cfg(test)]
mod tests {
    use super::{Db, MetadataUpdate};

    #[test]
    fn sqlite_durability_applies_to_every_connection() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("library.sqlite3");
        let path = file.to_str().unwrap();
        let expected = if cfg!(all(target_os = "linux", feature = "pid1")) {
            2 // FULL
        } else {
            1 // NORMAL
        };
        for db in [Db::open(path).unwrap(), Db::open_background(path).unwrap()] {
            let synchronous: i64 = db
                .conn
                .query_row("PRAGMA synchronous", [], |row| row.get(0))
                .unwrap();
            assert_eq!(synchronous, expected);
        }
    }

    fn temp_db(name: &str) -> Db {
        let root =
            std::env::temp_dir().join(format!("musicos-db-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Db::open(root.join("media.sqlite3").to_str().unwrap()).unwrap()
    }

    #[test]
    fn v2_migration_preserves_a_pre_migration_database_backup() {
        let root = std::env::temp_dir().join(format!(
            "musicos-db-test-migration-backup-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("media.sqlite3");
        let legacy = rusqlite::Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE tracks (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL,
                    title TEXT NOT NULL,
                    artist TEXT NOT NULL,
                    album TEXT NOT NULL,
                    duration INTEGER NOT NULL,
                    favorited INTEGER NOT NULL DEFAULT 0,
                    play_count INTEGER NOT NULL DEFAULT 0,
                    last_played INTEGER
                );",
            )
            .unwrap();
        drop(legacy);

        let db = Db::open(path.to_str().unwrap()).unwrap();
        assert_eq!(db.library_counts().unwrap().tracks, 0);
        assert!(path
            .with_extension("pre-v2-migration.sqlite3.bak")
            .is_file());
    }

    #[test]
    fn dynamic_mixes_reflect_track_metadata() {
        let db = temp_db("dynamic-mixes");
        let first = db
            .insert_track("/music/a.wav", "Alpha", "Artist", "Album", 120, 1, 10)
            .unwrap();
        let second = db
            .insert_track("/music/b.wav", "Beta", "Artist", "Album", 620, 1, 20)
            .unwrap();
        db.toggle_favorite(first).unwrap();
        db.increment_play_count(second).unwrap();

        let mixes = db.dynamic_mixes().unwrap();
        let favorites = mixes.iter().find(|mix| mix.id == "favorites").unwrap();
        let long_tracks = mixes.iter().find(|mix| mix.id == "long_tracks").unwrap();
        let most_played = mixes.iter().find(|mix| mix.id == "most_played").unwrap();
        assert_eq!(favorites.track_count, 1);
        assert_eq!(long_tracks.track_count, 1);
        assert_eq!(most_played.track_count, 1);

        let favorite_tracks = db.get_mix_tracks("favorites", 20).unwrap();
        assert_eq!(favorite_tracks.len(), 1);
        assert_eq!(favorite_tracks[0].id, first);
    }

    #[test]
    fn cover_art_cache_metadata_can_be_cleared() {
        let db = temp_db("cover-art-clear");
        let track = db
            .insert_track("/music/a.wav", "Alpha", "Artist", "Album", 120, 1, 10)
            .unwrap();
        db.set_track_cover_art(track, Some("/tmp/art.png"), 10)
            .unwrap();
        assert_eq!(db.cover_art_cache_count().unwrap(), 1);
        assert_eq!(db.clear_cover_art_cache_metadata().unwrap(), 1);
        assert_eq!(db.cover_art_cache_count().unwrap(), 0);
    }

    #[test]
    fn playlist_tracks_keep_manual_order() {
        let db = temp_db("playlist-order");
        let first = db
            .insert_track("/music/a.wav", "Alpha", "Artist", "Album", 120, 1, 10)
            .unwrap();
        let second = db
            .insert_track("/music/b.wav", "Beta", "Artist", "Album", 120, 1, 20)
            .unwrap();
        let third = db
            .insert_track("/music/c.wav", "Gamma", "Artist", "Album", 120, 1, 30)
            .unwrap();
        let playlist = db.create_playlist("Order").unwrap();
        assert_eq!(
            db.add_tracks_to_playlist(playlist, &[first, second, third, second])
                .unwrap(),
            3
        );

        let tracks = db.get_playlist_tracks(playlist).unwrap();
        assert_eq!(
            tracks.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![first, second, third]
        );

        db.reorder_playlist_tracks(playlist, &[third, first, second])
            .unwrap();
        let tracks = db.get_playlist_tracks(playlist).unwrap();
        assert_eq!(
            tracks.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![third, first, second]
        );

        assert_eq!(
            db.remove_tracks_from_playlist(playlist, &[third, second])
                .unwrap(),
            2
        );
        let tracks = db.get_playlist_tracks(playlist).unwrap();
        assert_eq!(
            tracks.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![first]
        );
    }

    #[test]
    fn repair_reports_integrity_and_compacts_playlist_positions() {
        let root = std::env::temp_dir().join(format!(
            "musicos-db-test-repair-real-files-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let first_path = root.join("a.wav");
        let second_path = root.join("b.wav");
        std::fs::write(&first_path, b"alpha").unwrap();
        std::fs::write(&second_path, b"beta").unwrap();
        let db = Db::open(root.join("media.sqlite3").to_str().unwrap()).unwrap();
        let first = db
            .insert_track(
                first_path.to_str().unwrap(),
                "Alpha",
                "Artist",
                "Album",
                120,
                1,
                10,
            )
            .unwrap();
        let second = db
            .insert_track(
                second_path.to_str().unwrap(),
                "Beta",
                "Artist",
                "Album",
                120,
                1,
                20,
            )
            .unwrap();
        let playlist = db.create_playlist("Repair").unwrap();
        db.add_track_to_playlist(playlist, first).unwrap();
        db.add_track_to_playlist(playlist, second).unwrap();
        db.reorder_playlist_tracks(playlist, &[second, first])
            .unwrap();

        let report = db.repair_library().unwrap();
        assert!(report.integrity_ok);
        assert_eq!(report.integrity_message, "ok");
        let tracks = db.get_playlist_tracks(playlist).unwrap();
        assert_eq!(
            tracks.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![second, first]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ratings_search_and_artist_albums_are_persisted_and_scoped() {
        let db = temp_db("ratings-search-artist-albums");
        let first = db
            .insert_track_with_number("/music/a.wav", "Aurora", "Tanger", "Shared", 1, 120, 1, 10)
            .unwrap();
        let second = db
            .insert_track_with_number(
                "/music/b.wav",
                "Blue Hour",
                "Other",
                "Shared",
                1,
                120,
                1,
                10,
            )
            .unwrap();
        db.insert_track(
            "/music/c.wav",
            "Signal",
            "Tanger",
            "Night Works",
            120,
            1,
            10,
        )
        .unwrap();

        assert_eq!(db.get_track_by_id(first).unwrap().unwrap().rating, 0);
        db.set_track_rating(first, 5).unwrap();
        assert_eq!(db.get_track_by_id(first).unwrap().unwrap().rating, 5);
        assert!(db.set_track_rating(first, 6).is_err());

        let matches = db.search_tracks("tanger night", 20).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "Signal");

        assert_eq!(
            db.get_artist_albums("Tanger").unwrap(),
            vec!["Night Works".to_string(), "Shared".to_string()]
        );
        let scoped = db.get_artist_album_tracks("Tanger", "Shared").unwrap();
        assert_eq!(
            scoped.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![first]
        );
        assert!(!scoped.iter().any(|track| track.id == second));
    }

    #[test]
    fn track_pages_preserve_effective_metadata_order_and_pagination() {
        let db = temp_db("page-order");
        for i in 0..9 {
            let id = db.insert_track_with_number(
                &format!("/music/{i}.wav"), &format!("Title {i}"),
                &format!("Artist {}", i % 3), &format!("Album {}", i % 2),
                i + 1, 100 + i, 10, i64::from(20 + i),
            ).unwrap();
            if i % 2 == 0 {
                db.toggle_favorite(id).unwrap();
                db.update_track_metadata_override(&[id], &MetadataUpdate {
                    title: (i != 4).then(|| if i == 0 { String::new() } else { format!("override {}", 9 - i) }),
                    artist: Some(format!("Artist {}", 2 - i % 3)),
                    album: Some(format!("Album {}", 1 - i % 2)),
                    track_number: Some(9 - i),
                    disc_number: Some(2),
                    ..Default::default()
                }).unwrap();
            }
        }
        db.conn.execute("UPDATE tracks SET play_count = id, indexed_at = id", []).unwrap();
        db.conn.execute("UPDATE tracks SET deleted_at = 1 WHERE id = 9", []).unwrap();
        for (sort, ascending) in [
            ("title", "title COLLATE NOCASE {d}"),
            ("artist", "artist COLLATE NOCASE {d}, album COLLATE NOCASE {d}, track_number {d}, title COLLATE NOCASE {d}"),
            ("album", "album COLLATE NOCASE {d}, disc_number {d}, track_number {d}, title COLLATE NOCASE {d}"),
            ("duration", "duration {d}, title COLLATE NOCASE ASC"),
            ("added", "indexed_at {d}, title COLLATE NOCASE ASC"),
            ("plays", "play_count {d}, title COLLATE NOCASE ASC"),
        ] {
            for descending in [false, true] {
                for filter in ["", "favorites"] {
                    let order = ascending.replace("{d}", if descending { "DESC" } else { "ASC" });
                    let mut statement = db.conn.prepare(&format!(
                        "SELECT t.id, COALESCE(o.title,t.title) AS title,
                         COALESCE(o.artist,t.artist) AS artist, COALESCE(o.album,t.album) AS album,
                         COALESCE(o.track_number,t.track_number) AS track_number,
                         COALESCE(o.disc_number,t.disc_number) AS disc_number,
                         t.duration, t.indexed_at, t.play_count
                         FROM tracks t LEFT JOIN track_metadata_overrides o ON o.track_id=t.id
                         WHERE t.deleted_at IS NULL AND (?1 = '' OR t.favorited=1)
                         ORDER BY {order}"
                    )).unwrap();
                    let expected: Vec<i64> = statement.query_map([filter], |row| row.get(0))
                        .unwrap().collect::<rusqlite::Result<_>>().unwrap();
                    for offset in [0, 2, 50] {
                        let page = db.query_tracks_page("", filter, "", "", sort, descending, 3, offset).unwrap();
                        assert_eq!(page.total, expected.len() as i64);
                        assert_eq!(
                            page.items.iter().map(|item| item.track.id).collect::<Vec<_>>(),
                            expected.iter().skip(offset).take(3).copied().collect::<Vec<_>>(),
                            "{sort} descending={descending} filter={filter} offset={offset}"
                        );
                    }
                }
            }
        }
        assert_eq!(db.query_tracks_page("no match", "", "", "", "title", false, 50, 0).unwrap().total, 0);
        db.conn.execute("DROP TABLE track_metadata_overrides", []).unwrap();
        assert!(db.query_tracks_page("", "", "", "", "title", false, 50, 0).is_err());
    }

    #[test]
    fn v2_pages_metadata_and_playlist_manifests_use_stable_identities() {
        let db = temp_db("v2-library");
        let first = db
            .insert_track_with_number(
                "/music/artist/album/01.wav",
                "First",
                "Artist",
                "Album",
                1,
                120,
                10,
                20,
            )
            .unwrap();
        db.insert_track_with_number(
            "/music/artist/album/02.wav",
            "Second",
            "Artist",
            "Album",
            2,
            140,
            10,
            20,
        )
        .unwrap();
        db.update_track_metadata(
            &[first],
            &MetadataUpdate {
                genre: Some("Electronic".to_string()),
                year: Some(2026),
                disc_number: Some(1),
                ..MetadataUpdate::default()
            },
        )
        .unwrap();

        let page = db
            .query_tracks_page("first", "", "", "", "title", false, 25, 0)
            .unwrap();
        assert_eq!(page.total, 1);
        assert!(page.items[0].stable_id.starts_with("track-"));
        assert_eq!(page.items[0].genre, "Electronic");
        assert_eq!(page.items[0].year, 2026);

        let playlist = db.create_playlist("Portable").unwrap();
        db.add_track_to_playlist(playlist, first).unwrap();
        let manifests = db.playlist_manifests().unwrap();
        assert_eq!(manifests.len(), 1);
        assert!(manifests[0].stable_id.starts_with("playlist-"));
        assert_eq!(manifests[0].track_ids.len(), 1);
        assert!(manifests[0].revision >= 2);
        let playlist_stable_id = manifests[0].stable_id.clone();
        db.delete_playlist(playlist).unwrap();
        let tombstones = db.playlist_tombstones().unwrap();
        assert_eq!(tombstones.len(), 1);
        assert_eq!(tombstones[0].stable_id, playlist_stable_id);
        assert!(tombstones[0].revision > manifests[0].revision);
    }

    #[test]
    fn reference_sources_keep_safe_metadata_overrides_and_never_delete_files() {
        let root = std::env::temp_dir().join(format!(
            "musicos-db-test-reference-source-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let original = root.join("original.wav");
        std::fs::write(&original, b"unchanged audio bytes").unwrap();
        let db = Db::open(root.join("media.sqlite3").to_str().unwrap()).unwrap();
        let source = db
            .add_library_source("reference", root.to_str().unwrap(), "Studio archive")
            .unwrap();
        let track = db
            .insert_track(
                original.to_str().unwrap(),
                "Original title",
                "Original artist",
                "Original album",
                120,
                21,
                10,
            )
            .unwrap();
        db.assign_track_source(track, source.id).unwrap();
        db.update_track_metadata_override(
            &[track],
            &MetadataUpdate {
                title: Some("Library title".to_string()),
                artist: Some("Library artist".to_string()),
                ..MetadataUpdate::default()
            },
        )
        .unwrap();

        let page = db
            .query_tracks_page("library title", "", "", "", "title", false, 25, 0)
            .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].track.title, "Library title");
        assert_eq!(page.items[0].source_kind.as_deref(), Some("reference"));
        assert!(page.items[0].metadata_overridden);
        assert_eq!(std::fs::read(&original).unwrap(), b"unchanged audio bytes");

        db.mark_library_source(source.id, false, Some("Folder is unavailable"))
            .unwrap();
        assert!(!db.library_source(source.id).unwrap().unwrap().available);
        assert!(db.remove_library_source(source.id).unwrap());
        assert!(original.is_file());
        assert!(db.get_track_by_id(track).unwrap().is_some());
        assert!(!db.library_source(source.id).unwrap().unwrap().enabled);
        let managed = db
            .library_sources()
            .unwrap()
            .into_iter()
            .find(|source| source.kind == "managed")
            .unwrap();
        assert!(db.remove_library_source(managed.id).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn relocation_verifies_bytes_and_preserves_disabled_records() {
        let db=temp_db("relocate");let root=std::env::temp_dir().join(format!("musicos-relocate-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);let old=root.join("old");let new=root.join("new");std::fs::create_dir_all(&old).unwrap();std::fs::create_dir_all(&new).unwrap();let old=std::fs::canonicalize(old).unwrap();let new=std::fs::canonicalize(new).unwrap();
        std::fs::write(old.join("one.wav"),b"same").unwrap();std::fs::write(new.join("one.wav"),b"same").unwrap();std::fs::write(old.join("two.wav"),b"old!").unwrap();std::fs::write(new.join("two.wav"),b"bad!").unwrap();
        let source=db.add_library_source("reference",old.to_str().unwrap(),"Archive").unwrap();let mut ids=Vec::new();for name in ["one.wav","two.wav"]{let id=db.insert_track(old.join(name).to_str().unwrap(),name,"Artist","Album",60,4,1).unwrap();db.assign_track_source(id,source.id).unwrap();ids.push(id);}
        let playlist=db.create_playlist("Keep membership").unwrap();db.add_tracks_to_playlist(playlist,&ids).unwrap();db.toggle_favorite(ids[0]).unwrap();db.update_track_metadata_override(&ids,&MetadataUpdate{title:Some("Keep override".into()),..Default::default()}).unwrap();db.remove_library_source(source.id).unwrap();assert_eq!(db.library_source_track_count(source.id).unwrap(),2);assert_eq!(db.relocate_library_source(source.id,&new).unwrap(),(1,1));
        assert!(!db.library_source(source.id).unwrap().unwrap().enabled);assert_eq!(db.playlist_track_count(playlist).unwrap(),2);db.update_library_source(source.id,None,Some(true)).unwrap();let first=db.get_track_by_id(ids[0]).unwrap().unwrap();assert!(first.favorited);assert_eq!(first.title,"Keep override");assert_eq!(first.path,new.join("one.wav").to_string_lossy());assert_eq!(db.get_track_by_id(ids[1]).unwrap().unwrap().path,old.join("two.wav").to_string_lossy());assert_eq!(std::fs::read(old.join("one.wav")).unwrap(),b"same");let _=std::fs::remove_dir_all(root);
    }

    #[test]
    fn user_metadata_prevents_later_automatic_replacement() {
        let db = temp_db("metadata-provenance");
        let track = db
            .insert_track(
                "/music/provenance.ogg",
                "Unclear title",
                "Unclear artist",
                "Unclear album",
                180,
                10,
                20,
            )
            .unwrap();
        assert!(db
            .apply_automatic_metadata_override(
                track,
                &MetadataUpdate {
                    title: Some("Matched title".to_string()),
                    artist: Some("Matched artist".to_string()),
                    ..MetadataUpdate::default()
                },
                "musicbrainz",
                0.96,
            )
            .unwrap());
        db.update_track_metadata_override(
            &[track],
            &MetadataUpdate {
                title: Some("My title".to_string()),
                ..MetadataUpdate::default()
            },
        )
        .unwrap();
        assert!(!db
            .apply_automatic_metadata_override(
                track,
                &MetadataUpdate {
                    title: Some("Provider changed title".to_string()),
                    ..MetadataUpdate::default()
                },
                "musicbrainz",
                1.0,
            )
            .unwrap());
        assert_eq!(
            db.get_track_by_id(track).unwrap().unwrap().title,
            "My title"
        );
        assert_eq!(
            db.metadata_override_origin(track).unwrap().as_deref(),
            Some("user")
        );
    }
}

include!("database_metadata_history.rs");
