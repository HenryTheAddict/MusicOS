//! Opt-in, client-only Apple Music research connector. Never part of playback.
use anyhow::{ensure, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};
use tauri::{Emitter, Manager};
#[path = "apple_background.rs"]
pub mod background;
#[path = "apple_transfers.rs"]
mod transfers;
#[derive(Default, Clone, Serialize)]
pub struct SidecarResult {
    written: u64,
    unchanged: u64,
    conflicts: u64,
    failed: u64,
}
#[derive(Default, Clone, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub signed_in: bool,
    pub sign_in: String,
    pub library: String,
    pub lyrics: String,
    pub artwork: String,
    pub phase: String,
    pub message: String,
    pub scanned: usize,
    pub matched: usize,
    pub ambiguous: usize,
    pub completed: usize,
    pub total: usize,
    pub lyrics_ready: usize,
    pub lyrics_failed: usize,
    pub artwork_ready: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidecars: Option<SidecarResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_files: Option<SidecarResult>,
}
fn state() -> &'static Mutex<Status> {
    static S: OnceLock<Mutex<Status>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(Status {
            sign_in: "not_checked".into(),
            library: "not_checked".into(),
            lyrics: "not_checked".into(),
            artwork: "not_checked".into(),
            phase: "disconnected".into(),
            message: "Apple Music enrichment is off. Local lyrics keep working.".into(),
            ..Default::default()
        })
    })
}
static CANCEL: AtomicBool = AtomicBool::new(false);
static JOB: AtomicBool = AtomicBool::new(false);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub action: String,
    pub track_id: Option<i64>,
    pub catalog_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Credentials {
    user: String,
    bearer: String,
    storefront: String,
}
fn entry(db: &Path) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(
        "musicOS Apple enrichment",
        &db.to_string_lossy(),
    )?)
}
fn credentials(db: &Path) -> Result<Credentials> {
    let raw = entry(db)?
        .get_password()
        .map_err(|_| anyhow::anyhow!("Connect Apple Music to continue."))?;
    let mut saved: Credentials = serde_json::from_str(&raw)?;
    // The web app's developer token can expire before the retained user session.
    // Refresh only that public token; a valid user session does not need sign-in.
    if token_expiry(&saved.bearer).is_some_and(|expiry| expiry <= apple_now().saturating_add(60)) {
        static REFRESH: Mutex<()> = Mutex::new(());
        let _guard = REFRESH.lock().unwrap_or_else(|e| e.into_inner());
        saved = serde_json::from_str(&entry(db)?.get_password()?)?;
        if token_expiry(&saved.bearer).is_some_and(|expiry| expiry <= apple_now().saturating_add(60)) {
            saved.bearer = bearer()?;
            entry(db)?.set_password(&serde_json::to_string(&saved)?)?;
        }
    }
    Ok(saved)
}
fn token_expiry(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice::<Value>(&bytes).ok()?["exp"].as_u64()
}
pub(crate) fn store(db: &Path) -> Result<rusqlite::Connection> {
    let c = rusqlite::Connection::open(db.with_file_name("desktop-apple-enrichment-v1.sqlite3"))?;
    c.busy_timeout(Duration::from_secs(5))?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS links(track_id INTEGER PRIMARY KEY,catalog_id TEXT NOT NULL,album_id TEXT,signature TEXT NOT NULL); CREATE TABLE IF NOT EXISTS choices(track_id INTEGER,catalog_id TEXT,payload TEXT,PRIMARY KEY(track_id,catalog_id)); CREATE TABLE IF NOT EXISTS lyric_cache(catalog_id TEXT PRIMARY KEY,document TEXT NOT NULL); CREATE TABLE IF NOT EXISTS lyric_attempts(signature TEXT PRIMARY KEY,checked INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS artwork_cache(album_id TEXT PRIMARY KEY,payload TEXT NOT NULL);")?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS asset_links(track_id INTEGER PRIMARY KEY,catalog_id TEXT NOT NULL,album_id TEXT,signature TEXT NOT NULL);
        CREATE TRIGGER IF NOT EXISTS retain_asset_link AFTER INSERT ON links BEGIN INSERT OR REPLACE INTO asset_links VALUES(new.track_id,new.catalog_id,new.album_id,new.signature); END;
        CREATE TRIGGER IF NOT EXISTS update_asset_link AFTER UPDATE ON links BEGIN INSERT OR REPLACE INTO asset_links VALUES(new.track_id,new.catalog_id,new.album_id,new.signature); END;")?;
    if c.pragma_query_value(None,"user_version",|r|r.get::<_,i64>(0))?<1 {
        c.execute_batch("INSERT OR IGNORE INTO asset_links SELECT * FROM links;PRAGMA user_version=1;")?;
    }
    if c.pragma_query_value(None,"user_version",|r|r.get::<_,i64>(0))?<2 {
        c.execute_batch("UPDATE asset_links SET album_id=(SELECT links.album_id FROM links WHERE links.track_id=asset_links.track_id AND links.catalog_id=asset_links.catalog_id AND links.signature=asset_links.signature) WHERE album_id IS NULL AND EXISTS(SELECT 1 FROM links WHERE links.track_id=asset_links.track_id AND links.catalog_id=asset_links.catalog_id AND links.signature=asset_links.signature AND links.album_id IS NOT NULL);PRAGMA user_version=2;")?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            db.with_file_name("desktop-apple-enrichment-v1.sqlite3"),
            std::fs::Permissions::from_mode(0o600),
        )?;
    }
    Ok(c)
}
fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(crate::product::provider_user_agent())
        .build()?)
}
static APPLE_COOLDOWN: AtomicU64 = AtomicU64::new(0);
fn apple_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn bounded(mut response: reqwest::blocking::Response) -> Result<String> {
    let status = response.status();
    if matches!(status.as_u16(), 429 | 502 | 503 | 504) {
        let seconds = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.parse::<u64>().ok().or_else(|| {
                    httpdate::parse_http_date(v)
                        .ok()
                        .and_then(|t| t.duration_since(std::time::SystemTime::now()).ok())
                        .map(|d| d.as_secs())
                })
            })
            .unwrap_or(60);
        APPLE_COOLDOWN.fetch_max(apple_now().saturating_add(seconds), Ordering::SeqCst);
    }

    ensure!(
        status.is_success(),
        "Apple Music returned HTTP {}. This capability is unavailable; reconnect or try later.",
        status.as_u16()
    );
    let mut data = String::new();
    response
        .by_ref()
        .take(8_000_001)
        .read_to_string(&mut data)
        .context("Apple Music returned unreadable data")?;
    ensure!(data.len() <= 8_000_000, "Apple Music response is too large");
    Ok(data)
}
fn get(c: &Credentials, path: &str) -> Result<Value> {
    ensure!(
        apple_now() >= APPLE_COOLDOWN.load(Ordering::SeqCst),
        "Apple Music is cooling down; retry after {} seconds",
        APPLE_COOLDOWN
            .load(Ordering::SeqCst)
            .saturating_sub(apple_now())
    );
    ensure!(
        path.starts_with("/v1/") && !path.contains("://"),
        "Invalid Apple request"
    );
    let mut request = client()?
        .get(format!("https://amp-api.music.apple.com{path}"))
        .bearer_auth(&c.bearer)
        .header("accept", "application/json")
        .header("origin", "https://music.apple.com")
        .header("referer", "https://music.apple.com/");
    if !c.user.is_empty() {
        request = request.header("media-user-token", &c.user);
    }
    let response = request.send().context("Could not reach Apple Music")?;
    Ok(serde_json::from_str(&bounded(response)?)
        .context("Apple Music returned an unsupported response")?)
}
fn library_page(c: &Credentials, path: &str, official: &mut Option<bool>) -> Result<Value> {
    ensure!(
        path.starts_with("/v1/") && !path.contains("://"),
        "Invalid Apple library request"
    );
    if *official != Some(false) {
        let response = client()?
            .get(format!("https://api.music.apple.com{path}"))
            .bearer_auth(&c.bearer)
            .header("Music-User-Token", &c.user)
            .send()
            .context("Could not reach Apple Music library")?;
        if matches!(response.status().as_u16(), 401 | 403 | 404) {
            *official = Some(false);
        } else {
            let result = serde_json::from_str(&bounded(response)?)
                .context("Apple Music returned an unsupported library response")?;
            *official = Some(true);
            return Ok(result);
        }
    }
    get(c, path)
}
fn public_text(client: &reqwest::blocking::Client, address: &str) -> Result<String> {
    let mut url = url::Url::parse(address)?;
    for _ in 0..5 {
        ensure!(
            url.scheme() == "https" && url.host_str() == Some("music.apple.com"),
            "Unsupported Apple public-site redirect"
        );
        let response = client.get(url.clone()).send()?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .context("Apple returned an unsupported redirect")?;
            url = url.join(location)?;
        } else {
            return bounded(response);
        }
    }
    anyhow::bail!("Apple’s public site redirected too many times")
}
fn web_token(body: &str) -> Option<String> {
    for (offset, _) in body.match_indices("eyJ") {
        let token = body[offset..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            .collect::<String>();
        let parts = token.split('.').collect::<Vec<_>>();
        let valid = parts.len() == 3
            && base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[0])
                .ok()
                .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
                .is_some_and(|h| h["alg"] == "ES256")
            && base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .ok()
                .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
                .and_then(|v| v["exp"].as_u64())
                .is_some_and(|exp| {
                    exp > std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                });
        if valid && token.len() > 100 {
            return Some(token);
        }
    }

    None
}
fn bearer() -> Result<String> {
    let client = client()?;
    let html = public_text(&client, "https://music.apple.com/us/browse")?;
    if let Some(token) = web_token(&html) {
        return Ok(token);
    }
    let mut visited = std::collections::BTreeSet::new();
    for fragment in html
        .split('"')
        .filter(|p| p.starts_with("/assets/") && p.ends_with(".js"))
        .filter(|p| visited.insert(p.to_string()))
        .take(6)
    {
        if let Ok(js) = public_text(&client, &format!("https://music.apple.com{fragment}")) {
            if let Some(token) = web_token(&js) {
                return Ok(token);
            }
        }
    }
    anyhow::bail!(
        "Apple’s web authorization format is unavailable. Experimental connection cannot continue."
    )
}

