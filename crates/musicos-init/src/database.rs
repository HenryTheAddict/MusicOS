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
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_album_title ON tracks(album COLLATE NOCASE, track_number, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_artist_title ON tracks(artist COLLATE NOCASE, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_tracks_favorited ON tracks(favorited, title COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist ON playlist_tracks(playlist_id, track_id);
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
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.id, t.path, t.title, t.artist, t.album, t.track_number, t.duration,
                    t.file_size, t.modified_at, t.indexed_at,
                    t.cover_art_path, t.cover_art_updated_at, t.cover_art_source_modified_at,
                    t.favorited, t.play_count, t.last_played
             FROM play_history h
             JOIN tracks t ON h.track_id = t.id
             ORDER BY h.played_at DESC LIMIT ?1",
        )?;
        let track_iter = stmt.query_map(params![limit], track_from_row)?;
        let mut list = Vec::new();
        for t in track_iter {
            list.push(t?);
        }
        Ok(list)
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

    pub fn add_track_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id) VALUES (?1, ?2)",
            params![playlist_id, track_id],
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
        Ok(())
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
             ORDER BY CASE WHEN t.track_number > 0 THEN 0 ELSE 1 END, t.track_number, t.title COLLATE NOCASE ASC"
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
