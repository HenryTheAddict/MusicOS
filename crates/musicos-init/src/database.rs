use anyhow::Result;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
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
    pub fn open(path: &str) -> Result<Self> {
        let path_buf = PathBuf::from(path);
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path_buf)?;
        let db = Self {
            conn,
            path: path_buf,
        };
        db.apply_pragmas()?;
        db.init_schema()?;
        Ok(db)
    }

    fn apply_pragmas(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
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
                last_played INTEGER
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
            );",
        )?;
        self.ensure_column("tracks", "file_size", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "modified_at", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "indexed_at", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "track_number", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_column("tracks", "cover_art_path", "TEXT")?;
        self.ensure_column("tracks", "cover_art_updated_at", "INTEGER")?;
        self.ensure_column("tracks", "cover_art_source_modified_at", "INTEGER")?;
        self.ensure_column("playlist_tracks", "position", "INTEGER NOT NULL DEFAULT 0")?;
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_album_title ON tracks(album COLLATE NOCASE, track_number, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_artist_title ON tracks(artist COLLATE NOCASE, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_favorited ON tracks(favorited, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist ON playlist_tracks(playlist_id, position, track_id);
             CREATE INDEX IF NOT EXISTS idx_playlist_tracks_track ON playlist_tracks(track_id);
             CREATE INDEX IF NOT EXISTS idx_pins_lookup ON pins(pin_type, target_id);
             CREATE INDEX IF NOT EXISTS idx_play_history_recent ON play_history(played_at DESC, track_id);",
        )?;
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
        let mut stmt = self
            .conn
            .prepare_cached("SELECT file_size, modified_at FROM tracks WHERE path = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![path])?;
        if let Some(row) = rows.next()? {
            let saved_size: i64 = row.get(0)?;
            let saved_modified: i64 = row.get(1)?;
            return Ok(saved_size == file_size && saved_modified == modified_at);
        }
        Ok(false)
    }

    pub fn get_tracks(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played
             FROM tracks ORDER BY title COLLATE NOCASE ASC",
        )?;
        let track_iter = stmt.query_map([], track_from_row)?;

        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
    }

    pub fn get_track_by_id(&self, id: i64) -> Result<Option<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played
             FROM tracks WHERE id = ?1",
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

    pub fn get_favorites(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, path, title, artist, album, track_number, duration, file_size, modified_at, indexed_at,
                    cover_art_path, cover_art_updated_at, cover_art_source_modified_at,
                    favorited, play_count, last_played
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
                    t.favorited, t.play_count, t.last_played
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
        self.conn.execute(
            "INSERT OR IGNORE INTO playlists (name) VALUES (?1)",
            params![name],
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

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
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
            "UPDATE playlists SET name = ?1 WHERE id = ?2",
            params![name.trim(), id],
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
        self.conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, next_position],
        )?;
        Ok(())
    }

    pub fn playlist_track_count(&self, playlist_id: i64) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )?)
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        self.compact_playlist_positions(playlist_id)?;
        Ok(())
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

    pub fn get_playlist_tracks(&self, playlist_id: i64) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.path, t.title, t.artist, t.album, t.track_number, t.duration,
                    t.file_size, t.modified_at, t.indexed_at,
                    t.cover_art_path, t.cover_art_updated_at, t.cover_art_source_modified_at,
                    t.favorited, t.play_count, t.last_played
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
                    favorited, play_count, last_played
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
                    favorited, play_count, last_played
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
    })
}

#[cfg(test)]
mod tests {
    use super::Db;

    fn temp_db(name: &str) -> Db {
        let root =
            std::env::temp_dir().join(format!("musicos-db-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Db::open(root.join("media.sqlite3").to_str().unwrap()).unwrap()
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
        db.add_track_to_playlist(playlist, first).unwrap();
        db.add_track_to_playlist(playlist, second).unwrap();
        db.add_track_to_playlist(playlist, third).unwrap();

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
}
