//! Revision-checked companion changes. User edits and the idempotency journal commit together.
use anyhow::{ensure, Context, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
fn connection(path: &Path) -> Result<musicos_init::database::Connection> {
    let c = musicos_init::database::Connection::open(path)?;
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    c.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS desktop_companion_changes_v1(peer TEXT NOT NULL,id TEXT NOT NULL,response TEXT NOT NULL,PRIMARY KEY(peer,id)); CREATE TABLE IF NOT EXISTS desktop_companion_ids_v1(peer TEXT NOT NULL,phone_id TEXT NOT NULL,playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,PRIMARY KEY(peer,phone_id)); CREATE TABLE IF NOT EXISTS desktop_companion_sequences_v1(playlist_id INTEGER PRIMARY KEY REFERENCES playlists(id) ON DELETE CASCADE,base TEXT NOT NULL,ids TEXT NOT NULL);")?;
    Ok(c)
}
fn revision(name: &str, ids: &[i64]) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(name, ids)).unwrap())
    )
}
pub fn sequence(path: &Path, id: i64, name: &str, ids: Vec<i64>) -> Result<Vec<i64>> {
    let c = connection(path)?;
    let stored: Option<(String, String)> = c
        .query_row(
            "SELECT base,ids FROM desktop_companion_sequences_v1 WHERE playlist_id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(match stored {
        Some((base, data)) if base == revision(name, &ids) => serde_json::from_str(&data)?,
        _ => ids,
    })
}
pub fn collection_revision(name: &str, ids: &[i64]) -> String {
    revision(name, ids)
}
pub fn favorites(path: &Path) -> Result<Vec<(i64, bool)>> {
    let c = connection(path)?;
    let values = c
        .prepare("SELECT id,favorited FROM tracks WHERE deleted_at IS NULL")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(values)
}
pub fn apply(path: &Path, peer: &str, request: Value, shared: &[i64]) -> Result<Value> {
    let operation = request["id"].as_str().context("Missing change identity")?;
    ensure!(
        !operation.is_empty()
            && operation.len() <= 80
            && operation
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'-'),
        "Invalid change identity"
    );
    let mut c = connection(path)?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if let Some(saved) = tx
        .query_row(
            "SELECT response FROM desktop_companion_changes_v1 WHERE peer=? AND id=?",
            params![peer, operation],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(serde_json::from_str(&saved)?);
    }
    let force = request["force"].as_bool().unwrap_or(false);
    let response = match request["kind"].as_str() {
        Some("favorite") => {
            let id = request["track_id"].as_i64().context("Choose a song")?;
            ensure!(
                shared.contains(&id),
                "This song is not shared with this phone"
            );
            let previous = tx
                .query_row(
                    "SELECT favorited FROM tracks WHERE id=? AND deleted_at IS NULL",
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .optional()?
                .context("Song is no longer available")?;
            let value = request["value"]
                .as_bool()
                .context("Missing favorite value")?;
            if !force
                && previous
                    != request["base"]
                        .as_bool()
                        .context("Missing favorite revision")?
                && previous != value
            {
                json!({"state":"conflict","current":previous})
            } else {
                tx.execute(
                    "UPDATE tracks SET favorited=?, revision=revision+1 WHERE id=?",
                    params![value, id],
                )?;
                json!({"state":"saved","track_id":id,"favorited":value})
            }
        }
        Some("playlist") => {
            let supplied = request["collection_id"]
                .as_str()
                .context("Choose a playlist")?;
            let name = request["name"]
                .as_str()
                .context("Name the playlist")?
                .trim();
            ensure!(
                !name.is_empty()
                    && name.chars().count() <= 120
                    && !name.chars().any(char::is_control),
                "Use a name between 1 and 120 characters"
            );
            let ids = request["track_ids"]
                .as_array()
                .context("Choose songs")?
                .iter()
                .map(|v| v.as_i64().context("Invalid song identity"))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                ids.len() <= 10000 && ids.iter().all(|id| shared.contains(id)),
                "Only complete shared playlists can be edited from this phone"
            );
            let existing=match supplied.strip_prefix("playlist/").and_then(|v|v.parse::<i64>().ok()){Some(id)=>Some(id),None=>tx.query_row("SELECT playlist_id FROM desktop_companion_ids_v1 WHERE peer=? AND phone_id=?",params![peer,supplied],|r|r.get::<_,i64>(0)).optional()?};
            let mut expected = request["base_revision"].as_str().unwrap_or("").to_owned();
            if let Some(dependency) = request["depends_on"].as_str() {
                if let Some(response) = tx
                    .query_row(
                        "SELECT response FROM desktop_companion_changes_v1 WHERE peer=? AND id=?",
                        params![peer, dependency],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?
                {
                    let predecessor: Value = serde_json::from_str(&response)?;
                    if predecessor["state"] == "saved"
                        && predecessor["collection"]["id"]
                            .as_str()
                            .and_then(|id| id.strip_prefix("playlist/"))
                            .and_then(|id| id.parse::<i64>().ok())
                            == existing
                    {
                        expected = predecessor["collection"]["revision"]
                            .as_str()
                            .unwrap_or("")
                            .into();
                    }
                }
            }
            let mut conflict = false;
            let mut current = Value::Null;
            if let Some(id) = existing {
                let row: Option<(String, i64)> = tx
                    .query_row(
                        "SELECT name,revision FROM playlists WHERE id=?",
                        [id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?;
                let Some((current_name, _)) = row else {
                    anyhow::bail!(
                        "The Mac playlist was removed. Save the phone version as a new playlist."
                    )
                };
                let actual=tx.prepare("SELECT track_id FROM playlist_tracks WHERE playlist_id=? ORDER BY position,track_id")?.query_map([id],|r|r.get(0))?.collect::<std::result::Result<Vec<i64>,_>>()?;
                let stored: Option<(String, String)> = tx
                    .query_row(
                        "SELECT base,ids FROM desktop_companion_sequences_v1 WHERE playlist_id=?",
                        [id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?;
                let actual = match stored {
                    Some((base, data)) if base == revision(&current_name, &actual) => {
                        serde_json::from_str(&data)?
                    }
                    _ => actual,
                };
                let rev = revision(&current_name, &actual);
                current = json!({"id":supplied,"name":current_name,"track_ids":actual,"revision":rev,"kind":"manual","complete":true});
                ensure!(actual.iter().all(|id|shared.contains(id)),"This playlist is only partly shared. Share all its songs before editing it on the phone.");
                conflict = expected != rev && !(name == current_name && ids == actual);
                // Concurrent playlist edits preserve both versions. Force resolves by saving a copy.
            }
            if conflict && !force {
                json!({"state":"conflict","current":current})
            } else {
                let id = if let Some(id) = existing.filter(|_| !conflict) {
                    tx.execute("UPDATE playlists SET name=?,revision=revision+1,updated_at=unixepoch() WHERE id=?",params![name,id])?;
                    id
                } else {
                    let mut next = if conflict {
                        format!("{name} (iPhone)")
                    } else {
                        name.to_owned()
                    };
                    let stem = next.clone();
                    let mut suffix = 2;
                    while tx
                        .query_row("SELECT 1 FROM playlists WHERE name=?", [&next], |_| Ok(()))
                        .optional()?
                        .is_some()
                    {
                        next = format!("{stem} {suffix}");
                        suffix += 1;
                    }
                    tx.execute("INSERT INTO playlists(name,stable_id,revision,updated_at) VALUES (?,?,1,unixepoch())",params![next,format!("companion/{peer}/{operation}")])?;
                    tx.last_insert_rowid()
                };
                tx.execute("DELETE FROM playlist_tracks WHERE playlist_id=?", [id])?;
                for (position, track) in ids.iter().enumerate() {
                    tx.execute("INSERT OR IGNORE INTO playlist_tracks(playlist_id,track_id,position) VALUES (?,?,?)",params![id,track,position])?;
                }
                if supplied.starts_with("phone/") {
                    tx.execute("INSERT INTO desktop_companion_ids_v1 VALUES (?,?,?) ON CONFLICT(peer,phone_id) DO UPDATE SET playlist_id=excluded.playlist_id",params![peer,supplied,id])?;
                }
                let saved_name: String =
                    tx.query_row("SELECT name FROM playlists WHERE id=?", [id], |r| r.get(0))?;
                let unique=tx.prepare("SELECT track_id FROM playlist_tracks WHERE playlist_id=? ORDER BY position,track_id")?.query_map([id],|r|r.get(0))?.collect::<std::result::Result<Vec<i64>,_>>()?;
                tx.execute("INSERT INTO desktop_companion_sequences_v1 VALUES (?,?,?) ON CONFLICT(playlist_id) DO UPDATE SET base=excluded.base,ids=excluded.ids",params![id,revision(&saved_name,&unique),serde_json::to_string(&ids)?])?;
                json!({"state":"saved","collection":{"id":format!("playlist/{id}"),"name":saved_name,"kind":"manual","track_ids":ids,"revision":revision(&saved_name,&ids),"complete":true}})
            }
        }
        _ => anyhow::bail!("Unknown companion change"),
    };
    tx.execute(
        "INSERT INTO desktop_companion_changes_v1 VALUES (?,?,?)",
        params![peer, operation, serde_json::to_string(&response)?],
    )?;
    tx.commit()?;
    Ok(response)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revisions_duplicates_idempotency_and_conflicts() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("test.db");
        let db = musicos_init::database::Db::open(path.to_str().unwrap()).unwrap();
        drop(db);
        let c = connection(&path).unwrap();
        for id in 1..=3 {
            c.execute("INSERT INTO tracks(id,path,title,artist,album,duration) VALUES (?,?, 'Song','Artist','Album',10)",params![id,format!("/{id}.mp3")]).unwrap();
        }
        drop(c);
        let r = json!({"id":"create-1","kind":"playlist","collection_id":"phone/new","name":"Phone mix","track_ids":[1,2,1],"base_revision":""});
        let a = apply(&path, "phone", r.clone(), &[1, 2, 3]).unwrap();
        assert_eq!(a, apply(&path, "phone", r, &[1, 2, 3]).unwrap());
        let id = a["collection"]["id"]
            .as_str()
            .unwrap()
            .strip_prefix("playlist/")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            sequence(&path, id, "Phone mix", vec![1, 2]).unwrap(),
            vec![1, 2, 1]
        );
        let edit = json!({"id":"edit-1","kind":"playlist","collection_id":format!("playlist/{id}"),"name":"Phone mix","track_ids":[3,1],"base_revision":"stale"});
        assert_eq!(
            apply(&path, "phone", edit.clone(), &[1, 2, 3]).unwrap()["state"],
            "conflict"
        );
        assert_eq!(
            sequence(&path, id, "Phone mix", vec![1, 2]).unwrap(),
            vec![1, 2, 1]
        );
        let mut force = edit;
        force["id"] = json!("resolve-1");
        force["force"] = json!(true);
        let copy = apply(&path, "phone", force, &[1, 2, 3]).unwrap();
        assert_ne!(a["collection"]["id"], copy["collection"]["id"]);
    }
    #[test]
    fn offline_creation_chain_reuses_identity_and_checks_scope() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("test.db");
        drop(musicos_init::database::Db::open(path.to_str().unwrap()).unwrap());
        let c = connection(&path).unwrap();
        for id in 1..=3 {
            c.execute("INSERT INTO tracks(id,path,title,artist,album,duration) VALUES (?,?, 'Song','Artist','Album',10)",params![id,format!("/{id}.mp3")]).unwrap();
        }
        drop(c);
        let create = json!({"id":"create","kind":"playlist","collection_id":"phone/offline","name":"New mix","track_ids":[1,1],"base_revision":""});
        let first = apply(&path, "phone-a", create, &[1, 2]).unwrap();
        let edit = json!({"id":"edit","depends_on":"create","kind":"playlist","collection_id":"phone/offline","name":"Renamed","track_ids":[2,1,2],"base_revision":""});
        let second = apply(&path, "phone-a", edit.clone(), &[1, 2]).unwrap();
        assert_eq!(second["state"], "saved");
        assert_eq!(first["collection"]["id"], second["collection"]["id"]);
        assert_eq!(second, apply(&path, "phone-a", edit, &[1, 2]).unwrap());
        assert_eq!(
            connection(&path)
                .unwrap()
                .query_row("SELECT count(*) FROM playlists", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let invalid = json!({"id":"forbidden","kind":"playlist","collection_id":"phone/offline","name":"Invalid","track_ids":[3]});
        assert!(apply(&path, "phone-a", invalid, &[1, 2]).is_err());
        let favorite =
            json!({"id":"favorite","kind":"favorite","track_id":1,"base":false,"value":true});
        let saved = apply(&path, "phone-a", favorite.clone(), &[1, 2]).unwrap();
        assert_eq!(saved["favorited"], true);
        assert_eq!(saved, apply(&path, "phone-a", favorite, &[1, 2]).unwrap());
        assert_eq!(
            favorites(&path)
                .unwrap()
                .iter()
                .find(|(id, _)| *id == 1)
                .unwrap()
                .1,
            true
        );
    }
}
