//! Opt-in, pinned-TLS offline transfer. This is separate from the embedded OS API.
mod media;
pub use media::CopyFormat;
use anyhow::{bail, ensure, Context, Result};
use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::Read,
    net::{IpAddr, SocketAddr, TcpListener},
    path::{Path as FsPath, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn random() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex(&bytes)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn equal(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |v, (x, y)| v | (x ^ y)) == 0
}
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
fn read<T>(m: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    m.read().unwrap_or_else(|e| e.into_inner())
}
fn write<T>(m: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    m.write().unwrap_or_else(|e| e.into_inner())
}
fn private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}
fn home() -> PathBuf {
    crate::preferences::path().with_file_name("wifi-sync-v1")
}
fn save<T: Serialize>(path: &FsPath, value: &T) -> Result<()> {
    let folder = path.parent().context("Missing sync directory")?;
    fs::create_dir_all(folder)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(folder, fs::Permissions::from_mode(0o700))?;
    }
    let temp = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&temp)?;
    use std::io::Write;
    f.write_all(&serde_json::to_vec(value)?)?;
    f.sync_all()?;
    fs::rename(temp, path)?;
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
struct Identity {
    id: String,
    certificate: String,
    key: String,
    fingerprint: String,
}
impl Identity {
    fn load(folder: &FsPath) -> Result<Self> {
        let path = folder.join("identity.json");
        if path.exists() {
            return Ok(serde_json::from_slice(&fs::read(path)?)?);
        }
        let key =
            rcgen::generate_simple_self_signed(vec!["musicos.local".into(), "localhost".into()])?;
        let der = key.cert.der().to_vec();
        let identity = Self {
            id: random(),
            certificate: STANDARD.encode(&der),
            key: STANDARD.encode(key.signing_key.serialize_der()),
            fingerprint: digest(&der),
        };
        save(&path, &identity)?;
        Ok(identity)
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Peer {
    #[serde(default)] sync_changes:bool,
    #[serde(default)] listening_allowed: bool,
    id: String,
    name: String,
    token_hash: String,
    approved_at: u64,
    last_seen_at: u64,
    last_sync_at: Option<u64>,
    last_revision: Option<String>,
    track_count: usize,
}
#[derive(Clone)]
struct Pending {
    sync_changes:bool,
    id: String,
    name: String,
    token_hash: String,
    created_at: u64,
}
struct Ticket {
    secret: String,
    expires_at: u64,
}
#[derive(Clone, Serialize, Deserialize, Default)]
struct Trust {
    peers: Vec<Peer>,
}
#[derive(Clone, Serialize)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_id: String,
    pub genre: String,
    pub year: i32,
    pub track_number: i32,
    pub disc_number: i32,
    pub duration_ms: i64,
    pub size: u64,
    pub sha256: String,
    pub mime: String,
    pub favorited: bool,
    pub artwork_sha256: Option<String>,
    pub lyrics_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
}
fn source_isrc(path: &FsPath) -> Option<String> {
    use lofty::{file::TaggedFileExt,probe::Probe,config::ParseOptions,tag::ItemKey};
    let tagged = Probe::open(path).ok()?.guess_file_type().ok()?
        .options(ParseOptions::new().read_properties(false).read_cover_art(false)).read().ok()?;
    let value = tagged.tags().iter().find_map(|tag|tag.get_string(&ItemKey::Isrc))?;
    let normalized = value.trim().replace('-', "").to_ascii_uppercase();
    let bytes = normalized.as_bytes();
    (bytes.len()==12 && bytes[..2].iter().all(u8::is_ascii_uppercase)
        && bytes[2..5].iter().all(u8::is_ascii_alphanumeric)
        && bytes[5..].iter().all(u8::is_ascii_digit)).then_some(normalized)
}
#[derive(Clone, Serialize)]
pub struct Collection {
    pub revision:String,
    pub complete:bool,
    pub full_track_ids:Vec<i64>,
    pub artwork_sha256:Option<String>,
    pub artwork_choice:Option<String>,
    pub id: String,
    pub name: String,
    pub kind: String,
    pub track_ids: Vec<i64>,
}
#[derive(Clone, Serialize)]
pub struct Manifest {
    pub capabilities:Vec<String>,
    pub portraits:BTreeMap<String,String>,
    pub version: u8,
    pub server_id: String,
    pub server_name: String,
    pub display_name: String,
    pub revision: String,
    pub tracks: Vec<Track>,
    pub collections: Vec<Collection>,
    pub bytes_total: u64,
    pub copy_format: CopyFormat,
}
#[derive(Clone,Serialize,Deserialize)]
struct File {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    sha256: String,
    mime: String,
}
#[derive(Default)]
pub struct Share {
    pub tracks: Vec<Track>,
    pub collections: Vec<Collection>,
    files: BTreeMap<String, File>,
    art: BTreeMap<String, File>,
    pub skipped: usize,
    portraits:BTreeMap<String,String>,
    pub display_name: String,
    pub format: CopyFormat,
}
fn file(path: &FsPath, mime: String) -> Result<File> {
    let path = fs::canonicalize(path)?;
    let metadata = fs::metadata(&path)?;
    ensure!(metadata.is_file(), "Only regular music files can be shared");
    let mut input = fs::File::open(&path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    ensure!(
        fs::metadata(&path)?.modified()? == metadata.modified()?,
        "A music file changed while preparing sync. Try again."
    );
    Ok(File {
        path,
        size: metadata.len(),
        modified: metadata.modified()?,
        sha256: hex(&hash.finalize()),
        mime,
    })
}
fn mime(path: &FsPath) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "flac" => Some("audio/flac"),
        "mp3" => Some("audio/mpeg"),
        "m4a" | "mp4" => Some("audio/mp4"),
        "aac" => Some("audio/aac"),
        "ogg" | "oga" | "opus" => Some("audio/ogg"),
        "wav" => Some("audio/wav"),
        "aiff" | "aif" => Some("audio/aiff"),
        _ => None,
    }
}
pub fn prepare(
    library: &crate::library::Library,
    collections: Vec<crate::collections::Collection>, ids:&[i64], display_name:String,
) -> Result<Share> { prepare_with(library,collections,ids,display_name,CopyFormat::Original,&home(),&||true,&|_,_,_|{}) }
pub fn prepare_with(
    library:&crate::library::Library, collections:Vec<crate::collections::Collection>, ids:&[i64], display_name:String,
    format:CopyFormat, folder:&FsPath, current:&dyn Fn()->bool, progress:&dyn Fn(usize,usize,&str),
) -> Result<Share> {
    ensure!(
        !ids.is_empty() && ids.len() <= 50000,
        "Choose between 1 and 50,000 songs to share"
    );
    let selected: HashSet<_> = ids.iter().copied().collect();
    let mut share = Share {
        display_name, format,
        ..Default::default()
    };
    let mut art_cache = HashMap::<PathBuf, Option<String>>::new();
    let mut cache=media::Cache::at(folder.to_path_buf());
    let albums:HashMap<_,_>=library.albums.iter().flat_map(|a|a.track_ids.iter().map(move |id|(*id,a.id.clone()))).collect();
    for (index,track) in library.tracks.iter().filter(|t| selected.contains(&t.id)).enumerate() {
        ensure!(current(),"Sharing preparation was cancelled");progress(index,selected.len(),&track.title);
        let path = FsPath::new(&track.path);
        if !track.available || !path.is_file() {
            share.skipped += 1;
            continue;
        }
        let Some(mime) = mime(path) else {
            share.skipped += 1;
            continue;
        };
        let media = cache.audio(path,mime,format,current)?;
        let artwork_sha256 = track.cover_art_path.as_ref().and_then(|path| {
            if let Some(value) = art_cache.get(path) {
                return value.clone();
            }
            let result = (|| -> Result<String> {
                let file=cache.artwork(path,current)?;let sha=file.sha256.clone();
                share.art.insert(sha.clone(),file);Ok(sha)
            })();
            let value = result.ok();
            art_cache.insert(path.clone(), value.clone());
            value
        });
        // Vorbis copies intentionally omit file tags. Carry the recording code
        // from the original so the iPhone can identify the same recording.
        let isrc = source_isrc(path);
        share.tracks.push(Track {
            id: track.id,
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            album_id: albums.get(&track.id).cloned().unwrap_or_else(||format!("album/{}",digest(track.album.as_bytes()))),
            genre: track.genre.clone(),
            year: track.year,
            track_number: track.track_number,
            disc_number: track.disc_number,
            duration_ms: i64::from(track.duration) * 1000,
            size: media.size,
            sha256: media.sha256.clone(),
            mime: media.mime.clone(),
            favorited: track.favorited,
            artwork_sha256,lyrics_sha256:None,isrc,
        });
        share.files.insert(track.id.to_string(), media);
        if format!=CopyFormat::Original||index%25==24 { cache.persist()?; }
        progress(index+1,selected.len(),&track.title);
    }
    cache.persist()?;
    ensure!(
        !share.tracks.is_empty(),
        "No available audio files in this selection"
    );
    let available: HashSet<_> = share.tracks.iter().map(|t| t.id).collect();
    for artist in &library.artists{if let Some(path)=&artist.portrait{if let Ok(asset)=cache.artwork(FsPath::new(path),current){let hash=asset.sha256.clone();share.art.insert(hash.clone(),asset);share.portraits.insert(artist.name.clone(),hash);}}}
    share.collections = collections
        .into_iter()
        .filter_map(|c| {
            let full_track_ids=c.track_ids.clone();
            let track_ids: Vec<_> = c
                .track_ids
                .into_iter()
                .filter(|id| available.contains(id))
                .collect();
            (!track_ids.is_empty()).then_some(Collection {
                artwork_choice:c.artwork_choice.clone(),revision:crate::companion_sync::collection_revision(&c.name,&full_track_ids),complete:track_ids.len()==full_track_ids.len(),full_track_ids,artwork_sha256:c.artwork.as_ref().and_then(|path|cache.artwork(FsPath::new(path),current).ok()).map(|asset|{let hash=asset.sha256.clone();share.art.insert(hash.clone(),asset);hash}),
                id: c.id,
                name: c.name,
                kind: c.kind,
                track_ids,
            })
        })
        .collect();
    Ok(share)
}
pub fn enrich_assets(share:&mut Share,db:&FsPath,folder:&FsPath)->Result<()> {
 let assets=folder.join("companion-assets");fs::create_dir_all(&assets)?;
 for track in &mut share.tracks {
  let response=crate::lyrics::execute(db,crate::lyrics::Request{action:"load".into(),track_id:Some(track.id),..Default::default()});
  if let Ok(response)=response {if !response["document"].is_null(){let data=serde_json::to_vec(&response["document"])?;let hash=digest(&data);let path=assets.join(&hash);if !path.exists(){fs::write(&path,&data)?;}let asset=file(&path,"application/json".into())?;track.lyrics_sha256=Some(hash.clone());share.art.insert(hash,asset);}}
 }
 Ok(())
}
pub struct Service {
    hub: Mutex<Option<Arc<Hub>>>,
    generation: AtomicU64,
    operation: tokio::sync::Mutex<()>,
    folder: PathBuf,
    preparation: Arc<Mutex<Preparation>>,
    cache_operation:Mutex<()>,
    database:Mutex<Option<PathBuf>>,
    changed:Mutex<Option<Arc<dyn Fn(Value)+Send+Sync>>>,
    integrations: Mutex<Option<crate::integrations::IntegrationService>>,
}
#[derive(Default,Clone,Serialize)]
struct Preparation { preparing:bool, completed:usize, total:usize, current:String, error:Option<String>, format:CopyFormat }
impl Default for Service {
    fn default() -> Self {
        Self::at(home())
    }
}

