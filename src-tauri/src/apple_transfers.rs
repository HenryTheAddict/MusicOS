//! Explicit cached, read-only Apple discovery; only complete playlists may become local collections.
use super::*;
use rusqlite::OptionalExtension;
use std::collections::{HashMap, HashSet};
fn cache(db: &Path) -> Result<rusqlite::Connection> {
    let c = store(db)?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS transfer_cache(id TEXT PRIMARY KEY,payload TEXT NOT NULL);CREATE TABLE IF NOT EXISTS transferred_playlists(id TEXT PRIMARY KEY,playlist_id INTEGER NOT NULL)")?;
    Ok(c)
}
fn pages(c: &Credentials, first: &str, official: &mut Option<bool>) -> Result<Vec<Value>> {
    let mut next = Some(first.to_owned());
    let mut seen = HashSet::new();
    let mut rows = vec![];
    while let Some(path) = next {
        ensure!(
            !CANCEL.load(Ordering::SeqCst),
            "Refresh cancelled; cached collections retained"
        );
        ensure!(
            seen.len() < 250 && seen.insert(path.clone()),
            "Apple pagination did not finish"
        );
        let value = library_page(c, &path, official)?;
        let data = value["data"]
            .as_array()
            .context("Unsupported Apple collection response")?;
        rows.extend(data.iter().cloned());
        ensure!(rows.len() <= 25000, "Apple collection is too large");
        next = value["next"].as_str().map(str::to_owned);
        if next.is_some() {
            std::thread::sleep(Duration::from_millis(300));
        }
    }
    Ok(rows)
}
fn safe_id(id: &str) -> Result<&str> {
    ensure!(
        !id.is_empty()
            && id.len() < 160
            && id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || v == b'.' || v == b'-'),
        "Invalid playlist identity"
    );
    Ok(id)
}
fn local_map(db: &Path) -> Result<HashMap<String, Vec<i64>>> {
    let c = store(db)?;
    let tracks = musicos_init::database::Db::open(&db.to_string_lossy())?.get_tracks()?;
    let mut map: HashMap<String, Vec<i64>> = HashMap::new();
    for t in tracks {
        if !Path::new(&t.path).is_file() {
            continue;
        }
        let cat = c
            .query_row(
                "SELECT catalog_id FROM links WHERE track_id=? AND signature=?",
                rusqlite::params![t.id, signature(&t)],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        if let Some(cat) = cat {
            map.entry(cat).or_default().push(t.id);
        }
    }
    Ok(map)
}
fn resolve(items: &[Value], map: &HashMap<String, Vec<i64>>) -> (Vec<i64>, usize, usize) {
    let (mut ids, mut missing, mut ambiguous) = (vec![], 0, 0);
    for item in items {
        let cat = catalog_record(item).and_then(|v| v["id"].as_str().map(str::to_owned));
        match cat.as_ref().and_then(|v| map.get(v)) {
            Some(v) if v.len() == 1 => ids.push(v[0]),
            Some(_) => ambiguous += 1,
            None => missing += 1,
        }
    }
    (ids, missing, ambiguous)
}
fn missing_albums(items: &[Value], map: &HashMap<String, Vec<i64>>) -> Vec<Value> {
    let mut albums = std::collections::BTreeMap::<(String, String), Vec<String>>::new();
    for item in items {
        let row = catalog_record(item).unwrap_or_else(|| item.clone());
        let cat = row["id"].as_str().unwrap_or("");
        if map.get(cat).is_some_and(|ids| ids.len() == 1) {
            continue;
        }
        let a = &row["attributes"];
        let album = a["albumName"]
            .as_str()
            .unwrap_or("Album not identified")
            .to_owned();
        let artist = a["artistName"]
            .as_str()
            .unwrap_or("Unknown artist")
            .to_owned();
        albums
            .entry((album, artist))
            .or_default()
            .push(a["name"].as_str().unwrap_or("Unidentified song").to_owned());
    }
    albums.into_iter().map(|((album,artist),songs)|json!({"album":album,"artist":artist,"needed":songs.len(),"songs":songs.into_iter().take(8).collect::<Vec<_>>()})).collect()
}
pub(super) fn snapshot(db: &Path) -> Result<Value> {
    let c = cache(db)?;
    let map = local_map(db)?;
    let mut q = c.prepare("SELECT id,payload FROM transfer_cache ORDER BY id")?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut out = vec![];
    let mut playlist_total = 0;
    for (id, raw) in rows {
        let v: Value = serde_json::from_str(&raw)?;
        if id == "_summary" {
            playlist_total = v["total"].as_u64().unwrap_or(0);
            continue;
        }
        let items = v["tracks"].as_array().cloned().unwrap_or_default();
        let (ids, missing, ambiguous) = resolve(&items, &map);
        let duplicates = ids.len() - ids.iter().collect::<HashSet<_>>().len();
        let imported = c
            .query_row(
                "SELECT playlist_id FROM transferred_playlists WHERE id=?",
                [&id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .filter(|id| {
                musicos_init::database::Db::open(&db.to_string_lossy())
                    .ok()
                    .and_then(|d| d.get_playlists().ok())
                    .is_some_and(|ps| ps.iter().any(|p| p.id == *id))
            });
        out.push(json!({"id":id,"name":v["name"],"kind":v["kind"],"total":items.len(),"matched":ids.len(),"missing":missing,"ambiguous":ambiguous,"duplicates":duplicates,"available":v["available"],"reason":v["reason"],"ready":v["available"]==true&&!ids.is_empty()&&missing==0&&ambiguous==0&&duplicates==0,"imported":imported.is_some(),"checked":v["checked"].as_bool().unwrap_or(true),"rank":v["rank"],"missingAlbums":missing_albums(&items,&map),"resources":v["resources"]}));
    }
    out.sort_by_key(|v| {
        (
            v["kind"] == "listening",
            v["rank"].as_u64().unwrap_or(u64::MAX),
        )
    });
    let mut local = musicos_init::database::Db::open(&db.to_string_lossy())?.get_tracks()?;
    local.retain(|t| t.play_count > 0 && Path::new(&t.path).is_file());
    local.sort_by(|a, b| {
        b.play_count
            .cmp(&a.play_count)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(
        json!({"transfers":out,"playlistTotal":playlist_total,"mostPlayed":local.into_iter().take(25).map(|t|json!({"trackId":t.id,"title":t.title,"artist":t.artist,"plays":t.play_count})).collect::<Vec<_>>() }),
    )
}
fn top_playlists(mut rows: Vec<Value>) -> (Vec<Value>, usize) {
    let total = rows.len();
    rows.sort_by(|a, b| {
        b.pointer("/attributes/lastModifiedDate")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(
                a.pointer("/attributes/lastModifiedDate")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
    });
    rows.truncate(8);
    (rows, total)
}
fn discover(db: &Path) -> Result<Value> {
    ensure!(
        !JOB.swap(true, Ordering::SeqCst),
        "A sync is already running"
    );
    let _job = Job;
    CANCEL.store(false, Ordering::SeqCst);
    let credentials = credentials(db)?;
    let mut official = None;
    {
        let mut s = state().lock().unwrap();
        s.phase = "collections_sync".into();
        s.message = "Reading playlists and listening summaries…".into();
        s.completed = 0;
    }
    let playlists = pages(
        &credentials,
        "/v1/me/library/playlists?limit=100",
        &mut official,
    )?;
    let (playlists, total) = top_playlists(playlists);
    let mut pending = vec![("_summary".into(), json!({"total":total}))];
    let existing_cache = cache(db)?;
    for (rank, p) in playlists.into_iter().enumerate() {
        let id = p["id"].as_str().context("Missing playlist identity")?;
        safe_id(id)?;
        let name = p
            .pointer("/attributes/name")
            .and_then(Value::as_str)
            .unwrap_or("Apple playlist");
        // Only the eight headers are prepared. Track pages load for one explicitly checked playlist.
        let modified = p
            .pointer("/attributes/lastModifiedDate")
            .and_then(Value::as_str)
            .unwrap_or("");
        let cached = existing_cache
            .query_row(
                "SELECT payload FROM transfer_cache WHERE id=?",
                [format!("playlist:{id}")],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
        let mut payload=cached.filter(|v|!modified.is_empty()&&v["modified"]==modified&&v["checked"]==true).unwrap_or_else(||json!({"kind":"playlist","tracks":[],"checked":false,"available":false,"reason":"Check songs before transfer. Every entry must be available locally."}));
        payload["name"] = json!(name);
        payload["modified"] = json!(modified);
        payload["rank"] = json!(rank);
        pending.push((format!("playlist:{id}"), payload));
        state().lock().unwrap().completed += 1;
    }
    for (id, name, path) in [
        (
            "recent",
            "Recently played",
            "/v1/me/recent/played/tracks?limit=30",
        ),
        (
            "rotation",
            "In heavy rotation",
            "/v1/me/history/heavy-rotation?limit=25",
        ),
    ] {
        let result = pages(&credentials, path, &mut official);
        if CANCEL.load(Ordering::SeqCst) {
            anyhow::bail!("Refresh cancelled; cached collections retained");
        }
        // Heavy rotation may contain albums/playlists: never pretend those are tracks or play counts.
        let payload = match result {
            Ok(rows) => {
                let count = rows.len();
                let resources=rows.iter().take(50).map(|v|json!({"title":v.pointer("/attributes/name").and_then(Value::as_str).unwrap_or("Untitled"),"artist":v.pointer("/attributes/artistName").and_then(Value::as_str).unwrap_or(""),"kind":v["type"].as_str().unwrap_or("content")})).collect::<Vec<_>>();
                let tracks = rows
                    .into_iter()
                    .filter(|v| matches!(v["type"].as_str(), Some("songs" | "library-songs")))
                    .collect::<Vec<_>>();
                let omitted = count - tracks.len();
                json!({"name":name,"kind":"listening","tracks":tracks,"resources":resources,"available":omitted==0,"reason":if omitted>0{"Apple returned albums or other content; a complete song sequence is unavailable."}else{"Apple order; no play counts imported."}})
            }
            Err(_) => {
                json!({"name":name,"kind":"listening","tracks":[],"available":false,"reason":"This listening summary is unavailable for this account."})
            }
        };
        pending.push((id.into(), payload));
    }
    ensure!(
        !CANCEL.load(Ordering::SeqCst),
        "Refresh cancelled; cached collections retained"
    );
    let mut c = cache(db)?;
    let tx = c.transaction()?;
    tx.execute("DELETE FROM transfer_cache", [])?;
    for (id, payload) in pending {
        tx.execute(
            "INSERT INTO transfer_cache VALUES(?,?)",
            rusqlite::params![id, serde_json::to_string(&payload)?],
        )?;
    }
    tx.commit()?;
    {
        let mut s = state().lock().unwrap();
        s.phase = "ready".into();
        s.message="Collections cached. Transfer only complete local matches; Apple play counts stay separate.".into();
    }
    snapshot(db)
}
fn check_playlist(db: &Path, key: &str) -> Result<Value> {
    ensure!(
        !JOB.swap(true, Ordering::SeqCst),
        "Finish the current sync first"
    );
    let _job = Job;
    CANCEL.store(false, Ordering::SeqCst);
    let id = key
        .strip_prefix("playlist:")
        .context("Choose an Apple playlist")?;
    safe_id(id)?;
    let c = cache(db)?;
    let raw = c
        .query_row(
            "SELECT payload FROM transfer_cache WHERE id=?",
            [key],
            |r| r.get::<_, String>(0),
        )
        .context("Refresh your playlist choices first")?;
    let mut value: Value = serde_json::from_str(&raw)?;
    let mut official = None;
    let tracks = pages(
        &credentials(db)?,
        &format!("/v1/me/library/playlists/{id}/tracks?limit=100&include=catalog"),
        &mut official,
    )?;
    ensure!(
        !CANCEL.load(Ordering::SeqCst),
        "Check cancelled; previous cache retained"
    );
    value["tracks"] = json!(tracks);
    value["checked"] = json!(true);
    value["available"] = json!(true);
    value["reason"] =
        json!("Cached song sequence; local availability is checked again before transfer.");
    c.execute(
        "UPDATE transfer_cache SET payload=? WHERE id=?",
        rusqlite::params![serde_json::to_string(&value)?, key],
    )?;
    snapshot(db)
}
fn transfer(db: &Path, id: &str) -> Result<Value> {
    ensure!(
        !JOB.swap(true, Ordering::SeqCst),
        "Finish the current sync first"
    );
    let _job = Job;
    let c = cache(db)?;
    let raw = c
        .query_row("SELECT payload FROM transfer_cache WHERE id=?", [id], |r| {
            r.get::<_, String>(0)
        })
        .context("Refresh collections first")?;
    let v: Value = serde_json::from_str(&raw)?;
    ensure!(v["available"] == true, "A complete sequence is unavailable");
    let tracks = v["tracks"]
        .as_array()
        .context("Playlist contents unavailable")?;
    let (ids, missing, ambiguous) = resolve(tracks, &local_map(db)?);
    ensure!(
        !ids.is_empty() && missing == 0 && ambiguous == 0,
        "Every song must match an available local file"
    );
    ensure!(
        ids.iter().collect::<HashSet<_>>().len() == ids.len(),
        "Repeated songs cannot be represented in the current local playlist format"
    );
    let d = musicos_init::database::Db::open(&db.to_string_lossy())?;
    if let Some(existing) = c
        .query_row(
            "SELECT playlist_id FROM transferred_playlists WHERE id=?",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
    {
        ensure!(
            !d.get_playlists()?.iter().any(|p| p.id == existing),
            "Already transferred; your local edits are preserved"
        );
    }
    let base = v["name"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Apple playlist")
        .chars()
        .take(95)
        .collect::<String>();
    d.begin_transaction()?;
    let created = (|| -> Result<i64> {
        let existing = d.get_playlists()?;
        let mut name = base.clone();
        let mut n = 2;
        while existing.iter().any(|p| p.name == name) {
            name = format!("{base} (Apple {n})");
            n += 1;
        }
        let playlist = d.create_playlist(&name)?;
        for track in &ids {
            d.add_track_to_playlist(playlist, *track)?;
        }
        d.commit_transaction()?;
        Ok(playlist)
    })();
    let playlist = match created {
        Ok(id) => id,
        Err(e) => {
            d.rollback_transaction()?;
            return Err(e);
        }
    };
    if let Err(e) = c.execute(
        "INSERT OR REPLACE INTO transferred_playlists VALUES(?,?)",
        rusqlite::params![id, playlist],
    ) {
        d.delete_playlist(playlist)?;
        return Err(e.into());
    }
    snapshot(db)
}
pub(super) fn action(db: &Path, action: &str, id: Option<&str>) -> Result<Value> {
    match action {
        "transfers" => snapshot(db),
        "refresh_transfers" => discover(db),
        "check_playlist" => check_playlist(db, id.context("Choose a playlist")?),
        "transfer_playlist" => transfer(db, id.context("Choose a playlist")?),
        _ => anyhow::bail!("Unknown transfer action"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_album_groups_only_include_unresolved_entries() {
        let mut map = HashMap::new();
        map.insert("1".into(), vec![10]);
        let rows = vec![
            json!({"id":"1","attributes":{"name":"Have","albumName":"A","artistName":"X"}}),
            json!({"id":"2","attributes":{"name":"Need","albumName":"A","artistName":"X"}}),
        ];
        let groups = missing_albums(&rows, &map);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["needed"], 1);
        assert_eq!(groups[0]["songs"], json!(["Need"]));
    }
    #[test]
    fn top_eight_headers_do_not_expand_four_hundred_playlists() {
        let rows=(0..400).map(|i|json!({"id":format!("p.{i}"),"attributes":{"lastModifiedDate":format!("2026-10-{:02}T00:00:00Z",i%30+1)}})).collect();
        let (top, total) = top_playlists(rows);
        assert_eq!(total, 400);
        assert_eq!(top.len(), 8);
        assert!(top.iter().all(|p| p
            .pointer("/attributes/lastModifiedDate")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("10-30")));
    }
    #[test]
    fn strict_sequence_matches_require_every_recording() {
        let mut m = HashMap::new();
        m.insert("1".into(), vec![10]);
        m.insert("2".into(), vec![20, 21]);
        let rows = vec![
            json!({"id":"1"}),
            json!({"id":"3"}),
            json!({"id":"2"}),
            json!({"id":"1"}),
        ];
        let (ids, missing, ambiguous) = resolve(&rows, &m);
        assert_eq!(ids, vec![10, 10]);
        assert_eq!((missing, ambiguous), (1, 1));
    }
    #[test]
    fn transfer_is_all_or_nothing_and_idempotent() {
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
        let t = d.get_track_by_id(id).unwrap().unwrap();
        let c = cache(&db).unwrap();
        c.execute(
            "INSERT INTO links VALUES(?,?,NULL,?)",
            rusqlite::params![id, "1", signature(&t)],
        )
        .unwrap();
        let save = |tracks: Value| {
            c.execute(
                "INSERT OR REPLACE INTO transfer_cache VALUES('test',?)",
                [json!({"name":"Test","available":true,"tracks":tracks}).to_string()],
            )
            .unwrap();
        };
        save(json!([{"id":"1"},{"id":"3"}]));
        assert!(transfer(&db, "test").is_err());
        assert!(d.get_playlists().unwrap().is_empty());
        save(json!([{"id":"1"}]));
        transfer(&db, "test").unwrap();
        assert_eq!(d.get_playlists().unwrap().len(), 1);
        assert!(transfer(&db, "test").is_err());
        assert_eq!(
            d.get_playlist_tracks(d.get_playlists().unwrap()[0].id)
                .unwrap()[0]
                .id,
            id
        );
        assert_eq!(std::fs::read(audio).unwrap(), b"audio");
    }
}

pub(super) fn clear(db: &Path) -> Result<()> {
    cache(db)?.execute_batch("DELETE FROM transfer_cache;DELETE FROM transferred_playlists;")?;
    Ok(())
}