fn cookie_token(raw: &str) -> Result<String> {
    let raw = raw.trim_matches('"');
    ensure!(raw.len() <= 16000, "Apple returned an unsupported session");
    let mut out = vec![];
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&raw[i + 1..i + 3], 16) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    let token = String::from_utf8(out).context("Apple returned an unsupported session encoding")?;
    ensure!(
        !token.contains(['\r', '\n', '\0']),
        "Apple returned an unsupported session"
    );
    Ok(token)
}
fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn artist_key(artist: &str) -> String {
    artist.replace(" + ", " & ").replace(';', " & ").split(" & ")
        .map(|credit| if musicos_desktop::local::enrichment::ye_artist_credit(credit) {
            "kanyewest".to_owned()
        } else {
            norm(credit)
        }).collect::<Vec<_>>().join("&")
}

fn apple_title_key(title: &str) -> String {
    let trimmed = title.trim();
    let lower = trimmed.to_ascii_lowercase();
    for suffix in [" [explicit]", " (explicit)", " [clean]", " (clean)"] {
        if lower.ends_with(suffix) {
            return norm(&trimmed[..trimmed.len()-suffix.len()]);
        }
    }
    norm(trimmed)
}

fn catalog_record(song: &Value) -> Option<Value> {
    let catalog = song.pointer("/relationships/catalog/data/0");
    let catalog_id = catalog.and_then(|row| row["id"].as_str())
        .or_else(|| song.pointer("/attributes/playParams/catalogId").and_then(Value::as_str))
        .or_else(|| (song["type"] == "songs" || song["type"].is_null()).then(|| song["id"].as_str()).flatten())?;
    digits(catalog_id).ok()?;
    // Apple sometimes returns just a relationship ID. The library row still
    // carries recording metadata, and catalog attributes win when present.
    let mut attributes = song["attributes"].as_object().cloned().unwrap_or_default();
    if let Some(catalog_attributes) = catalog.and_then(|row| row["attributes"].as_object()) {
        for (key,value) in catalog_attributes.iter().filter(|(_,value)| !value.is_null()) {
            attributes.insert(key.clone(),value.clone());
        }
    }
    Some(json!({"id":catalog_id,"attributes":attributes,
        "relationships":catalog.map(|row| &row["relationships"]).unwrap_or(&Value::Null)}))
}
fn signature(t: &musicos_init::database::Track) -> String {
    format!("{}|{}|{}|{}", t.title, t.artist, t.album, t.duration)
}
fn catalog_for(db: &Path, id: i64) -> Result<String> {
    let c = store(db)?;
    let track = musicos_init::database::Db::open(&db.to_string_lossy())?
        .get_track_by_id(id)?
        .context("Song is no longer available")?;
    Ok(c.query_row(
        "SELECT catalog_id FROM asset_links WHERE track_id=? AND signature=?",
        rusqlite::params![id, signature(&track)],
        |r| r.get(0),
    )
    .context("Connect Apple Music in Settings → Integrations and match this local song first")?)
}
fn digits(s: &str) -> Result<&str> {
    ensure!(
        !s.is_empty() && s.len() < 30 && s.bytes().all(|v| v.is_ascii_digit()),
        "Invalid catalog identity"
    );
    Ok(s)
}
fn ttml(value: &Value) -> Option<&str> {
    match value {
        Value::String(s) if s.contains("<tt") => Some(s),
        Value::Object(m) => m.values().find_map(ttml),
        Value::Array(a) => a.iter().find_map(ttml),
        _ => None,
    }
}
fn identify_document(db: &Path, id: i64, doc: &mut crate::lyrics::Document) -> Result<()> {
    let track = musicos_init::database::Db::open(&db.to_string_lossy())?
        .get_track_by_id(id)?
        .context("This local song is unavailable")?;
    doc.metadata.title = track.title;
    doc.metadata.artist = track.artist;
    doc.metadata.album = track.album;
    doc.metadata.duration_ms = Some(track.duration as i64 * 1000);
    Ok(())
}
pub fn cached_lyric(db: &Path, id: i64) -> Result<Option<crate::lyrics::Document>> {
    use rusqlite::OptionalExtension;
    let catalog = match catalog_for(db, id) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let raw = store(db)?
        .query_row(
            "SELECT document FROM lyric_cache WHERE catalog_id=?",
            [catalog],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    raw.map(|v| {
        let mut doc = serde_json::from_str(&v)?;
        identify_document(db, id, &mut doc)?;
        Ok(doc)
    })
    .transpose()
}
pub fn edition_stale(db:&Path,id:i64)->Result<bool>{let sql=store(db)?;let track=musicos_init::database::Db::open(&db.to_string_lossy())?.get_track_by_id(id)?.context("Song unavailable")?;let p=crate::lyric_edition::policy(&sql,&track)?;Ok(catalog_for(db,id).ok().is_some_and(|catalog|!crate::lyric_edition::valid(&sql,id,&catalog,&p).unwrap_or(false)))}
pub fn document_stale(db:&Path,id:i64,doc:&crate::lyrics::Document)->Result<bool>{let catalog=doc.extensions.get("x-musicos").and_then(|v|v.pointer("/attribution/catalogId")).and_then(Value::as_str);Ok(edition_stale(db,id)?||catalog.is_some_and(|id_in_file|catalog_for(db,id).is_ok_and(|current|current!=id_in_file)))}
// A connected account enriches this local recording on demand. No library scan,
// audio download, or metadata replacement is needed to open its lyrics.
fn lyric_catalog(db: &Path, id: i64) -> Result<String> {
    let sql=store(db)?;
    let policy_track=musicos_init::database::Db::open(&db.to_string_lossy())?.get_track_by_id(id)?.context("Song unavailable")?;
    let policy=crate::lyric_edition::policy(&sql,&policy_track)?;
    if let Ok(catalog)=catalog_for(db,id) {if crate::lyric_edition::valid(&sql,id,&catalog,&policy)? {return Ok(catalog);}}
    let auth=credentials(db)?;
    let track=musicos_init::database::Db::open(&db.to_string_lossy())?
        .get_track_by_id(id)?.context("This local song is unavailable")?;
    let search_artist = if musicos_desktop::local::enrichment::ye_artist_credit(&track.artist) {
        "Kanye West"
    } else {
        &track.artist
    };
    let term=url::form_urlencoded::Serializer::new(String::new())
        .append_pair("term",&format!("{} {}",track.title,search_artist)).finish();
    let value=get(&auth,&format!("/v1/catalog/{}/search?types=songs&limit=25&include=albums&{}",auth.storefront,term))?;
    let rows=value.pointer("/results/songs/data").and_then(Value::as_array)
        .context("Apple Music returned no recording matches for this song")?;
    let mut matches=std::collections::BTreeMap::new();
    for row in rows {
        let a=&row["attributes"];
        if norm(a["name"].as_str().unwrap_or(""))==norm(&track.title)
            && artist_key(a["artistName"].as_str().unwrap_or(""))==artist_key(&track.artist)
            && norm(a["albumName"].as_str().unwrap_or(""))==norm(&track.album)
            && a["durationInMillis"].as_f64().is_some_and(|ms|(ms/1000.-track.duration as f64).abs()<=2.) {
            if let Some(catalog)=row["id"].as_str().filter(|v|digits(v).is_ok()) {matches.insert(catalog,row);}
        }
    }
    let chosen=crate::lyric_edition::choose(&matches,&policy);
    if chosen.is_none(){for (catalog,row) in &matches{sql.execute("INSERT OR REPLACE INTO choices VALUES(?,?,?)",rusqlite::params![id,catalog,serde_json::to_string(row)?])?;}}
    ensure!(chosen.is_some(),if matches.is_empty(){"Apple Music has no exact recording and album match. Your local song is unchanged; LRCLIB is still available."}else{"Apple Music returned multiple matching editions. Choose the recording in Settings → Integrations before using its lyrics."});
    let (catalog,row)=chosen.unwrap();
    let fresh=musicos_init::database::Db::open(&db.to_string_lossy())?.get_track_by_id(id)?.context("This song is unavailable")?;
    ensure!(signature(&fresh)==signature(&track),"The recording changed during lookup. Try again.");
    ensure!(crate::lyric_edition::policy(&sql,&fresh)?==policy,"Lyric edition changed during lookup. Try again.");
    sql.execute("INSERT OR REPLACE INTO links VALUES(?,?,?,?)",rusqlite::params![id,catalog,row.pointer("/relationships/albums/data/0/id").and_then(Value::as_str),signature(&track)])?;
    crate::lyric_edition::record(&sql,id,catalog,&policy,false)?;
    Ok(catalog.to_owned())
}
/// Offline cache first. Failed Apple attempts never prevent the independent LRCLIB path.
pub fn automatic_lyric(db:&Path,id:i64)->Result<Option<crate::lyrics::Document>> {
    if !edition_stale(db,id)? {if let Some(doc)=cached_lyric(db,id)? {return Ok(Some(doc));}}
    if credentials(db).is_err() {return cached_lyric(db,id);}
    let track=musicos_init::database::Db::open(&db.to_string_lossy())?.get_track_by_id(id)?.context("This song is unavailable")?;
    let sql=store(db)?;
    sql.execute_batch("CREATE TABLE IF NOT EXISTS lyric_attempts(signature TEXT PRIMARY KEY,checked INTEGER NOT NULL)")?;
    let key=format!("{}:{}",signature(&track),crate::lyric_edition::policy(&sql,&track)?.key);
    let checked=sql.query_row("SELECT checked FROM lyric_attempts WHERE signature=?",[&key],|r|r.get::<_,u64>(0)).unwrap_or(0);
    if apple_now().saturating_sub(checked)<60 {return Ok(None);}
    match lyric(db,id) {
        Ok(value)=>Ok(value.get("document").cloned().map(serde_json::from_value).transpose()?),
        Err(_)=>{sql.execute("INSERT OR REPLACE INTO lyric_attempts VALUES(?,?)",rusqlite::params![key,apple_now()])?;Ok(None)}
    }
}
pub fn lyric(db: &Path, id: i64) -> Result<Value> {
    let catalog = lyric_catalog(db, id)?;
    let sql = store(db)?;
    ensure!(!edition_stale(db,id)?,"Lyric edition changed. Try again.");
    if let Ok(raw) = sql.query_row(
        "SELECT document FROM lyric_cache WHERE catalog_id=?",
        [&catalog],
        |r| r.get::<_, String>(0),
    ) {
        let mut doc = serde_json::from_str::<crate::lyrics::Document>(&raw)?;
        identify_document(db, id, &mut doc)?;
        return Ok(json!({"document":doc,"catalogId":catalog,"cached":true}));
    }
    let c = credentials(db)?;
    let value = get(
        &c,
        &format!(
            "/v1/catalog/{}/songs/{}/syllable-lyrics?extend=ttmlLocalizations",
            c.storefront,
            digits(&catalog)?
        ),
    )?;
    let xml =
        ttml(&value).context("Apple Music has no supported timed lyrics for this recording")?;
    let mut doc = crate::lyric_formats::parse_ttml(xml)?;
    if let Some(v) = doc
        .extensions
        .get_mut("x-musicos")
        .and_then(|v| v.as_object_mut())
    {
        v.insert(
            "attribution".into(),
            json!({"provider":"Apple Music","catalogId":catalog}),
        );
    }
    ensure!(
        catalog_for(db, id)? == catalog && !edition_stale(db,id)?,
        "The recording changed during lookup. Match it again before using these lyrics."
    );
    identify_document(db, id, &mut doc)?;
    sql.execute(
        "INSERT OR REPLACE INTO lyric_cache VALUES(?,?)",
        rusqlite::params![catalog, serde_json::to_string(&doc)?],
    )?;
    state().lock().unwrap().lyrics = "available".into();
    Ok(json!({"document":doc,"catalogId":catalog,"cached":false}))
}
fn low_power() -> bool {
    #[cfg(target_os = "macos")]
    {
        if objc2::available!(macos = 12.0) {
            return objc2_foundation::NSProcessInfo::processInfo().isLowPowerModeEnabled();
        }
    }
    false
}
fn media_url(value: &str) -> Result<url::Url> {
    let u = url::Url::parse(value)?;
    ensure!(
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.host_str()
                .is_some_and(|h| h.ends_with(".apple.com") || h.ends_with(".mzstatic.com")),
        "Unsupported artwork host"
    );
    Ok(u)
}
fn sdr_motion(value: &str) -> Result<String> {
    let u = media_url(value)?;
    let raw = bounded(client()?.get(u.clone()).send()?)?;
    ensure!(
        raw.starts_with("#EXTM3U"),
        "Unsupported artwork video playlist"
    );
    let mut choices = vec![];
    let mut descriptor = None;
    for line in raw.lines() {
        if line.starts_with("#EXT-X-STREAM-INF:") {
            descriptor = Some(line.to_string());
        } else if !line.starts_with('#') && !line.trim().is_empty() {
            if let Some(info) = descriptor.take() {
                let lower = info.to_ascii_lowercase();
                if lower.contains("video-range=pq")
                    || lower.contains("video-range=hlg")
                    || lower.contains("dvh1")
                    || lower.contains("dvhe")
                    || !(lower.contains("video-range=sdr")
                        || lower.contains("avc1.42")
                        || lower.contains("avc1.4d")
                        || lower.contains("avc1.64"))
                {
                    continue;
                }
                let size = info
                    .split("RESOLUTION=")
                    .nth(1)
                    .and_then(|v| v.split(',').next())
                    .and_then(|v| v.split_once('x'))
                    .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)));
                if let Some((w, h)) = size {
                    if w <= 1080 && h <= 1080 && w == h {
                        let variant = u.join(line)?;
                        media_url(variant.as_str())?;
                        choices.push((w, variant.to_string()));
                    }
                }
            }
        }
    }
    choices.sort_by_key(|v| v.0);
    choices
        .pop()
        .map(|v| v.1)
        .context("No bounded SDR square artwork variant is available")
}
fn motion_urls(value: &Value) -> Vec<String> {
    let mut urls = vec![];
    match value {
        Value::Object(m) => {
            for (key, v) in m {
                if key == "motionDetailSquare" {
                    if let Some(url) = v
                        .get("video")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            v.get("video")
                                .and_then(|v| v.get("url"))
                                .and_then(|v| v.as_str())
                        })
                        .or_else(|| v.get("url").and_then(|v| v.as_str()))
                    {
                        urls.push(url.to_string());
                    }
                } else {
                    urls.extend(motion_urls(v));
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                urls.extend(motion_urls(v));
            }
        }
        _ => {}
    }
    urls
}
pub fn artwork(db: &Path, id: i64) -> Result<Value> {
    let _ = catalog_for(db, id)?;
    if low_power() {
        return Ok(json!({"state":"low_power","url":null}));
    }
    let sql = store(db)?;
    let album: Option<String> =
        sql.query_row("SELECT album_id FROM links WHERE track_id=?", [id], |r| {
            r.get(0)
        })?;
    let album = if let Some(album) = album {
        album
    } else {
        let catalog = catalog_for(db, id)?;
        let auth = credentials(db)?;
        let detail = get(
            &auth,
            &format!(
                "/v1/catalog/{}/songs/{}?include=albums",
                auth.storefront,
                digits(&catalog)?
            ),
        )?;
        let albums = detail
            .pointer("/data/0/relationships/albums/data")
            .and_then(|v| v.as_array())
            .context("No matching Apple album")?;
        ensure!(
            albums.len() == 1,
            "Choose an unambiguous album edition before using motion artwork"
        );
        let album = albums[0]["id"]
            .as_str()
            .context("Apple album identity is unavailable")?
            .to_string();
        digits(&album)?;
        sql.execute(
            "UPDATE links SET album_id=? WHERE track_id=?",
            rusqlite::params![album, id],
        )?;
        album
    };
    if let Ok(raw) = sql.query_row(
        "SELECT payload FROM artwork_cache WHERE album_id=?",
        [&album],
        |r| r.get::<_, String>(0),
    ) {
        let cached: Value = serde_json::from_str(&raw)?;
        let age = apple_now().saturating_sub(cached["checkedAt"].as_u64().unwrap_or(0));
        if age
            < if cached["url"].is_string() {
                7 * 86400
            } else {
                86400
            }
        {
            return Ok(cached);
        }
    }
    let c = credentials(db)?;
    let value = get(
        &c,
        &format!(
            "/v1/catalog/{}/albums/{}?extend=editorialVideo",
            c.storefront,
            digits(&album)?
        ),
    )?;
    let url = motion_urls(&value)
        .into_iter()
        .find_map(|s| sdr_motion(&s).ok());
    let payload = json!({"albumId":album,"url":url,"checkedAt":apple_now(),"state":if url.is_some(){"available"}else{"unavailable"}});
    sql.execute(
        "INSERT OR REPLACE INTO artwork_cache VALUES(?,?)",
        rusqlite::params![album, serde_json::to_string(&payload)?],
    )?;

    state().lock().unwrap().artwork = payload["state"].as_str().unwrap().into();
    Ok(payload)
}
struct Job;
impl Drop for Job {
    fn drop(&mut self) {
        JOB.store(false, Ordering::SeqCst);
    }
}
fn sync(db: &Path) -> Result<Value> {
    ensure!(
        !JOB.swap(true, Ordering::SeqCst),
        "An Apple sync is already running"
    );
    let _job = Job;
    CANCEL.store(false, Ordering::SeqCst);
    let credentials = credentials(db)?;
    let local = musicos_init::database::Db::open(&db.to_string_lossy())?.get_tracks()?;
    let mut songs = vec![];
    let mut next = "/v1/me/library/songs?limit=100&include=catalog".to_string();
    let mut visited = std::collections::BTreeSet::new();
    let mut official = None;
    {
        let mut s = state().lock().unwrap();
        s.phase = "matching".into();
        s.scanned = 0;
        s.matched = 0;
        s.ambiguous = 0;
        s.message = "Reading your Apple library. Your local files stay unchanged.".into();
    }
    for _ in 0..250 {
        ensure!(
            !CANCEL.load(Ordering::SeqCst),
            "Sync cancelled. Existing matches are retained."
        );
        ensure!(
            visited.insert(next.clone()),
            "Apple returned a pagination cycle"
        );
        let page = library_page(&credentials, &next, &mut official)?;
        let rows = page["data"]
            .as_array()
            .context("Apple library format is unavailable")?;
        ensure!(
            rows.len() <= 1000 && songs.len() + rows.len() <= 25000,
            "Apple library exceeds this prototype’s bounded import size"
        );
        for song in rows {
            songs.push(song.clone());
        }
        state().lock().unwrap().scanned = songs.len();
        if let Some(n) = page["next"].as_str() {
            ensure!(
                n.starts_with("/v1/") && !n.contains("://"),
                "Unsupported library pagination"
            );
            next = n.to_string();
            std::thread::sleep(Duration::from_millis(300));
        } else {
            next.clear();
            break;
        }
    }
    ensure!(
        next.is_empty(),
        "Apple library exceeds this prototype’s paging limit"
    );
    let songs = songs
        .into_iter()
        .filter_map(|song| catalog_record(&song))
        .collect::<Vec<_>>();
    let mut index = std::collections::BTreeMap::<(String, String, String), Vec<&Value>>::new();
    for song in &songs {
        let row = song
            .pointer("/relationships/catalog/data/0")
            .unwrap_or(song);
        let a = &row["attributes"];
        index
            .entry((
                apple_title_key(a["name"].as_str().unwrap_or("")),
                artist_key(a["artistName"].as_str().unwrap_or("")),
                norm(a["albumName"].as_str().unwrap_or("")),
            ))
            .or_default()
            .push(row);
    }
    let mut sql = store(db)?;
    let tx = sql.transaction()?;
    tx.execute("DELETE FROM choices", [])?;

    let mut matched = 0;
    let mut ambiguous = 0;
    for track in &local {
        ensure!(
            !CANCEL.load(Ordering::SeqCst),
            "Sync cancelled. Existing matches are retained."
        );
        let saved: Option<String> = tx
            .query_row(
                "SELECT catalog_id FROM asset_links WHERE track_id=? AND signature=?",
                rusqlite::params![track.id, signature(track)],
                |r| r.get(0),
            )
            .ok();
        let policy=crate::lyric_edition::policy(&tx,track)?;
        if saved.as_deref().is_some_and(|id|crate::lyric_edition::valid(&tx,track.id,id,&policy).unwrap_or(false)) {
            tx.execute("INSERT OR REPLACE INTO links SELECT * FROM asset_links WHERE track_id=?",[track.id])?;
            matched += 1;
            continue;
        }
        let key = (apple_title_key(&track.title), artist_key(&track.artist), norm(&track.album));
        let candidates = index
            .get(&key)
            .into_iter()
            .flatten()
            .copied()
            .filter(|row| {
                (row["attributes"]["durationInMillis"]
                    .as_f64()
                    .unwrap_or(-999.)
                    / 1000.
                    - track.duration as f64)
                    .abs()
                    <= 2.
            });
        let mut seen = std::collections::BTreeMap::new();
        for row in candidates {
            if let Some(id) = row["id"].as_str().filter(|v| digits(v).is_ok()) {
                seen.insert(id, row);
            }
        }
        if let Some((id,row))=crate::lyric_edition::choose(&seen,&policy) {
            let album = row
                .pointer("/relationships/albums/data/0/id")
                .and_then(|v| v.as_str());
            tx.execute(
                "INSERT OR REPLACE INTO links VALUES(?,?,?,?)",
                rusqlite::params![track.id, id, album, signature(track)],
            )?;
            crate::lyric_edition::record(&tx,track.id,id,&policy,false)?;
            matched += 1;
        } else if !seen.is_empty() {
            ambiguous += 1;
            for (id, row) in seen {
                tx.execute(
                    "INSERT OR REPLACE INTO choices VALUES(?,?,?)",
                    rusqlite::params![track.id, id, serde_json::to_string(row)?],
                )?;
            }
        }
    }
    tx.commit()?;
    let mut s = state().lock().unwrap();
    s.library = "available".into();
    s.phase = "ready".into();
    s.matched = matched;
    s.ambiguous = ambiguous;
    s.message = format!(
        "{matched} local songs matched. {ambiguous} need a choice. No music files were changed."
    );
    Ok(json!({"status":s.clone()}))
}
static AUTH_EPOCH: AtomicU64 = AtomicU64::new(0);
fn apple_navigation(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|h| {
            h == "apple.com" || h.ends_with(".apple.com") || h.ends_with(".icloud.com")
        })
}
fn close_auth_popups(app: &tauri::AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("apple-enrichment-popup-") {
            let _ = w.close();
        }
    }
}
static VERIFYING: AtomicBool = AtomicBool::new(false);
struct Verification;
impl Drop for Verification {
    fn drop(&mut self) {
        VERIFYING.store(false, Ordering::SeqCst);
    }
}
fn verify_connection(app: &tauri::AppHandle, db: &Path) -> Result<Value> {
    if VERIFYING.swap(true, Ordering::SeqCst) {
        return Ok(json!({"status":state().lock().unwrap().clone()}));
    }
    let _verification = Verification;
    let epoch = AUTH_EPOCH.load(Ordering::SeqCst);

    let window = app
        .get_webview_window("apple-enrichment-signin")
        .context("Open the Apple sign-in window first")?;
    let cookie=window.cookies_for_url("https://music.apple.com/".parse()?)?.into_iter().find(|c|c.name()=="media-user-token").context("Apple has not provided a supported session. Finish signing in; if Apple blocks this window, this experimental capability is unavailable.")?;
    let mut c = Credentials {
        user: cookie_token(cookie.value())?,
        bearer: bearer()?,
        storefront: "us".into(),
    };
    let sf = get(&c, "/v1/me/storefront")?;
    let region = sf
        .pointer("/data/0/id")
        .and_then(|v| v.as_str())
        .context("Apple storefront is unavailable")?;
    ensure!(
        region.len() == 2 && region.bytes().all(|v| v.is_ascii_alphabetic()),
        "Unsupported storefront"
    );
    c.storefront = region.to_lowercase();
    ensure!(
        epoch == AUTH_EPOCH.load(Ordering::SeqCst)
            && app.get_webview_window("apple-enrichment-signin").is_some(),
        "Sign-in was cancelled before verification completed"
    );
    // Reconnecting a session must not discard recording choices, asset links,
    // cached lyrics, or the existing library index. Refresh is a separate action.
    entry(db)?
        .set_password(&serde_json::to_string(&c)?)
        .map_err(|_| anyhow::anyhow!("Could not save the Apple session in the system keychain"))?;
    close_auth_popups(app);
    window.close()?;
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
    let retained=store(db)?;
    let matched:usize=retained.query_row("SELECT count(*) FROM links",[],|r|r.get(0))?;
    let ambiguous:usize=retained.query_row("SELECT count(DISTINCT track_id) FROM choices",[],|r|r.get(0))?;
    let mut s = state().lock().unwrap();
    s.signed_in = true;
    s.scanned = 0;
    s.matched = matched;
    s.ambiguous = ambiguous;
    s.completed = 0;
    s.total = 0;
    s.lyrics_ready = 0;
    s.lyrics_failed = 0;
    s.library = if matched>0 {"available"}else{"not_checked"}.into();
    s.lyrics = "not_checked".into();
    s.artwork = "not_checked".into();
    s.sign_in = "available".into();
    s.phase = "connected".into();
    s.message = "Signed in. Library, lyrics, and motion artwork are checked independently.".into();
    let status=s.clone();drop(s);let _=app.emit_to("main","apple-enrichment-status",status.clone());
    Ok(json!({"status":status}))
}
fn watch_signin(app: tauri::AppHandle, db: std::path::PathBuf, epoch: u64) {
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(600) {
            if epoch != AUTH_EPOCH.load(Ordering::SeqCst) {
                return;
            }
            let Some(window) = app.get_webview_window("apple-enrichment-signin") else {
                return;
            };
            let ready = window
                .cookies_for_url("https://music.apple.com/".parse().unwrap())
                .ok()
                .is_some_and(|cookies| {
                    cookies
                        .iter()
                        .any(|c| c.name() == "media-user-token" && !c.value().is_empty())
                });
            if ready {
                let _ = action(
                    &app,
                    &db,
                    Request {
                        action: "verify".into(),
                        track_id: None,
                        catalog_id: None,
                    },
                );
                let _ = app.emit_to(
                    "main",
                    "apple-enrichment-status",
                    state().lock().unwrap().clone(),
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(800));
        }
    });
}
pub fn action(app: &tauri::AppHandle, db: &Path, req: Request) -> Result<Value> {
    let result = (|| -> Result<Value> {
        match req.action.as_str() {
            "transfers" | "refresh_transfers" | "check_playlist" | "transfer_playlist" => {
                transfers::action(db, &req.action, req.catalog_id.as_deref())
            }
            "start_cache" => background::start(app, db),
            "stop_cache" => {
                background::stop(db)?;
                Ok(json!({"status":state().lock().unwrap().clone()}))
            }
            "motion_policy" => Ok(json!({"lowPower":low_power()})),
            "status" => {
                let waiting = state().lock().unwrap().phase == "signing_in";
                if waiting && !VERIFYING.load(Ordering::SeqCst) {
                    if let Some(w) = app.get_webview_window("apple-enrichment-signin") {
                        let ready = w
                            .cookies_for_url("https://music.apple.com/".parse()?)?
                            .iter()
                            .any(|c| c.name() == "media-user-token" && !c.value().is_empty());
                        if ready {
                            return verify_connection(app, db);
                        }
                    } else {
                        let mut s = state().lock().unwrap();
                        s.phase = "cancelled".into();
                        s.sign_in = "not_checked".into();
                        s.message="Apple’s window closed before the connection was verified. Open it to try again.".into();
                    }
                }
                Ok(json!({"status":state().lock().unwrap().clone()}))
            }
            "begin" => {
                ensure!(!JOB.load(Ordering::SeqCst), "Finish the current sync first");
                ensure!(
                    !VERIFYING.load(Ordering::SeqCst),
                    "Finishing the current connection check"
                );
                AUTH_EPOCH.fetch_add(1, Ordering::SeqCst);
                if let Some(window) = app.get_webview_window("apple-enrichment-signin") {
                    window.set_focus()?;
                } else {
                    let popup_app = app.clone();
                    tauri::WebviewWindowBuilder::new(
                        app,
                        "apple-enrichment-signin",
                        tauri::WebviewUrl::External("https://music.apple.com/".parse()?),
                    )
                    .title("Connect Apple Music · musicOS")
                    .inner_size(980., 760.)
                    .min_inner_size(600., 480.)
                    .incognito(true)
                    .on_navigation(apple_navigation)
                    .on_new_window(move |url, features| {
                        use tauri::webview::NewWindowResponse;
                        if !apple_navigation(&url) && url.as_str() != "about:blank" {
                            return NewWindowResponse::Deny;
                        }
                        if popup_app
                            .webview_windows()
                            .keys()
                            .filter(|k| k.starts_with("apple-enrichment-popup-"))
                            .count()
                            >= 2
                        {
                            return NewWindowResponse::Deny;
                        }
                        let built = tauri::WebviewWindowBuilder::new(
                            &popup_app,
                            format!("apple-enrichment-popup-{}", rand::random::<u64>()),
                            tauri::WebviewUrl::External("about:blank".parse().unwrap()),
                        )
                        .window_features(features)
                        .title("Apple account · musicOS")
                        .inner_size(600., 720.)
                        .on_navigation(|url| url.as_str() == "about:blank" || apple_navigation(url))
                        .build();
                        match built {
                            Ok(window) => NewWindowResponse::Create { window },
                            Err(_) => NewWindowResponse::Deny,
                        }
                    })
                    .build()?;
                }
                let mut s = state().lock().unwrap();
                s.enabled = true;
                s.phase = "signing_in".into();
                s.sign_in = "waiting".into();
                s.message = "Finish signing in in Apple’s window. musicOS will check the session and bring you back automatically.".into();
                let status = s.clone();
                drop(s);
                watch_signin(
                    app.clone(),
                    db.to_owned(),
                    AUTH_EPOCH.load(Ordering::SeqCst),
                );
                Ok(json!({"status":status}))
            }
            "verify" => {
                if VERIFYING.load(Ordering::SeqCst) {
                    return Ok(json!({"status":state().lock().unwrap().clone()}));
                }
                verify_connection(app, db)
            }
            "sync" => sync(db),
            "sync_lyrics" => {
                ensure!(
                    !JOB.swap(true, Ordering::SeqCst),
                    "A sync is already running"
                );
                let _job = Job;
                CANCEL.store(false, Ordering::SeqCst);
                let sql = store(db)?;
                let ids = {
                    let mut q = sql.prepare("SELECT track_id FROM links ORDER BY track_id")?;
                    let result = q
                        .query_map([], |r| r.get::<_, i64>(0))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    result
                };
                {
                    let mut s = state().lock().unwrap();
                    s.phase = "lyrics_sync".into();
                    s.completed = 0;
                    s.total = ids.len();
                    s.lyrics_ready = 0;
                    s.lyrics_failed = 0;
                    s.message =
                        "Fetching lyrics for matched songs. Your saved selections stay unchanged."
                            .into();
                }
                for &id in &ids {
                    if CANCEL.load(Ordering::SeqCst) {
                        break;
                    }
                    let result = lyric(db, id);
                    {
                        let mut s = state().lock().unwrap();
                        s.completed += 1;
                        if result.is_ok() {
                            s.lyrics_ready += 1;
                        } else {
                            s.lyrics_failed += 1;
                            if s.lyrics_ready == 0 {
                                s.lyrics = "unavailable".into();
                            }
                        }
                    }
                    if result.is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(350));
                }
                let mut s = state().lock().unwrap();
                s.phase = if CANCEL.load(Ordering::SeqCst) {
                    "cancelled"
                } else if s.lyrics_failed > 0 {
                    "partial"
                } else {
                    "ready"
                }
                .into();
                let ready = s.lyrics_ready;
                let failed = s.lyrics_failed;
                drop(s);
                let files = if CANCEL.load(Ordering::SeqCst) {
                    None
                } else {
                    Some(crate::lyric_sidecars::sync_tracks(db, Some(&ids))?)
                };
                let mut s = state().lock().unwrap();
                let sidecar_ids = files
                    .as_ref()
                    .map(|f| f["ready_ids"].clone())
                    .unwrap_or(json!([]));
                s.message = if let Some(files) = files {
                    format!("{ready} lyrics cached; {failed} unavailable. {} LyricsFiles saved, {} current, {} existing edits kept, {} file errors. Saved lyric choices stay authoritative.",files["written"],files["unchanged"],files["conflicts"],files["failed"])
                } else {
                    "Cancelled. Cached lyrics are retained; no sidecar sync started.".into()
                };
                Ok(json!({"status":s.clone(),"sidecarTrackIds":sidecar_ids}))
            }
            "cancel" => {
                AUTH_EPOCH.fetch_add(1, Ordering::SeqCst);
                close_auth_popups(app);
                CANCEL.store(true, Ordering::SeqCst);
                if let Some(w) = app.get_webview_window("apple-enrichment-signin") {
                    w.close()?;
                }
                let mut s = state().lock().unwrap();
                s.phase = "cancelled".into();
                s.message = "Cancelled. Your local music and saved lyrics are unchanged.".into();
                Ok(json!({"status":s.clone()}))
            }
            "disconnect" => {
                background::stop(db)?;
                AUTH_EPOCH.fetch_add(1, Ordering::SeqCst);
                close_auth_popups(app);
                CANCEL.store(true, Ordering::SeqCst);
                ensure!(
                    !JOB.load(Ordering::SeqCst),
                    "Cancel sync before disconnecting"
                );
                if let Some(w) = app.get_webview_window("apple-enrichment-signin") {
                    w.close()?;
                }
                transfers::clear(db)?;
                background::clear(db)?;
                match entry(db)?.delete_credential() {
                    Ok(()) | Err(keyring::Error::NoEntry) => {}
                    Err(_) => {
                        anyhow::bail!("Could not remove the session from the system keychain")
                    }
                };
                store(db)?.execute_batch("DELETE FROM links;DELETE FROM choices;DELETE FROM lyric_attempts;")?;
                *state().lock().unwrap() = Status {
                    phase: "disconnected".into(),
                    message: "Disconnected. Your selected lyrics are preserved.".into(),
                    ..Default::default()
                };
                Ok(json!({"status":state().lock().unwrap().clone()}))
            }
            "lyrics" => lyric(db, req.track_id.context("Choose a local song")?),
            "artwork" => artwork(db, req.track_id.context("Choose a local song")?),
            "expanded_artwork" => {
                let id=req.track_id.context("Choose a local song")?;
                // Still and video capabilities are independent. Keep a durable
                // still cover usable when motion artwork or the network fails.
                let still_error=background::cover(db,id).err().map(|e|e.to_string());
                let still=background::cached_cover(db,id);
                let mut result=artwork(db,id).unwrap_or_else(|e|json!({"state":"unavailable","url":null,"motionError":e.to_string()}));
                if let Some(message)=still_error.as_deref().filter(|m|m.contains("HTTP 401")).or_else(||result["motionError"].as_str().filter(|m|m.contains("HTTP 401"))){apply_action_error(&mut state().lock().unwrap(),"artwork",message);}
                result["staticPath"]=json!(still);result["staticError"]=json!(still_error);result["status"]=json!(state().lock().unwrap().clone());
                Ok(result)
            },
            "matches" => {
                let c = store(db)?;
                let tracks =
                    musicos_init::database::Db::open(&db.to_string_lossy())?.get_tracks()?;
                let mut matches = vec![];
                for t in tracks {
                    let mut query=c.prepare("SELECT links.catalog_id,EXISTS(SELECT 1 FROM lyric_cache WHERE lyric_cache.catalog_id=links.catalog_id) FROM links WHERE track_id=? AND signature=?")?;
                    if let Ok((catalog, cached)) = query
                        .query_row(rusqlite::params![t.id, signature(&t)], |r| {
                            Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?))
                        })
                    {
                        matches.push(json!({"trackId":t.id,"catalogId":catalog,"title":t.title,"artist":t.artist,"album":t.album,"cached":cached}));
                    }
                }
                Ok(json!({"matches":matches}))
            }
            "choices" => {
                let c = store(db)?;
                let mut q =
                    c.prepare("SELECT track_id,catalog_id,payload FROM choices ORDER BY track_id")?;
                let rows = q.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))?;
                let library = musicos_init::database::Db::open(&db.to_string_lossy())?;
                let mut choices = Vec::new();
                let mut local = std::collections::HashMap::new();
                for row in rows {
                    let (id, catalog, raw) = row?;
                    if let std::collections::hash_map::Entry::Vacant(entry) = local.entry(id) {
                        entry.insert(library.get_track_by_id(id)?);
                    }
                    let Some(track) = local[&id].as_ref() else { continue };
                    let Ok(item) = serde_json::from_str::<Value>(&raw) else { continue };
                    choices.push(json!({"trackId":id, "catalogId":catalog, "item":item,
                        "local":{"title":track.title, "artist":track.artist, "album":track.album}}));
                }
                Ok(json!({"choices":choices}))
            }
            "select_match" => {
                let id = req.track_id.context("Choose a song")?;
                let catalog = req.catalog_id.context("Choose a version")?;
                let c = store(db)?;
                let raw: String = c.query_row(
                    "SELECT payload FROM choices WHERE track_id=? AND catalog_id=?",
                    rusqlite::params![id, catalog],
                    |r| r.get(0),
                )?;
                let row: Value = serde_json::from_str(&raw)?;
                let track = musicos_init::database::Db::open(&db.to_string_lossy())?
                    .get_track_by_id(id)?
                    .context("Song not available")?;
                c.execute(
                    "INSERT OR REPLACE INTO links VALUES(?,?,?,?)",
                    rusqlite::params![
                        id,
                        catalog,
                        row.pointer("/relationships/albums/data/0/id")
                            .and_then(|v| v.as_str()),
                        signature(&track)
                    ],
                )?;
                crate::lyric_edition::record(&c,id,&catalog,&crate::lyric_edition::policy(&c,&track)?,true)?;
                c.execute("DELETE FROM choices WHERE track_id=?", [id])?;
                Ok(json!({"status":state().lock().unwrap().clone()}))
            }
            _ => anyhow::bail!("Unknown Apple enrichment action"),
        }
    })();
    if let Err(error) = &result {
        let status={let mut s=state().lock().unwrap();apply_action_error(&mut s,&req.action,&error.to_string());s.clone()};
        let _=app.emit_to("main","apple-enrichment-status",status);
    }
    result
}
fn apply_action_error(s: &mut Status, action: &str, message: &str) {
    s.message = message.to_owned();
    if message.contains("HTTP 401") {
        s.signed_in = false;
        s.sign_in = "unavailable".into();
        s.phase = "reconnect_required".into();
    } else if matches!(action, "verify" | "status") && s.phase == "signing_in" {
        // Keep Apple's window and the explicit Check connection action usable.
        // A temporary verification failure does not silently restart requests.
        s.sign_in = "waiting".into();
    } else if matches!(action, "sync" | "sync_lyrics" | "refresh_transfers") {
        s.phase = if s.signed_in { "ready" } else { "unavailable" }.into();
    }
    match action {
        "sync" => s.library = "unavailable".into(),
        "lyrics" | "sync_lyrics" => s.lyrics = "unavailable".into(),
        "artwork" => s.artwork = "unavailable".into(),
        _ => {}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_relationship_stub_keeps_library_match_metadata() {
        let song = json!({"id":"i.local","type":"library-songs",
            "attributes":{"name":"Song [Explicit]","artistName":"Ye & Guest","albumName":"Release","durationInMillis":180000},
            "relationships":{"catalog":{"data":[{"id":"123","type":"songs"}]}}});
        let row = catalog_record(&song).unwrap();
        assert_eq!(row["id"],"123");
        assert_eq!(row["attributes"]["durationInMillis"],180000);
        assert_eq!(apple_title_key(row["attributes"]["name"].as_str().unwrap()),"song");
        assert_eq!(artist_key("Kanye West; Guest"),artist_key("Ye & Guest"));
        assert_ne!(apple_title_key("Song (Live)"),apple_title_key("Song"));
        assert!(catalog_record(&json!({"id":"123","type":"library-songs"})).is_none());
    }
    #[test]
    fn artist_alias_is_whole_credit_only() {
        assert_eq!(artist_key(" Ye "), artist_key("Kanye West"));
        assert_eq!(artist_key("Kanye"), artist_key("Kanye West"));
        assert_ne!(artist_key("Ye & JAY-Z"), artist_key("Kanye West"));
        assert_ne!(artist_key("Yeasayer"), artist_key("Kanye West"));
        assert_ne!(norm("Ye"), norm("Kanye West"));
    }
    #[test]
    fn capability_errors_do_not_disconnect_or_end_background_work() {
        let mut s = Status { signed_in: true, sign_in: "available".into(), phase: "background_cache".into(), ..Default::default() };
        apply_action_error(&mut s, "artwork", "Artwork is unavailable");
        assert!(s.signed_in);assert_eq!(s.phase,"background_cache");assert_eq!(s.artwork,"unavailable");
        apply_action_error(&mut s,"lyrics","Apple Music returned HTTP 401");
        assert!(!s.signed_in);assert_eq!(s.phase,"reconnect_required");
        s.phase="signing_in".into();apply_action_error(&mut s,"verify","Could not reach Apple Music");
        assert_eq!(s.phase,"signing_in");assert_eq!(s.sign_in,"waiting");
    }
    #[test]
    fn session_token_expiry_is_read_without_exposing_token_contents() {
        let payload=base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp":1234}"#);
        assert_eq!(token_expiry(&format!("header.{payload}.signature")),Some(1234));
        assert_eq!(token_expiry("unsupported"),None);
    }
    #[test]
    fn identity_and_url_guards() {
        assert!(digits("1234").is_ok());
        assert!(digits("../bad").is_err());
        assert_eq!(norm("Test—Song"), "testsong");
        assert!(apple_navigation(
            &"https://idmsa.apple.com/appleauth/auth".parse().unwrap()
        ));
        assert!(!apple_navigation(
            &"https://apple.com.attacker.test/".parse().unwrap()
        ));
        assert!(!apple_navigation(
            &"http://music.apple.com/".parse().unwrap()
        ));
    }
    #[test]
    fn cached_apple_lyrics_are_default_offline_and_local_edits_win() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("library.db");
        let db=musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();
        let id=db.insert_track("/absent.flac","Test","Artist","Album",180,1,1).unwrap();
        let track=db.get_track_by_id(id).unwrap().unwrap();
        let sql=store(&path).unwrap();
        sql.execute("INSERT INTO links VALUES(?,?,?,?)",rusqlite::params![id,"123","456",signature(&track)]).unwrap();
        let doc=crate::lyrics::parse("[00:01.00]Original fixture words","lrc").unwrap();
        sql.execute("INSERT INTO lyric_cache VALUES(?,?)",rusqlite::params!["123",serde_json::to_string(&doc).unwrap()]).unwrap();
        assert!(automatic_lyric(&path,id).unwrap().is_some());
        let load=||crate::lyrics::execute(&path,crate::lyrics::Request{action:"load".into(),track_id:Some(id),..Default::default()}).unwrap();
        let ready=load();assert_eq!(ready["source"]["label"],"Apple Music");assert_eq!(ready["document"]["metadata"]["title"],"Test");
        crate::lyrics::execute(&path,crate::lyrics::Request{action:"save".into(),track_id:Some(id),revision:Some(0),document:Some(crate::lyrics::parse("My local edit","plain").unwrap()),..Default::default()}).unwrap();
        assert_eq!(load()["document"]["plain"],"My local edit");
    }
    #[test]
    fn late_album_identity_updates_the_durable_artwork_link() {
        let root=tempfile::tempdir().unwrap();let path=root.path().join("library.db");let c=store(&path).unwrap();
        c.execute("INSERT INTO links VALUES(1,'123',NULL,'recording')",[]).unwrap();
        c.execute("UPDATE links SET album_id='456' WHERE track_id=1",[]).unwrap();
        assert_eq!(c.query_row("SELECT album_id FROM asset_links WHERE track_id=1",[],|r|r.get::<_,String>(0)).unwrap(),"456");
        c.execute("DELETE FROM links WHERE track_id=1",[]).unwrap();
        assert_eq!(c.query_row("SELECT catalog_id FROM asset_links WHERE track_id=1",[],|r|r.get::<_,String>(0)).unwrap(),"123");
    }
    #[test]
    fn existing_null_album_links_are_repaired_without_changing_recordings() {
        let root=tempfile::tempdir().unwrap();let path=root.path().join("library.db");let c=store(&path).unwrap();
        c.execute("INSERT INTO links VALUES(1,'123','456','recording')",[]).unwrap();
        c.execute("UPDATE asset_links SET album_id=NULL WHERE track_id=1",[]).unwrap();
        c.pragma_update(None,"user_version",1).unwrap();drop(c);
        let repaired=store(&path).unwrap();
        assert_eq!(repaired.query_row("SELECT album_id FROM asset_links WHERE track_id=1 AND catalog_id='123' AND signature='recording'",[],|r|r.get::<_,String>(0)).unwrap(),"456");
    }
    #[test]
    fn nested_lyrics() {
        assert_eq!(
            ttml(&json!({"data":[{"attributes":{"ttml":"<tt />"}}]})),
            Some("<tt />")
        );
    }
}