struct Hub {
    database:Option<PathBuf>,
    changed:Option<Arc<dyn Fn(Value)+Send+Sync>>,
    integrations: Option<crate::integrations::IntegrationService>,
    listening_scope: String,
    identity: Identity,
    name: String,
    folder: PathBuf,
    trust: Mutex<Trust>,
    pending: Mutex<Vec<Pending>>,
    ticket: Mutex<Option<Ticket>>,
    attempts: Mutex<HashMap<IpAddr, (u64, u8)>>,
    share: RwLock<Share>,
    running: AtomicBool,
    error: Mutex<Option<String>>,
    handle: axum_server::Handle<SocketAddr>,
    port: u16,
    addresses: Vec<String>,
    transferred: Mutex<HashMap<String, u64>>,
    mdns: Mutex<Option<mdns_sd::ServiceDaemon>>,
    preparation: Arc<Mutex<Preparation>>,
}
impl Drop for Service {
    fn drop(&mut self) {
        if let Some(hub) = lock(&self.hub).take() {
            hub.stop();
        }
    }
}
impl Hub {
    fn persist(&self) -> Result<()> {
        save(&self.folder.join("peers.json"), &*lock(&self.trust))
    }
    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.handle.shutdown();
        if let Some(service) = &self.integrations {
            service.clear_phone(&self.listening_scope, None);
        }
        if let Some(mdns) = lock(&self.mdns).take() {
            let _ = mdns.shutdown();
        }
    }
    fn manifest(&self) -> Manifest {
        let share = read(&self.share);
        let mut manifest = Manifest {
            portraits:share.portraits.clone(),
            capabilities:if self.database.is_some(){vec!["collection-mutations-v1".into(),"lyric-assets-v1".into()]}else{vec![]},
            version: 1,
            server_id: self.identity.id.clone(),
            server_name: self.name.clone(),
            display_name: share.display_name.clone(),
            revision: String::new(),
            tracks: share.tracks.clone(),
            collections: share.collections.clone(),
            bytes_total: share.tracks.iter().map(|t| t.size).sum(),copy_format:share.format,
        };
        if let Some(db)=&self.database {
            if let Ok(favorites)=crate::companion_sync::favorites(db){for t in &mut manifest.tracks{if let Some((_,value))=favorites.iter().find(|(id,_)|*id==t.id){t.favorited=*value;}}}
            let repo=musicos_desktop::local::LocalLibraryRepository::new(db,db.parent().unwrap_or(FsPath::new(".")));
            if let Ok(items)=crate::collections::list(&repo){let available:HashSet<_>=manifest.tracks.iter().map(|t|t.id).collect();manifest.collections=items.into_iter().filter_map(|c|{let ids=c.track_ids.iter().filter(|id|available.contains(id)).copied().collect::<Vec<_>>();if ids.is_empty(){return None}Some(Collection{revision:crate::companion_sync::collection_revision(&c.name,&c.track_ids),complete:ids.len()==c.track_ids.len(),full_track_ids:c.track_ids,id:c.id.clone(),name:c.name,kind:c.kind,track_ids:ids,artwork_choice:c.artwork_choice,artwork_sha256:share.collections.iter().find(|old|old.id==c.id).and_then(|old|old.artwork_sha256.clone())})}).collect();}
        }
        manifest.revision = digest(&serde_json::to_vec(&manifest).unwrap_or_default());
        manifest
    }
    fn listening_allowed(&self, peer: &str) -> bool {
        self.running.load(Ordering::Relaxed)
            && lock(&self.trust).peers.iter().any(|p| p.id == peer && p.listening_allowed)
    }
    fn authorized(&self, headers: &HeaderMap) -> Option<String> {
        let bearer = headers
            .get(header::AUTHORIZATION)?
            .to_str()
            .ok()?
            .strip_prefix("Bearer ")?;
        let (id, token) = bearer.split_once('.')?;
        if token.len() != 64 {
            return None;
        }
        let mut trust = lock(&self.trust);
        let peer = trust
            .peers
            .iter_mut()
            .find(|p| equal(&p.id, id) && equal(&p.token_hash, &digest(token.as_bytes())))?;
        peer.last_seen_at = now();
        Some(id.into())
    }
    fn status(&self) -> Value {
        let share = read(&self.share);
        let peers=lock(&self.trust).peers.iter().map(|p|json!({"id":p.id,"name":p.name,"listening_allowed":p.listening_allowed,"approved_at":p.approved_at,"last_seen_at":p.last_seen_at,"last_sync_at":p.last_sync_at,"track_count":p.track_count,"bytes_transferred":lock(&self.transferred).get(&p.id).copied().unwrap_or(0)})).collect::<Vec<_>>();
        let mut pending = lock(&self.pending);
        pending.retain(|p| now() - p.created_at <= 120);
        let ticket = lock(&self.ticket);
        let pairing = ticket.as_ref().filter(|t| t.expires_at > now()).map(|t| {
            let host = self
                .addresses
                .first()
                .cloned()
                .unwrap_or_else(|| "127.0.0.1".into());
            let link = format!(
                "musicos://pair?host={host}&port={}&server={}&fingerprint={}&ticket={}",
                self.port, self.identity.id, self.identity.fingerprint, t.secret
            );
            let qr = qrcode::QrCode::new(link.as_bytes())
                .map(|code| {
                    code.render::<qrcode::render::svg::Color>()
                        .min_dimensions(220, 220)
                        .build()
                })
                .unwrap_or_default();
            json!({"link":link,"qr":qr,"expires_at":t.expires_at})
        });
        json!({"running":self.running.load(Ordering::Relaxed),"server_id":self.identity.id,"name":self.name,"port":self.port,"addresses":self.addresses,"selected_ids":share.tracks.iter().map(|t|t.id).collect::<Vec<_>>(),"track_count":share.tracks.len(),"bytes_total":share.tracks.iter().map(|t|t.size).sum::<u64>(),"skipped":share.skipped,"peers":peers,"pending":pending.iter().map(|p|json!({"id":p.id,"name":p.name,"created_at":p.created_at,"sync_changes":p.sync_changes})).collect::<Vec<_>>(),"pairing":pairing,"error":*lock(&self.error),"format":share.format,"preparation":*lock(&self.preparation),"vorbis_available":media::vorbis_available()})
    }
}
impl Service {
    fn at(folder: PathBuf) -> Self {
        Self {
            hub: Mutex::new(None),
            generation: AtomicU64::new(0),
            operation: tokio::sync::Mutex::new(()),
            folder, preparation:Arc::new(Mutex::new(Preparation::default())),cache_operation:Mutex::new(()),database:Mutex::new(None),changed:Mutex::new(None),integrations:Mutex::new(None),
        }
    }
    pub fn configure(&self,db:PathBuf,changed:Arc<dyn Fn(Value)+Send+Sync>){*lock(&self.database)=Some(db);*lock(&self.changed)=Some(changed);}
    pub fn set_integrations(&self, service: crate::integrations::IntegrationService) {
        *lock(&self.integrations) = Some(service);
    }
    pub fn begin(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub fn current(&self, id: u64) -> bool {
        self.generation.load(Ordering::SeqCst) == id
    }
    pub fn begin_preparation(&self,format:CopyFormat,total:usize)->u64 {
        let _active=lock(&self.hub);let mut preparation=lock(&self.preparation);let generation=self.begin();*preparation=Preparation{preparing:true,total,format,..Default::default()};generation
    }
    pub fn fail(&self,generation:u64,message:String) {let mut p=lock(&self.preparation);if self.current(generation){p.preparing=false;p.current.clear();p.error=Some(message);}}
    pub fn progress(&self,generation:u64,completed:usize,total:usize,current:&str) {
        let mut p=lock(&self.preparation);if self.current(generation) {p.completed=completed;p.total=total;p.current=current.into();}
    }
    pub fn finish(&self,generation:u64,result:Result<Share>)->Result<()> {
        let active=lock(&self.hub);ensure!(self.current(generation),"Sharing preparation was cancelled");
        let hub=active.as_ref().context("Sharing stopped")?;
        match result {Ok(share)=>{*write(&hub.share)=share;let mut p=lock(&self.preparation);p.preparing=false;p.completed=p.total;p.current.clear();p.error=None;},Err(error)=>{let mut p=lock(&self.preparation);p.preparing=false;p.current.clear();p.error=Some(error.to_string());}}
        Ok(())
    }
    pub fn prepare_guard(&self)->std::sync::MutexGuard<'_,()> {lock(&self.cache_operation)}
    pub fn cache_folder(&self)->PathBuf {self.folder.clone()}
    pub fn status(&self) -> Value {
        if let Some(hub) = lock(&self.hub).as_ref() {
            return hub.status();
        }
        let trust = fs::read(self.folder.join("peers.json"))
            .ok()
            .and_then(|v| serde_json::from_slice::<Trust>(&v).ok())
            .unwrap_or_default();
        json!({"running":false,"peers":trust.peers.iter().map(|p|json!({"id":p.id,"name":p.name,"listening_allowed":p.listening_allowed,"last_sync_at":p.last_sync_at,"track_count":p.track_count})).collect::<Vec<_>>(),"pending":[],"selected_ids":[],"track_count":0,"bytes_total":0,"skipped":0,"pairing":null,"error":null,"preparation":*lock(&self.preparation),"vorbis_available":media::vorbis_available()})
    }
    pub async fn start(&self, share: Share, name: String, generation: u64) -> Result<Value> {
        let _operation = self.operation.lock().await;
        ensure!(
            self.current(generation),
            "Sharing preparation was cancelled"
        );
        if let Some(hub) = lock(&self.hub)
            .as_ref()
            .filter(|h| h.running.load(Ordering::Relaxed))
            .cloned()
        {
            if !share.tracks.is_empty()||!lock(&self.preparation).preparing {*write(&hub.share)=share;}
            return Ok(hub.status());
        }
        let folder = self.folder.clone();
        let identity = Identity::load(&folder)?;
        let trust = if folder.join("peers.json").exists() {
            serde_json::from_slice(&fs::read(folder.join("peers.json"))?)?
        } else {
            Trust::default()
        };
        let _ = rustls::crypto::ring::default_provider().install_default();
        let tls = axum_server::tls_rustls::RustlsConfig::from_der(
            vec![STANDARD.decode(&identity.certificate)?],
            STANDARD.decode(&identity.key)?,
        )
        .await?;
        ensure!(
            self.current(generation),
            "Sharing preparation was cancelled"
        );
        let listener = TcpListener::bind((if cfg!(test) { "127.0.0.1" } else { "0.0.0.0" }, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let mut interfaces = if_addrs::get_if_addrs()?
            .into_iter()
            .filter(|i| {
                !i.is_loopback()
                    && matches!(i.ip(),IpAddr::V4(ip)if ip.is_private()||ip.is_link_local())
            })
            .collect::<Vec<_>>();
        interfaces.sort_by_key(|i| (!i.name.starts_with("en"), i.name.clone()));
        let addresses = interfaces
            .iter()
            .map(|i| i.ip().to_string())
            .collect::<Vec<_>>();
        let hub = Arc::new(Hub {
            database:lock(&self.database).clone(),changed:lock(&self.changed).clone(),
            integrations:lock(&self.integrations).clone(),
            listening_scope:random(),
            identity,
            name: name.chars().filter(|c| !c.is_control()).take(50).collect(),
            folder,
            trust: Mutex::new(trust),
            pending: Mutex::new(vec![]),
            ticket: Mutex::new(None),
            attempts: Mutex::new(HashMap::new()),
            share: RwLock::new(share),
            running: AtomicBool::new(true),
            error: Mutex::new(None),
            handle: axum_server::Handle::new(),
            port,
            addresses,
            transferred: Mutex::new(HashMap::new()),
            mdns: Mutex::new(None), preparation:self.preparation.clone(),
        });
        let props = HashMap::from([
            ("id".to_owned(), hub.identity.id.clone()),
            ("fingerprint".into(), hub.identity.fingerprint.clone()),
            ("version".into(), "1".into()),
        ]);
        let advertisement = (|| -> Result<mdns_sd::ServiceDaemon> {
            ensure!(!cfg!(test), "Discovery disabled for isolated tests");
            let mdns = mdns_sd::ServiceDaemon::new()?;
            let info = mdns_sd::ServiceInfo::new(
                "_musicos-sync._tcp.local.",
                &hub.name,
                &format!("musicos-{}.local.", &hub.identity.id[..12]),
                hub.addresses.join(",").as_str(),
                port,
                props,
            )?;
            mdns.register(info)?;
            Ok(mdns)
        })();
        match advertisement {
            Ok(mdns) => *lock(&hub.mdns) = Some(mdns),
            Err(e) => {
                *lock(&hub.error) = Some(format!(
                    "Automatic discovery unavailable; the pairing link still works: {e}"
                ))
            }
        }
        let router = Router::new()
            .route("/v1/info", get(info))
            .route("/v1/pair", post(pair))
            .route("/v1/pair/status", get(pair_status))
            .route("/v1/manifest", get(manifest))
            .route("/v1/preparation", get(preparation))
            .route("/v1/tracks/{id}", get(audio))
            .route("/v1/artwork/{id}", get(artwork))
            .route("/v1/assets/{id}", get(artwork))
            .route("/v1/changes",post(changes).layer(DefaultBodyLimit::max(256*1024)))
            .route("/v1/sync-complete", post(complete))
            .route("/v1/listening", get(listening_status).post(listening_update))
            .layer(axum::middleware::from_fn(
                |ConnectInfo(remote): ConnectInfo<SocketAddr>,
                 request: axum::extract::Request,
                 next: axum::middleware::Next| async move {
                    if private(remote.ip()) {
                        next.run(request).await
                    } else {
                        Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Body::empty())
                            .unwrap()
                    }
                },
            ))
            .layer(DefaultBodyLimit::max(4096))
            .with_state(hub.clone());
        let server = axum_server::from_tcp_rustls(listener, tls)?.handle(hub.handle.clone());
        let task_hub = hub.clone();
        tauri::async_runtime::spawn(async move {
            let result = server
                .serve(router.into_make_service_with_connect_info::<SocketAddr>())
                .await;
            if let Err(e) = result {
                *lock(&task_hub.error) = Some(format!("Wi-Fi sharing stopped: {e}"));
            }
            task_hub.running.store(false, Ordering::Relaxed);
        });
        {
            let mut active = lock(&self.hub);
            if !self.current(generation) {
                hub.stop();
                bail!("Sharing preparation was cancelled");
            }
            if let Some(old) = active.replace(hub.clone()) {
                old.stop();
            }
        }
        Ok(hub.status())
    }
    pub fn action(&self, action: &str, id: Option<&str>) -> Result<Value> {
        let hub = lock(&self.hub).clone();
        if action == "stop" {
            self.begin();*lock(&self.preparation)=Preparation::default();
            if let Some(hub) = hub {
                hub.stop();
            }
            return Ok(self.status());
        }
        if hub.is_none() && action == "revoke" {
            let path = self.folder.join("peers.json");
            let mut trust: Trust = if path.exists() {
                serde_json::from_slice(&fs::read(&path)?)?
            } else {
                Trust::default()
            };
            trust.peers.retain(|p| Some(p.id.as_str()) != id);
            save(&path, &trust)?;
            return Ok(self.status());
        }
        let hub = hub.context("Start Wi-Fi sharing first")?;
        match action {
            "pair" => {
                ensure!(
                    !hub.addresses.is_empty(),
                    "Connect this Mac to Wi-Fi or Ethernet first"
                );
                *lock(&hub.ticket) = Some(Ticket {
                    secret: random(),
                    expires_at: now() + 120,
                });
            }
            "approve" => {
                let id = id.context("Choose a phone")?;
                let mut pending = lock(&hub.pending);
                let index = pending
                    .iter()
                    .position(|p| p.id == id && now() - p.created_at <= 120)
                    .context("This pairing request expired. Try again.")?;
                let p = pending.remove(index);
                let mut trust = lock(&hub.trust);
                ensure!(
                    trust.peers.len() < 20,
                    "Forget an old phone before pairing another"
                );
                trust.peers.retain(|peer| peer.id != p.id);
                trust.peers.push(Peer {
                    sync_changes:p.sync_changes,
                    listening_allowed:false,
                    id: p.id,
                    name: p.name,
                    token_hash: p.token_hash,
                    approved_at: now(),
                    last_seen_at: now(),
                    last_sync_at: None,
                    last_revision: None,
                    track_count: 0,
                });
                drop(trust);
                hub.persist()?;
            }
            "reject" => lock(&hub.pending).retain(|p| Some(p.id.as_str()) != id),
            "revoke" | "allow_listening" | "block_listening" => {
                let id = id.context("Choose a phone")?;
                let mut trust = lock(&hub.trust);
                let mut next = trust.clone();
                if action == "revoke" {
                    next.peers.retain(|p| p.id != id);
                } else {
                    let peer = next.peers.iter_mut().find(|p| p.id == id).context("Phone is no longer paired")?;
                    peer.listening_allowed = action == "allow_listening";
                }
                save(&hub.folder.join("peers.json"), &next)?;
                *trust = next;
                drop(trust);
                if action != "allow_listening" {
                    if let Some(service) = &hub.integrations {
                        service.clear_phone(&hub.listening_scope, Some(id));
                    }
                }
            }
            _ => bail!("Unknown Wi-Fi action"),
        };
        Ok(hub.status())
    }
}
async fn info(State(hub): State<Arc<Hub>>) -> Json<Value> {
    Json(json!({"version":1,"server_id":hub.identity.id,"name":hub.name,"capabilities":if hub.database.is_some(){vec!["collection-mutations-v1","lyric-assets-v1"]}else{vec![]}}))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairRequest {
    #[serde(default)] capabilities:Vec<String>,
    ticket: String,
    client_id: String,
    client_name: String,
    client_token: String,
}
type HttpError = (StatusCode, &'static str);
async fn pair(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    Json(request): Json<PairRequest>,
) -> Result<Json<Value>, HttpError> {
    if !private(address.ip()) {
        return Err((StatusCode::FORBIDDEN, "Local connections only"));
    }
    let mut attempts = lock(&hub.attempts);
    let bucket = attempts.entry(address.ip()).or_insert((now(), 0));
    if now() - bucket.0 > 60 {
        *bucket = (now(), 0);
    }
    if bucket.1 >= 5 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Try pairing again in a minute",
        ));
    }
    bucket.1 += 1;
    drop(attempts);
    if request.client_id.len() != 64
        || request.client_token.len() != 64
        || !request
            .client_id
            .bytes()
            .chain(request.client_token.bytes())
            .all(|c| c.is_ascii_hexdigit())
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid phone identity"));
    }
    let mut ticket = lock(&hub.ticket);
    if !ticket
        .as_ref()
        .is_some_and(|t| t.expires_at > now() && equal(&t.secret, &request.ticket))
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Pairing link expired. Create another on the Mac.",
        ));
    }
    let mut pending = lock(&hub.pending);
    pending.retain(|p| now() - p.created_at <= 120);
    if pending.len() >= 5 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Review the pending phones on the Mac",
        ));
    }
    pending.retain(|p| p.id != request.client_id);
    pending.push(Pending {
        sync_changes:request.capabilities.iter().any(|c|c=="collection-mutations-v1"),
        id: request.client_id.clone(),
        name: request
            .client_name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(60)
            .collect(),
        token_hash: digest(request.client_token.as_bytes()),
        created_at: now(),
    });
    *ticket = None;
    Ok(Json(
        json!({"state":"pending","request_id":request.client_id,"server_id":hub.identity.id,"server_name":hub.name}),
    ))
}
async fn pair_status(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
) -> Result<Json<Value>, HttpError> {
    if hub.authorized(&headers).is_some() {
        return Ok(Json(
            json!({"state":"approved","server_id":hub.identity.id,"server_name":hub.name}),
        ));
    }
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|v| v.split_once('.'))
        .ok_or((StatusCode::UNAUTHORIZED, "Pair this phone first"))?;
    let pending = lock(&hub.pending);
    let exists = pending.iter().any(|p| {
        now() - p.created_at <= 120
            && equal(&p.id, bearer.0)
            && equal(&p.token_hash, &digest(bearer.1.as_bytes()))
    });
    if exists {
        Ok(Json(json!({"state":"pending"})))
    } else {
        Err((StatusCode::UNAUTHORIZED, "Pairing was declined or expired"))
    }
}
async fn changes(State(hub):State<Arc<Hub>>,headers:HeaderMap,Json(request):Json<Value>)->Result<Json<Value>,HttpError>{
 let peer=hub.authorized(&headers).ok_or((StatusCode::UNAUTHORIZED,"Pair this phone first"))?;
 if !lock(&hub.trust).peers.iter().any(|p|p.id==peer&&p.sync_changes){return Err((StatusCode::FORBIDDEN,"Approve this phone's favorite and playlist changes in Devices first"))}
 let db=hub.database.clone().ok_or((StatusCode::SERVICE_UNAVAILABLE,"Collection sync is unavailable"))?;
 let ids=read(&hub.share).tracks.iter().map(|t|t.id).collect::<Vec<_>>();
 let value=tokio::task::spawn_blocking(move||crate::companion_sync::apply(&db,&peer,request,&ids)).await.map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"Sync could not finish"))?.map_err(|_|(StatusCode::CONFLICT,"The change could not be applied. Refresh the complete playlist and try again."))?;
 if value["state"]=="saved"{if let Some(changed)=&hub.changed{changed(value.clone());}}Ok(Json(value))
}
async fn listening_status(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
) -> Result<Json<Value>, HttpError> {
    let peer = hub.authorized(&headers).ok_or((StatusCode::UNAUTHORIZED, "Pair this phone first"))?;
    let service = hub.integrations.clone().ok_or((StatusCode::NOT_FOUND, "Listening sync is unavailable"))?;
    if !hub.listening_allowed(&peer) {
        return Ok(Json(json!({"allowed":false,"lastfm_ready":false,"discord_ready":false})));
    }
    let status = tokio::task::spawn_blocking(move || service.phone_status())
        .await.map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Listening sync is unavailable"))?;
    Ok(Json(json!({"allowed":hub.listening_allowed(&peer),"lastfm_ready":status.lastfm_ready,"discord_ready":status.discord_ready})))
}

