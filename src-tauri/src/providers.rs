use anyhow::Result;
use musicos_desktop::storage::{atomic_write_json, desktop_data_dir};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime},
};
const UA: &str = "MusicOS/0.1 (https://github.com/HenryTheAddict/MusicOS)";
fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .build()?)
}
fn cache_path(key: &str) -> PathBuf {
    desktop_data_dir()
        .join("catalog-v1")
        .join(format!("{:x}.json", Sha256::digest(key.as_bytes())))
}
static LIMITS: OnceLock<Mutex<std::collections::HashMap<String, Instant>>> = OnceLock::new();
fn cached_json(url: &str, params: &[(&str, &str)], provider: &str) -> Result<Value> {
    let key = format!("{url}:{params:?}");
    let path = cache_path(&key);
    if let Ok(meta) = std::fs::metadata(&path) {
        if !REVALIDATE.with(|v| v.get())
            && meta.modified()?.elapsed().unwrap_or_default() < Duration::from_secs(86400)
        {
            if let Ok(v) = serde_json::from_slice::<Value>(&std::fs::read(&path)?) {
                if !v.get("results").and_then(Value::as_array).is_some_and(Vec::is_empty)||meta.modified()?.elapsed().unwrap_or_default()<Duration::from_secs(60){return Ok(v);}
            }
        }
    }
    let interval = if provider == "apple" {
        Duration::from_millis(3100)
    } else {
        Duration::from_millis(1100)
    };
    let mut limits = LIMITS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("Catalog request state unavailable"))?;
    if let Some(last) = limits.get(provider) {
        let wait = (*last + interval).saturating_duration_since(Instant::now());
        anyhow::ensure!(
            wait <= interval,
            "Provider is temporarily unavailable; retry later"
        );
        std::thread::sleep(wait)
    }
    limits.insert(provider.into(), Instant::now());
    let response = client()?.get(url).query(params).send()?;
    if response.status().as_u16() == 429 || response.status().as_u16() == 503 {
        let wait = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60)
            .min(300);
        limits.insert(provider.into(), Instant::now() + Duration::from_secs(wait));
    }
    let v = response.error_for_status()?.json::<Value>()?;
    drop(limits);
    atomic_write_json(&path, &v)?;
    Ok(v)
}
async fn cancellable<T>(
    future: impl std::future::Future<Output = T>,
    generation: &AtomicU64,
    token: u64,
) -> Result<T> {
    tokio::pin!(future);
    loop {
        anyhow::ensure!(
            generation.load(Ordering::Relaxed) == token,
            "Search cancelled"
        );
        tokio::select! {
            result = &mut future => return Ok(result),
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }
}
async fn catalog_json(
    url: &str,
    params: &[(&str, &str)],
    generation: &AtomicU64,
    token: u64,
) -> Result<Value> {
    let key = format!("{url}:{params:?}");
    let path = cache_path(&key);
    if std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .is_some_and(|t| t.elapsed().unwrap_or_default() < Duration::from_secs(86400))
    {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(v) = serde_json::from_slice(&bytes) {
                return Ok(v);
            }
        }
    }
    static GATE: tokio::sync::Mutex<Option<Instant>> = tokio::sync::Mutex::const_new(None);
    let mut last = cancellable(GATE.lock(), generation, token).await?;
    if let Some(at) = *last {
        cancellable(
            tokio::time::sleep(Duration::from_millis(3100).saturating_sub(at.elapsed())),
            generation,
            token,
        )
        .await?;
    }
    *last = Some(Instant::now());
    let client = reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    let response = cancellable(client.get(url).query(params).send(), generation, token)
        .await??
        .error_for_status()?;
    let value = cancellable(response.json::<Value>(), generation, token).await??;
    drop(last);
    atomic_write_json(&path, &value)?;
    Ok(value)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<String>,
    pub album_id: Option<String>,
    pub album: Option<String>,
    pub cover: Option<String>,
    pub genre: Option<String>,
    pub duration: Option<i64>,
    pub track_number: Option<i64>,
    pub store_url: Option<String>,
    pub local: bool,
}
fn string(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}
pub fn normalize_catalog(v: &Value) -> Vec<CatalogItem> {
    v["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| {
            let (kind, id, title) = if v["wrapperType"] == "artist" {
                ("artist", v["artistId"].as_u64()?, string(v, "artistName")?)
            } else if v["kind"] == "song" {
                ("song", v["trackId"].as_u64()?, string(v, "trackName")?)
            } else if v["wrapperType"] == "collection" {
                (
                    "album",
                    v["collectionId"].as_u64()?,
                    string(v, "collectionName")?,
                )
            } else {
                return None;
            };
            Some(CatalogItem {
                id: format!("apple/{kind}/{id}"),
                kind: kind.into(),
                title,
                artist: string(v, "artistName").unwrap_or_default(),
                artist_id: v["artistId"].as_u64().map(|n| format!("apple/artist/{n}")),
                album_id: v["collectionId"]
                    .as_u64()
                    .map(|n| format!("apple/album/{n}")),
                album: string(v, "collectionName"),
                cover: string(v, "artworkUrl100").map(|s| s.replace("100x100bb", "600x600bb")),
                genre: string(v, "primaryGenreName"),
                duration: v["trackTimeMillis"].as_i64().map(|n| n / 1000),
                track_number: v["trackNumber"].as_i64(),
                store_url: string(
                    v,
                    if kind == "song" {
                        "trackViewUrl"
                    } else if kind == "artist" {
                        "artistLinkUrl"
                    } else {
                        "collectionViewUrl"
                    },
                ),
                local: false,
            })
        })
        .collect()
}
pub async fn discover(
    term: &str,
    kind: &str,
    lookup: Option<&str>,
    generation: &AtomicU64,
    token: u64,
) -> Result<Vec<CatalogItem>> {
    anyhow::ensure!(term.chars().count() <= 200, "Search is too long.");
    let genre_id = match term.to_lowercase().as_str() {
        "alternative" => Some("20"),
        "electronic" => Some("7"),
        "rock" => Some("21"),
        "pop" => Some("14"),
        "hip-hop/rap" => Some("18"),
        "r&b/soul" => Some("15"),
        "jazz" => Some("11"),
        "classical" => Some("5"),
        "country" => Some("6"),
        "dance" => Some("17"),
        _ => None,
    };
    let search_term = if kind == "genres" {
        genre_id.unwrap_or(term)
    } else {
        term
    };
    let entity = match kind {
        "artists" => "musicArtist",
        "songs" => "song",
        "albums" | "genres" => "album",
        _ => anyhow::bail!("Choose Albums, Artists, Genres, or Songs."),
    };
    let v = if let Some(id) = lookup {
        let id = id.rsplit('/').next().unwrap_or("");
        anyhow::ensure!(
            id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty(),
            "Invalid catalog identity"
        );
        catalog_json(
            "https://itunes.apple.com/lookup",
            &[
                ("id", id),
                ("entity", entity),
                ("limit", "100"),
                ("country", "us"),
            ],
            generation,
            token,
        )
        .await?
    } else {
        catalog_json(
            "https://itunes.apple.com/search",
            &[
                ("term", search_term),
                (
                    "attribute",
                    if kind == "genres" && genre_id.is_some() {
                        "genreIndex"
                    } else {
                        "mixTerm"
                    },
                ),
                ("entity", entity),
                ("limit", "60"),
                ("country", "us"),
                ("media", "music"),
            ],
            generation,
            token,
        )
        .await?
    };
    anyhow::ensure!(
        generation.load(Ordering::Relaxed) == token,
        "Search superseded."
    );
    Ok(normalize_catalog(&v))
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtistInfo {
    #[serde(default)]
    pub verified_names:Vec<String>,
    #[serde(default)]
    pub lookup_status: String,
    pub biography: Option<String>,
    pub source_url: Option<String>,
    pub portrait: Option<String>,
    pub artist_mbid: Option<String>,
    #[serde(default)]
    pub portrait_source_url: Option<String>,
    #[serde(default)]
    pub portrait_credit: Option<String>,
    #[serde(default)]
    pub portrait_license: Option<String>,
}
fn artist_matches(a:&Value,name:&str)->bool{
 let key=crate::library::artist_name_key(name);
 (a["name"].as_str().is_some_and(|n|crate::library::artist_name_key(n)==key)
 ||a["aliases"].as_array().is_some_and(|aliases|aliases.iter().any(|v|v["name"].as_str().is_some_and(|n|crate::library::artist_name_key(n)==key))))&&a["score"].as_u64().unwrap_or(0)>=95
}
pub fn fetch_artist_info(_db: &Path, name: &str, cancel: &AtomicBool) -> Result<ArtistInfo> {
    let path = cache_path(&format!("artist-info-v2:{name}"));
    let mut info = ArtistInfo::default();
    let query = format!("artist:\"{}\"", name.replace(['"', '\\'], " "));
    let results = cached_json(
        "https://musicbrainz.org/ws/2/artist/",
        &[("query", &query), ("fmt", "json"), ("limit", "5")],
        "musicbrainz",
    )?;
    let matches: Vec<_> = results["artists"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a|artist_matches(a,name))
        .collect();
    if matches.len() != 1 {
        // A failed or ambiguous revalidation cannot erase a verified cached profile.
        if let Some(cached)=cached_artist_info(name){if cached.artist_mbid.is_some(){return Ok(cached);}}
        info.lookup_status="unavailable".into();save_artist(&path,&info)?;return Ok(info);
    }
    let Some(id) = matches[0]["id"].as_str() else {
        return Ok(info);
    };
    info.artist_mbid = Some(id.into());
    info.verified_names=vec![name.into()];if let Some(primary)=matches[0]["name"].as_str(){if primary!=name{info.verified_names.push(primary.into());}}
    if cancel.load(Ordering::Relaxed) {
        return Ok(info);
    }
    let lookup = cached_json(
        &format!("https://musicbrainz.org/ws/2/artist/{id}"),
        &[("inc", "url-rels"), ("fmt", "json")],
        "musicbrainz",
    )?;
    let relations = lookup["relations"].as_array();
    let wiki = relations
        .into_iter()
        .flatten()
        .filter_map(|r| r["url"]["resource"].as_str())
        .find(|u| u.starts_with("https://en.wikipedia.org/wiki/"));
    let title = if let Some(wiki) = wiki {
        url::Url::parse(wiki)
            .ok()
            .and_then(|u| u.path().strip_prefix("/wiki/").map(str::to_owned))
    } else {
        let wikidata = relations
            .into_iter()
            .flatten()
            .filter_map(|r| r["url"]["resource"].as_str())
            .find(|u| u.starts_with("https://www.wikidata.org/wiki/"));
        if let Some(q) = wikidata
            .and_then(|u| u.rsplit('/').next())
            .filter(|q| q.starts_with('Q') && q[1..].chars().all(|c| c.is_ascii_digit()))
        {
            let v = cached_json(
                "https://www.wikidata.org/w/api.php",
                &[
                    ("action", "wbgetentities"),
                    ("ids", q),
                    ("props", "sitelinks"),
                    ("format", "json"),
                ],
                "wikimedia",
            )?;
            v["entities"][q]["sitelinks"]["enwiki"]["title"]
                .as_str()
                .map(str::to_owned)
        } else {
            None
        }
    };
    if let Some(title) = title {
        // MediaWiki accepts decoded titles; a URL parser decodes path through form pairs here.
        let title = url::form_urlencoded::parse(format!("t={title}").as_bytes())
            .next()
            .map(|(_, v)| v.into_owned())
            .unwrap_or(title);
        let summary = cached_json(
            "https://en.wikipedia.org/w/api.php",
            &[
                ("action", "query"),
                ("prop", "extracts|info"),
                ("exintro", "1"),
                ("explaintext", "1"),
                ("inprop", "url"),
                ("titles", &title),
                ("format", "json"),
                ("formatversion", "2"),
            ],
            "wikimedia",
        )?;
        if let Some(page) = summary["query"]["pages"].as_array().and_then(|p| p.first()) {
            info.biography = string(page, "extract");
            info.source_url = string(page, "fullurl");
        }
    }
    if !cancel.load(Ordering::Relaxed) {
        // Resolve P18 from this verified identity, preserving the image's own credit.
        if let Some(q) = relations
            .into_iter()
            .flatten()
            .filter_map(|r| r["url"]["resource"].as_str())
            .find(|u| u.starts_with("https://www.wikidata.org/wiki/"))
            .and_then(|u| u.rsplit('/').next())
            .filter(|q| q.starts_with('Q') && q[1..].chars().all(|c| c.is_ascii_digit()))
        {
            if let Ok(entity) = cached_json(
                "https://www.wikidata.org/w/api.php",
                &[
                    ("action", "wbgetentities"),
                    ("ids", q),
                    ("props", "claims"),
                    ("format", "json"),
                ],
                "wikimedia",
            ) {
                if let Some(filename) = entity["entities"][q]["claims"]["P18"][0]["mainsnak"]
                    ["datavalue"]["value"]
                    .as_str()
                {
                    let title = format!("File:{filename}");
                    if let Ok(images) = cached_json(
                        "https://commons.wikimedia.org/w/api.php",
                        &[
                            ("action", "query"),
                            ("prop", "imageinfo"),
                            ("titles", &title),
                            ("iiprop", "url|extmetadata"),
                            ("iiextmetadatafilter", "Artist|LicenseShortName"),
                            ("iiurlwidth", "900"),
                            ("format", "json"),
                            ("formatversion", "2"),
                        ],
                        "wikimedia",
                    ) {
                        if let Some(image) = images["query"]["pages"][0]["imageinfo"][0].as_object()
                        {
                            let image = Value::Object(image.clone());
                            let url = image["thumburl"].as_str().or_else(|| image["url"].as_str());
                            if let Some(url) = url {
                                if let Ok(portrait) = cache_image(url, true) {
                                    info.portrait = Some(portrait.to_string_lossy().into_owned());
                                    info.portrait_source_url = string(&image, "descriptionurl");
                                    info.portrait_credit = image["extmetadata"]["Artist"]["value"]
                                        .as_str()
                                        .map(plain_credit);
                                    info.portrait_license = image["extmetadata"]
                                        ["LicenseShortName"]["value"]
                                        .as_str()
                                        .map(plain_credit);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(old)=cached_artist_info(name).filter(|v|v.artist_mbid==info.artist_mbid){
        if info.portrait.is_none(){info.portrait=old.portrait;info.portrait_source_url=old.portrait_source_url;info.portrait_credit=old.portrait_credit;info.portrait_license=old.portrait_license;}
        if info.biography.is_none(){info.biography=old.biography;info.source_url=old.source_url;}
    }
    save_artist(&path,&info)?;for verified in &info.verified_names{save_artist(&cache_path(&format!("artist-lookup-v3:{}",crate::library::artist_name_key(verified))),&info)?;}
    Ok(info)
}
fn save_artist(path: &Path, info: &ArtistInfo) -> Result<()> {
    let value = serde_json::to_value(info)?;
    if std::fs::read(path)
        .ok()
        .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
        .as_ref()
        != Some(&value)
    {
        atomic_write_json(path, info)?;
    }
    Ok(())
}
fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
thread_local! {static REVALIDATE: std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
static ARTIST_ATTEMPTS: OnceLock<Mutex<std::collections::HashMap<String, (Instant, u8)>>> =
    OnceLock::new();
fn artist_lookup_key(name:&str)->String{cached_artist_info(name).and_then(|p|p.artist_mbid).map(|id|format!("mbid:{id}")).unwrap_or_else(||crate::library::artist_name_key(name))}
pub fn artist_due(name: &str) -> bool {
    let key=artist_lookup_key(name);
    if std::fs::read(cache_path(&format!("artist-status-v2:{key}")))
        .or_else(|_|std::fs::read(cache_path(&format!("artist-status-v1:{name}"))))
        .ok()
        .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
        .and_then(|v| v["retry_at"].as_u64())
        .is_some_and(|at| at > unix_seconds())
    {
        return false;
    }
    let attempts = ARTIST_ATTEMPTS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    attempts.get(&key).is_none_or(|(at, failures)| {
        at.elapsed()
            >= Duration::from_secs(match failures {
                0 => 60,
                1 => 60,
                2 => 240,
                _ => 960,
            })
    })
}
pub fn artist_info(db: &Path, name: &str, cancel: &AtomicBool) -> Result<ArtistInfo> {
    if let Some(info) = cached_artist_info(name) {
        return Ok(info);
    }
    revalidate_artist(db, name, cancel)
}
pub fn revalidate_artist(db: &Path, name: &str, cancel: &AtomicBool) -> Result<ArtistInfo> {
    if !artist_due(name) {
        return Ok(cached_artist_info(name).unwrap_or_default());
    }
    let key=artist_lookup_key(name);
    let failures = {
        let mut attempts = ARTIST_ATTEMPTS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let failures = attempts.get(&key).map(|(_, v)| *v).unwrap_or(0);
        attempts.insert(key.clone(), (Instant::now(), failures));
        failures
    };
    REVALIDATE.with(|v| v.set(true));
    let result = fetch_artist_info(db, name, cancel);
    REVALIDATE.with(|v| v.set(false));
    let refreshed_key=artist_lookup_key(name);
    ARTIST_ATTEMPTS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            refreshed_key.clone(),
            (
                Instant::now(),
                if result.is_ok() {
                    0
                } else {
                    failures.saturating_add(1)
                },
            ),
        );
    let failed = result.is_err();
    let wait = if failed {
        match failures {
            0 => 60,
            1 => 240,
            _ => 960,
        }
    } else {
        60
    };
    let _ = atomic_write_json(
        &cache_path(&format!("artist-status-v2:{refreshed_key}")),
        &serde_json::json!({"checked_at":unix_seconds(),"retry_at":unix_seconds()+wait,"status":if failed{"unavailable"}else{"cached"},"failures":if failed{failures+1}else{0}}),
    );
    result.or_else(|e| {
        eprintln!("Artist lookup {name}: {e:#}");
        cached_artist_info(name).ok_or(e)
    })
}
pub fn cached_artist_info(name: &str) -> Option<ArtistInfo> {
    std::fs::read(cache_path(&format!("artist-lookup-v3:{}",crate::library::artist_name_key(name))))
        .or_else(|_|std::fs::read(cache_path(&format!("artist-info-v2:{name}"))))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlbumInfo {
    #[serde(default)]
    pub release_type: Option<String>,
    pub description: Option<String>,
    pub source_url: Option<String>,
}
pub fn album_info(
    db: &Path,
    album: &crate::library::Album,
    cancel: &AtomicBool,
) -> Result<AlbumInfo> {
    let c = musicos_init::database::Connection::open(db)?;
    let placeholders = std::iter::repeat_n("?", album.track_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    if album.track_ids.is_empty() {
        return Ok(AlbumInfo::default());
    }
    let sql=format!("SELECT DISTINCT release_group_mbid FROM metadata_enrichment_status WHERE state='applied' AND release_group_mbid IS NOT NULL AND track_id IN ({placeholders})");
    let mut ids = c
        .prepare(&sql)?
        .query_map(rusqlite::params_from_iter(album.track_ids.iter()), |r| {
            r.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if let Ok(mut statement) = c.prepare(&format!(
        "SELECT value FROM desktop_file_metadata_v1 WHERE track_id IN ({placeholders})"
    )) {
        if let Ok(rows) = statement
            .query_map(rusqlite::params_from_iter(album.track_ids.iter()), |r| {
                r.get::<_, String>(0)
            })
        {
            for row in rows.flatten() {
                if let Ok(value) = serde_json::from_str::<Value>(&row) {
                    if let Some(id) = value["release_group_id"].as_str() {
                        if !ids.iter().any(|v| v == id) {
                            ids.push(id.into());
                        }
                    }
                }
            }
        }
    }
    if ids.len() != 1 {
        return Ok(AlbumInfo::default());
    }
    let id = &ids[0];
    anyhow::ensure!(
        id.len() == 36 && id.chars().all(|v| v.is_ascii_hexdigit() || v == '-'),
        "Invalid verified album identity"
    );
    let path = cache_path(&format!("album-info-v2:{id}"));
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(info) = serde_json::from_slice(&bytes) {
            return Ok(info);
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Ok(AlbumInfo::default());
    }
    let release = cached_json(
        &format!("https://musicbrainz.org/ws/2/release-group/{id}"),
        &[("inc", "url-rels"), ("fmt", "json")],
        "musicbrainz",
    )?;
    let relations = release["relations"].as_array();
    let wiki = relations
        .into_iter()
        .flatten()
        .filter_map(|r| r["url"]["resource"].as_str())
        .find(|u| u.starts_with("https://en.wikipedia.org/wiki/"));
    let title = if let Some(wiki) = wiki {
        url::Url::parse(wiki)
            .ok()
            .and_then(|u| u.path().strip_prefix("/wiki/").map(str::to_owned))
    } else {
        let q = relations
            .into_iter()
            .flatten()
            .filter_map(|r| r["url"]["resource"].as_str())
            .find(|u| u.starts_with("https://www.wikidata.org/wiki/"))
            .and_then(|u| u.rsplit('/').next())
            .filter(|q| q.starts_with('Q') && q[1..].chars().all(|c| c.is_ascii_digit()));
        if let Some(q) = q {
            let value = cached_json(
                "https://www.wikidata.org/w/api.php",
                &[
                    ("action", "wbgetentities"),
                    ("ids", q),
                    ("props", "sitelinks"),
                    ("format", "json"),
                ],
                "wikimedia",
            )?;
            value["entities"][q]["sitelinks"]["enwiki"]["title"]
                .as_str()
                .map(str::to_owned)
        } else {
            None
        }
    };
    let mut info = AlbumInfo::default();
    info.release_type=release["primary-type"].as_str().map(str::to_ascii_lowercase).filter(|s|["album","single","ep"].contains(&s.as_str()));
    if let Some(title) = title {
        let title = url::form_urlencoded::parse(format!("t={title}").as_bytes())
            .next()
            .map(|(_, v)| v.into_owned())
            .unwrap_or(title);
        let summary = cached_json(
            "https://en.wikipedia.org/w/api.php",
            &[
                ("action", "query"),
                ("prop", "extracts|info"),
                ("exintro", "1"),
                ("explaintext", "1"),
                ("inprop", "url"),
                ("titles", &title),
                ("format", "json"),
                ("formatversion", "2"),
            ],
            "wikimedia",
        )?;
        if let Some(page) = summary["query"]["pages"].as_array().and_then(|p| p.first()) {
            info.description = string(page, "extract").filter(|s| !s.trim().is_empty());
            info.source_url = string(page, "fullurl");
        }
    }
    atomic_write_json(&path, &info)?;
    Ok(info)
}
/// Discord must fetch a public image; local paths and embedded artwork are never uploaded.
pub fn discord_album_cover(artist: &str, album: &str) -> Result<Option<String>> {
    if artist.trim().is_empty()
        || album.trim().is_empty()
        || album.eq_ignore_ascii_case("Unknown album")
    {
        return Ok(None);
    }
    let term = format!("{artist} {album}");
    let value = cached_json(
        "https://itunes.apple.com/search",
        &[("term", &term), ("entity", "album"), ("limit", "15")],
        "apple",
    )?;
    Ok(exact_album_cover(&value, artist, album))
}
fn cover_identity(value:&str)->String{use unicode_normalization::UnicodeNormalization;let cleaned=value.nfkc().collect::<String>().to_lowercase().replace(['’','‘'],"'").replace(['–','—'],"-");let trimmed=cleaned.trim();let trimmed=trimmed.strip_suffix(" - single").or_else(||trimmed.strip_suffix(" - ep")).unwrap_or(trimmed);trimmed.split_whitespace().collect::<Vec<_>>().join(" ")}
fn exact_album_cover(value: &Value, artist: &str, album: &str) -> Option<String> {
    let item = value["results"].as_array()?.iter().find(|item| {
        item["artistName"]
            .as_str()
            .is_some_and(|name| cover_identity(name)==cover_identity(artist))
            && item["collectionName"]
                .as_str()
                .is_some_and(|name| cover_identity(name)==cover_identity(album))
    })?;
    let url = item["artworkUrl100"].as_str()?;
    let mut parsed = url::Url::parse(url).ok()?;
    if parsed.scheme()=="http"&&parsed.host_str()?.ends_with(".mzstatic.com"){parsed.set_scheme("https").ok()?;}
    if parsed.scheme() != "https" || !parsed.host_str()?.ends_with(".mzstatic.com") {
        return None;
    }
    Some(parsed.to_string().replace("100x100bb", "600x600bb"))
}
fn plain_credit(html: &str) -> String {
    let mut in_tag = false;
    html.chars()
        .filter(|c| match c {
            '<' => {
                in_tag = true;
                false
            }
            '>' => {
                in_tag = false;
                false
            }
            _ => !in_tag,
        })
        .collect::<String>()
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_owned()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub background: String,
    pub accent: String,
    pub blob: String,
    pub text: String,
}
pub fn palette(path: &Path) -> Result<Palette> {
    let metadata = std::fs::metadata(path)?;
    anyhow::ensure!(metadata.len() <= 128 * 1024 * 1024, "Artwork exceeds the file size limit.");
    let stamp = metadata
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_nanos();
    let cache = cache_path(&format!("palette-v6:{}:{stamp}", path.display()));
    if let Ok(bytes) = std::fs::read(&cache) {
        if let Ok(p) = serde_json::from_slice(&bytes) {
            return Ok(p);
        }
    }
    // Embedded/manual originals can legitimately exceed 20 MB. Use the same
    // pixel-bounded rendition cache as the UI rather than silently losing their
    // theme, or decoding a full-resolution image on every navigation.
    let sample=crate::artwork_renditions::rendition(path,128)?;
    let image = image::io::Reader::open(sample)?
        .with_guessed_format()?
        .decode()?
        .thumbnail(32, 32)
        .to_rgb8();
    let mut sum = [0u64; 3];
    let mut count = 0;
    let mut bins = std::collections::HashMap::<u16, (u64, [u64; 3], u64)>::new();
    for p in image.pixels() {
        let max = *p.0.iter().max().unwrap();
        let min = *p.0.iter().min().unwrap();
        let chroma = u64::from(max - min);
        if chroma > 20 && max > 60 {
            let key = ((p[0] as u16 / 32) << 6) | ((p[1] as u16 / 32) << 3) | (p[2] as u16 / 32);
            let bin = bins.entry(key).or_insert((0, [0; 3], 0));
            bin.0 += 1;
            bin.2 += chroma;
            for i in 0..3 {
                bin.1[i] += u64::from(p[i]);
            }
        }
        for (i, s) in sum.iter_mut().enumerate() {
            *s += u64::from(p[i])
        }
        count += 1;
    }
    anyhow::ensure!(count > 0, "Artwork is empty");
    let rgb = sum.map(|n| (n / count) as u8);
    // Area-weighted chroma avoids choosing a single bright compression pixel.
    let vivid = bins
        .iter()
        .max_by_key(|(key, bin)| (bin.2, **key))
        .map(|(_, bin)| bin.1.map(|n| (n / bin.0) as u8))
        .unwrap_or(rgb);
    let hex = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    // Saturate colored surfaces, stopping before readable text contrast is lost.
    let luminance=|rgb:[u8;3]|->f64{let c=rgb.map(|v|{let x=f64::from(v)/255.;if x<=0.04045{x/12.92}else{((x+0.055)/1.055).powf(2.4)}});c[0]*0.2126+c[1]*0.7152+c[2]*0.0722};
    let mut strength=70_u16;
    let blob=loop{let rgb=vivid.map(|v|((u16::from(v)*strength+255*(100-strength))/100)as u8);if (luminance(rgb)+0.05)/(luminance([18,32,42])+0.05)>=4.5||strength==0{break rgb;}strength-=1;};
    let p = Palette {
        background: hex(std::array::from_fn(|i| {
            if vivid.iter().max().unwrap()-vivid.iter().min().unwrap()<20 {
                ((u16::from(rgb[i])*25+255*75)/100) as u8
            } else {
                ((u16::from(rgb[i])*6+u16::from(vivid[i])*32+255*62)/100) as u8
            }
        })),
        accent: hex(vivid.map(|v| ((u16::from(v) * 40) / 100) as u8)),
        blob: hex(blob),
        text: "#12202a".into(),
    };
    atomic_write_json(&cache, &p)?;
    Ok(p)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discord_cover_requires_exact_identity_and_public_artwork() {
        let response = serde_json::json!({"results":[{"artistName":"M83","collectionName":"Fantasy","artworkUrl100":"https://is1-ssl.mzstatic.com/image/100x100bb.jpg"}]});
        assert_eq!(
            exact_album_cover(&response, "m83", "Fantasy").unwrap(),
            "https://is1-ssl.mzstatic.com/image/600x600bb.jpg"
        );
        assert!(exact_album_cover(&response, "Other artist", "Fantasy").is_none());
        assert!(exact_album_cover(&response, "M83", "Other album").is_none());
        let local = serde_json::json!({"results":[{"artistName":"M83","collectionName":"Fantasy","artworkUrl100":"file:///private/artwork.png"}]});
        assert!(exact_album_cover(&local, "M83", "Fantasy").is_none());
    }
    #[test]fn discord_matches_punctuation_and_single_labels_without_swapping_editions(){let response=serde_json::json!({"results":[{"artistName":"Artist","collectionName":"We're Here - Single","artworkUrl100":"http://is1-ssl.mzstatic.com/image/100x100bb.jpg"}]});assert!(exact_album_cover(&response,"Artist","We’re Here").unwrap().starts_with("https://"));assert!(exact_album_cover(&response,"Artist","We're Here (Deluxe)").is_none());}
    #[test]
    fn catalog_never_claims_local_playback() {
        let data = serde_json::json!({"results":[{"wrapperType":"track","kind":"song","trackId":7,"artistId":9,"collectionId":3,"trackName":"Say \"Hello\"","artistName":"A","trackTimeMillis":203000}]});
        let rows = normalize_catalog(&data);
        assert_eq!(rows[0].id, "apple/song/7");
        assert!(!rows[0].local);
        assert_eq!(rows[0].duration, Some(203));
    }
    #[test]
    fn obsolete_network_request_is_cancelled() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let _ = accepted_tx.send(());
            let mut bytes = Vec::new();
            let _ = std::io::Read::read_to_end(&mut socket, &mut bytes);
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let generation = AtomicU64::new(1);
            let request = reqwest::Client::new()
                .get(format!("http://{address}"))
                .send();
            let cancel = async {
                accepted_rx.await.unwrap();
                generation.store(2, Ordering::Relaxed);
            };
            let started = Instant::now();
            let (result, _) = tokio::join!(cancellable(request, &generation, 1), cancel);
            assert!(result.unwrap_err().to_string().contains("cancelled"));
            assert!(started.elapsed() < Duration::from_secs(1));
        });
        server.join().unwrap();
    }
    #[test]
    fn light_dark_and_vivid_palettes_keep_text_readable() {
        fn luminance(hex: &str) -> f64 {
            let c: Vec<f64> = (1..7)
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as f64 / 255.)
                .map(|c| {
                    if c <= 0.04045 {
                        c / 12.92
                    } else {
                        ((c + 0.055) / 1.055).powf(2.4)
                    }
                })
                .collect();
            c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
        }
        let t = tempfile::tempdir().unwrap();
        for (index, rgb) in [
            [0, 0, 0],
            [255, 255, 255],
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
        ]
        .into_iter()
        .enumerate()
        {
            let path = t.path().join(format!("{index}.png"));
            image::RgbImage::from_pixel(2, 2, image::Rgb(rgb))
                .save(&path)
                .unwrap();
            let p = palette(&path).unwrap();
            assert!((luminance(&p.background) + 0.05) / (luminance(&p.text) + 0.05) >= 4.5);
            assert!((luminance(&p.blob) + 0.05) / (luminance(&p.text) + 0.05) >= 4.5);
            assert!(1.05 / (luminance(&p.accent) + 0.05) >= 4.5);
            assert_eq!(
                serde_json::to_value(&p).unwrap(),
                serde_json::to_value(palette(&path).unwrap()).unwrap()
            );
        }
        assert!(palette(&t.path().join("missing.png")).is_err());
        let invalid = t.path().join("unreadable.png");
        std::fs::write(&invalid, b"not an image").unwrap();
        assert!(palette(&invalid).is_err());
    }
    #[test]
    fn large_original_artwork_uses_bounded_cached_color_sampling() {
        use std::io::Write;
        let temp=tempfile::tempdir().unwrap();let path=temp.path().join("large-original.png");
        image::RgbImage::from_pixel(512,512,image::Rgb([180,35,45])).save(&path).unwrap();
        // Valid PNG with trailing bytes reproduces the old encoded-size gate
        // without a large decoded allocation in this regression fixture.
        let mut file=std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        let padding=[0u8;1024*1024];for _ in 0..21 {file.write_all(&padding).unwrap();}drop(file);
        let first=palette(&path).unwrap();let second=palette(&path).unwrap();
        assert_eq!(first.background,second.background);assert_eq!(first.accent,second.accent);
        let red=u8::from_str_radix(&first.accent[1..3],16).unwrap();let blue=u8::from_str_radix(&first.accent[5..7],16).unwrap();assert!(red>blue*2);
        let sample=crate::artwork_renditions::rendition(&path,128).unwrap();assert_ne!(sample,path);assert_eq!(image::image_dimensions(&sample).unwrap(),(128,128));
    }
    #[test]
    fn palette_prefers_color_area_over_one_bright_pixel() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("dominant.png");
        let mut image = image::RgbImage::from_pixel(32, 32, image::Rgb([40, 160, 80]));
        image.put_pixel(0, 0, image::Rgb([255, 0, 255]));
        image.save(&path).unwrap();
        let p = palette(&path).unwrap();
        let r = u8::from_str_radix(&p.accent[1..3], 16).unwrap();
        let g = u8::from_str_radix(&p.accent[3..5], 16).unwrap();
        assert!(g > r);
    }
    #[test]
    fn palettes_are_cached_and_readable() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("black.png");
        image::RgbImage::from_pixel(2, 2, image::Rgb([0, 0, 0]))
            .save(&path)
            .unwrap();
        let p = palette(&path).unwrap();
        assert_eq!(p.background, "#bfbfbf");
        assert_eq!(p.text, "#12202a");
    }
    #[test]
    fn richer_artwork_colors_keep_text_readable() {
        let temp=tempfile::tempdir().unwrap();
        let parse=|hex:&str|[1,3,5].map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap());
        let luminance=|rgb:[u8;3]|{let c=rgb.map(|v|{let x=f64::from(v)/255.;if x<=0.04045{x/12.92}else{((x+0.055)/1.055).powf(2.4)}});c[0]*0.2126+c[1]*0.7152+c[2]*0.0722};
        for (index,color) in [[40,160,80],[0,0,255],[255,0,0],[255,210,0],[120,20,180],[20,20,20],[240,240,240]].into_iter().enumerate(){
            let path=temp.path().join(format!("{index}.png"));image::RgbImage::from_pixel(8,8,image::Rgb(color)).save(&path).unwrap();
            let result=palette(&path).unwrap();let text=luminance(parse(&result.text));
            for surface in [&result.background,&result.blob]{assert!((luminance(parse(surface))+0.05)/(text+0.05)>=4.5,"{color:?}: {surface}");}
            if index==0 {let rgb=parse(&result.blob);assert!(rgb[1]-rgb[0]>70);let rgb=parse(&result.background);assert!(rgb[1]-rgb[0]>40);}
            assert_eq!(serde_json::to_value(&result).unwrap(),serde_json::to_value(palette(&path).unwrap()).unwrap());
        }
    }
}

/// Only catalog artwork hosts are accepted; never fetch arbitrary URLs from the webview.
pub fn cache_artwork(url: &str) -> Result<PathBuf> {
    // A size variant keeps the exact catalog asset path; never search for alternate art.
    let mut rendition=url::Url::parse(url)?;let path=rendition.path().to_owned();if let Some((prefix,last))=path.rsplit_once('/') {if let Some((size,suffix))=last.split_once("bb."){if size.split_once('x').is_some_and(|(w,h)|w.parse::<u32>().is_ok()&&h.parse::<u32>().is_ok()){rendition.set_path(&format!("{prefix}/3000x3000bb.{suffix}"));}}}
    cache_image(rendition.as_str(),false).or_else(|_|cache_image(url,false))
}
fn cache_image(url: &str, wikimedia: bool) -> Result<PathBuf> {
    use std::io::Read;
    let parsed = url::Url::parse(url)?;
    anyhow::ensure!(
        parsed.scheme() == "https"
            && parsed.host_str().is_some_and(|h| if wikimedia {
                h == "upload.wikimedia.org" || h == "thumb.wikimedia.org"
            } else {
                h.ends_with(".mzstatic.com")
            }),
        "Unsupported artwork provider."
    );
    let path = desktop_data_dir()
        .join("catalog-v1/artwork")
        .join(format!("{:x}.png", Sha256::digest(url.as_bytes())));
    if path.is_file() {
        return Ok(path);
    }
    static DOWNLOAD: Mutex<()> = Mutex::new(());
    let _guard = DOWNLOAD
        .lock()
        .map_err(|_| anyhow::anyhow!("Artwork cache is busy"))?;
    if path.is_file() {
        return Ok(path);
    }
    let response = client()?.get(url).send()?.error_for_status()?;
    anyhow::ensure!(
        response.content_length().unwrap_or(0) <= 20 * 1024 * 1024,
        "Artwork is too large."
    );
    let mut bytes = Vec::new();
    response
        .take(20 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 20 * 1024 * 1024, "Artwork is too large.");
    let mut reader = image::io::Reader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let temp = path.with_extension("tmp.png");
    image.save(&temp)?;
    std::fs::rename(temp, &path)?;
    Ok(path)
}

#[cfg(test)]
mod album_description_tests {
    use super::*;
    #[test]
    fn descriptions_require_one_applied_release_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");
        let c = musicos_init::database::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE metadata_enrichment_status(track_id INTEGER,state TEXT,release_group_mbid TEXT); INSERT INTO metadata_enrichment_status VALUES(1,'review','aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa')").unwrap();
        let album = crate::library::group(musicos_desktop::models::LibrarySnapshot {
            tracks: vec![
                musicos_desktop::models::Track {
                    id: 1,
                    artist: "Artist".into(),
                    album: "Album".into(),
                    ..Default::default()
                },
                musicos_desktop::models::Track {
                    id: 2,
                    artist: "Artist".into(),
                    album: "Album".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .albums
        .remove(0);
        assert!(album_info(&path, &album, &AtomicBool::new(false))
            .unwrap()
            .description
            .is_none());
        c.execute_batch("UPDATE metadata_enrichment_status SET state='applied'; INSERT INTO metadata_enrichment_status VALUES(2,'applied','bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb')").unwrap();
        assert!(album_info(&path, &album, &AtomicBool::new(false))
            .unwrap()
            .description
            .is_none());
    }
}

#[cfg(test)]mod artist_identity_tests{use super::*;
 #[test]fn accents_require_a_verified_alias(){let mut value=serde_json::json!({"name":"Beyoncé","score":100});assert!(!artist_matches(&value,"Beyonce"));value["aliases"]=serde_json::json!([{"name":"Beyonce"}]);assert!(artist_matches(&value,"Beyonce"));value["score"]=serde_json::json!(80);assert!(!artist_matches(&value,"Beyonce"));}
 #[test]fn typography_and_whitespace_are_equivalent(){assert_eq!(crate::library::artist_name_key(" Eazy‐E  "),crate::library::artist_name_key("eazy-e"));assert_ne!(crate::library::artist_name_key("Beyoncé"),crate::library::artist_name_key("Beyonce"));}
}