#[cfg(test)]
mod live_probe {
    #[test]
    #[ignore = "Explicit public-site feasibility check; never uses an account"]
    fn public_web_authorization() {
        let token = super::bearer().expect("Apple public web authorization is unavailable");
        assert!(token.len() > 100);
        println!("Apple public web authorization is available; token value omitted.");
    }
}

#[cfg(test)]
mod live_art_probe {
    use super::*;
    #[test]
    #[ignore = "Explicit public catalog feasibility check; never uses an account"]
    fn public_sdr_motion() {
        let c = Credentials {
            user: String::new(),
            bearer: bearer().unwrap(),
            storefront: "us".into(),
        };
        let search=get(&c,"/v1/catalog/us/search?term=billie%20eilish%20hit%20me%20hard%20and%20soft&types=albums&limit=1").unwrap();
        let id = search
            .pointer("/results/albums/data/0/id")
            .and_then(|v| v.as_str())
            .unwrap();
        let album = get(
            &c,
            &format!(
                "/v1/catalog/us/albums/{}?extend=editorialVideo",
                digits(id).unwrap()
            ),
        )
        .unwrap();
        let attrs = &album["data"][0]["attributes"];
        println!("Public album title: {}", attrs["name"]);
        println!(
            "Motion descriptor keys: {:?}",
            attrs
                .get("editorialVideo")
                .and_then(|v| v.as_object())
                .map(|m| m.keys().collect::<Vec<_>>())
        );
        if let Some(m) = attrs.get("editorialVideo").and_then(|v| v.as_object()) {
            for (k, v) in m {
                println!(
                    "{k} fields: {:?}",
                    v.as_object().map(|m| m.keys().collect::<Vec<_>>())
                );
            }
        }
        let urls = motion_urls(&album);
        assert!(
            !urls.is_empty(),
            "No square motion descriptor in the public catalog response"
        );
        assert!(
            urls.iter().any(|u| sdr_motion(u).is_ok()),
            "No bounded SDR square variant available"
        );
        println!("Public Apple catalog motion descriptor and bounded SDR variant verified; URLs omitted.");
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;
    #[test]
    fn library_ids_resolve_to_catalog_without_importing_cloud_rows() {
        let item =
            json!({"id":"i.local","attributes":{"name":"Song","playParams":{"catalogId":"1234"}}});
        assert_eq!(catalog_record(&item).unwrap()["id"], "1234");
        assert!(catalog_record(&json!({"id":"i.uploaded","attributes":{}})).is_none());
        assert!(media_url("https://example.com/video.m3u8").is_err());
        assert!(media_url("https://video.apple.com.evil.test/video.m3u8").is_err());
        assert!(media_url("http://video.apple.com/video.m3u8").is_err());
        assert!(media_url("https://user:password@video.apple.com/video.m3u8").is_err());
        assert!(media_url("https://video.apple.com/video.m3u8").is_ok());
        assert_eq!(cookie_token("a%2Bb+c").unwrap(), "a+b+c");
        assert!(cookie_token("bad%0Dsession").is_err());
    }
}

#[cfg(target_os = "macos")]
thread_local! {static POWER_OBSERVER:std::cell::RefCell<Option<objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_foundation::NSObjectProtocol>>>>=const{std::cell::RefCell::new(None)};}
#[cfg(target_os = "macos")]
pub fn install_power_events(app: &tauri::AppHandle) {
    if !objc2::available!(macos = 12.0) {
        return;
    }
    POWER_OBSERVER.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let app = app.clone();
        let block = block2::RcBlock::new(
            move |_: std::ptr::NonNull<objc2_foundation::NSNotification>| {
                use tauri::Emitter;
                let _ = app.emit_to(
                    "main",
                    "musicos-motion-policy",
                    json!({"lowPower":low_power()}),
                );
            },
        );
        let center = objc2_foundation::NSNotificationCenter::defaultCenter();
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(objc2_foundation::NSProcessInfoPowerStateDidChangeNotification),
                None,
                None,
                &block,
            )
        };
        *slot.borrow_mut() = Some(observer);
    });
}
#[cfg(target_os = "macos")]
pub fn shutdown_power_events() {
    POWER_OBSERVER.with(|slot| {
        if let Some(observer) = slot.borrow_mut().take() {
            unsafe {
                objc2_foundation::NSNotificationCenter::defaultCenter().removeObserver(AsRef::<
                    objc2::runtime::AnyObject,
                >::as_ref(
                    &*observer,
                ));
            }
        }
    });
}