async fn listening_update(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
    Json(update): Json<crate::integrations::PhoneUpdate>,
) -> Result<Json<Value>, HttpError> {
    use musicos_desktop::{local::library::track_from_init, models::Track};
    let peer = hub.authorized(&headers).ok_or((StatusCode::UNAUTHORIZED, "Pair this phone first"))?;
    if !hub.listening_allowed(&peer) {
        return Err((StatusCode::FORBIDDEN, "Allow listening activity for this phone in Devices on the Mac"));
    }
    if !update.valid() {
        return Err((StatusCode::BAD_REQUEST, "Invalid listening update"));
    }
    let service = hub.integrations.clone().ok_or((StatusCode::NOT_FOUND, "Listening sync is unavailable"))?;
    let db_path = hub.database.clone().ok_or((StatusCode::SERVICE_UNAVAILABLE, "Library unavailable"))?;
    let result = tokio::task::spawn_blocking(move || -> Result<Value, HttpError> {
        let ids = update.listens.iter().map(|listen| listen.track_id)
            .chain(update.presence.iter().map(|state| state.track_id))
            .collect::<HashSet<_>>();
        let mut tracks = BTreeMap::new();
        {
            let share = read(&hub.share);
            for track in share.tracks.iter().filter(|track| ids.contains(&track.id)) {
                tracks.insert(track.id, Track {
                    id: track.id,
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration: (track.duration_ms / 1000).clamp(0, i32::MAX as i64) as i32,
                    ..Default::default()
                });
            }
        }
        if tracks.len() != ids.len() {
            // Older downloads can still refer to a song outside the current sharing selection.
            let db = musicos_init::database::Db::open(&db_path.to_string_lossy())
                .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Library unavailable"))?;
            for id in ids {
                if let std::collections::btree_map::Entry::Vacant(slot) = tracks.entry(id) {
                    if let Some(track) = db.get_track_by_id(id)
                        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Library unavailable"))? {
                        slot.insert(track_from_init(track));
                    }
                }
            }
        }
        let reply = service.phone_update(&hub.listening_scope, &peer, update, &tracks, || hub.listening_allowed(&peer))
            .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Listening changes could not be saved"))?;
        if !hub.listening_allowed(&peer) {
            service.clear_phone(&hub.listening_scope, Some(&peer));
            return Err((StatusCode::FORBIDDEN, "Listening access was turned off"));
        }
        serde_json::to_value(reply).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Listening sync could not finish"))
    }).await.map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Listening sync is unavailable"))??;
    Ok(Json(result))
}

