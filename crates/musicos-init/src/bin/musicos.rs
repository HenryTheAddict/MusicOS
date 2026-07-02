use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use musicos_init::audio::{EqPreset, LoopMode};
use musicos_init::database::Db;
use musicos_init::preferences::{self, LibraryGroup, LibrarySort, MotionMode, Preferences};
use musicos_init::ui::theme::ThemeName;
use musicos_init::{media, usb};
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    name = "musicos",
    version,
    about = "MusicOS library, device, sync, update, and settings CLI"
)]
#[command(
    after_help = "Examples:\n  musicos scan --all\n  musicos library stats --json\n  musicos playlists create Roadtrip\n  musicos settings set device-name MusicOS\n  musicos doctor --json"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Print machine-readable JSON where supported"
    )]
    json: bool,
    #[arg(long, global = true, help = "Print extra diagnostic details")]
    verbose: bool,
    #[arg(
        long,
        global = true,
        env = "MUSICOS_MEDIA_DB",
        default_value = "out/desktop-media.sqlite3"
    )]
    db: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Scan(ScanArgs),
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
    Playlists {
        #[command(subcommand)]
        command: PlaylistCommand,
    },
    Devices {
        #[command(subcommand)]
        command: DeviceCommand,
    },
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
    Updates {
        #[command(subcommand)]
        command: UpdateCommand,
    },
    Settings {
        #[command(subcommand)]
        command: SettingsCommand,
    },
    Doctor,
}

