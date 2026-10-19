//! Opt-in desktop integrations. Credentials never cross back into the webview.
use anyhow::{bail, Context, Result};
use musicos_desktop::{
    local::{LocalPlaybackClient, LocalPlaybackSnapshot},
    storage,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub discord_enabled: bool,
    pub discord_client_id: String,
    pub lastfm_enabled: bool,
    pub lastfm_username: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub config: Config,
    pub discord: String,
    pub lastfm: String,
    pub queued: usize,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    Configure {
        discord_enabled: bool,
        discord_client_id: String,
        lastfm_enabled: bool,
    },
    Authorize {
        api_key: String,
        shared_secret: String,
    },
    FinishAuthorization,
    DisconnectLastfm,
}
#[derive(Serialize)]
pub struct Response {
    pub status: Status,
    pub auth_url: Option<String>,
}
#[derive(Default, Serialize, Deserialize)]
struct Credentials {
    api_key: String,
    shared_secret: String,
    session_key: String,
    username: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Scrobble {
    artist: String,
    track: String,
    album: String,
    duration: i32,
    timestamp: u64,
    username: String,
}
#[derive(Default)]
struct Listen {
    id: Option<i64>,
    entry: Option<u64>,
    position: f32,
    playing: bool,
    listened: f32,
    started: u64,
    sent: bool,
    now_sent: bool,
}
impl Listen {
    fn update(&mut self, s: &LocalPlaybackSnapshot, elapsed: f32, now: u64) -> bool {
        let id = s.track.as_ref().map(|t| t.id);
        let entry = s
            .queue_index
            .and_then(|i| s.queue_entry_ids.get(i))
            .copied();
        // A natural repeat is a new listen; a seek does not inflate listened time.
        let repeated = id == self.id
            && self.playing
            && self.position > 2.0
            && s.position < 1.0
            && s.track
                .as_ref()
                .is_some_and(|t| self.position >= t.duration as f32 - 2.0);
        if id != self.id || entry != self.entry || repeated {
            *self = Self {
                id,
                entry,
                started: now,
                ..Default::default()
            };
        } else if self.playing && s.is_playing {
            self.listened += elapsed.min(2.0);
        }
        if let Some(listened) = s.listened {
            self.listened = listened.max(0.);
        }
        self.position = s.position;
        self.playing = s.is_playing;
        s.track.as_ref().is_some_and(|t| {
            t.duration > 30 && self.listened >= threshold(t.duration) && !self.sent
        })
    }
}
fn threshold(duration: i32) -> f32 {
    (duration as f32 / 2.0).min(240.0)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn persist<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    std::fs::create_dir_all(path.parent().context("Missing settings directory")?)?;
    let temporary = path.with_extension("pending");
    let mut file = std::fs::File::create(&temporary)?;
    use std::io::Write;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    std::fs::rename(temporary, path)?;
    Ok(())
}
fn signature(params: &BTreeMap<String, String>, secret: &str) -> String {
    let mut input = String::new();
    for (k, v) in params {
        if !matches!(k.as_str(), "format" | "api_sig" | "callback") {
            input.push_str(k);
            input.push_str(v);
        }
    }
    input.push_str(secret);
    format!("{:x}", md5::compute(input))
}
struct Worker {
    config: Config,
    directory: PathBuf,
    credentials: Option<Credentials>,
    token: Option<String>,
    http: reqwest::blocking::Client,
    discord: Discord,
    status: Status,
    queue: Vec<Scrobble>,
    listen: Listen,
    last_tick: Instant,
    last_retry: Instant,
    discord_key: String,
    discord_retry: Instant,
    covers: BTreeMap<String, CoverLookup>,
    cover_sender: std::sync::mpsc::Sender<(String, Option<String>)>,
    cover_receiver: std::sync::mpsc::Receiver<(String, Option<String>)>,
}
struct CoverLookup{url:Option<String>,attempts:u32,checked:Instant,pending:bool}
impl CoverLookup{fn due(&self)->bool{let delay=if self.pending{Duration::from_secs(45)}else if self.url.is_some(){Duration::from_secs(6*3600)}else{Duration::from_secs((30_u64*2_u64.pow(self.attempts.min(5))).min(15*60))};self.checked.elapsed()>=delay}}
fn discord_artist(track:&musicos_desktop::models::Track)->String{track.album_artist.as_deref().filter(|s|!s.trim().is_empty()).map(str::to_owned).unwrap_or_else(||crate::library::split_artist_credits(&track.artist)[0].to_owned())}
impl Worker {
    fn entry(&self) -> Result<keyring::Entry> {
        let profile = format!(
            "{:x}",
            md5::compute(self.directory.to_string_lossy().as_bytes())
        );
        keyring::Entry::new("MusicOS Last.fm", &profile)
            .map_err(|_| anyhow::anyhow!("System credential storage is unavailable."))
    }
    fn load_credentials(&mut self) -> Result<()> {
        if self.credentials.is_none() {
            let raw = self
                .entry()?
                .get_password()
                .map_err(|_| anyhow::anyhow!("Reconnect Last.fm to restore its secure session."))?;
            self.credentials = Some(
                serde_json::from_str(&raw)
                    .context("Reconnect Last.fm: the saved session could not be read.")?,
            );
        }
        Ok(())
    }
    fn save_credentials(&self) -> Result<()> {
        let raw = serde_json::to_string(
            self.credentials
                .as_ref()
                .context("No Last.fm credentials")?,
        )?;
        self.entry()?.set_password(&raw).map_err(|_| {
            anyhow::anyhow!("Could not save Last.fm credentials in the system keychain.")
        })
    }
    fn lastfm(
        &self,
        method: &str,
        mut params: BTreeMap<String, String>,
        session: bool,
    ) -> Result<Value> {
        let c = self
            .credentials
            .as_ref()
            .context("Connect your Last.fm account first.")?;
        params.insert("method".into(), method.into());
        params.insert("api_key".into(), c.api_key.clone());
        if session {
            params.insert("sk".into(), c.session_key.clone());
        }
        params.insert("api_sig".into(), signature(&params, &c.shared_secret));
        params.insert("format".into(), "json".into());
        let response = self
            .http
            .post("https://ws.audioscrobbler.com/2.0/")
            .form(&params)
            .send()
            .map_err(|_| {
                anyhow::anyhow!(
                    "Last.fm is unreachable. Check your connection; queued listens are retained."
                )
            })?;
        let value: Value = response
            .error_for_status()
            .context("Last.fm is temporarily unavailable.")?
            .json()
            .context("Last.fm returned an unreadable response.")?;
        if let Some(code) = value["error"].as_i64() {
            bail!("{}",match code { 9=>"Last.fm session expired. Reconnect your account.", 4|10|13|26=>"Last.fm rejected the app credentials. Check your API key and shared secret.",14=>"Authorize MusicOS in your browser, then try again.",15=>"Authorization expired. Start connecting again.",_=>"Last.fm could not complete the request. Try again later." });
        }
        Ok(value)
    }
    fn response(&mut self, auth_url: Option<String>) -> Response {
        self.status.config = self.config.clone();
        self.status.queued = self.queue.len();
        Response {
            status: self.status.clone(),
            auth_url,
        }
    }
    fn action(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::Configure {
                discord_enabled,
                discord_client_id,
                lastfm_enabled,
            } => {
                let id = discord_client_id.trim().to_owned();
                if discord_enabled && !(17..=20).contains(&id.len())
                    || discord_enabled && !id.bytes().all(|c| c.is_ascii_digit())
                {
                    bail!("Enter the Application ID from your Discord developer application.");
                }
                if lastfm_enabled {
                    self.load_credentials()?;
                    if self
                        .credentials
                        .as_ref()
                        .is_none_or(|c| c.session_key.is_empty())
                    {
                        bail!("Connect Last.fm before enabling scrobbling.");
                    }
                }
                let config = Config {
                    discord_enabled,
                    discord_client_id: id,
                    lastfm_enabled,
                    lastfm_username: self.config.lastfm_username.clone(),
                };
                persist(&self.directory.join("integrations-v1.json"), &config)
                    .context("Could not save MusicOS Labs settings.")?;
                if !discord_enabled || config.discord_client_id != self.config.discord_client_id {
                    self.discord.clear();
                    self.discord = Discord::default();
                }
                let listening_changed = config.lastfm_enabled != self.config.lastfm_enabled;
                self.config = config;
                self.discord_key.clear();
                self.discord_retry = Instant::now() - Duration::from_secs(20);
                if listening_changed {
                    self.listen = Listen::default();
                }
                self.status.discord = if discord_enabled {
                    "Waiting for Discord…"
                } else {
                    "Off"
                }
                .into();
                self.status.lastfm = if lastfm_enabled {
                    "Ready to scrobble"
                } else {
                    "Off"
                }
                .into();
            }
            Request::Authorize {
                api_key,
                shared_secret,
            } => {
                if self.config.lastfm_username.is_some() {
                    bail!("Disconnect the existing Last.fm account before connecting another.");
                }
                if api_key.trim().len() != 32 || shared_secret.trim().len() != 32 {
                    bail!("Enter the 32-character Last.fm API key and shared secret for your MusicOS application.");
                }
                self.credentials = Some(Credentials {
                    api_key: api_key.trim().into(),
                    shared_secret: shared_secret.trim().into(),
                    ..Default::default()
                });
                let value = self.lastfm("auth.getToken", BTreeMap::new(), false)?;
                let token = value["token"]
                    .as_str()
                    .context("Last.fm did not return an authorization token.")?
                    .to_owned();
                let mut url = url::Url::parse("https://www.last.fm/api/auth/")?;
                url.query_pairs_mut()
                    .append_pair("api_key", &self.credentials.as_ref().unwrap().api_key)
                    .append_pair("token", &token);
                self.token = Some(token);
                self.status.lastfm = "Waiting for browser authorization".into();
                return Ok(self.response(Some(url.to_string())));
            }
            Request::FinishAuthorization => {
                let token = self
                    .token
                    .clone()
                    .context("Start connecting Last.fm again.")?;
                let value = self.lastfm(
                    "auth.getSession",
                    BTreeMap::from([("token".into(), token)]),
                    false,
                )?;
                let session = value["session"]["key"]
                    .as_str()
                    .context("Last.fm did not return a session.")?
                    .to_owned();
                let username = value["session"]["name"]
                    .as_str()
                    .context("Last.fm did not return an account name.")?
                    .to_owned();
                let c = self
                    .credentials
                    .as_mut()
                    .context("Start connecting again.")?;
                c.session_key = session;
                c.username = username.clone();
                self.save_credentials()?;
                self.config.lastfm_username = Some(username);
                self.config.lastfm_enabled = false;
                persist(&self.directory.join("integrations-v1.json"), &self.config)?;
                self.token = None;
                self.status.lastfm = "Connected. Enable scrobbling when you're ready.".into();
            }
            Request::DisconnectLastfm => {
                match self.entry()?.delete_credential() {
                    Ok(()) | Err(keyring::Error::NoEntry) => {}
                    Err(_) => {
                        bail!("Could not remove the Last.fm session from the system keychain.")
                    }
                }
                self.config.lastfm_enabled = false;
                self.config.lastfm_username = None;
                persist(&self.directory.join("integrations-v1.json"), &self.config)?;
                self.credentials = None;
                self.token = None;
                self.queue.clear();
                persist(&self.directory.join("lastfm-queue-v1.json"), &self.queue)?;
                self.status.lastfm = "Disconnected".into();
            }
        }
        Ok(self.response(None))
    }
    fn tick(&mut self, s: LocalPlaybackSnapshot) {
        let elapsed = self.last_tick.elapsed().as_secs_f32();
        self.last_tick = Instant::now();
        if self.config.discord_enabled {
            for (key, url) in self.cover_receiver.try_iter() {
                if let Some(lookup)=self.covers.get_mut(&key){lookup.url=url;lookup.pending=false;lookup.checked=Instant::now();}
            }
            let album_key = s
                .track
                .as_ref()
                .map(|t| format!("{}\0{}", discord_artist(t), t.album));
            if s.is_playing {
                if let (Some(track), Some(key)) = (s.track.as_ref(), album_key.as_ref()) {
                    if self.covers.get(key).is_none_or(CoverLookup::due) {
                        if self.covers.len()>=256&&!self.covers.contains_key(key){if let Some(oldest)=self.covers.iter().min_by_key(|(_,entry)|entry.checked).map(|(key,_)|key.clone()){self.covers.remove(&oldest);}}
                        let attempts=self.covers.get(key).map_or(1,|entry|entry.attempts+1);let url=self.covers.get(key).and_then(|entry|entry.url.clone());self.covers.insert(key.clone(),CoverLookup{url,attempts,checked:Instant::now(),pending:true});
                        let sender = self.cover_sender.clone();
                        let key = key.clone();
                        let artist = discord_artist(track);
                        let album = track.album.clone();
                        std::thread::spawn(move || {
                            let url = crate::providers::discord_album_cover(&artist, &album)
                                .ok()
                                .flatten();
                            let _ = sender.send((key, url));
                        });
                    }
                }
            }
            let cover = album_key
                .as_ref()
                .and_then(|key| self.covers.get(key))
                .and_then(|lookup| lookup.url.as_deref());
            let key = format!(
                "{:?}:{}:{:?}:{}",
                s.track.as_ref().map(|t| t.id),
                s.is_playing,
                cover,
                if s.is_playing {
                    (now() as f64 - s.position as f64).round() as i64 / 3
                } else {
                    0
                }
            );
            if key != self.discord_key && self.discord_retry.elapsed() > Duration::from_secs(2) {
                self.discord_retry = Instant::now();
                match self
                    .discord
                    .update(&self.config.discord_client_id, &s, cover)
                {
                    Ok(()) => {
                        self.discord_key = key;
                        self.status.discord = if s.is_playing {
                            "Sharing your music"
                        } else {
                            "Connected · playback paused"
                        }
                        .into();
                    }
                    Err(_) => {
                        self.discord = Discord::default();
                        self.status.discord =
                            "Open Discord desktop, and check your Application ID.".into();
                        self.discord_retry = Instant::now() + Duration::from_secs(13);
                    }
                }
            }
        }
        if !self.config.lastfm_enabled {
            return;
        }
        if let Err(e) = self.load_credentials() {
            self.status.lastfm = e.to_string();
            return;
        }
        let eligible = self.listen.update(&s, elapsed, now());
        if let Some(t) = s.track.as_ref() {
            let p = BTreeMap::from([
                ("artist".into(), t.artist.clone()),
                ("track".into(), t.title.clone()),
                ("album".into(), t.album.clone()),
                ("duration".into(), t.duration.to_string()),
            ]);
            if s.is_playing && !self.listen.now_sent {
                // Now-playing failures are deliberately not queued or retried.
                self.listen.now_sent = true;
                if let Err(e) = self.lastfm("track.updateNowPlaying", p.clone(), true) {
                    self.status.lastfm = e.to_string();
                }
            }
            if eligible {
                let item = Scrobble {
                    artist: t.artist.clone(),
                    track: t.title.clone(),
                    album: t.album.clone(),
                    duration: t.duration,
                    timestamp: self.listen.started,
                    username: self.credentials.as_ref().unwrap().username.clone(),
                };
                let mut queue = self.queue.clone();
                queue.push(item);
                if queue.len() > 500 {
                    queue.remove(0);
                }
                match persist(&self.directory.join("lastfm-queue-v1.json"), &queue) {
                    Ok(()) => {
                        self.queue = queue;
                        self.listen.sent = true;
                    }
                    Err(_) => {
                        self.status.lastfm =
                            "Could not save listen history. Check available disk space.".into();
                        return;
                    }
                }
            }
        }
        if !self.queue.is_empty() && self.last_retry.elapsed() > Duration::from_secs(30) {
            self.last_retry = Instant::now();
            let item = self.queue[0].clone();
            if self.credentials.as_ref().unwrap().username != item.username {
                self.status.lastfm =
                    "Queued listens belong to another account. Disconnect to clear them.".into();
                return;
            }
            let p = BTreeMap::from([
                ("artist".into(), item.artist),
                ("track".into(), item.track),
                ("album".into(), item.album),
                ("duration".into(), item.duration.to_string()),
                ("timestamp".into(), item.timestamp.to_string()),
            ]);
            match self.lastfm("track.scrobble", p, true) {
                Ok(v) => {
                    let accepted = v["scrobbles"]["@attr"]["accepted"].as_str() == Some("1");
                    let ignored = v["scrobbles"]["@attr"]["ignored"].as_str() == Some("1");
                    if accepted || ignored {
                        let queue = self.queue[1..].to_vec();
                        if persist(&self.directory.join("lastfm-queue-v1.json"), &queue).is_ok() {
                            self.queue = queue;
                            self.status.lastfm = if accepted {
                                "Last listen scrobbled"
                            } else {
                                "Last.fm declined this listen"
                            }
                            .into();
                        }
                    } else {
                        self.status.lastfm =
                            "Last.fm did not confirm the listen; retained for retry.".into();
                    }
                }
                Err(e) => self.status.lastfm = e.to_string(),
            }
        }
        self.status.queued = self.queue.len();
    }
}
#[derive(Clone)]
pub struct IntegrationService {
    active: Arc<AtomicBool>,
    worker: Arc<Mutex<Worker>>,
    stopped: Arc<AtomicBool>,
}
impl IntegrationService {
    pub fn start(client: LocalPlaybackClient) -> Result<Self> {
        let directory = storage::desktop_data_dir();
        let config: Config = std::fs::read(directory.join("integrations-v1.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let queue: Vec<Scrobble> = std::fs::read(directory.join("lastfm-queue-v1.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let status = Status {
            config: config.clone(),
            discord: if config.discord_enabled {
                "Waiting for Discord…"
            } else {
                "Off"
            }
            .into(),
            lastfm: if config.lastfm_enabled {
                "Ready to scrobble"
            } else {
                "Off"
            }
            .into(),
            queued: queue.len(),
        };
        let (cover_sender, cover_receiver) = std::sync::mpsc::channel();
        let worker = Arc::new(Mutex::new(Worker {
            config,
            directory,
            credentials: None,
            token: None,
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(8))
                .user_agent("MusicOS/0.1 (desktop beta)")
                .build()?,
            discord: Discord::default(),
            status,
            queue,
            listen: Listen::default(),
            last_tick: Instant::now(),
            last_retry: Instant::now() - Duration::from_secs(30),
            discord_key: String::new(),
            discord_retry: Instant::now() - Duration::from_secs(20),
            covers: BTreeMap::new(),
            cover_sender,
            cover_receiver,
        }));
        let stopped = Arc::new(AtomicBool::new(false));
        let active=Arc::new(AtomicBool::new(false));
        let enabled=active.clone();
        let w = worker.clone();
        let stop = stopped.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if !enabled.load(Ordering::Relaxed){std::thread::sleep(Duration::from_millis(100));continue;}
                w.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .tick(client.snapshot());
                std::thread::sleep(Duration::from_millis(500));
            }
            w.lock().unwrap_or_else(|e| e.into_inner()).discord.clear();
        });
        Ok(Self { worker, stopped, active })
    }
    pub fn activate(&self){self.active.store(true,Ordering::Relaxed);}
    pub fn status(&self) -> Status {
        let mut w = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        w.response(None).status
    }
    pub fn action(&self, r: Request) -> Result<Response> {
        self.worker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .action(r)
    }
    pub fn shutdown(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}