async fn preparation(State(hub):State<Arc<Hub>>,headers:HeaderMap)->Result<Json<Value>,HttpError> {
    hub.authorized(&headers).ok_or((StatusCode::UNAUTHORIZED,"Pair this phone first"))?;
    Ok(Json(json!(*lock(&hub.preparation))))
}
async fn manifest(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
) -> Result<Json<Manifest>, HttpError> {
    hub.authorized(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "Pair this phone first"))?;
    if lock(&hub.preparation).preparing {return Err((StatusCode::SERVICE_UNAVAILABLE,"The Mac is preparing your music. Keep sharing open."));}
    if lock(&hub.preparation).error.is_some() {return Err((StatusCode::CONFLICT,"Preparation failed. Check Devices on the Mac."));}
    Ok(Json(hub.manifest()))
}
/// A single bounded HTTP range; malformed/multipart requests never expand a transfer.
fn range(value: Option<&str>, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    let Some(value) = value else {
        return Some((0, size - 1));
    };
    let (from, to) = value.strip_prefix("bytes=")?.split_once('-')?;
    if from.is_empty() {
        let suffix = to.parse::<u64>().ok()?;
        return (suffix > 0).then_some((size.saturating_sub(suffix), size - 1));
    }
    let start = from.parse::<u64>().ok()?;
    let end = if to.is_empty() {
        size - 1
    } else {
        to.parse::<u64>().ok()?.min(size - 1)
    };
    (start < size && end >= start).then_some((start, end))
}
async fn transfer(
    hub: Arc<Hub>,
    headers: HeaderMap,
    id: String,
    art: bool,
) -> Result<Response, HttpError> {
    let peer = hub
        .authorized(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "Pair this phone first"))?;
    let file = {
        let share = read(&hub.share);
        if art {
            share.art.get(&id)
        } else {
            share.files.get(&id)
        }
        .cloned()
    }
    .ok_or((StatusCode::NOT_FOUND, "This file is not shared"))?;
    if let Some(expected)=headers.get(header::IF_MATCH).and_then(|v|v.to_str().ok()) {if expected!=format!("\"{}\"",file.sha256) {return Err((StatusCode::PRECONDITION_FAILED,"The copy format or selection changed. Refresh the phone's sync."));}}
    let mut input = tokio::fs::File::open(&file.path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "The file is unavailable"))?;
    let metadata = input
        .metadata()
        .await
        .map_err(|_| (StatusCode::CONFLICT, "Refresh the sync selection"))?;
    if metadata.len() != file.size || metadata.modified().ok() != Some(file.modified) {
        return Err((
            StatusCode::CONFLICT,
            "The file changed. Refresh the sync selection.",
        ));
    }
    let Some((start, end)) = range(
        headers.get(header::RANGE).and_then(|v| v.to_str().ok()),
        file.size,
    ) else {
        return Ok(Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{}", file.size))
            .body(Body::empty())
            .unwrap());
    };
    input
        .seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Cannot read the file"))?;
    let mut remaining = end - start + 1;
    let length = remaining;
    let partial = headers.contains_key(header::RANGE);
    let stream = async_stream::stream! {let mut buffer=vec![0u8;65536];while remaining>0{if !hub.running.load(Ordering::Relaxed)||!lock(&hub.trust).peers.iter().any(|p|p.id==peer){yield Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied,"Sharing ended"));break;}let limit=remaining.min(buffer.len()as u64)as usize;let count=match input.read(&mut buffer[..limit]).await{Ok(n)=>n,Err(e)=>{yield Err(e);break;}};if count==0{yield Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof,"File changed"));break;}remaining-=count as u64;*lock(&hub.transferred).entry(peer.clone()).or_default()+=count as u64;yield Ok::<Bytes,std::io::Error>(Bytes::copy_from_slice(&buffer[..count]));}};
    let mut response = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, &file.mime)
        .header(header::CONTENT_LENGTH, length)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, format!("\"{}\"", file.sha256))
        .header(header::CACHE_CONTROL, "private, no-store");
    if partial {
        response = response.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{}", file.size),
        );
    }
    Ok(response.body(Body::from_stream(stream)).unwrap())
}
async fn audio(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, HttpError> {
    transfer(hub, headers, id, false).await
}
async fn artwork(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, HttpError> {
    transfer(hub, headers, id, true).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Complete {
    revision: String,
    track_count: usize,
    bytes: u64,
}
async fn complete(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
    Json(request): Json<Complete>,
) -> Result<Json<Value>, HttpError> {
    let id = hub
        .authorized(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "Pair this phone first"))?;
    let manifest = hub.manifest();
    if request.revision != manifest.revision
        || request.track_count != manifest.tracks.len()
        || request.bytes != manifest.bytes_total
    {
        return Err((
            StatusCode::CONFLICT,
            "Selection changed. Sync again to finish.",
        ));
    }
    if let Some(peer) = lock(&hub.trust).peers.iter_mut().find(|p| p.id == id) {
        peer.last_sync_at = Some(now());
        peer.track_count = request.track_count;
        peer.last_revision = Some(request.revision);
    }
    hub.persist().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not save sync status",
        )
    })?;
    Ok(Json(json!({"complete":true})))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_share(root: &FsPath) -> Share {
        let path = root.join("song.flac");
        fs::write(&path, b"offline fixture bytes").unwrap();
        let media = file(&path, "audio/flac".into()).unwrap();
        Share {
            tracks: vec![Track {
                id: 7,
                title: "Fixture".into(),
                artist: "MusicOS".into(),
                album: "Offline".into(),
                album_id: "album/1".into(),
                genre: "".into(),
                year: 2026,
                track_number: 1,
                disc_number: 1,
                duration_ms: 1000,
                size: media.size,
                sha256: media.sha256.clone(),
                mime: media.mime.clone(),
                favorited: true,
                artwork_sha256: None,lyrics_sha256:None,isrc:None,
            }],
            files: BTreeMap::from([("7".into(), media)]),
            collections: vec![Collection {
                revision:String::new(),complete:true,full_track_ids:vec![7,7],artwork_sha256:None,artwork_choice:None,
                id: "sequence".into(),
                name: "Duplicates".into(),
                kind: "manual".into(),
                track_ids: vec![7, 7],
            }],
            ..Default::default()
        }
    }
    #[tokio::test]
    async fn phone_listening_requires_permission_and_acknowledges_a_retry_once() {
        use crate::integrations::{Config, IntegrationService};
        let root = tempfile::tempdir().unwrap();
        let integration_dir = root.path().join("integrations");
        save(&integration_dir.join("integrations-v1.json"), &Config {
            lastfm_enabled: true, lastfm_username: Some("fixture".into()), ..Default::default()
        }).unwrap();
        let dir = integration_dir.clone();
        let integrations = tokio::task::spawn_blocking(move || IntegrationService::open(dir).unwrap()).await.unwrap();
        let service = Service::at(root.path().join("identity"));
        service.configure(root.path().join("fixture.sqlite"), Arc::new(|_| {}));
        service.set_integrations(integrations.clone());
        let mut share = fixture_share(root.path());
        share.tracks[0].duration_ms = 80_000;
        service.start(share,"Test".into(),service.begin()).await.unwrap();
        let hub = lock(&service.hub).as_ref().unwrap().clone();
        let cert = reqwest::Certificate::from_der(&STANDARD.decode(&hub.identity.certificate).unwrap()).unwrap();
        let client = reqwest::Client::builder().add_root_certificate(cert).build().unwrap();
        let url = format!("https://localhost:{}/v1/listening",hub.port);
        let id = random();
        let token = random();
        lock(&hub.pending).push(Pending {
            sync_changes: false, id: id.clone(), name: "Fixture iPhone".into(),
            token_hash: digest(token.as_bytes()), created_at: now(),
        });
        service.action("approve",Some(&id)).unwrap();
        let auth = format!("{id}.{token}");
        let receipt = "5df08ed1-f335-43ca-8c8d-4446ac778500";
        let update = json!({"listens":[{"id":receipt,"track_id":7,"started_at":now()-80,"listened_seconds":40}]});
        assert_eq!(client.post(&url).json(&update).send().await.unwrap().status(),401);
        assert_eq!(client.post(&url).bearer_auth(&auth).json(&update).send().await.unwrap().status(),403);
        let blocked: Value = client.get(&url).bearer_auth(&auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(blocked["allowed"],false);
        service.action("allow_listening",Some(&id)).unwrap();
        let allowed: Value = client.get(&url).bearer_auth(&auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(allowed["allowed"],true);
        assert_eq!(allowed["lastfm_ready"],true);
        for _ in 0..2 {
            let reply: Value = client.post(&url).bearer_auth(&auth).json(&update).send().await.unwrap().json().await.unwrap();
            assert_eq!(reply["accepted"],json!([receipt]));
        }
        let saved: Value = serde_json::from_slice(&fs::read(integration_dir.join("lastfm-journal-v2.json")).unwrap()).unwrap();
        assert_eq!(saved["queue"].as_array().unwrap().len(),1);
        assert_eq!(saved["receipts"].as_object().unwrap().len(),1);
        assert_eq!(saved["queue"][0]["track"],"Fixture");
        let mut short = update.clone();
        short["listens"][0]["id"] = json!("5df08ed1-f335-43ca-8c8d-4446ac778501");
        short["listens"][0]["listened_seconds"] = json!(25);
        let reply: Value = client.post(&url).bearer_auth(&auth).json(&short).send().await.unwrap().json().await.unwrap();
        assert_eq!(reply["discarded"],json!(["5df08ed1-f335-43ca-8c8d-4446ac778501"]));
        service.action("block_listening",Some(&id)).unwrap();
        assert_eq!(client.post(&url).bearer_auth(&auth).json(&update).send().await.unwrap().status(),403);
        drop(hub);
        drop(service);
        tokio::task::spawn_blocking(move || drop(integrations)).await.unwrap();
    }

    #[tokio::test]
    async fn pairing_is_available_during_preparation_and_stale_completion_cannot_publish() {
        let root=tempfile::tempdir().unwrap();let service=Service::at(root.path().join("identity"));let generation=service.begin_preparation(CopyFormat::Vorbis256,508);
        let started=std::time::Instant::now();service.start(Share::default(),"Test Mac".into(),generation).await.unwrap();assert!(started.elapsed()<std::time::Duration::from_secs(2));
        let hub=lock(&service.hub).as_ref().unwrap().clone();let cert=reqwest::Certificate::from_der(&STANDARD.decode(&hub.identity.certificate).unwrap()).unwrap();let client=reqwest::Client::builder().add_root_certificate(cert).build().unwrap();let base=format!("https://localhost:{}/v1",hub.port);
        let ticket=random();*lock(&hub.ticket)=Some(Ticket{secret:ticket.clone(),expires_at:now()+120});let id=random();let token=random();let auth=format!("{id}.{token}");
        assert_eq!(client.post(format!("{base}/pair")).json(&json!({"ticket":ticket,"client_id":id,"client_name":"Phone","client_token":token})).send().await.unwrap().status(),200);
        service.action("approve",Some(&id)).unwrap();service.progress(generation,42,508,"Fixture song");
        assert_eq!(client.get(format!("{base}/manifest")).bearer_auth(&auth).send().await.unwrap().status(),503);
        let progress:Value=client.get(format!("{base}/preparation")).bearer_auth(&auth).send().await.unwrap().json().await.unwrap();assert_eq!(progress["completed"],42);assert!(!progress.to_string().contains(&ticket));assert!(!progress.to_string().contains(&token));
        service.finish(generation,Ok(fixture_share(root.path()))).unwrap();assert_eq!(client.get(format!("{base}/manifest")).bearer_auth(&auth).send().await.unwrap().status(),200);
        let old=service.begin_preparation(CopyFormat::Original,1);service.action("stop",None).unwrap();assert!(service.finish(old,Ok(fixture_share(root.path()))).is_err());assert_eq!(service.status()["running"],false);assert_eq!(service.status()["preparation"]["preparing"],false);
    }
    #[tokio::test]
    async fn tls_pair_approval_ranges_revision_and_revocation() {
        let root = tempfile::tempdir().unwrap();
        let service = Service::at(root.path().join("identity"));
        let generation = service.begin();
        service
            .start(fixture_share(root.path()), "Test Mac".into(), generation)
            .await
            .unwrap();
        let hub = lock(&service.hub).as_ref().unwrap().clone();
        let cert =
            reqwest::Certificate::from_der(&STANDARD.decode(&hub.identity.certificate).unwrap())
                .unwrap();
        let client = reqwest::Client::builder()
            .add_root_certificate(cert)
            .build()
            .unwrap();
        let base = format!("https://localhost:{}/v1", hub.port);
        assert_eq!(
            client
                .get(format!("{base}/manifest"))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let ticket = random();
        *lock(&hub.ticket) = Some(Ticket {
            secret: ticket.clone(),
            expires_at: now() + 120,
        });
        let id = random();
        let token = random();
        let payload = json!({"ticket":ticket,"client_id":id,"client_name":"Offline phone","client_token":token});
        assert_eq!(
            client
                .post(format!("{base}/pair"))
                .json(&payload)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            client
                .post(format!("{base}/pair"))
                .json(&payload)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let auth = format!("{id}.{token}");
        assert_eq!(
            client
                .get(format!("{base}/manifest"))
                .bearer_auth(&auth)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        service.action("approve", Some(&id)).unwrap();
        assert_eq!(client.get(format!("{base}/tracks/7")).bearer_auth(&auth).header(header::IF_MATCH,"\"wrong-version\"").send().await.unwrap().status(),412);
        let manifest: Value = client
            .get(format!("{base}/manifest"))
            .bearer_auth(&auth)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(manifest["collections"][0]["track_ids"], json!([7, 7]));
        assert!(manifest.to_string().find("song.flac").is_none());
        let response = client
            .get(format!("{base}/tracks/7"))
            .bearer_auth(&auth)
            .header("Range", "bytes=8-")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["content-range"], "bytes 8-20/21");
        assert_eq!(response.bytes().await.unwrap().as_ref(), b"fixture bytes");
        let response = client
            .get(format!("{base}/tracks/7"))
            .bearer_auth(&auth)
            .header("Range", "bytes=900-")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 416);
        assert_eq!(response.headers()["content-range"], "bytes */21");
        assert_eq!(
            client
                .get(format!("{base}/tracks/8"))
                .bearer_auth(&auth)
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
        assert_eq!(
            client
                .post(format!("{base}/sync-complete"))
                .bearer_auth(&auth)
                .json(&json!({"revision":"wrong","track_count":1,"bytes":21}))
                .send()
                .await
                .unwrap()
                .status(),
            409
        );
        assert_eq!(
            client
                .post(format!("{base}/sync-complete"))
                .bearer_auth(&auth)
                .json(&json!({"revision":manifest["revision"],"track_count":1,"bytes":21}))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        let saved = fs::read_to_string(root.path().join("identity/peers.json")).unwrap();
        assert!(!saved.contains(&token));
        assert!(saved.contains(&digest(token.as_bytes())));
        fs::write(root.path().join("song.flac"), b"changed").unwrap();
        assert_eq!(
            client
                .get(format!("{base}/tracks/7"))
                .bearer_auth(&auth)
                .send()
                .await
                .unwrap()
                .status(),
            409
        );
        service.action("revoke", Some(&id)).unwrap();
        assert_eq!(
            client
                .get(format!("{base}/manifest"))
                .bearer_auth(&auth)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        service.action("stop", None).unwrap();
    }
    #[tokio::test]
    async fn cancelled_preparation_cannot_restart_a_stopped_share() {
        let root = tempfile::tempdir().unwrap();
        let service = Service::at(root.path().join("identity"));
        let generation = service.begin();
        service.action("stop", None).unwrap();
        assert!(service
            .start(fixture_share(root.path()), "Test".into(), generation)
            .await
            .is_err());
        assert!(!service.status()["running"].as_bool().unwrap());
        assert!(!root.path().join("identity").exists());
    }
    #[test]
    fn forgetting_a_stopped_peer_preserves_identity() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("identity");
        let identity = Identity::load(&folder).unwrap();
        save(
            &folder.join("peers.json"),
            &Trust {
                peers: vec![Peer {
                    sync_changes:false,
                    listening_allowed:false,
                    id: "phone".into(),
                    name: "Phone".into(),
                    token_hash: digest(b"secret"),
                    approved_at: now(),
                    last_seen_at: now(),
                    last_sync_at: None,
                    last_revision: None,
                    track_count: 0,
                }],
            },
        )
        .unwrap();
        let service = Service::at(folder.clone());
        service.action("revoke", Some("phone")).unwrap();
        assert_eq!(service.status()["peers"], json!([]));
        assert_eq!(Identity::load(&folder).unwrap().id, identity.id);
    }
    /// Explicit emulator-only fixture: never opens the user's library or profile.
    #[tokio::test]
    #[ignore]
    async fn android_tls_fixture() {
        let output = PathBuf::from(
            std::env::var("MUSICOS_ANDROID_FIXTURE").expect("Choose a fixture-only directory"),
        );
        fs::create_dir_all(&output).unwrap();
        let root = tempfile::tempdir().unwrap();
        let mut share = fixture_share(root.path());
        let samples = vec![0u8; 48000 * 2 * 30];
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&(36u32 + samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&48000u32.to_le_bytes());
        wav.extend_from_slice(&96000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(&samples);
        let path = root.path().join("offline.wav");
        fs::write(&path, &wav).unwrap();
        let media = file(&path, "audio/wav".into()).unwrap();
        share.tracks[0].sha256 = media.sha256.clone();
        share.tracks[0].size = media.size;
        share.tracks[0].mime = media.mime.clone();
        share.tracks[0].duration_ms = 30000;
        share.files.insert("7".into(), media);
        share.display_name = "Henry".into();
        let cover = root.path().join("cover.jpg");
        image::RgbImage::from_pixel(96, 96, image::Rgb([25, 90, 210]))
            .save(&cover)
            .unwrap();
        let art = file(&cover, "image/jpeg".into()).unwrap();
        share.tracks[0].artwork_sha256 = Some(art.sha256.clone());
        share.art.insert(art.sha256.clone(), art);
        let service = Service::at(root.path().join("identity"));
        let generation = service.begin();
        service
            .start(share, "Fixture Mac".into(), generation)
            .await
            .unwrap();
        let hub = lock(&service.hub).as_ref().unwrap().clone();
        let secret = random();
        *lock(&hub.ticket) = Some(Ticket {
            secret: secret.clone(),
            expires_at: now() + 120,
        });
        let link = format!(
            "musicos://pair?host=10.0.2.2&port={}&server={}&fingerprint={}&ticket={}",
            hub.port, hub.identity.id, hub.identity.fingerprint, secret
        );
        save(&output.join("pair.json"), &json!({"link":link})).unwrap();
        for _ in 0..1800 {
            if output.join("done").exists() || lock(&hub.trust).peers.iter().any(|p|p.last_sync_at.is_some()) {
                tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                break;
            }
            let pending = lock(&hub.pending)
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<_>>();
            for id in pending {
                service.action("approve", Some(&id)).unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        service.action("stop", None).unwrap();
    }
    #[test]
    fn ranges_are_bounded_and_resumable() {
        assert_eq!(range(None, 10), Some((0, 9)));
        assert_eq!(range(Some("bytes=4-"), 10), Some((4, 9)));
        assert_eq!(range(Some("bytes=-3"), 10), Some((7, 9)));
        assert_eq!(range(Some("bytes=5-100"), 10), Some((5, 9)));
        for value in [
            "bytes=10-",
            "bytes=5-4",
            "bytes=1-3,4-5",
            "bytes=-0",
            "../secret",
            "bytes=NaN-",
        ] {
            assert_eq!(range(Some(value), 10), None, "{}", value);
        }
    }
    #[test]
    fn tokens_are_fixed_random_capabilities() {
        let a = random();
        let b = random();
        assert_eq!(a.len(), 64);
        assert!(!equal(&a, &b));
        assert!(equal(&a, &a));
        assert!(!equal(&a, ""));
    }
    #[test]
    fn discovery_does_not_advertise_public_networks() {
        assert!(private("192.168.1.2".parse().unwrap()));
        assert!(private("127.0.0.1".parse().unwrap()));
        assert!(!private("8.8.8.8".parse().unwrap()));
    }
}