#[derive(Args)]
struct ScanArgs {
    #[arg(long, help = "Force a full metadata and cover-art refresh")]
    all: bool,
    #[arg(help = "Optional folder to index instead of default MusicOS roots")]
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum LibraryCommand {
    List,
    Stats,
    Add { path: PathBuf },
}

#[derive(Subcommand)]
enum PlaylistCommand {
    List,
    Create { name: String },
    Delete { name_or_id: String },
    AddTrack { playlist_id: i64, track_id: i64 },
}

#[derive(Subcommand)]
enum DeviceCommand {
    List,
    Status,
    Eject { device_id: String },
}

#[derive(Subcommand)]
enum SyncCommand {
    Plan {
        device_id: String,
    },
    Start {
        device_id: String,
        #[arg(long)]
        playlist: Option<i64>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum UpdateCommand {
    Scan,
    Stage { path: PathBuf },
    Install,
}

#[derive(Subcommand)]
enum SettingsCommand {
    Get,
    Set { key: String, value: String },
}

#[derive(Serialize)]
struct SyncPlan {
    device_id: String,
    source: String,
    files: usize,
    bytes: u64,
    dry_run: bool,
    writes: bool,
}

#[derive(Serialize)]
struct SyncResult {
    device_id: String,
    target: String,
    source: String,
    files: usize,
    uploaded: usize,
    skipped: usize,
    failed: usize,
    errors: Vec<String>,
    dry_run: bool,
    writes: bool,
}

#[derive(Serialize)]
struct DoctorReport {
    ok: bool,
    db_path: String,
    db_ok: bool,
    settings_path: String,
    settings_ok: bool,
    music_roots: Vec<String>,
    usb_roots_env: Option<String>,
    audio_backend: String,
    issues: Vec<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(exit_code_for(&err.to_string()));
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let db = Db::open(&cli.db).with_context(|| format!("opening {}", cli.db))?;
    let json = cli.json;
    let db_path = cli.db.clone();

    match cli.command {
        Command::Scan(args) => {
            if args.all {
                std::env::set_var("MUSICOS_REINDEX_ALL", "1");
            }
            let indexed = if let Some(path) = args.path {
                media::index_location(&db, &path)?
            } else if args.all {
                media::reindex_default_locations_with_progress(&db, |_| {})?
            } else {
                media::index_default_locations(&db)?
            };
            print_value(
                json,
                &serde_json::json!({ "indexed_or_updated": indexed, "db": db_path }),
                || {
                    println!("indexed_or_updated={indexed} db={db_path}");
                    Ok(())
                },
            )
        }
        Command::Library { command } => match command {
            LibraryCommand::List => print_value(json, &db.get_tracks()?, || {
                for track in db.get_tracks()? {
                    println!(
                        "{}\t{}\t{}\t{}",
                        track.id, track.title, track.artist, track.album
                    );
                }
                Ok(())
            }),
            LibraryCommand::Stats => print_value(json, &db.library_counts()?, || {
                let counts = db.library_counts()?;
                println!(
                    "tracks={} albums={} artists={} playlists={} favorites={}",
                    counts.tracks,
                    counts.albums,
                    counts.artists,
                    counts.playlists,
                    counts.favorites
                );
                Ok(())
            }),
            LibraryCommand::Add { path } => {
                let indexed = media::index_location(&db, &path)?;
                print_value(
                    json,
                    &serde_json::json!({ "indexed_or_updated": indexed, "path": path }),
                    || {
                        println!("indexed_or_updated={indexed} path={}", path.display());
                        Ok(())
                    },
                )
            }
        },
        Command::Playlists { command } => match command {
            PlaylistCommand::List => print_value(json, &db.get_playlists()?, || {
                for playlist in db.get_playlists()? {
                    println!("{}\t{}", playlist.id, playlist.name);
                }
                Ok(())
            }),
            PlaylistCommand::Create { name } => {
                let id = db.create_playlist(&name)?;
                print_value(json, &serde_json::json!({ "id": id, "name": name }), || {
                    println!("created playlist {id}: {name}");
                    Ok(())
                })
            }
            PlaylistCommand::Delete { name_or_id } => {
                let id = resolve_playlist(&db, &name_or_id)?;
                db.delete_playlist(id)?;
                print_value(json, &serde_json::json!({ "deleted": id }), || {
                    println!("deleted playlist {id}");
                    Ok(())
                })
            }
            PlaylistCommand::AddTrack {
                playlist_id,
                track_id,
            } => {
                db.add_track_to_playlist(playlist_id, track_id)?;
                print_value(
                    json,
                    &serde_json::json!({ "playlist_id": playlist_id, "track_id": track_id }),
                    || {
                        println!("added track {track_id} to playlist {playlist_id}");
                        Ok(())
                    },
                )
            }
        },
        Command::Devices { command } => match command {
            DeviceCommand::List | DeviceCommand::Status => {
                let status = serde_json::json!({
                    "devices": [{
                        "id": "desktop",
                        "name": preferences::load().device_name,
                        "kind": "desktop-runtime",
                        "db": db_path,
                        "sync_target": std::env::var("MUSICOS_SYNC_TARGET").unwrap_or_else(|_| "http://127.0.0.1:18080".to_string())
                    }]
                });
                print_value(json, &status, || {
                    println!("desktop\t{}\t{}", preferences::load().device_name, db_path);
                    Ok(())
                })
            }
            DeviceCommand::Eject { device_id } => {
                bail!("device not found or eject unsupported: {device_id}");
            }
        },
        Command::Sync { command } => match command {
            SyncCommand::Plan { device_id } => print_sync_plan(json, device_id, true),
            SyncCommand::Start {
                device_id,
                playlist,
                dry_run,
            } => {
                if let Some(playlist_id) = playlist {
                    let tracks = db.get_playlist_tracks(playlist_id)?;
                    let bytes = tracks
                        .iter()
                        .map(|track| track.file_size.max(0) as u64)
                        .sum();
                    if dry_run {
                        let plan = SyncPlan {
                            device_id,
                            source: format!("playlist:{playlist_id}"),
                            files: tracks.len(),
                            bytes,
                            dry_run: true,
                            writes: false,
                        };
                        print_value(json, &plan, || {
                            println!(
                                "dry_run playlist={playlist_id} files={} bytes={}",
                                plan.files, plan.bytes
                            );
                            Ok(())
                        })
                    } else {
                        let files = tracks
                            .into_iter()
                            .map(|track| PathBuf::from(track.path))
                            .collect();
                        sync_to_target(
                            json,
                            device_id,
                            format!("playlist:{playlist_id}"),
                            files,
                            None,
                        )
                    }
                } else {
                    let source =
                        std::env::var("MUSICOS_SYNC_DIR").unwrap_or_else(|_| "./music".to_string());
                    let files = collect_audio_files(Path::new(&source));
                    if dry_run {
                        print_sync_plan(json, device_id, true)
                    } else {
                        sync_to_target(
                            json,
                            device_id,
                            source.clone(),
                            files,
                            Some(PathBuf::from(source)),
                        )
                    }
                }
            }
        },
        Command::Updates { command } => match command {
            UpdateCommand::Scan => {
                let status = usb::sync_from_usb(&db)?;
                print_value(json, &status, || {
                    println!("{}", status.message);
                    Ok(())
                })
            }
            UpdateCommand::Stage { path } => {
                let root = path.parent().unwrap_or_else(|| Path::new("."));
                std::env::set_var("MUSICOS_USB_ROOTS", root);
                let status = usb::sync_from_usb(&db)?;
                print_value(json, &status, || {
                    println!("{}", status.message);
                    Ok(())
                })
            }
            UpdateCommand::Install => {
                let installed = usb::install_pending_update()?;
                print_value(json, &serde_json::json!({ "installed": installed }), || {
                    match installed {
                        Some(path) => println!("installed {}", path.display()),
                        None => println!("no staged update found"),
                    }
                    Ok(())
                })
            }
        },
        Command::Settings { command } => match command {
            SettingsCommand::Get => {
                let settings = preferences::load();
                print_value(json, &settings, || {
                    println!("user-name={}", settings.user_name);
                    println!("device-name={}", settings.device_name);
                    println!("theme={}", settings.theme.to_str());
                    println!("audio-backend={}", settings.audio_backend);
                    println!("alsa-device={}", settings.alsa_device);
                    println!("volume={}", settings.volume);
                    println!("brightness={}", settings.brightness);
                    Ok(())
                })
            }
            SettingsCommand::Set { key, value } => {
                let mut settings = preferences::load();
                apply_setting(&mut settings, &key, &value)?;
                preferences::save(&settings)?;
                print_value(
                    json,
                    &serde_json::json!({ "key": key, "value": value, "settings_path": preferences::path() }),
                    || {
                        println!("saved {key}={value}");
                        Ok(())
                    },
                )
            }
        },
        Command::Doctor => {
            let report = doctor_report(&db_path);
            print_value(json, &report, || {
                println!("db={} ok={}", report.db_path, report.db_ok);
                println!(
                    "settings={} ok={}",
                    report.settings_path, report.settings_ok
                );
                println!("audio-backend={}", report.audio_backend);
                for issue in &report.issues {
                    println!("issue: {issue}");
                }
                Ok(())
            })
        }
    }
}

fn print_sync_plan(json: bool, device_id: String, dry_run: bool) -> Result<()> {
    let source = std::env::var("MUSICOS_SYNC_DIR").unwrap_or_else(|_| "./music".to_string());
    let (files, bytes) = collect_audio_stats(Path::new(&source));
    let plan = SyncPlan {
        device_id,
        source,
        files,
        bytes,
        dry_run,
        writes: false,
    };
    print_value(json, &plan, || {
        println!(
            "{} source={} files={} bytes={} writes=false",
            if dry_run { "dry_run" } else { "plan" },
            plan.source,
            plan.files,
            plan.bytes
        );
        Ok(())
    })
}

fn sync_to_target(
    json: bool,
    device_id: String,
    source: String,
    files: Vec<PathBuf>,
    relative_root: Option<PathBuf>,
) -> Result<()> {
    let target = HttpTarget::for_device(&device_id)?;
    let mut result = SyncResult {
        device_id,
        target: target.label(),
        source,
        files: files.len(),
        uploaded: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
        dry_run: false,
        writes: true,
    };

    for file in files {
        match upload_file(&target, relative_root.as_deref(), &file) {
            Ok(UploadOutcome::Uploaded) => result.uploaded += 1,
            Ok(UploadOutcome::Skipped) => result.skipped += 1,
            Err(err) => {
                result.failed += 1;
                result.errors.push(format!("{}: {err:#}", file.display()));
            }
        }
    }

    if result.failed == 0 {
        target.post_json("/api/control", r#"{"command":"scan_library"}"#)?;
    }

    print_value(json, &result, || {
        println!(
            "sync target={} files={} uploaded={} skipped={} failed={}",
            result.target, result.files, result.uploaded, result.skipped, result.failed
        );
        for error in &result.errors {
            println!("error: {error}");
        }
        Ok(())
    })?;

    if result.failed > 0 {
        bail!("sync failed for {} file(s)", result.failed);
    }
    Ok(())
}

enum UploadOutcome {
    Uploaded,
    Skipped,
}

fn upload_file(
    target: &HttpTarget,
    relative_root: Option<&Path>,
    path: &Path,
) -> Result<UploadOutcome> {
    let body = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let metadata = std::fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    let sha256 = hex_sha256(&body);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload.ogg")
        .to_string();
    let relative_path = relative_root
        .and_then(|root| path.strip_prefix(root).ok())
        .and_then(|path| path.to_str())
        .unwrap_or(filename.as_str())
        .replace('\\', "/");
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|| "0".to_string());
    let response = target.post(
        "/api/upload",
        body,
        vec![
            ("X-Filename".to_string(), header_value(&filename)),
            ("X-Relative-Path".to_string(), header_value(&relative_path)),
            ("X-Sha256".to_string(), sha256),
            ("X-File-Size".to_string(), metadata.len().to_string()),
            ("X-Modified-At".to_string(), modified_at),
            ("X-Trigger-Scan".to_string(), "0".to_string()),
        ],
    )?;
    let status = serde_json::from_slice::<serde_json::Value>(&response)
        .ok()
        .and_then(|value| {
            value
                .get("status")
                .and_then(|status| status.as_str())
                .map(str::to_string)
        });
    if status.as_deref() == Some("skipped") {
        Ok(UploadOutcome::Skipped)
    } else {
        Ok(UploadOutcome::Uploaded)
    }
}

fn collect_audio_stats(root: &Path) -> (usize, u64) {
    let files = collect_audio_files(root);
    let mut count = 0;
    let mut bytes = 0;
    for file in files {
        count += 1;
        bytes += std::fs::metadata(file)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
    }
    (count, bytes)
}

fn collect_audio_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && is_audio(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    files
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "flac" | "m4a" | "ogg" | "opus" | "aac"
            )
        })
        .unwrap_or(false)
}