#[derive(Default)]
struct Discord {
    #[cfg(unix)]
    socket: Option<std::os::unix::net::UnixStream>,
    nonce: u64,
}
impl Discord {
    #[cfg(unix)]
    fn exchange(&mut self, opcode: u32, value: Value) -> Result<Value> {
        use std::io::{Read, Write};
        let stream = self.socket.as_mut().context("Discord is not connected")?;
        let bytes = serde_json::to_vec(&value)?;
        stream.write_all(&opcode.to_le_bytes())?;
        stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
        stream.write_all(&bytes)?;
        for _ in 0..8 {
            let mut header = [0; 8];
            stream.read_exact(&mut header)?;
            let op = u32::from_le_bytes(header[..4].try_into()?);
            let length = u32::from_le_bytes(header[4..].try_into()?) as usize;
            if length > 65536 {
                bail!("Invalid Discord IPC frame");
            }
            let mut payload = vec![0; length];
            stream.read_exact(&mut payload)?;
            if op == 3 {
                stream.write_all(&4u32.to_le_bytes())?;
                stream.write_all(&(length as u32).to_le_bytes())?;
                stream.write_all(&payload)?;
                continue;
            }
            if op != 1 {
                bail!("Discord closed the connection");
            }
            let response: Value = serde_json::from_slice(&payload)?;
            if response["evt"] == "ERROR" {
                bail!("Discord rejected the activity");
            }
            if opcode == 0 && response["evt"] == "READY" || response["nonce"] == value["nonce"] {
                return Ok(response);
            }
        }
        bail!("Discord did not acknowledge the activity")
    }
    #[cfg(unix)]
    fn update(&mut self, id: &str, s: &LocalPlaybackSnapshot, cover: Option<&str>) -> Result<()> {
        if self.socket.is_none() {
            let mut paths = vec![std::env::temp_dir(), PathBuf::from("/tmp")];
            for name in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
                if let Some(p) = std::env::var_os(name) {
                    paths.push(p.into());
                }
            }
            'search: for p in paths {
                for i in 0..10 {
                    if let Ok(stream) =
                        std::os::unix::net::UnixStream::connect(p.join(format!("discord-ipc-{i}")))
                    {
                        stream.set_read_timeout(Some(Duration::from_millis(750)))?;
                        stream.set_write_timeout(Some(Duration::from_millis(750)))?;
                        self.socket = Some(stream);
                        break 'search;
                    }
                }
            }
            if self.socket.is_none() {
                bail!("Open Discord desktop");
            }
            self.exchange(0, json!({"v":1,"client_id":id}))?;
        }
        let activity = discord_activity(s, cover);
        self.nonce += 1;
        self.exchange(1,json!({"cmd":"SET_ACTIVITY","args":{"pid":std::process::id(),"activity":activity},"nonce":self.nonce.to_string()}))?;
        Ok(())
    }
    #[cfg(not(unix))]
    fn update(
        &mut self,
        _id: &str,
        _s: &LocalPlaybackSnapshot,
        _cover: Option<&str>,
    ) -> Result<()> {
        bail!("Discord presence is currently available on macOS and Linux.")
    }
    fn clear(&mut self) {
        #[cfg(unix)]
        if self.socket.is_some() {
            self.nonce += 1;
            let _=self.exchange(1,json!({"cmd":"SET_ACTIVITY","args":{"pid":std::process::id(),"activity":null},"nonce":self.nonce.to_string()}));
            self.socket = None;
        }
    }
}
fn discord_activity(s: &LocalPlaybackSnapshot, cover: Option<&str>) -> Value {
    if !s.is_playing {
        return Value::Null;
    }
    s.track.as_ref().map(|t|{
        let mut activity=json!({"type":2,"details":t.title.chars().take(128).collect::<String>(),"state":format!("{} · {}",t.artist,t.album).chars().take(128).collect::<String>(),"timestamps":{"start":now().saturating_sub(s.position as u64),"end":now().saturating_sub(s.position as u64)+t.duration.max(0) as u64}});
        if let Some(url)=cover.filter(|url|url.starts_with("https://")) {activity["assets"]=json!({"large_image":url,"large_text":t.album.chars().take(128).collect::<String>()});}
        activity
    }).unwrap_or(Value::Null)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn covers_use_album_artist_and_failed_lookups_expire(){let track=musicos_desktop::models::Track{artist:"Main; Guest".into(),..Default::default()};assert_eq!(discord_artist(&track),"Main");assert_eq!(discord_artist(&musicos_desktop::models::Track{album_artist:Some("Album artist".into()),..track}),"Album artist");assert!(!CoverLookup{url:None,attempts:1,checked:Instant::now(),pending:false}.due());assert!(CoverLookup{url:None,attempts:1,checked:Instant::now()-Duration::from_secs(61),pending:false}.due());assert!(CoverLookup{url:None,attempts:1,checked:Instant::now()-Duration::from_secs(46),pending:true}.due());}
    #[test]
    fn discord_artwork_is_included_and_pause_clears_activity() {
        use musicos_desktop::models::Track;
        let mut snapshot = LocalPlaybackSnapshot {
            is_playing: true,
            track: Some(Track {
                title: "Song".into(),
                album: "Album".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let activity = discord_activity(&snapshot, Some("https://is1-ssl.mzstatic.com/cover.jpg"));
        assert_eq!(activity["assets"]["large_text"], "Album");
        assert!(activity["assets"]["large_image"]
            .as_str()
            .unwrap()
            .starts_with("https://"));
        assert!(discord_activity(&snapshot, Some("file:///cover.png"))["assets"].is_null());
        snapshot.is_playing = false;
        assert!(discord_activity(&snapshot, None).is_null());
    }
    #[test]
    fn defaults_are_private_and_disabled() {
        let c: Config = serde_json::from_str("{}").unwrap();
        assert!(!c.discord_enabled && !c.lastfm_enabled);
        assert!(c.lastfm_username.is_none());
    }
    #[test]
    fn signature_is_sorted_and_excludes_transport_fields() {
        let p = BTreeMap::from([
            ("method".into(), "auth.getToken".into()),
            ("api_key".into(), "key".into()),
            ("format".into(), "json".into()),
        ]);
        assert_eq!(
            signature(&p, "secret"),
            format!("{:x}", md5::compute("api_keykeymethodauth.getTokensecret"))
        );
    }
    #[test]
    fn scrobble_thresholds() {
        assert_eq!(threshold(60), 30.);
        assert_eq!(threshold(1000), 240.);
    }
    #[test]
    fn atomic_config_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        persist(&p, &Config::default()).unwrap();
        let c: Config = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert!(!c.lastfm_enabled);
        assert!(!d.path().join("settings.pending").exists());
    }
    #[test]
    fn audible_overlap_is_carried_into_the_incoming_occurrence() {
        let mut listen = Listen::default();
        let mut snapshot = LocalPlaybackSnapshot {
            track: Some(musicos_desktop::models::Track {
                id: 1,
                duration: 120,
                ..Default::default()
            }),
            queue_index: Some(0),
            queue_entry_ids: vec![10, 11],
            is_playing: true,
            listened: Some(25.),
            ..Default::default()
        };
        listen.update(&snapshot, 0.5, 100);
        assert_eq!(listen.listened, 25.);
        snapshot.queue_index = Some(1);
        snapshot.position = 8.;
        snapshot.listened = Some(8.);
        listen.update(&snapshot, 0.5, 101);
        assert_eq!(listen.entry, Some(11));
        assert_eq!(listen.listened, 8.);
        snapshot.is_playing = false;
        listen.update(&snapshot, 100., 201);
        assert_eq!(listen.listened, 8.);
    }
    #[test]
    fn paused_time_is_not_listening() {
        let mut l = Listen {
            playing: false,
            listened: 12.,
            ..Default::default()
        };
        let s = LocalPlaybackSnapshot::default();
        l.update(&s, 100., 0);
        assert_eq!(l.listened, 12.);
    }
    #[test]
    fn listening_excludes_pauses_and_seeks_and_scrobbles_once() {
        use musicos_desktop::models::Track;
        let mut s = LocalPlaybackSnapshot {
            track: Some(Track {
                id: 1,
                duration: 60,
                ..Default::default()
            }),
            is_playing: true,
            ..Default::default()
        };
        let mut l = Listen::default();
        assert!(!l.update(&s, 1., 100));
        for n in 1..16 {
            s.position = n as f32;
            assert!(!l.update(&s, 1., 100 + n));
        }
        s.is_playing = false;
        l.update(&s, 1., 116);
        l.update(&s, 100., 216);
        assert_eq!(l.listened, 15.);
        s.is_playing = true;
        s.position = 50.;
        assert!(!l.update(&s, 1., 217));
        assert_eq!(l.listened, 15.);
        for _ in 0..14 {
            assert!(!l.update(&s, 1., 218));
        }
        assert!(l.update(&s, 1., 219));
        l.sent = true;
        assert!(!l.update(&s, 1., 220));
        s.track.as_mut().unwrap().id = 2;
        s.position = 0.;
        assert!(!l.update(&s, 1., 221));
        assert_eq!(l.listened, 0.);
        assert!(!l.sent);
    }
    #[cfg(unix)]
    #[test]
    fn discord_ipc_handshake_ping_ack_and_frame_limits() {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        let (client, mut server) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let peer = std::thread::spawn(move || {
            fn read(s: &mut UnixStream) -> Value {
                let mut h = [0; 8];
                s.read_exact(&mut h).unwrap();
                let size = u32::from_le_bytes(h[4..].try_into().unwrap());
                let mut b = vec![0; size as usize];
                s.read_exact(&mut b).unwrap();
                serde_json::from_slice(&b).unwrap()
            }
            fn send(s: &mut UnixStream, op: u32, v: Value) {
                let b = serde_json::to_vec(&v).unwrap();
                s.write_all(&op.to_le_bytes()).unwrap();
                s.write_all(&(b.len() as u32).to_le_bytes()).unwrap();
                s.write_all(&b).unwrap();
            }
            assert_eq!(read(&mut server)["client_id"], "123456789012345678");
            send(&mut server, 3, json!({"ping":true}));
            assert_eq!(read(&mut server)["ping"], true);
            send(&mut server, 1, json!({"evt":"READY"}));
            let activity = read(&mut server);
            assert_eq!(activity["cmd"], "SET_ACTIVITY");
            send(&mut server, 1, json!({"nonce":activity["nonce"]}));
            read(&mut server);
            server.write_all(&1u32.to_le_bytes()).unwrap();
            server.write_all(&65537u32.to_le_bytes()).unwrap();
        });
        let mut discord = Discord {
            socket: Some(client),
            nonce: 0,
        };
        discord
            .exchange(0, json!({"v":1,"client_id":"123456789012345678"}))
            .unwrap();
        discord
            .exchange(1, json!({"cmd":"SET_ACTIVITY","nonce":"1"}))
            .unwrap();
        assert!(discord.exchange(1, json!({"nonce":"2"})).is_err());
        peer.join().unwrap();
    }
}
