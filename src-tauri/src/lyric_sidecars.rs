//! Explicit offline-only sync. Never replaces a sidecar edited outside musicOS.
use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Mutex};
static SYNC: Mutex<()> = Mutex::new(());
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write(
    c: &rusqlite::Connection,
    audio: &Path,
    doc: &crate::lyrics::Document,
) -> Result<&'static str> {
    anyhow::ensure!(audio.is_file(), "Audio file unavailable");
    let path = audio.with_extension("lyricsfile.yaml");
    anyhow::ensure!(!path.is_symlink(), "Sidecar is a symbolic link");
    let text = crate::lyrics::export(doc, "lyricsfile")?;
    let digest = hash(text.as_bytes());
    let old = c
        .query_row(
            "SELECT digest FROM sidecar_sync WHERE path=?",
            [path.to_string_lossy()],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        let current = hash(&bytes);
        if current == digest {
            return Ok("unchanged");
        }
        if old.as_deref() != Some(&current) {
            return Ok("conflict");
        }
    }
    let tmp = path.with_file_name(format!(".musicos-lyrics-{}.tmp", rand::random::<u64>()));
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    if let Err(e) = f.write_all(text.as_bytes()).and_then(|_| f.sync_all()) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    let result = (|| -> Result<()> {
        if path.exists() {
            let bytes = std::fs::read(&path)?;
            anyhow::ensure!(
                old.as_deref() == Some(hash(&bytes).as_str()),
                "Sidecar changed while syncing"
            );
            let backup = path.with_file_name(format!(
                "{}.{}.backup",
                path.file_name().unwrap().to_string_lossy(),
                rand::random::<u64>()
            ));
            std::fs::copy(&path, &backup)?;
            std::fs::rename(&tmp, &path)?;
        } else {
            std::fs::hard_link(&tmp, &path)
                .context("Could not create LyricsFile without replacing an existing file")?;
        }
        c.execute("INSERT INTO sidecar_sync VALUES(?,?) ON CONFLICT(path) DO UPDATE SET digest=excluded.digest",rusqlite::params![path.to_string_lossy(),digest])?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&tmp);
    result?;
    Ok("written")
}
pub fn sync(db: &Path) -> Result<serde_json::Value> {
    sync_tracks(db, None)
}
pub fn sync_tracks(db: &Path, ids: Option<&[i64]>) -> Result<serde_json::Value> {
    let _lock = SYNC
        .try_lock()
        .map_err(|_| anyhow::anyhow!("LyricsFile sync is already running"))?;
    let c = rusqlite::Connection::open(db.with_file_name("desktop-lyrics-v1.sqlite3"))?;
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS sidecar_sync(path TEXT PRIMARY KEY,digest TEXT NOT NULL)",
    )?;
    let tracks = musicos_init::database::Db::open(&db.to_string_lossy())?.get_tracks()?;
    let (mut written, mut unchanged, mut conflicts, mut missing, mut failed) = (0, 0, 0, 0, 0);
    let mut ready_ids = vec![];
    for t in tracks {
        if ids.is_some_and(|ids| !ids.contains(&t.id)) {
            continue;
        }
        let r = crate::lyrics::execute(
            db,
            crate::lyrics::Request {
                action: "load".into(),
                track_id: Some(t.id),
                ..Default::default()
            },
        );
        let r = match r {
            Ok(r) => r,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        let doc = r
            .get("document")
            .cloned()
            .filter(|v| !v.is_null())
            .map(serde_json::from_value::<crate::lyrics::Document>)
            .transpose();
        let doc = match doc {
            Ok(Some(d)) => Some(d),
            Ok(None) => crate::apple_enrichment::cached_lyric(db, t.id)
                .ok()
                .flatten(),
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        let Some(doc) = doc else {
            missing += 1;
            continue;
        };
        match write(&c, Path::new(&t.path), &doc) {
            Ok("written") => {
                written += 1;
                ready_ids.push(t.id);
            }
            Ok("unchanged") => {
                unchanged += 1;
                ready_ids.push(t.id);
            }
            Ok(_) => conflicts += 1,
            Err(_) => failed += 1,
        }
    }
    Ok(
        json!({"written":written,"unchanged":unchanged,"conflicts":conflicts,"missing":missing,"failed":failed,"ready_ids":ready_ids}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sync_preserves_external_edits_and_audio() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("song.flac");
        std::fs::write(&audio, b"audio").unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE sidecar_sync(path TEXT PRIMARY KEY,digest TEXT NOT NULL)")
            .unwrap();
        let mut d = crate::lyrics::Document::default();
        d.plain = "one".into();
        assert_eq!(write(&c, &audio, &d).unwrap(), "written");
        assert_eq!(write(&c, &audio, &d).unwrap(), "unchanged");
        d.plain = "two".into();
        assert_eq!(write(&c, &audio, &d).unwrap(), "written");
        let p = audio.with_extension("lyricsfile.yaml");
        std::fs::write(&p, "external edit").unwrap();
        d.plain = "three".into();
        assert_eq!(write(&c, &audio, &d).unwrap(), "conflict");
        assert_eq!(std::fs::read_to_string(p).unwrap(), "external edit");
        assert_eq!(std::fs::read(audio).unwrap(), b"audio");
    }
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    #[test]
    fn selected_lyrics_sync_offline_and_remain_authoritative() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("library.db");
        let audio = dir.path().join("song.flac");
        std::fs::write(&audio, b"original audio").unwrap();
        let database = musicos_init::database::Db::open(&db.to_string_lossy()).unwrap();
        let id = database
            .insert_track(
                audio.to_str().unwrap(),
                "Song",
                "Artist",
                "Album",
                1,
                60,
                14,
            )
            .unwrap();
        let mut doc = crate::lyrics::Document::default();
        doc.plain = "Selected words".into();
        let save = |doc: crate::lyrics::Document, revision| {
            crate::lyrics::execute(
                &db,
                crate::lyrics::Request {
                    action: "save".into(),
                    track_id: Some(id),
                    document: Some(doc),
                    revision: Some(revision),
                    ..Default::default()
                },
            )
            .unwrap()
        };
        save(doc.clone(), 0);
        assert_eq!(sync(&db).unwrap()["written"], 1);
        doc.plain = "Corrected words".into();
        save(doc.clone(), 1);
        assert_eq!(sync(&db).unwrap()["written"], 1);
        let sidecar = audio.with_extension("lyricsfile.yaml");
        assert_eq!(
            crate::lyrics::parse(&std::fs::read_to_string(&sidecar).unwrap(), "lyricsfile")
                .unwrap()
                .plain,
            "Corrected words"
        );
        std::fs::write(&sidecar, "Outside edit").unwrap();
        assert_eq!(sync(&db).unwrap()["conflicts"], 1);
        assert_eq!(std::fs::read_to_string(sidecar).unwrap(), "Outside edit");
        assert_eq!(std::fs::read(audio).unwrap(), b"original audio");
        let loaded = crate::lyrics::execute(
            &db,
            crate::lyrics::Request {
                action: "load".into(),
                track_id: Some(id),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(loaded["document"]["plain"], "Corrected words");
        assert_eq!(loaded["revision"], 2);
    }
}