fn resolve_playlist(db: &Db, name_or_id: &str) -> Result<i64> {
    if let Ok(id) = name_or_id.parse::<i64>() {
        return Ok(id);
    }
    db.get_playlists()?
        .into_iter()
        .find(|playlist| playlist.name.eq_ignore_ascii_case(name_or_id))
        .map(|playlist| playlist.id)
        .with_context(|| format!("playlist not found: {name_or_id}"))
}

fn apply_setting(settings: &mut Preferences, key: &str, value: &str) -> Result<()> {
    match key.replace('_', "-").as_str() {
        "user-name" => settings.user_name = value.to_string(),
        "device-name" => settings.device_name = value.to_string(),
        "theme" => settings.theme = ThemeName::from_str(value),
        "eq" | "eq-preset" => {
            settings.eq_preset = match value.to_ascii_lowercase().as_str() {
                "bass" => EqPreset::Bass,
                "vocal" => EqPreset::Vocal,
                "treble" => EqPreset::Treble,
                "night" => EqPreset::Night,
                _ => EqPreset::Flat,
            }
        }
        "audio-backend" => settings.audio_backend = value.to_string(),
        "alsa-device" => settings.alsa_device = value.to_string(),
        "brightness" => {
            settings.brightness = value.parse::<u8>().context("brightness must be 0-100")?
        }
        "volume" => settings.volume = value.parse::<f32>().context("volume must be 0.0-1.0")?,
        "shuffle" => settings.shuffle = parse_bool(value)?,
        "loop-mode" => {
            settings.loop_mode = match value.to_ascii_lowercase().as_str() {
                "all" => LoopMode::All,
                "one" => LoopMode::One,
                _ => LoopMode::None,
            }
        }
        "library-sort" => settings.library_sort = LibrarySort::from_str(value),
        "library-group" | "library-grouping" => {
            settings.library_group = LibraryGroup::from_str(value)
        }
        "motion-mode" => settings.motion_mode = MotionMode::from_str(value),
        other => bail!("unknown setting key: {other}"),
    }
    *settings = settings.clone().sanitized();
    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => bail!("expected boolean value"),
    }
}

