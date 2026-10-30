//! Desktop-owned lyric documents. Provider cache never overwrites a user selection.
use anyhow::{bail, ensure, Context, Result};
use lofty::{file::TaggedFileExt, tag::ItemKey};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Word {
    pub text: String,
    pub start_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<i64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub text: String,
    pub start_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<Word>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Info {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub offset_ms: i64,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub instrumental: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub version: String,
    pub metadata: Info,
    #[serde(default)]
    pub lines: Vec<Line>,
    #[serde(default)]
    pub plain: String,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: "1.0".into(),
            metadata: Info::default(),
            lines: vec![],
            plain: String::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub kind: String,
    pub label: String,
    pub url: Option<String>,
    pub provider_id: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub document: Document,
}
#[derive(Serialize)]
pub struct Response {
    pub track_id: i64,
    pub revision: i64,
    pub document: Option<Document>,
    pub source: Option<Source>,
    pub state: String,
    pub message: Option<String>,
    pub candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_status: Option<u16>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub action: String,
    pub track_id: Option<i64>,
    pub revision: Option<i64>,
    pub document: Option<Document>,
    pub candidate_id: Option<i64>,
    pub text: Option<String>,
    pub format: Option<String>,
    pub path: Option<PathBuf>,
    pub overwrite: Option<bool>,
    pub history_id: Option<i64>,
    pub retry: Option<bool>,
    pub write_files: Option<bool>,
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn store(db: &Path) -> Result<Connection> {
    let p = db.with_file_name("desktop-lyrics-v1.sqlite3");
    let c = Connection::open(p)?;
    c.busy_timeout(Duration::from_secs(10))?;
    c.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS selections(track_id INTEGER PRIMARY KEY,revision INTEGER NOT NULL,document TEXT,source TEXT); CREATE TABLE IF NOT EXISTS cache(signature TEXT PRIMARY KEY,checked INTEGER NOT NULL,payload TEXT NOT NULL); CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY,track_id INTEGER NOT NULL,at INTEGER NOT NULL,before_doc TEXT,before_source TEXT,after_doc TEXT,after_source TEXT,revision INTEGER NOT NULL,undone INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS exports(id INTEGER PRIMARY KEY,path TEXT NOT NULL,backup TEXT,at INTEGER NOT NULL);")?;
    Ok(c)
}
fn validate(doc: &Document) -> Result<()> {
    ensure!(doc.version == "1.0", "Unsupported LyricsFile version");
    ensure!(
        doc.plain.len() <= 1_000_000 && doc.lines.len() <= 10000,
        "Lyrics are too large"
    );
    ensure!(
        doc.metadata.offset_ms.abs() <= 3_600_000,
        "Timing offset is too large"
    );
    let mut last = -1;
    for line in &doc.lines {
        ensure!(
            line.start_ms >= last && line.start_ms <= 86_400_000 && line.text.len() <= 20000,
            "Check line timestamps and text length"
        );
        last = line.start_ms;
        ensure!(
            line.end_ms.is_none_or(|end| end >= line.start_ms),
            "A line ends before it starts"
        );
        let mut word_time = line.start_ms;
        ensure!(line.words.len() <= 2000, "Too many words in a line");
        for word in &line.words {
            ensure!(
                word.start_ms >= word_time && word.end_ms.is_none_or(|end| end >= word.start_ms),
                "Check word timestamps"
            );
            ensure!(word.start_ms <= 86_400_000, "Word timestamp is too large");
            word_time = word.start_ms;
        }
        if !line.words.is_empty() {
            let text = line
                .words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<String>();
            ensure!(
                normalize(&text) == normalize(&line.text),
                "Word timing does not match its line text"
            );
        }
    }
    ensure!(
        !doc.metadata.instrumental || (doc.lines.is_empty() && doc.plain.trim().is_empty()),
        "Instrumental songs cannot also contain lyrics"
    );
    Ok(())
}
fn timestamp(value: &str) -> Option<i64> {
    let (min, sec) = value.split_once(':')?;
    let min = min.parse::<i64>().ok()?;
    let sec = sec.parse::<f64>().ok()?;
    if min < 0 || !sec.is_finite() || !(0. ..60.).contains(&sec) {
        return None;
    }
    Some(min * 60000 + (sec * 1000.).round() as i64)
}
pub fn parse(text: &str, format: &str) -> Result<Document> {
    ensure!(
        text.len() <= 2_000_000 && !text.contains('\0'),
        "Choose a lyric file smaller than 2 MB"
    );
    let text = text.trim_start_matches('\u{feff}');
    if format == "lyricsfile" {
        let doc: Document = serde_yaml::from_str(text).context("Invalid LyricsFile document")?;
        validate(&doc)?;
        return Ok(doc);
    }
    if format == "plain" {
        let doc = Document {
            plain: text.replace("\r\n", "\n"),
            ..Default::default()
        };
        validate(&doc)?;
        return Ok(doc);
    }
    let mut doc = Document::default();
    let mut plain = Vec::new();
    let mut timed = false;
    for raw in text.lines() {
        let mut rest = raw;
        let mut times = vec![];
        while rest.starts_with('[') {
            let Some(end) = rest.find(']') else { break };
            let token = &rest[1..end];
            if let Some(ms) = timestamp(token) {
                times.push(ms);
                timed = true;
            } else if let Some(offset) = token.strip_prefix("offset:") {
                doc.metadata.offset_ms = offset.trim().parse().context("Invalid LRC offset")?;
            } else if let Some(title) = token.strip_prefix("ti:") {
                doc.metadata.title = title.into();
            } else if let Some(artist) = token.strip_prefix("ar:") {
                doc.metadata.artist = artist.into();
            } else if let Some(album) = token.strip_prefix("al:") {
                doc.metadata.album = album.into();
            } else {
                break;
            }
            rest = &rest[end + 1..];
        }
        if !times.is_empty() {
            for ms in times {
                doc.lines.push(Line {
                    text: rest.into(),
                    start_ms: ms,
                    ..Default::default()
                });
            }
            plain.push(rest.to_owned());
        } else if rest == raw || !rest.is_empty() {
            plain.push(rest.into());
        }
    }
    if format == "lrc" && !timed && !text.trim().is_empty() {
        bail!("This LRC file has no valid line timestamps. Import as plain text instead.")
    }
    doc.lines.sort_by_key(|l| l.start_ms);
    doc.plain = plain.join("\n");
    validate(&doc)?;
    Ok(doc)
}
pub fn export(doc: &Document, format: &str) -> Result<String> {
    validate(doc)?;
    if format == "lyricsfile" {
        return Ok(serde_yaml::to_string(doc)?);
    }
    ensure!(format == "lrc", "Choose LyricsFile or LRC");
    ensure!(
        !doc.lines.is_empty(),
        "Plain lyrics have no timestamps. Export as LyricsFile."
    );
    let mut out = format!(
        "[ti:{}]\n[ar:{}]\n[al:{}]\n[offset:{}]\n",
        doc.metadata.title.replace(['\n', '\r'], " "),
        doc.metadata.artist.replace(['\n', '\r'], " "),
        doc.metadata.album.replace(['\n', '\r'], " "),
        doc.metadata.offset_ms
    );
    for line in &doc.lines {
        let t = line.start_ms;
        out += &format!(
            "[{:02}:{:02}.{:03}]{}\n",
            t / 60000,
            t / 1000 % 60,
            t % 1000,
            line.text
        );
    }
    Ok(out)
}
fn current(c: &Connection, id: i64) -> Result<(i64, Option<String>, Option<String>)> {
    Ok(c.query_row(
        "SELECT revision,document,source FROM selections WHERE track_id=?",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?
    .unwrap_or((0, None, None)))
}
fn response(
    c: &Connection,
    id: i64,
    document: Option<Document>,
    source: Option<Source>,
    state: &str,
) -> Result<Response> {
    Ok(Response {
        track_id: id,
        revision: current(c, id)?.0,
        document,
        source,
        state: state.into(),
        message: None,
        candidates: vec![],
        retry_after_seconds: None,
        provider_status: None,
    })
}
fn signature(track: &musicos_init::database::Track) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        track.title, track.artist, track.album, track.duration
    )
}
fn describe(mut doc: Document, track: &musicos_init::database::Track) -> Document {
    if doc.metadata.title.is_empty() {
        doc.metadata.title = track.title.clone();
    }
    if doc.metadata.artist.is_empty() {
        doc.metadata.artist = track.artist.clone();
    }
    if doc.metadata.album.is_empty() {
        doc.metadata.album = track.album.clone();
    }
    if doc.metadata.duration_ms.is_none() {
        doc.metadata.duration_ms = Some(i64::from(track.duration) * 1000);
    }
    doc
}
fn embedded(text: &str) -> Result<Document> {
    // Lyrics tags commonly contain LRC despite being named "unsynchronized" by the container.
    // Explicit plain-text imports remain literal; only this read-only tag adapter detects timing.
    let timed = text.lines().any(|line| {
        line.trim_start()
            .strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .is_some_and(|(stamp, _)| timestamp(stamp).is_some())
    });
    parse(text, if timed { "lrc" } else { "plain" })
}
fn local(c: &Connection, track: &musicos_init::database::Track) -> Result<Response> {
    let (_, doc, source) = current(c, track.id)?;
    if let Some(doc) = doc {
        return response(
            c,
            track.id,
            Some(serde_json::from_str(&doc)?),
            source.map(|s| serde_json::from_str(&s)).transpose()?,
            "ready",
        );
    }
    let audio = Path::new(&track.path);
    for ext in ["lyricsfile", "lrc"] {
        let p = audio.with_extension(ext);
        if p.is_file() {
            let meta = p.metadata()?;
            ensure!(meta.len() <= 2_000_000, "Local lyrics exceed 2 MB");
            let doc = describe(parse(&std::fs::read_to_string(&p)?, ext)?, track);
            return response(
                c,
                track.id,
                Some(doc),
                Some(Source {
                    kind: "sidecar".into(),
                    label: format!("Local .{ext}"),
                    url: None,
                    provider_id: None,
                }),
                "ready",
            );
        }
    }
    if audio.is_file() {
        if let Ok(tags) = lofty::read_from_path(audio) {
            if let Some(text) = tags
                .primary_tag()
                .and_then(|t| t.get_string(&ItemKey::Lyrics))
            {
                return response(
                    c,
                    track.id,
                    Some(describe(embedded(text)?, track)),
                    Some(Source {
                        kind: "embedded".into(),
                        label: "Embedded lyrics".into(),
                        url: None,
                        provider_id: None,
                    }),
                    "ready",
                );
            }
        }
    }
    response(c, track.id, None, None, "missing")
}
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn matches(track: &musicos_init::database::Track, c: &Candidate) -> bool {
    normalize(&track.title) == normalize(&c.title)
        && normalize(&track.artist) == normalize(&c.artist)
        && (track.album.is_empty() || normalize(&track.album) == normalize(&c.album))
        && (track.duration as f64 - c.duration).abs() <= 2.
}
fn provider_source(c: &Candidate) -> Source {
    Source {
        kind: "lrclib".into(),
        label: "LRCLIB".into(),
        url: Some(format!("https://lrclib.net/api/get/{}", c.id)),
        provider_id: Some(c.id),
    }
}
fn cached(
    c: &Connection,
    track: &musicos_init::database::Track,
    retry: bool,
) -> Result<Option<Vec<Candidate>>> {
    let item: Option<(i64, String)> = c
        .query_row(
            "SELECT checked,payload FROM cache WHERE signature=?",
            [signature(track)],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((at, data)) = item {
        let items: Vec<Candidate> = serde_json::from_str(&data)?;
        if !retry && (!items.is_empty() || now() - at < 86400) {
            return Ok(Some(items));
        }
    }
    Ok(None)
}
fn rank(
    c: &Connection,
    track: &musicos_init::database::Track,
    items: Vec<Candidate>,
) -> Result<Response> {
    let exact = items
        .iter()
        .filter(|v| matches(track, v))
        .collect::<Vec<_>>();
    let mut r = if exact.len() == 1 {
        response(
            c,
            track.id,
            Some(exact[0].document.clone()),
            Some(provider_source(exact[0])),
            "ready",
        )?
    } else {
        response(
            c,
            track.id,
            None,
            None,
            if items.is_empty() {
                "not_found"
            } else {
                "choose"
            },
        )?
    };
    r.candidates = items;
    Ok(r)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    id: i64,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    synced_lyrics: Option<String>,
    #[serde(default)]
    lyricsfile: Option<String>,
}
fn candidate(r: Record) -> Result<Candidate> {
    let mut d = r.lyricsfile.as_deref().filter(|s| !s.trim().is_empty())
        .and_then(|text| parse(text, "lyricsfile").ok())
        .filter(|d| d.metadata.instrumental || !d.lines.is_empty() || !d.plain.trim().is_empty())
        .or_else(|| r.synced_lyrics.as_deref().filter(|s| !s.trim().is_empty()).and_then(|text| parse(text,"lrc").ok()))
        .unwrap_or(parse(r.plain_lyrics.as_deref().unwrap_or(""),"plain")?);
    d.metadata.title = r.track_name.clone();
    d.metadata.artist = r.artist_name.clone();
    d.metadata.album = r.album_name.clone();
    d.metadata.duration_ms = Some((r.duration * 1000.) as i64);
    d.metadata.instrumental = r.instrumental;
    if r.instrumental {
        d.lines.clear();
        d.plain.clear();
    }
    validate(&d)?;
    Ok(Candidate {
        id: r.id,
        title: r.track_name,
        artist: r.artist_name,
        album: r.album_name,
        duration: r.duration,
        document: d,
    })
}
struct Gate { next: Instant, status: Option<u16> }
fn gate() -> &'static Mutex<Gate> {
    static GATE: OnceLock<Mutex<Gate>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(Gate { next: Instant::now(), status: None }))
}
fn retry_seconds(value:Option<&str>,status:u16,at:SystemTime)->u64 {
    value.and_then(|value|value.parse::<u64>().ok().or_else(||httpdate::parse_http_date(value).ok().map(|date|date.duration_since(at).unwrap_or_default().as_secs().saturating_add(1))))
        .unwrap_or(if status==503 {2}else{30}).clamp(1,86400)
}
fn resting(c:&Connection,track:&musicos_init::database::Track,gate:&Gate)->Result<Response>{
    let seconds=gate.next.saturating_duration_since(Instant::now()).as_secs()+1;
    let mut r=response(c,track.id,None,None,"retry_later")?;
    r.retry_after_seconds=Some(seconds);r.provider_status=gate.status;
    r.message=Some(format!("LRCLIB is temporarily busy. Retry will be available in {seconds} seconds."));Ok(r)
}
fn lookup(c:&Connection,track:&musicos_init::database::Track,retry:bool,search:bool)->Result<Response>{
    lookup_using(c,track,retry,search,"https://lrclib.net",gate())
}
fn lookup_using(c:&Connection,track:&musicos_init::database::Track,retry:bool,search:bool,base:&str,lock:&Mutex<Gate>)->Result<Response>{
    lookup_impl(c,track,retry,search,false,base,lock)
}
fn lookup_impl(c:&Connection,track:&musicos_init::database::Track,retry:bool,search:bool,exact_search:bool,base:&str,lock:&Mutex<Gate>)->Result<Response>{
    if !search {let local=local(c,track)?;if local.document.is_some(){return Ok(local);}if let Some(items)=cached(c,track,retry)?{return rank(c,track,items);}}
    let mut gate=lock.lock().unwrap_or_else(|e|e.into_inner());
    if !search {if let Some(items)=cached(c,track,retry)?{return rank(c,track,items);}}
    let client=reqwest::blocking::Client::builder().user_agent(crate::product::provider_user_agent()).timeout(Duration::from_secs(12)).build()?;
    let mut queries=vec![];
    if !search || exact_search {
        let mut signature=vec![("track_name",track.title.clone()),("artist_name",track.artist.clone()),("album_name",track.album.clone())];
        if track.duration>0&&track.duration<=3600 {signature.push(("duration",track.duration.to_string()));}
        queries.push(("get",signature));
    }
    // Album labels and contributor strings vary between editions. Broader results
    // remain choices; only the complete original signature can auto-match.
    queries.push(("search",vec![("track_name",track.title.clone()),("artist_name",track.artist.clone())]));
    queries.push(("search",vec![("track_name",track.title.clone())]));
    for (endpoint,query) in queries {
        if gate.next>Instant::now(){let delay=gate.next.saturating_duration_since(Instant::now());if delay<=Duration::from_millis(500){std::thread::sleep(delay);}else{return resting(c,track,&gate);}}
        let result=client.get(format!("{base}/api/{endpoint}")).header(reqwest::header::ACCEPT,"application/json").query(&query).send();
        gate.next=Instant::now()+Duration::from_millis(300);gate.status=None;
        let http=match result{Ok(http)=>http,Err(_)=>{let mut r=response(c,track.id,None,None,"offline")?;r.message=Some("Couldn’t reach LRCLIB. Check your connection, then Retry. Local lyrics remain available.".into());return Ok(r);}};
        let status=http.status().as_u16();
        if status==429||status==503 {
            let seconds=retry_seconds(http.headers().get("retry-after").and_then(|v|v.to_str().ok()),status,SystemTime::now());
            let jitter=rand::random::<u64>()%(seconds*250+1);
            gate.next=Instant::now()+Duration::from_millis(seconds*1000+jitter);gate.status=Some(status);
            return resting(c,track,&gate);
        }
        if status==404 {continue;}
        if !http.status().is_success(){let mut r=response(c,track.id,None,None,"provider_error")?;r.provider_status=Some(status);r.message=Some(format!("LRCLIB returned HTTP {status}. Retry or add a local lyric file."));return Ok(r);}
        ensure!(http.content_length().is_none_or(|n|n<=4_000_000),"Lyrics response is too large");
        use std::io::Read;let mut bytes=Vec::new();http.take(4_000_001).read_to_end(&mut bytes)?;ensure!(bytes.len()<=4_000_000,"Lyrics response is too large");
        let value:serde_json::Value=match serde_json::from_slice(&bytes){Ok(value)=>value,Err(_)=>{let mut r=response(c,track.id,None,None,"provider_error")?;r.provider_status=Some(status);r.message=Some("LRCLIB returned an unreadable response. Retry or import lyrics.".into());return Ok(r);}};
        let records=if endpoint=="get"{vec![value]}else{value.as_array().cloned().context("LRCLIB returned an invalid search response")?};
        let had_records=!records.is_empty();
        let items:Vec<Candidate>=records.into_iter().take(20).filter_map(|value|serde_json::from_value::<Record>(value).ok().and_then(|record|candidate(record).ok())).filter(|c|c.document.metadata.instrumental||!c.document.lines.is_empty()||!c.document.plain.trim().is_empty()).collect();
        if items.is_empty(){if had_records {let mut r=response(c,track.id,None,None,"provider_error")?;r.message=Some("These LRCLIB versions contain no readable lyrics. Try another version or import a lyric file.".into());return Ok(r);}continue;}
        c.execute("INSERT INTO cache VALUES(?,?,?) ON CONFLICT(signature) DO UPDATE SET checked=excluded.checked,payload=excluded.payload",params![signature(track),now(),serde_json::to_string(&items)?])?;
        if search {let mut r=response(c,track.id,None,None,"choose")?;r.candidates=items;return Ok(r);}
        return rank(c,track,items);
    }
    c.execute("INSERT INTO cache VALUES(?,?,?) ON CONFLICT(signature) DO UPDATE SET checked=excluded.checked,payload=excluded.payload",params![signature(track),now(),"[]"])?;
    rank(c,track,vec![])
}

