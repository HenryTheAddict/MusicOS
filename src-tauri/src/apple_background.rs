//! Opted-in, bounded background enrichment. Does not alter audio or embedded artwork.
use super::*;
use rusqlite::OptionalExtension;
use std::path::PathBuf;
fn sql(db: &Path) -> Result<rusqlite::Connection> {
    let c = store(db)?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS background_settings(id INTEGER PRIMARY KEY CHECK(id=1),enabled INTEGER NOT NULL);CREATE TABLE IF NOT EXISTS static_covers(album_id TEXT PRIMARY KEY,path TEXT NOT NULL,bytes INTEGER NOT NULL,checked INTEGER NOT NULL);CREATE TABLE IF NOT EXISTS cache_outcomes(track_id INTEGER PRIMARY KEY,lyrics TEXT,artwork TEXT,checked INTEGER NOT NULL)")?;
    Ok(c)
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn fatal(message: &str) -> bool {
    [
        "HTTP 401",
        "HTTP 403",
        "HTTP 429",
        "HTTP 502",
        "HTTP 503",
        "HTTP 504",
        "Could not reach",
        "cooling down",
    ]
    .iter()
    .any(|s| message.contains(s))
}
fn cover(db: &Path, id: i64) -> Result<bool> {
    let catalog = catalog_for(db, id)?;
    let c = sql(db)?;
    let known = c
        .query_row("SELECT album_id FROM links WHERE track_id=?", [id], |r| {
            r.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten();
    if let Some(album) = known.as_ref() {
        if c.query_row(
            "SELECT path FROM static_covers WHERE album_id=?",
            [album],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .is_some_and(|p| Path::new(&p).is_file())
        {
            return Ok(true);
        }
    }
    let creds = credentials(db)?;
    let row = get(
        &creds,
        &format!(
            "/v1/catalog/{}/songs/{}?include=albums",
            creds.storefront,
            digits(&catalog)?
        ),
    )?;
    let song = row
        .pointer("/data/0")
        .context("Matching artwork is unavailable")?;
    ensure!(
        song.pointer("/relationships/albums/data")
            .and_then(Value::as_array)
            .is_some_and(|v| v.len() == 1),
        "Artwork edition is ambiguous"
    );
    let album = song
        .pointer("/relationships/albums/data/0/id")
        .and_then(Value::as_str)
        .context("Album identity is unavailable")?;
    digits(album)?;
    c.execute(
        "UPDATE links SET album_id=? WHERE track_id=? AND catalog_id=?",
        rusqlite::params![album, id, catalog],
    )?;
    if c.query_row(
        "SELECT path FROM static_covers WHERE album_id=?",
        [album],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .is_some_and(|p| Path::new(&p).is_file())
    {
        return Ok(true);
    }
    let source = song
        .pointer("/attributes/artwork/url")
        .and_then(Value::as_str)
        .context("Static artwork is unavailable")?
        .replace("{w}", "800")
        .replace("{h}", "800")
        .replace("{f}", "jpg")
        .replace("{c}", "bb");
    let url = media_url(&source)?;
    // CDN requests never receive account credentials; redirects are disabled.
    let response = client()?.get(url).send()?;
    ensure!(
        response.status().is_success(),
        "Artwork download unavailable (HTTP {})",
        response.status().as_u16()
    );
    let mut bytes = vec![];
    response.take(12_000_001).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 12_000_000, "Artwork exceeds cache limit");
    let mut reader = image::io::Reader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::io::Limits::default();
    limits.max_alloc = Some(48 * 1024 * 1024);
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    reader.limits(limits);
    let image = reader.decode()?.thumbnail(800, 800).to_rgb8();
    let dir = db.with_file_name("apple-cover-cache-v1");
    ensure!(!dir.is_symlink(), "Choose a real cache folder");
    std::fs::create_dir_all(&dir)?;
    let target = dir.join(format!("{album}.jpg"));
    let tmp = dir.join(format!(".{album}-{}.tmp", rand::random::<u64>()));
    image.save_with_format(&tmp, image::ImageFormat::Jpeg)?;
    std::fs::rename(tmp, &target)?;
    c.execute(
        "INSERT OR REPLACE INTO static_covers VALUES(?,?,?,?)",
        rusqlite::params![
            album,
            target.to_string_lossy(),
            target.metadata()?.len(),
            now()
        ],
    )?;
    // Downloaded album images are durable library assets, not evictable renditions.
    Ok(true)
}
pub fn cached_cover(db: &Path, id: i64) -> Option<String> {
    let catalog = catalog_for(db, id).ok()?;
    let c = sql(db).ok()?;
    let path=c.query_row("SELECT static_covers.path FROM asset_links JOIN static_covers ON asset_links.album_id=static_covers.album_id WHERE asset_links.track_id=? AND asset_links.catalog_id=?",rusqlite::params![id,catalog],|r|r.get::<_,String>(0)).optional().ok().flatten()?;
    Path::new(&path).is_file().then_some(path)
}
pub(super) fn start(app: &tauri::AppHandle, db: &Path) -> Result<Value> {
    ensure!(
        !JOB.swap(true, Ordering::SeqCst),
        "Finish or cancel the current sync first"
    );
    let prepared = (|| -> Result<Vec<i64>> {
        let c = sql(db)?;
        let _ = credentials(db)?;
        let mut q = c.prepare("SELECT track_id FROM links ORDER BY track_id")?;
        let ids = q
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(!ids.is_empty(), "Match your local songs first");
        c.execute("INSERT OR REPLACE INTO background_settings VALUES(1,1)", [])?;
        Ok(ids)
    })();
    let ids = match prepared {
        Ok(ids) => ids,
        Err(e) => {
            JOB.store(false, Ordering::SeqCst);
            return Err(e);
        }
    };
    CANCEL.store(false, Ordering::SeqCst);
    {
        let mut s = state().lock().unwrap();
        s.phase = "background_cache".into();
        s.completed = 0;
        s.total = ids.len();
        s.lyrics_ready = 0;
        s.lyrics_failed = 0;
        s.artwork_ready = 0;
        s.message =
            "Caching matched lyrics and artwork in the background. You can leave this page.".into();
    }
    let app = app.clone();
    let db = db.to_owned();
    std::thread::spawn(move || {
        let _job = Job;
        let mut stopped = None;
        for id in &ids {
            if CANCEL.load(Ordering::SeqCst) {
                break;
            }
            let recent = sql(&db)
                .ok()
                .and_then(|c| {
                    c.query_row(
                        "SELECT lyrics,artwork,checked FROM cache_outcomes WHERE track_id=?",
                        [id],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, i64>(2)?,
                            ))
                        },
                    )
                    .optional()
                    .ok()
                    .flatten()
                })
                .filter(|v| now() - v.2 < 86400 && !edition_stale(&db,*id).unwrap_or(true));
            let result = if recent.as_ref().is_some_and(|v| v.0 == "unavailable") {
                Err(anyhow::anyhow!(
                    "No cached lyric match; skipped until explicit future refresh"
                ))
            } else {
                lyric(&db, *id)
            };
            let lyric_ok = result.is_ok();
            if let Err(e) = &result {
                if fatal(&e.to_string()) {
                    stopped = Some(e.to_string());
                    break;
                }
            }
            if CANCEL.load(Ordering::SeqCst) {
                break;
            }
            let art = if recent.as_ref().is_some_and(|v| v.1 == "unavailable") {
                Err(anyhow::anyhow!("Artwork unavailable"))
            } else {
                cover(&db, *id)
            };
            if let Err(e) = &art {
                if fatal(&e.to_string()) {
                    stopped = Some(e.to_string());
                    break;
                }
            }
            if art.is_ok() {
                crate::app::publish_cached_cover(&app, *id);
            }
            // Motion descriptors are optional. Unsupported motion never blocks static covers.
            if art.is_ok() && !CANCEL.load(Ordering::SeqCst) {
                let _ = artwork(&db, *id);
            }
            if let Ok(c) = sql(&db) {
                let _ = c.execute(
                    "INSERT OR REPLACE INTO cache_outcomes VALUES(?,?,?,?)",
                    rusqlite::params![
                        id,
                        if lyric_ok { "cached" } else { "unavailable" },
                        if art.is_ok() { "cached" } else { "unavailable" },
                        now()
                    ],
                );
            }
            {
                let mut s = state().lock().unwrap();
                s.completed += 1;
                if lyric_ok {
                    s.lyrics_ready += 1
                } else {
                    s.lyrics_failed += 1;
                }
                if art.is_ok() {
                    s.artwork_ready += 1;
                }
            }
            std::thread::sleep(Duration::from_millis(400));
        }
        let cancelled = CANCEL.load(Ordering::SeqCst);
        let files = if !cancelled {
            crate::lyric_sidecars::sync_tracks(&db, Some(&ids)).ok()
        } else {
            None
        };
        if let Some(ids) = files.as_ref().and_then(|f| f["ready_ids"].as_array()) {
            let _ = app.emit_to("main", "lyrics-sidecars-updated", ids);
        }
        if let Ok(c) = sql(&db) {
            let _ = c.execute("INSERT OR REPLACE INTO background_settings VALUES(1,0)", []);
        }
        let mut s = state().lock().unwrap();
        s.phase = if cancelled {
            "cancelled"
        } else if stopped.is_some() {
            "partial"
        } else {
            "ready"
        }
        .into();
        s.message = if let Some(error) = stopped {
            format!("Caching paused: {error}. Cached results are kept; resume explicitly.")
        } else {
            format!(
                "{} lyrics and {} artwork matches cached. {} unavailable lyrics. {}",
                s.lyrics_ready,
                s.artwork_ready,
                s.lyrics_failed,
                if cancelled {
                    "Cancelled; progress is retained."
                } else {
                    if files.is_some() {
                        "LyricsFile sync finished; existing edits kept."
                    } else {
                        "LyricsFile sync needs attention; cached lyrics remain available."
                    }
                }
            )
        };
        if let Some(files) = &files {
            s.message.push_str(&format!(
                " {} files saved, {} kept edits, {} file errors.",
                files["written"], files["conflicts"], files["failed"]
            ));
        }
        if s.message.contains("HTTP 401") {
            s.signed_in = false;
            s.sign_in = "unavailable".into();
        }
        let status = s.clone();
        drop(s);
        let _ = app.emit_to("main", "apple-enrichment-status", status);
    });
    Ok(json!({"status":state().lock().unwrap().clone()}))
}
pub(super) fn stop(db: &Path) -> Result<()> {
    CANCEL.store(true, Ordering::SeqCst);
    sql(db)?.execute("INSERT OR REPLACE INTO background_settings VALUES(1,0)", [])?;
    Ok(())
}
pub(super) fn clear(db:&Path)->Result<()> {
    // Disconnect cancels account work; downloaded library assets stay local.
    sql(db)?.execute_batch("DELETE FROM cache_outcomes;UPDATE background_settings SET enabled=0")?;
    Ok(())
}
pub fn resume(app: tauri::AppHandle, db: PathBuf) {
    std::thread::spawn(move || {
        let Ok(c) = sql(&db) else {
            return;
        };
        let enabled = c
            .query_row(
                "SELECT enabled FROM background_settings WHERE id=1",
                [],
                |r| r.get::<_, bool>(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(false);
        let matches = c
            .query_row("SELECT COUNT(*) FROM links", [], |r| r.get::<_, usize>(0))
            .unwrap_or(0);
        if matches > 0 && credentials(&db).is_ok() {
            let mut s = state().lock().unwrap();
            s.enabled = true;
            s.signed_in = true;
            s.sign_in = "stored".into();
            s.library = "available".into();
            s.matched = matches;
            s.phase = "ready".into();
            s.message =
                "Saved connection and cached matches. Online capabilities are checked when used."
                    .into();
            drop(s);
            if enabled {
                let _ = start(&app, &db);
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outage_classification_stops_jobs_without_retry_loops() {
        assert!(fatal("Apple Music returned HTTP 429"));
        assert!(fatal("Could not reach Apple Music"));
        assert!(fatal("Apple Music is cooling down"));
        assert!(!fatal("Apple Music has no supported timed lyrics"));
        assert!(!fatal("Apple Music returned HTTP 404"));
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn cached_cover_is_recording_scoped_and_stop_keeps_files() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("db");
        let audio = dir.path().join("song.flac");
        std::fs::write(&audio, b"audio").unwrap();
        let d = musicos_init::database::Db::open(&db.to_string_lossy()).unwrap();
        let id = d
            .insert_track(
                audio.to_str().unwrap(),
                "Song",
                "Artist",
                "Album",
                1,
                100,
                5,
            )
            .unwrap();
        let track = d.get_track_by_id(id).unwrap().unwrap();
        let c = sql(&db).unwrap();
        c.execute(
            "INSERT INTO links VALUES(?,?,?,?)",
            rusqlite::params![id, "123", "456", signature(&track)],
        )
        .unwrap();
        let cover = dir.path().join("cover.jpg");
        image::RgbImage::new(20, 20).save(&cover).unwrap();
        c.execute(
            "INSERT INTO static_covers VALUES(?,?,?,?)",
            rusqlite::params![
                "456",
                cover.to_string_lossy(),
                cover.metadata().unwrap().len(),
                now()
            ],
        )
        .unwrap();
        assert_eq!(cached_cover(&db, id).as_deref(), cover.to_str());
        clear(&db).unwrap();c.execute("DELETE FROM links",[]).unwrap();assert!(cover.exists());assert_eq!(cached_cover(&db,id).as_deref(),cover.to_str());
        stop(&db).unwrap();
        assert!(cover.is_file());
        assert!(d
            .get_track_by_id(id)
            .unwrap()
            .unwrap()
            .cover_art_path
            .is_none());
        d.update_track_metadata_override(
            &[id],
            &musicos_init::database::MetadataUpdate {
                title: Some("Another recording".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(cached_cover(&db, id).is_none());
        assert_eq!(std::fs::read(audio).unwrap(), b"audio");
    }
}