#[derive(Clone)]
struct HttpTarget {
    host: String,
    port: u16,
}

impl HttpTarget {
    fn for_device(device_id: &str) -> Result<Self> {
        if device_id.eq_ignore_ascii_case("desktop") {
            let target = std::env::var("MUSICOS_SYNC_TARGET")
                .unwrap_or_else(|_| "http://127.0.0.1:18080".to_string());
            return Self::parse(&target);
        }
        if device_id.starts_with("http://") {
            return Self::parse(device_id);
        }
        bail!("device not found or unsupported sync target: {device_id}")
    }

    fn parse(raw: &str) -> Result<Self> {
        let without_scheme = raw
            .strip_prefix("http://")
            .ok_or_else(|| anyhow::anyhow!("only http:// sync targets are supported"))?;
        let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_string(), port.parse()?),
            None => (authority.to_string(), 80),
        };
        if host.is_empty() {
            bail!("empty sync target host");
        }
        Ok(Self { host, port })
    }

    fn label(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    fn post_json(&self, path: &str, body: &str) -> Result<Vec<u8>> {
        self.post(
            path,
            body.as_bytes().to_vec(),
            vec![("Content-Type".to_string(), "application/json".to_string())],
        )
    }

    fn post(&self, path: &str, body: Vec<u8>, headers: Vec<(String, String)>) -> Result<Vec<u8>> {
        self.request("POST", path, body, headers)
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
    ) -> Result<Vec<u8>> {
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .with_context(|| format!("connecting to {}", self.label()))?;
        stream.set_read_timeout(Some(Duration::from_secs(6)))?;
        stream.set_write_timeout(Some(Duration::from_secs(6)))?;

        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
            self.host,
            body.len()
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes())?;
        if !body.is_empty() {
            stream.write_all(&body)?;
        }

        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;
        parse_http_response(&response)
    }
}