fn save(
    c: &mut Connection,
    id: i64,
    revision: i64,
    doc: Document,
    source: Source,
) -> Result<Response> {
    validate(&doc)?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (old, old_doc, old_source) = current(&tx, id)?;
    ensure!(
        old == revision,
        "Lyrics changed. Reload before saving; your draft is retained."
    );
    let d = serde_json::to_string(&doc)?;
    let s = serde_json::to_string(&source)?;
    tx.execute("INSERT INTO history(track_id,at,before_doc,before_source,after_doc,after_source,revision) VALUES(?,?,?,?,?,?,?)",params![id,now(),old_doc,old_source,d,s,old+1])?;
    tx.execute("INSERT INTO selections VALUES(?,?,?,?) ON CONFLICT(track_id) DO UPDATE SET revision=excluded.revision,document=excluded.document,source=excluded.source",params![id,old+1,d,s])?;
    tx.commit()?;
    response(c, id, Some(doc), Some(source), "ready")
}
pub fn execute(db: &Path, req: Request) -> Result<serde_json::Value> {
    let mut c = store(db)?;
    if req.action == "history" {
        let mut q = c.prepare(
            "SELECT id,track_id,at,undone,revision FROM history ORDER BY id DESC LIMIT 50",
        )?;
        let rows=q.query_map([],|r|Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"track_id":r.get::<_,i64>(1)?,"at":r.get::<_,i64>(2)?,"undone":r.get::<_,i64>(3)?!=0,"revision":r.get::<_,i64>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        return Ok(serde_json::json!({"history":rows}));
    }
    if req.action == "comparison" {
        let (before, after): (Option<String>, Option<String>) = c.query_row(
            "SELECT before_doc,after_doc FROM history WHERE id=?",
            [req.history_id.context("Choose a lyric change")?],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        return Ok(
            serde_json::json!({"before":before.map(|s|serde_json::from_str::<Document>(&s)).transpose()?,"after":after.map(|s|serde_json::from_str::<Document>(&s)).transpose()?}),
        );
    }
    if req.action == "undo" {
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (id,doc,source,after,after_source,undone):(i64,Option<String>,Option<String>,Option<String>,Option<String>,bool)=tx.query_row("SELECT track_id,before_doc,before_source,after_doc,after_source,undone FROM history WHERE id=?",[req.history_id.context("Choose a lyric change")?],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
        let (_, current_doc, current_source) = current(&tx, id)?;
        ensure!(
            !undone && current_doc == after && current_source == after_source,
            "Later lyric edits exist. Compare them before Undo."
        );
        tx.execute(
            "UPDATE selections SET revision=revision+1,document=?,source=? WHERE track_id=?",
            params![doc, source, id],
        )?;
        tx.execute("UPDATE history SET undone=1 WHERE id=?", [req.history_id])?;
        tx.commit()?;
        return Ok(serde_json::json!({"undone":true}));
    }
    let id = req.track_id.context("Choose a song")?;
    let database = musicos_init::database::Db::open(&db.to_string_lossy())?;
    let track = database
        .get_track_by_id(id)?
        .context("This song is no longer in the library")?;
    let value = match req.action.as_str() {
        "load" => {
            let r = local(&c, &track)?;
            if r.document.is_some() {
                r
            } else if let Some(items) = cached(&c, &track, false)? {
                rank(&c, &track, items)?
            } else {
                r
            }
        }
        "find" => lookup_impl(&c,&track,req.retry.unwrap_or(false),true,true,"https://lrclib.net",gate())?,
        "lookup" | "search" => lookup(
            &c,
            &track,
            req.retry.unwrap_or(false),
            req.action == "search",
        )?,
        "parse" | "import" => {
            let text = if req.action == "import" {
                let p = req.path.context("Choose a lyric file")?;
                ensure!(
                    p.is_file() && p.metadata()?.len() <= 2_000_000,
                    "Choose a lyric file smaller than 2 MB"
                );
                std::fs::read_to_string(p)?
            } else {
                req.text.context("Paste lyrics")?
            };
            let d = describe(
                parse(&text, req.format.as_deref().unwrap_or("plain"))?,
                &track,
            );
            response(&c, id, Some(d), None, "draft")?
        }
        "select" => {
            let items =
                cached(&c, &track, false)?.context("Search again before selecting these lyrics")?;
            let candidate = items
                .into_iter()
                .find(|v| Some(v.id) == req.candidate_id)
                .context("Search again before selecting these lyrics")?;
            save(
                &mut c,
                id,
                req.revision.context("Reload lyric revision")?,
                candidate.document.clone(),
                provider_source(&candidate),
            )?
        }
        "write_tags" => {
            let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let (revision, document, _) = current(&tx, id)?;
            ensure!(
                Some(revision) == req.revision,
                "Lyrics changed. Review them again before writing files."
            );
            let doc: Document =
                serde_json::from_str(&document.context("Save lyrics in musicOS first")?)?;
            let plain = if doc.plain.is_empty() {
                doc.lines
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                doc.plain.clone()
            };
            crate::file_tags::write_plain_lyrics(db, id, &plain)?;
            tx.commit()?;
            response(&c, id, Some(doc), None, "tags_written")?
        }
        "save" => {
            ensure!(
                !req.write_files.unwrap_or(false),
                "Review file tagging separately"
            );
            let provider = req.candidate_id;
            if let Some(candidate_id) = provider {
                let known = cached(&c, &track, false)?
                    .unwrap_or_default()
                    .iter()
                    .any(|v| v.id == candidate_id)
                    || current(&c, id)?
                        .2
                        .and_then(|s| serde_json::from_str::<Source>(&s).ok())
                        .is_some_and(|s| s.provider_id == provider);
                ensure!(
                    known,
                    "Reload this lyric source before saving; your draft is retained."
                );
            }
            let source = Source {
                kind: "edited".into(),
                label: if provider.is_some() {
                    "Edited from LRCLIB"
                } else {
                    "Your lyrics"
                }
                .into(),
                url: provider.map(|id| format!("https://lrclib.net/api/get/{id}")),
                provider_id: provider,
            };
            save(
                &mut c,
                id,
                req.revision.context("Reload lyric revision")?,
                req.document.context("Add lyrics first")?,
                source,
            )?
        }
        "export" => {
            let doc = req.document.context("Choose lyrics to export")?;
            let text = export(&doc, req.format.as_deref().unwrap_or("lyricsfile"))?;
            let p = req.path.context("Choose an export destination")?;
            ensure!(
                p.parent().is_some_and(Path::is_dir),
                "Choose an existing folder"
            );
            ensure!(!p.is_symlink(), "Choose a file instead of a symbolic link");
            if p.exists() {
                ensure!(
                    req.overwrite.unwrap_or(false),
                    "This file exists. Confirm replacement first."
                );
                let backup = db.with_extension("lyric-exports").join(format!(
                    "{}-{}.backup",
                    now(),
                    rand::random::<u64>()
                ));
                std::fs::create_dir_all(backup.parent().unwrap())?;
                std::fs::copy(&p, &backup)?;
                c.execute(
                    "INSERT INTO exports(path,backup,at) VALUES(?,?,?)",
                    params![p.to_string_lossy(), backup.to_string_lossy(), now()],
                )?;
            }
            let staged = p.with_file_name(format!(".musicos-lyrics-{}.tmp", rand::random::<u64>()));
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&staged)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
            if let Err(e) = std::fs::rename(&staged, &p) {
                let _ = std::fs::remove_file(staged);
                return Err(e.into());
            }
            response(&c, id, Some(doc), None, "exported")?
        }
        _ => bail!("Unknown lyric action"),
    };
    Ok(serde_json::to_value(value)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_offsets_repeated_times_unicode_and_roundtrip() {
        let d = parse(
            "[ar:Artist]\n[offset:-100]\n[00:01.20][00:02.300]こんにちは\n[00:03.000]\n",
            "lrc",
        )
        .unwrap();
        assert_eq!(d.lines.len(), 3);
        assert_eq!(d.metadata.offset_ms, -100);
        assert_eq!(
            parse(&export(&d, "lyricsfile").unwrap(), "lyricsfile").unwrap(),
            d
        );
        assert_eq!(
            parse(&export(&d, "lrc").unwrap(), "lrc").unwrap().lines,
            d.lines
        );
        assert!(parse("[90:no]bad", "lrc").is_err());
    }
    #[test]
    fn word_times_and_instrumental_validate() {
        let mut d = Document::default();
        d.lines.push(Line {
            text: "One two".into(),
            start_ms: 100,
            words: vec![
                Word {
                    text: "One ".into(),
                    start_ms: 100,
                    end_ms: Some(200),
                },
                Word {
                    text: "two".into(),
                    start_ms: 200,
                    end_ms: None,
                },
            ],
            end_ms: None,
        });
        validate(&d).unwrap();
        assert_eq!(
            parse(&export(&d, "lyricsfile").unwrap(), "lyricsfile").unwrap(),
            d
        );
        d.metadata.instrumental = true;
        assert!(validate(&d).is_err());
    }
    #[test]
    fn revision_and_undo_preserve_song_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("library.db");
        let database = musicos_init::database::Db::open(&db.to_string_lossy()).unwrap();
        let id = database
            .insert_track("/no.flac", "Song", "Artist", "Album", 1, 100, 1)
            .unwrap();
        let before = serde_json::to_value(database.get_track_by_id(id).unwrap()).unwrap();
        let request = |rev| Request {
            action: "save".into(),
            track_id: Some(id),
            revision: Some(rev),
            document: Some(parse("My words", "plain").unwrap()),
            ..Default::default()
        };
        assert!(execute(&db, request(0)).is_ok());
        assert!(execute(&db, request(0)).is_err());
        execute(
            &db,
            Request {
                action: "undo".into(),
                history_id: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            execute(
                &db,
                Request {
                    action: "load".into(),
                    track_id: Some(id),
                    ..Default::default()
                }
            )
            .unwrap()["state"],
            "missing"
        );
        assert_eq!(
            before,
            serde_json::to_value(database.get_track_by_id(id).unwrap()).unwrap()
        );
    }
}
#[cfg(test)]
mod provider_tests {
    use super::*;
    #[test]
    fn cached_misses_expire_and_edits_take_precedence() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("library.db");
        let db = musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();
        let id = db
            .insert_track("/no.flac", "Title", "Artist", "Album", 180, 1, 1)
            .unwrap();
        let track = db.get_track_by_id(id).unwrap().unwrap();
        let c = store(&path).unwrap();
        c.execute(
            "INSERT INTO cache VALUES(?,?,?)",
            params![signature(&track), now(), "[]"],
        )
        .unwrap();
        assert!(cached(&c, &track, false).unwrap().is_some());
        assert!(cached(&c, &track, true).unwrap().is_none());
        c.execute("UPDATE cache SET checked=?", [now() - 86401])
            .unwrap();
        assert!(cached(&c, &track, false).unwrap().is_none());
        execute(
            &path,
            Request {
                action: "save".into(),
                track_id: Some(id),
                revision: Some(0),
                document: Some(parse("My words", "plain").unwrap()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            local(&c, &track).unwrap().document.unwrap().plain,
            "My words"
        );
    }
    #[test]
    fn editions_and_duration_must_match_and_ambiguous_records_require_selection() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("library.db");
        let db = musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();
        let id = db
            .insert_track("/no.flac", "Title", "Artist", "Album", 180, 1, 1)
            .unwrap();
        let track = db.get_track_by_id(id).unwrap().unwrap();
        let mut item = Candidate {
            id: 1,
            title: "Title".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration: 180.,
            document: Document::default(),
        };
        assert!(matches(&track, &item));
        item.album = "Album (Live)".into();
        assert!(!matches(&track, &item));
        item.album = "Album".into();
        item.duration = 183.;
        assert!(!matches(&track, &item));
        item.duration = 180.;
        let c = store(&path).unwrap();
        assert_eq!(
            rank(&c, &track, vec![item.clone(), Candidate { id: 2, ..item }])
                .unwrap()
                .state,
            "choose"
        );
    }
    #[test]
    fn local_lyricsfile_precedes_lrc_and_plain_keeps_paragraphs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("library.db");
        let song = tmp.path().join("song.flac");
        let db = musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();
        let id = db
            .insert_track(
                &song.to_string_lossy(),
                "Title",
                "Artist",
                "Album",
                180,
                1,
                1,
            )
            .unwrap();
        std::fs::write(song.with_extension("lrc"), "[00:01]Local LRC").unwrap();
        let mut doc = Document::default();
        doc.plain = "First paragraph\n\nSecond paragraph".into();
        std::fs::write(
            song.with_extension("lyricsfile"),
            export(&doc, "lyricsfile").unwrap(),
        )
        .unwrap();
        let r = execute(
            &path,
            Request {
                action: "load".into(),
                track_id: Some(id),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r["document"]["plain"], doc.plain);
        assert_eq!(
            parse("[Verse 1]\n\nText", "plain").unwrap().plain,
            "[Verse 1]\n\nText"
        );
    }
}
#[cfg(test)]
mod live_test {
    use super::*;
    #[test]
    #[ignore = "Explicit LRCLIB network contract check"]
    fn lyric_live_contract() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("library.db");
        let db = musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();
        let id = db
            .insert_track(
                "/nonexistent.flac",
                "Midnight City",
                "M83",
                "Hurry Up, We're Dreaming",
                244,
                1,
                1,
            )
            .unwrap();
        let r = execute(
            &path,
            Request {
                action: "lookup".into(),
                track_id: Some(id),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            r["document"].is_object() || r["candidates"].as_array().is_some_and(|r| !r.is_empty()),
            "LRCLIB did not return a usable record: {}",
            r["state"]
        );
    }
}
#[cfg(test)]
mod embedded_tests {
    use super::*;
    #[test]
    fn embedded_timestamps_are_live_but_explicit_plain_import_stays_literal() {
        let text = "[00:00.00]First line\n[00:05.50]\n[00:05.50]Second line";
        let doc = embedded(text).unwrap();
        assert_eq!(doc.lines.len(), 3);
        assert_eq!(doc.lines[2].start_ms, 5500);
        assert_eq!(doc.lines[2].text, "Second line");
        assert!(!doc.plain.contains("[00:"));
        assert_eq!(parse(text, "plain").unwrap().plain, text);
        assert_eq!(
            embedded("[Verse 1]\nWords without timing")
                .unwrap()
                .lines
                .len(),
            0
        );
    }
    #[test]
    fn container_lyrics_retain_local_precedence_and_export_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let audio = tmp.path().join("song.flac");
        assert!(std::process::Command::new("/opt/homebrew/bin/ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.2",
                "-c:a",
                "flac",
                "-metadata",
                "lyrics=[00:00.10]My test words"
            ])
            .arg(&audio)
            .status()
            .unwrap()
            .success());
        let path = tmp.path().join("library.db");
        let database = musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();
        let id = database
            .insert_track(
                &audio.to_string_lossy(),
                "My song",
                "My artist",
                "My album",
                20,
                1,
                1,
            )
            .unwrap();
        let r = execute(
            &path,
            Request {
                action: "load".into(),
                track_id: Some(id),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r["source"]["kind"], "embedded");
        assert_eq!(r["document"]["lines"][0]["start_ms"], 100);
        assert_eq!(r["document"]["metadata"]["title"], "My song");
    }
}

#[cfg(test)]
mod transport_contract {
    use super::*;
    #[test]
    #[ignore = "Explicit native LRCLIB transport check"]
    fn helmet_native_transport() {
        let client=reqwest::blocking::Client::builder().user_agent(crate::product::provider_user_agent()).timeout(Duration::from_secs(15)).build().unwrap();
        let response=client.get("https://lrclib.net/api/get").query(&[("track_name","Helmet"),("artist_name","Steve Lacy"),("album_name","Gemini Rights"),("duration","201")]).send().unwrap();
        let status=response.status();
        println!("native transport: status={status} version={:?} retry={:?} type={:?}",response.version(),response.headers().get("retry-after"),response.headers().get("content-type"));
        if !status.is_success(){let body=response.text().unwrap_or_default();panic!("LRCLIB {status}: {}",body.chars().take(300).collect::<String>());}
        let record:Record=response.json().unwrap();let c=candidate(record).unwrap();assert!(!c.document.lines.is_empty());assert!(c.title.eq_ignore_ascii_case("Helmet"));
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use std::{io::{Read,Write},net::TcpListener};
    #[test]
    fn overload_recovers_then_broadens_without_losing_edition_identity_or_poisoning_cache(){
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let base=format!("http://{}",listener.local_addr().unwrap());
        let worker=std::thread::spawn(move||{
            for (expected,status,extra,body) in [
                ("/api/get?","503 Service Unavailable","Retry-After: 1\r\n","busy".to_string()),
                ("/api/get?","404 Not Found","","{}".to_string()),
                ("/api/search?","200 OK","",serde_json::json!([{"id":42,"trackName":"Title","artistName":"Artist","albumName":"Album","duration":180.,"lyricsfile":"invalid document","syncedLyrics":"[00:01.00]My original test words"}]).to_string())
            ] {let(mut stream,_)=listener.accept().unwrap();let mut buffer=[0;8192];let count=stream.read(&mut buffer).unwrap();let request=String::from_utf8_lossy(&buffer[..count]);assert!(request.starts_with(&format!("GET {expected}")));assert!(request.to_lowercase().contains("user-agent: musicos"));if expected.contains("search"){assert!(!request.contains("album_name="));}write!(stream,"HTTP/1.1 {status}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();}
        });
        let tmp=tempfile::tempdir().unwrap();let path=tmp.path().join("library.db");let db=musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();let id=db.insert_track("/absent.flac","Title","Artist","Album",180,1,1).unwrap();let track=db.get_track_by_id(id).unwrap().unwrap();let c=store(&path).unwrap();let gate=Mutex::new(Gate{next:Instant::now(),status:None});
        let busy=lookup_using(&c,&track,false,false,&base,&gate).unwrap();assert_eq!(busy.state,"retry_later");assert_eq!(busy.provider_status,Some(503));assert!(busy.retry_after_seconds.unwrap()>=1);assert!(cached(&c,&track,false).unwrap().is_none());
        let still_busy=lookup_using(&c,&track,true,false,&base,&gate).unwrap();assert_eq!(still_busy.state,"retry_later");gate.lock().unwrap().next=Instant::now();
        let found=lookup_using(&c,&track,true,false,&base,&gate).unwrap();assert_eq!(found.state,"ready");assert_eq!(found.document.unwrap().lines[0].text,"My original test words");worker.join().unwrap();
        // The server is gone: reopening must use the successful cache.
        assert_eq!(lookup_using(&c,&track,false,false,&base,&gate).unwrap().state,"ready");
        assert_eq!(retry_seconds(Some("8"),429,SystemTime::now()),8);assert_eq!(retry_seconds(None,503,SystemTime::now()),2);
    }
    #[test]
    #[ignore="Explicit full native LRCLIB lookup and cache check"]
    fn native_lookup_and_offline_cache(){
        let tmp=tempfile::tempdir().unwrap();let path=tmp.path().join("library.db");let db=musicos_init::database::Db::open(&path.to_string_lossy()).unwrap();let id=db.insert_track("/absent.flac","Helmet","Steve Lacy","Gemini Rights",201,1,1).unwrap();let track=db.get_track_by_id(id).unwrap().unwrap();let c=store(&path).unwrap();let gate=Mutex::new(Gate{next:Instant::now(),status:None});
        let mut found=lookup_using(&c,&track,false,false,"https://lrclib.net",&gate).unwrap();for _ in 0..2{if found.state!="retry_later"{break;}std::thread::sleep(gate.lock().unwrap().next.saturating_duration_since(Instant::now()).min(Duration::from_secs(15)));found=lookup_using(&c,&track,true,false,"https://lrclib.net",&gate).unwrap();}
        println!("Full lookup: state={}, timed={}, candidates={}",found.state,found.document.as_ref().is_some_and(|d|!d.lines.is_empty()),found.candidates.len());assert_eq!(found.state,"ready");assert!(!found.document.unwrap().lines.is_empty());let choices=lookup_impl(&c,&track,true,true,true,"https://lrclib.net",&gate).unwrap();println!("Editor find: state={}, candidates={}",choices.state,choices.candidates.len());assert_eq!(choices.state,"choose");assert!(choices.document.is_none());assert!(!choices.candidates.is_empty());assert_eq!(lookup_using(&c,&track,false,false,"http://127.0.0.1:1",&gate).unwrap().state,"ready");
    }
}