fn parse_http_response(response: &[u8]) -> Result<Vec<u8>> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("bad HTTP response"))?;
    let header = std::str::from_utf8(&response[..header_end])?;
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        bail!("HTTP {status}");
    }
    Ok(response[header_end + 4..].to_vec())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn header_value(value: &str) -> String {
    value.replace(['\r', '\n'], "_")
}

fn doctor_report(db_path: &str) -> DoctorReport {
    let db_ok = Db::open(db_path).is_ok();
    let settings = preferences::load();
    let settings_path = preferences::path();
    let settings_ok = settings_path
        .parent()
        .map(|path| path.exists())
        .unwrap_or(false);
    let mut issues = Vec::new();
    if !db_ok {
        issues.push("database could not be opened".to_string());
    }
    if !settings_ok {
        issues.push("settings directory does not exist yet".to_string());
    }
    DoctorReport {
        ok: issues.is_empty(),
        db_path: db_path.to_string(),
        db_ok,
        settings_path: settings_path.display().to_string(),
        settings_ok,
        music_roots: media::last_scan_roots(),
        usb_roots_env: std::env::var("MUSICOS_USB_ROOTS").ok(),
        audio_backend: settings.audio_backend,
        issues,
    }
}

fn print_value<T: Serialize>(
    json: bool,
    value: &T,
    text: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
        Ok(())
    } else {
        text()
    }
}

fn exit_code_for(message: &str) -> i32 {
    if message.contains("not found") {
        3
    } else if message.contains("settings") || message.contains("database") {
        5
    } else if message.contains("sync") {
        4
    } else {
        1
    }
}
