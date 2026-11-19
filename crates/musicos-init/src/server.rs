use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tiny_http::{Header, Response, Server, StatusCode};

use crate::audio::PlaybackStatus;
use crate::database::{LibraryCounts, MetadataUpdate, Track};
use crate::input::{Button, InputEvent, SwipeDirection};
use crate::preferences::Preferences;
use crate::renderer;

const DEFAULT_MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;

pub enum ServerEvent {
    Button(Button),
    Pointer(InputEvent),
    Control(ControlCommand),
    UploadComplete,
    UsbSync,
    ShowControls,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub user_name: Option<String>,
    pub device_name: Option<String>,
    pub theme: Option<String>,
    pub language: Option<String>,
    pub audio_backend: Option<String>,
    pub alsa_device: Option<String>,
    pub audio_output_mode: Option<String>,
    pub brightness: Option<u8>,
    pub volume: Option<f32>,
    pub eq_preset: Option<String>,
    pub shuffle: Option<bool>,
    pub loop_mode: Option<String>,
    pub library_sort: Option<String>,
    pub library_group: Option<String>,
    pub sleep_timeout_minutes: Option<u8>,
    pub dim_timeout_seconds: Option<u16>,
    pub motion_mode: Option<String>,
    pub seek_step_seconds: Option<u8>,
    pub sleep_timer_default_minutes: Option<u8>,
    pub resume_queue_enabled: Option<bool>,
    pub shuffle_default_enabled: Option<bool>,
    pub first_run_audio_test_complete: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ControlCommand {
    PlayPause,
    Next,
    Prev,
    Volume {
        value: f32,
    },
    Brightness {
        value: u8,
    },
    Seek {
        value: f32,
    },
    SeekRelative {
        value: f32,
    },
    SeekAbsolute {
        value: f32,
    },
    SleepTimer {
        minutes: u8,
    },
    Shuffle,
    Loop,
    PlayTrack {
        id: i64,
    },
    PlayTracks {
        ids: Vec<i64>,
        start_index: usize,
    },
    PlayPlaylist {
        id: i64,
        start_index: usize,
    },
    PlayMix {
        id: String,
        start_index: Option<usize>,
    },
    SetTheme {
        name: String,
    },
    SetLanguage {
        name: String,
    },
    ToggleFavorite {
        id: i64,
    },
    SetRating {
        id: i64,
        rating: u8,
    },
    DeletePlaylist {
        id: i64,
    },
    RenamePlaylist {
        id: i64,
        name: String,
    },
    CreatePlaylist {
        name: String,
    },
    AddTrackToPlaylist {
        playlist_id: i64,
        track_id: i64,
    },
    RemoveTrackFromPlaylist {
        playlist_id: i64,
        track_id: i64,
    },
    SetSettings {
        settings: SettingsUpdate,
    },
    ResetSettings,
    ScanLibrary,
    RescanLibrary {
        mode: String,
        #[serde(default)]
        source_id: Option<i64>,
    },
    AudioTest,
    QueueAdd {
        id: i64,
        position: Option<String>,
    },
    QueueRemove {
        index: usize,
    },
    QueueMove {
        from: usize,
        to: usize,
    },
    QueueSelect {
        index: usize,
    },
    QueueClear,
    CleanupMissingFiles,
}

#[derive(Deserialize)]
struct PlaylistPayload {
    name: String,
}

#[derive(Deserialize)]
struct PlaylistTrackPayload {
    track_id: i64,
}

#[derive(Deserialize)]
struct PlaylistReorderPayload {
    track_ids: Vec<i64>,
}

#[derive(Deserialize)]
struct RescanPayload {
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LibrarySourcePayload {
    path: String,
    #[serde(default = "default_reference_kind")]
    kind: String,
    #[serde(default)]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct LibrarySourceUpdatePayload {
    display_name: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MetadataPayload {
    ids: Vec<i64>,
    #[serde(default)]
    write_to_file: bool,
    #[serde(flatten)]
    update: MetadataUpdate,
}

fn default_reference_kind() -> String {
    "reference".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistSyncSnapshot {
    pub stable_id: String,
    pub name: String,
    pub revision: i64,
    pub track_ids: Vec<String>,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Deserialize)]
struct PlaylistReconcilePayload {
    base: PlaylistSyncSnapshot,
    desktop: PlaylistSyncSnapshot,
    device: PlaylistSyncSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistSyncConflict {
    pub field: String,
    pub desktop: serde_json::Value,
    pub device: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistReconcileResult {
    pub merged: PlaylistSyncSnapshot,
    pub conflicts: Vec<PlaylistSyncConflict>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanInfo {
    pub state: String,
    pub indexed: usize,
    pub processed: usize,
    pub discovered: usize,
    pub current_root: Option<String>,
    pub elapsed_ms: u128,
    pub last_error: Option<String>,
}

impl Default for ScanInfo {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            indexed: 0,
            processed: 0,
            discovered: 0,
            current_root: None,
            elapsed_ms: 0,
            last_error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BuildVersion {
    pub app: &'static str,
    pub git: &'static str,
    pub build_profile: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeLogEntry {
    pub at: u64,
    pub source: String,
    pub line: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FrameTimingSummary {
    pub count: usize,
    pub avg_ms: f32,
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub max_ms: f32,
    pub slow_frames: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeSnapshot {
    pub settings: Preferences,
    pub player: PlaybackStatus,
    pub queue: Vec<Track>,
    pub queue_index: Option<usize>,
    pub counts: LibraryCounts,
    pub db_path: String,
    pub settings_path: String,
    pub scan_status: String,
    pub scan: ScanInfo,
    pub usb_status: String,
    pub battery_status: String,
    pub indexed_roots: Vec<String>,
    pub cover_art_cache_count: i64,
    pub cover_art_memory_cache_count: usize,
    pub frame_ms: f32,
}

static RUNTIME_SNAPSHOT: OnceLock<Mutex<Option<RuntimeSnapshot>>> = OnceLock::new();
static RUNTIME_LOGS: OnceLock<Mutex<VecDeque<RuntimeLogEntry>>> = OnceLock::new();
static FRAME_SAMPLES: OnceLock<Mutex<VecDeque<f32>>> = OnceLock::new();
const MAX_RUNTIME_LOGS: usize = 300;
const MAX_FRAME_SAMPLES: usize = 240;

pub fn publish_runtime_snapshot(snapshot: RuntimeSnapshot) {
    record_frame_sample(snapshot.frame_ms);
    let lock = RUNTIME_SNAPSHOT.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = lock.lock() {
        *current = Some(snapshot);
    }
}

fn record_frame_sample(frame_ms: f32) {
    if !frame_ms.is_finite() || frame_ms <= 0.0 {
        return;
    }
    let lock = FRAME_SAMPLES.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Ok(mut samples) = lock.lock() {
        samples.push_back(frame_ms);
        while samples.len() > MAX_FRAME_SAMPLES {
            samples.pop_front();
        }
    }
}

fn frame_timing_summary() -> FrameTimingSummary {
    let mut samples: Vec<f32> = FRAME_SAMPLES
        .get()
        .and_then(|lock| {
            lock.lock()
                .ok()
                .map(|samples| samples.iter().copied().collect())
        })
        .unwrap_or_default();
    if samples.is_empty() {
        return FrameTimingSummary::default();
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = samples.len();
    let sum: f32 = samples.iter().sum();
    let percentile = |pct: f32| -> f32 {
        let idx = ((count.saturating_sub(1)) as f32 * pct).round() as usize;
        samples[idx.min(count.saturating_sub(1))]
    };
    FrameTimingSummary {
        count,
        avg_ms: sum / count as f32,
        p50_ms: percentile(0.50),
        p95_ms: percentile(0.95),
        max_ms: *samples.last().unwrap_or(&0.0),
        slow_frames: samples.iter().filter(|value| **value > 24.0).count(),
    }
}

fn runtime_snapshot() -> Option<RuntimeSnapshot> {
    RUNTIME_SNAPSHOT
        .get()
        .and_then(|lock| lock.lock().ok().and_then(|snapshot| snapshot.clone()))
}

fn playback_revision(snapshot: &RuntimeSnapshot) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    snapshot
        .player
        .track
        .as_ref()
        .map(|track| track.id)
        .hash(&mut hasher);
    snapshot.player.is_playing.hash(&mut hasher);
    (snapshot.player.position.max(0.0) as u64).hash(&mut hasher);
    snapshot.player.queue_index.hash(&mut hasher);
    snapshot.player.backend.hash(&mut hasher);
    snapshot.player.last_error.hash(&mut hasher);
    snapshot
        .queue
        .iter()
        .map(|track| track.id)
        .collect::<Vec<_>>()
        .hash(&mut hasher);
    hasher.finish()
}

pub fn reconcile_playlist_snapshots(
    base: &PlaylistSyncSnapshot,
    desktop: &PlaylistSyncSnapshot,
    device: &PlaylistSyncSnapshot,
) -> PlaylistReconcileResult {
    let mut conflicts = Vec::new();
    let name = if desktop.name == device.name {
        desktop.name.clone()
    } else if desktop.name == base.name {
        device.name.clone()
    } else if device.name == base.name {
        desktop.name.clone()
    } else {
        conflicts.push(PlaylistSyncConflict {
            field: "name".to_string(),
            desktop: serde_json::json!(desktop.name),
            device: serde_json::json!(device.name),
        });
        desktop.name.clone()
    };

    let deleted = if desktop.deleted == device.deleted {
        desktop.deleted
    } else if desktop.deleted == base.deleted {
        device.deleted
    } else if device.deleted == base.deleted {
        desktop.deleted
    } else {
        conflicts.push(PlaylistSyncConflict {
            field: "deleted".to_string(),
            desktop: serde_json::json!(desktop.deleted),
            device: serde_json::json!(device.deleted),
        });
        desktop.deleted
    };
    let desktop_edited = desktop.name != base.name || desktop.track_ids != base.track_ids;
    let device_edited = device.name != base.name || device.track_ids != base.track_ids;
    if desktop.deleted != device.deleted
        && ((desktop.deleted && device_edited) || (device.deleted && desktop_edited))
    {
        conflicts.push(PlaylistSyncConflict {
            field: "delete-versus-edit".to_string(),
            desktop: serde_json::json!({
                "deleted": desktop.deleted,
                "name": desktop.name,
                "track_ids": desktop.track_ids
            }),
            device: serde_json::json!({
                "deleted": device.deleted,
                "name": device.name,
                "track_ids": device.track_ids
            }),
        });
    }

    let desktop_changed_order = desktop.track_ids != base.track_ids;
    let device_changed_order = device.track_ids != base.track_ids;
    let track_ids = if desktop.track_ids == device.track_ids {
        desktop.track_ids.clone()
    } else if !desktop_changed_order {
        device.track_ids.clone()
    } else if !device_changed_order {
        desktop.track_ids.clone()
    } else {
        let base_set = base
            .track_ids
            .iter()
            .collect::<std::collections::HashSet<_>>();
        let desktop_set = desktop
            .track_ids
            .iter()
            .collect::<std::collections::HashSet<_>>();
        let device_set = device
            .track_ids
            .iter()
            .collect::<std::collections::HashSet<_>>();
        let desktop_removed = base_set
            .difference(&desktop_set)
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let device_removed = base_set
            .difference(&device_set)
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let order_only_conflict =
            desktop_set == device_set && desktop.track_ids != device.track_ids;
        if order_only_conflict {
            conflicts.push(PlaylistSyncConflict {
                field: "order".to_string(),
                desktop: serde_json::json!(desktop.track_ids),
                device: serde_json::json!(device.track_ids),
            });
            desktop.track_ids.clone()
        } else {
            let mut merged = Vec::new();
            for id in desktop.track_ids.iter().chain(device.track_ids.iter()) {
                let removed_by_other = (desktop_removed.contains(id) && device_set.contains(id))
                    || (device_removed.contains(id) && desktop_set.contains(id));
                if !removed_by_other && !merged.contains(id) {
                    merged.push(id.clone());
                }
            }
            merged
        }
    };

    PlaylistReconcileResult {
        merged: PlaylistSyncSnapshot {
            stable_id: desktop.stable_id.clone(),
            name,
            revision: desktop.revision.max(device.revision) + 1,
            track_ids,
            deleted,
        },
        conflicts,
    }
}

pub fn push_runtime_log(source: &str, line: impl Into<String>) {
    let lock = RUNTIME_LOGS.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Ok(mut logs) = lock.lock() {
        logs.push_back(RuntimeLogEntry {
            at: unix_seconds(),
            source: source.to_string(),
            line: line.into(),
        });
        while logs.len() > MAX_RUNTIME_LOGS {
            logs.pop_front();
        }
    }
}

pub fn recent_runtime_logs(limit: usize) -> Vec<RuntimeLogEntry> {
    RUNTIME_LOGS
        .get()
        .and_then(|lock| {
            lock.lock().ok().map(|logs| {
                let take = limit.min(logs.len());
                logs.iter()
                    .skip(logs.len().saturating_sub(take))
                    .cloned()
                    .collect()
            })
        })
        .unwrap_or_default()
}

pub fn build_version() -> BuildVersion {
    BuildVersion {
        app: env!("CARGO_PKG_VERSION"),
        git: option_env!("MUSICOS_GIT_SHA").unwrap_or("dev"),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub struct CompanionServer {
    server: Server,
    tx: EventSender,
    db_path: String,
}

struct EventSender {
    tx: Sender<ServerEvent>,
    wake: Option<crate::input::WakeSignal>,
}

#[cfg(test)]
mod event_sender_tests {
    use super::*;

    #[test]
    fn sends_events_and_reports_a_disconnected_receiver() {
        let (tx, rx) = std::sync::mpsc::channel();
        let sender = EventSender {
            tx,
            wake: Some(crate::input::WakeSignal::new().unwrap()),
        };
        assert!(sender.send(ServerEvent::ShowControls).is_ok());
        assert!(matches!(rx.try_recv(), Ok(ServerEvent::ShowControls)));
        drop(rx);
        assert!(sender.send(ServerEvent::ShowControls).is_err());
    }
}

impl EventSender {
    fn send(&self, event: ServerEvent) -> Result<(), std::sync::mpsc::SendError<ServerEvent>> {
        self.tx.send(event)?;
        if let Some(wake) = &self.wake {
            wake.signal();
        }
        Ok(())
    }
}

impl CompanionServer {
    pub fn start(port: u16, tx: Sender<ServerEvent>, db_path: String) -> Result<()> {
        Self::start_with_wake(port, tx, db_path, None)
    }

    pub fn start_with_wake(
        port: u16,
        tx: Sender<ServerEvent>,
        db_path: String,
        wake: Option<crate::input::WakeSignal>,
    ) -> Result<()> {
        let server = Server::http(format!("0.0.0.0:{port}"))
            .map_err(|e| anyhow::anyhow!("failed to start HTTP server: {e}"))?;
        log::info!("Companion HTTP server listening on port {port}");

        let companion = Self {
            server,
            tx: EventSender { tx, wake },
            db_path,
        };
        thread::spawn(move || {
            if let Err(err) = companion.run_loop() {
                log::error!("HTTP server loop error: {err}");
            }
        });
        Ok(())
    }

    fn run_loop(self) -> Result<()> {
        for mut request in self.server.incoming_requests() {
            let url = request.url().to_string();
            let path = url.split('?').next().unwrap_or(&url).to_string();
            let method = request.method().as_str();

            if path == "/" || path == "/index.html" {
                let response = Response::from_string(
                    "{\"status\":\"ok\",\"app\":\"MusicOS API\",\"control_center\":\"cargo run -p musicos-desktop --bin musicos-control-center\"}",
                )
                .with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                )
                .with_header(
                    Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                );
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/preview/show-controls" && method == "POST" {
                let _ = self.tx.send(ServerEvent::ShowControls);
                let response = json_response(&serde_json::json!({"status": "ok"}));
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/settings" && method == "GET" {
                let response = json_response(&crate::preferences::load());
                let _ = request.respond(response);
                continue;
            }

            if path.starts_with("/api/logs") && method == "GET" {
                let params = query_params(&url);
                let limit = params
                    .get("limit")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(120)
                    .clamp(1, MAX_RUNTIME_LOGS);
                let response = json_response(&recent_runtime_logs(limit));
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/settings" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                if let Ok(settings) = serde_json::from_str::<SettingsUpdate>(&body) {
                    let _ = self
                        .tx
                        .send(ServerEvent::Control(ControlCommand::SetSettings {
                            settings,
                        }));
                    let response = json_response(&serde_json::json!({"status": "ok"}));
                    let _ = request.respond(response);
                } else {
                    let response = json_response_status(
                        &serde_json::json!({"status": "invalid_settings"}),
                        StatusCode(400),
                    );
                    let _ = request.respond(response);
                }
                continue;
            }

            if path == "/api/v2/tracks" && method == "GET" {
                let params = query_params(&url);
                let response = match crate::database::Db::open(&self.db_path).and_then(|db| {
                    db.query_tracks_page(
                        params
                            .get("query")
                            .or_else(|| params.get("q"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        params.get("filter").map(String::as_str).unwrap_or(""),
                        params.get("artist").map(String::as_str).unwrap_or(""),
                        params.get("album").map(String::as_str).unwrap_or(""),
                        params.get("sort").map(String::as_str).unwrap_or("title"),
                        params
                            .get("direction")
                            .map(|value| value == "desc")
                            .unwrap_or(false),
                        params
                            .get("limit")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(100),
                        params
                            .get("offset")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0),
                    )
                }) {
                    Ok(page) => json_response(&page),
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "error", "message": err.to_string()}),
                        StatusCode(500),
                    ),
                };
                let _ = respond_buffered(request, response);
                continue;
            }

            if path == "/api/v2/albums" && method == "GET" {
                let params = query_params(&url);
                let response = match crate::database::Db::open(&self.db_path).and_then(|db| {
                    db.album_summaries(
                        params.get("query").map(String::as_str).unwrap_or(""),
                        params
                            .get("limit")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(100),
                        params
                            .get("offset")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0),
                    )
                }) {
                    Ok(items) => json_response(&serde_json::json!({"items": items})),
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "error", "message": err.to_string()}),
                        StatusCode(500),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/v2/artists" && method == "GET" {
                let params = query_params(&url);
                let response = match crate::database::Db::open(&self.db_path).and_then(|db| {
                    db.artist_summaries(
                        params.get("query").map(String::as_str).unwrap_or(""),
                        params
                            .get("limit")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(100),
                        params
                            .get("offset")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0),
                    )
                }) {
                    Ok(items) => json_response(&serde_json::json!({"items": items})),
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "error", "message": err.to_string()}),
                        StatusCode(500),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/v2/genres" && method == "GET" {
                let response =
                    match crate::database::Db::open(&self.db_path).and_then(|db| db.genres()) {
                        Ok(items) => json_response(&serde_json::json!({
                            "items": items.into_iter().map(|(name, track_count)| serde_json::json!({
                                "name": name,
                                "track_count": track_count
                            })).collect::<Vec<_>>()
                        })),
                        Err(err) => json_response_status(
                            &serde_json::json!({"status": "error", "message": err.to_string()}),
                            StatusCode(500),
                        ),
                    };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/v2/tracks/metadata" && method == "PATCH" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let response = match serde_json::from_str::<MetadataPayload>(&body) {
                    Ok(payload) => match crate::database::Db::open(&self.db_path).and_then(|db| {
                        let mut writable_ids = Vec::new();
                        let mut override_ids = Vec::new();
                        for id in &payload.ids {
                            if payload.write_to_file || db.track_source_kind(*id)? == "managed" {
                                writable_ids.push(*id);
                            } else {
                                override_ids.push(*id);
                            }
                        }
                        let mut updated = 0;
                        if !writable_ids.is_empty() {
                            let paths = writable_ids
                                .iter()
                                .map(|id| {
                                    db.get_track_by_id(*id)?
                                        .map(|track| track.path)
                                        .ok_or_else(|| anyhow::anyhow!("track {id} was not found"))
                                })
                                .collect::<Result<Vec<_>>>()?;
                            updated += crate::media::write_metadata_batch_with(
                                &paths,
                                &payload.update,
                                || db.update_track_metadata(&writable_ids, &payload.update),
                            )?;
                        }
                        if !override_ids.is_empty() {
                            updated +=
                                db.update_track_metadata_override(&override_ids, &payload.update)?;
                        }
                        Ok(updated)
                    }) {
                        Ok(updated) => json_response(&serde_json::json!({
                            "status": "ok",
                            "updated": updated
                        })),
                        Err(err) => json_response_status(
                            &serde_json::json!({"status": "error", "message": err.to_string()}),
                            StatusCode(400),
                        ),
                    },
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "invalid_metadata", "message": err.to_string()}),
                        StatusCode(400),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/v2/playlists/manifest" && method == "GET" {
                let response = match crate::database::Db::open(&self.db_path)
                    .and_then(|db| Ok((db.playlist_manifests()?, db.playlist_tombstones()?)))
                {
                    Ok((playlists, tombstones)) => json_response(&serde_json::json!({
                        "playlists": playlists,
                        "tombstones": tombstones,
                        "generated_at": unix_seconds()
                    })),
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "error", "message": err.to_string()}),
                        StatusCode(500),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/v2/playlists/reconcile" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let response = match serde_json::from_str::<PlaylistReconcilePayload>(&body) {
                    Ok(payload) => json_response(&reconcile_playlist_snapshots(
                        &payload.base,
                        &payload.desktop,
                        &payload.device,
                    )),
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "invalid_manifest", "message": err.to_string()}),
                        StatusCode(400),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/player" && method == "GET" {
                let body = if let Some(snapshot) = runtime_snapshot() {
                    serde_json::json!({
                        "status": "ok",
                        "player": snapshot.player,
                        "queue": snapshot.queue,
                        "queue_index": snapshot.queue_index,
                        "revision": playback_revision(&snapshot),
                        "active_output": snapshot.settings.audio_output_mode,
                    })
                } else {
                    serde_json::json!({
                        "status": "warming_up",
                        "player": null,
                        "queue": [],
                        "queue_index": null,
                    })
                };
                let response = json_response(&body);
                let _ = respond_buffered(request, response);
                continue;
            }

            if path == "/api/diagnostics" && method == "GET" {
                let body = if let Some(snapshot) = runtime_snapshot() {
                    serde_json::json!({
                        "status": "ok",
                        "db_path": snapshot.db_path,
                        "settings_path": snapshot.settings_path,
                        "counts": snapshot.counts,
                        "audio": crate::system::audio_diagnostics(),
                        "player": snapshot.player,
                        "scan_status": snapshot.scan_status,
                        "scan": snapshot.scan,
                        "version": build_version(),
                        "indexed_roots": snapshot.indexed_roots,
                        "usb_status": snapshot.usb_status,
                        "battery_status": snapshot.battery_status,
                        "cover_art_cache_count": snapshot.cover_art_cache_count,
                        "cover_art_memory_cache_count": snapshot.cover_art_memory_cache_count,
                        "frame_ms": snapshot.frame_ms,
                        "frame_timing": frame_timing_summary(),
                        "runtime_log_count": recent_runtime_logs(MAX_RUNTIME_LOGS).len(),
                        "settings": snapshot.settings,
                    })
                } else {
                    let (counts, cover_art_cache_count) = crate::database::Db::open(&self.db_path)
                        .map(|db| {
                            (
                                db.library_counts().unwrap_or_default(),
                                db.cover_art_cache_count().unwrap_or_default(),
                            )
                        })
                        .unwrap_or_default();
                    serde_json::json!({
                        "status": "warming_up",
                        "db_path": self.db_path,
                        "settings_path": crate::preferences::path(),
                        "counts": counts,
                        "audio": crate::system::audio_diagnostics(),
                        "indexed_roots": crate::media::last_scan_roots(),
                        "scan": ScanInfo::default(),
                        "version": build_version(),
                        "cover_art_cache_count": cover_art_cache_count,
                        "cover_art_memory_cache_count": crate::media::cover_art_memory_cache_len(),
                        "runtime_log_count": recent_runtime_logs(MAX_RUNTIME_LOGS).len(),
                    })
                };
                let response = json_response(&body);
                let _ = respond_buffered(request, response);
                continue;
            }

            if path == "/api/library" && method == "GET" {
                let body = if let Ok(db) = crate::database::Db::open(&self.db_path) {
                    let counts = db.library_counts().unwrap_or_default();
                    let playlists = db.get_playlists().unwrap_or_default();
                    let sources = db.library_sources().unwrap_or_default();
                    let cover_art_cache_count = db.cover_art_cache_count().unwrap_or_default();
                    if let Some(snapshot) = runtime_snapshot() {
                        serde_json::json!({
                            "status": "ok",
                            "db_path": snapshot.db_path,
                            "settings_path": snapshot.settings_path,
                            "counts": counts,
                            "playlists": playlists,
                            "sources": sources,
                            "player": snapshot.player,
                            "queue": snapshot.queue,
                            "queue_index": snapshot.queue_index,
                            "scan_status": snapshot.scan_status,
                            "scan": snapshot.scan,
                            "version": build_version(),
                            "indexed_roots": snapshot.indexed_roots,
                            "audio": crate::system::audio_diagnostics(),
                            "battery_status": snapshot.battery_status,
                            "usb_status": snapshot.usb_status,
                            "cover_art_cache_count": cover_art_cache_count,
                            "cover_art_memory_cache_count": snapshot.cover_art_memory_cache_count,
                            "frame_ms": snapshot.frame_ms,
                            "frame_timing": frame_timing_summary(),
                        })
                    } else {
                        serde_json::json!({
                            "status": "warming_up",
                            "db_path": self.db_path,
                            "settings_path": crate::preferences::path(),
                            "counts": counts,
                            "playlists": playlists,
                            "sources": sources,
                            "player": null,
                            "queue": [],
                            "queue_index": null,
                            "scan_status": "Warming up",
                            "scan": ScanInfo::default(),
                            "version": build_version(),
                            "indexed_roots": crate::media::last_scan_roots(),
                            "audio": crate::system::audio_diagnostics(),
                            "cover_art_cache_count": cover_art_cache_count,
                            "cover_art_memory_cache_count": crate::media::cover_art_memory_cache_len(),
                            "frame_ms": 0.0,
                        })
                    }
                } else {
                    serde_json::json!({
                        "status": "db_unavailable",
                        "db_path": self.db_path,
                    })
                };
                let response = json_response(&body);
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/performance" && method == "GET" {
                let body = if let Some(snapshot) = runtime_snapshot() {
                    let motion_mode = snapshot.settings.motion_mode;
                    serde_json::json!({
                        "status": "ok",
                        "frame_ms": snapshot.frame_ms,
                        "frame_timing": frame_timing_summary(),
                        "motion_mode": motion_mode,
                        "motion_mode_label": motion_mode.label(),
                        "cover_art_memory_cache_count": snapshot.cover_art_memory_cache_count,
                        "cover_art_cache_count": snapshot.cover_art_cache_count,
                        "scan": snapshot.scan,
                        "active_render_hint": snapshot.player.is_playing || snapshot.scan.state == "running",
                        "version": build_version(),
                    })
                } else {
                    serde_json::json!({
                        "status": "warming_up",
                        "frame_ms": 0.0,
                        "motion_mode": crate::preferences::load().motion_mode,
                        "cover_art_memory_cache_count": crate::media::cover_art_memory_cache_len(),
                        "version": build_version(),
                    })
                };
                let response = json_response(&body);
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/mixes" && method == "GET" {
                let mixes = crate::database::Db::open(&self.db_path)
                    .and_then(|db| db.dynamic_mixes())
                    .unwrap_or_default();
                let response = json_response(&mixes);
                let _ = request.respond(response);
                continue;
            }

            if let Some(mix_id) = parse_mix_tracks_path(&path) {
                if method == "GET" {
                    let params = query_params(&url);
                    let limit = params
                        .get("limit")
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(200);
                    let tracks = crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.get_mix_tracks(&mix_id, limit))
                        .unwrap_or_default();
                    let response = json_response(&tracks);
                    let _ = request.respond(response);
                    continue;
                }
            }

            if path.starts_with("/api/history") && method == "GET" {
                let params = query_params(&url);
                let limit = params
                    .get("limit")
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(100)
                    .clamp(1, 500);
                let offset = params
                    .get("offset")
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0)
                    .max(0);
                let tracks = crate::database::Db::open(&self.db_path)
                    .and_then(|db| db.get_history_paged(limit, offset))
                    .unwrap_or_default();
                let response = json_response(&tracks);
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/library/cover-art-cache/clear" && method == "POST" {
                match crate::database::Db::open(&self.db_path) {
                    Ok(db) => {
                        let dir = db.cover_art_dir();
                        let cleared = db.clear_cover_art_cache_metadata().unwrap_or_default();
                        crate::media::clear_cover_art_memory_cache();
                        if let Err(err) = std::fs::remove_dir_all(&dir) {
                            if err.kind() != std::io::ErrorKind::NotFound {
                                push_runtime_log(
                                    "cover-art",
                                    format!("cache directory clear failed: {err}"),
                                );
                            }
                        }
                        push_runtime_log("cover-art", format!("Cleared {cleared} cached art rows"));
                        let response = json_response(&serde_json::json!({
                            "status": "ok",
                            "cleared": cleared,
                            "cache_dir": dir,
                        }));
                        let _ = request.respond(response);
                    }
                    Err(err) => {
                        let response = json_response_status(
                            &serde_json::json!({"status": "error", "message": err.to_string()}),
                            StatusCode(500),
                        );
                        let _ = request.respond(response);
                    }
                }
                continue;
            }

            if path == "/api/library/repair" && method == "POST" {
                match crate::database::Db::open(&self.db_path).and_then(|db| db.repair_library()) {
                    Ok(report) => {
                        push_runtime_log(
                            "repair",
                            format!(
                                "DB repair integrity={} missing_removed={} orphan_rows={}",
                                report.integrity_ok,
                                report.missing_tracks_removed,
                                report.orphan_playlist_rows_removed
                            ),
                        );
                        let response = json_response(&serde_json::json!({
                            "status": "ok",
                            "report": report,
                        }));
                        let _ = request.respond(response);
                    }
                    Err(err) => {
                        let response = json_response_status(
                            &serde_json::json!({"status": "error", "message": err.to_string()}),
                            StatusCode(500),
                        );
                        let _ = request.respond(response);
                    }
                }
                continue;
            }

            if path == "/api/library/sources" && method == "GET" {
                let response = match crate::database::Db::open(&self.db_path)
                    .and_then(|db| db.library_sources())
                {
                    Ok(sources) => json_response(&serde_json::json!({"items": sources})),
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "error", "message": err.to_string()}),
                        StatusCode(500),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/library/sources" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let response = match serde_json::from_str::<LibrarySourcePayload>(&body) {
                    Ok(payload) => match crate::database::Db::open(&self.db_path).and_then(|db| {
                        db.add_library_source(&payload.kind, &payload.path, &payload.display_name)
                    }) {
                        Ok(source) => {
                            let _ =
                                self.tx
                                    .send(ServerEvent::Control(ControlCommand::RescanLibrary {
                                        mode: "changed".to_string(),
                                        source_id: Some(source.id),
                                    }));
                            json_response(&serde_json::json!({"status": "ok", "source": source}))
                        }
                        Err(err) => json_response_status(
                            &serde_json::json!({"status": "error", "message": err.to_string()}),
                            StatusCode(400),
                        ),
                    },
                    Err(err) => json_response_status(
                        &serde_json::json!({"status": "invalid_source", "message": err.to_string()}),
                        StatusCode(400),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if let Some((source_id, action)) = parse_library_source_path(&path) {
                if method == "PATCH" && action.is_none() {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    let response = match serde_json::from_str::<LibrarySourceUpdatePayload>(&body) {
                        Ok(payload) => {
                            match crate::database::Db::open(&self.db_path).and_then(|db| {
                                db.update_library_source(
                                    source_id,
                                    payload.display_name.as_deref(),
                                    payload.enabled,
                                )
                            }) {
                                Ok(Some(source)) => json_response(
                                    &serde_json::json!({"status": "ok", "source": source}),
                                ),
                                Ok(None) => json_response_status(
                                    &serde_json::json!({"status": "not_found"}),
                                    StatusCode(404),
                                ),
                                Err(err) => json_response_status(
                                    &serde_json::json!({"status": "error", "message": err.to_string()}),
                                    StatusCode(400),
                                ),
                            }
                        }
                        Err(err) => json_response_status(
                            &serde_json::json!({"status": "invalid_source", "message": err.to_string()}),
                            StatusCode(400),
                        ),
                    };
                    let _ = request.respond(response);
                    continue;
                }
                if method == "DELETE" && action.is_none() {
                    let response = match crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.remove_library_source(source_id))
                    {
                        Ok(true) => json_response(&serde_json::json!({"status": "ok"})),
                        Ok(false) => json_response_status(
                            &serde_json::json!({"status": "not_found"}),
                            StatusCode(404),
                        ),
                        Err(err) => json_response_status(
                            &serde_json::json!({"status": "error", "message": err.to_string()}),
                            StatusCode(400),
                        ),
                    };
                    let _ = request.respond(response);
                    continue;
                }
                if method == "POST" && action.as_deref() == Some("rescan") {
                    let _ = self
                        .tx
                        .send(ServerEvent::Control(ControlCommand::RescanLibrary {
                            mode: "changed".to_string(),
                            source_id: Some(source_id),
                        }));
                    let response = json_response(&serde_json::json!({
                        "status": "ok",
                        "source_id": source_id,
                        "mode": "changed",
                    }));
                    let _ = request.respond(response);
                    continue;
                }
            }

            if path == "/api/library/rescan" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let mode = serde_json::from_str::<RescanPayload>(&body)
                    .ok()
                    .and_then(|payload| payload.mode)
                    .unwrap_or_else(|| "changed".to_string());
                let mode = match mode.to_ascii_lowercase().as_str() {
                    "full" | "all" | "rebuild" => "full",
                    _ => "changed",
                }
                .to_string();
                let _ = self
                    .tx
                    .send(ServerEvent::Control(ControlCommand::RescanLibrary {
                        mode: mode.clone(),
                        source_id: None,
                    }));
                let response = json_response(&serde_json::json!({
                    "status": "ok",
                    "mode": mode,
                }));
                let _ = request.respond(response);
                continue;
            }

            if path.starts_with("/api/screenshot.png") || path.starts_with("/api/screenshot.bmp") {
                let use_png = url.starts_with("/api/screenshot.png");
                if let Some(frame) = renderer::get_latest_screenshot() {
                    let mut response_bytes = Vec::new();
                    let write_ok = if use_png {
                        write_screenshot_png(
                            &mut response_bytes,
                            &frame.bytes,
                            frame.resolution.width as usize,
                            frame.resolution.height as usize,
                        )
                    } else {
                        write_screenshot_bmp(
                            &mut response_bytes,
                            &frame.bytes,
                            frame.resolution.width as usize,
                            frame.resolution.height as usize,
                        )
                    };
                    if write_ok.is_ok() {
                        let content_type = if use_png { "image/png" } else { "image/bmp" };
                        let response = Response::from_data(response_bytes)
                            .with_header(
                                Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
                                    .unwrap(),
                            )
                            .with_header(
                                Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                                    .unwrap(),
                            )
                            .with_header(
                                Header::from_bytes(
                                    &b"Cache-Control"[..],
                                    &b"no-cache, no-store, must-revalidate"[..],
                                )
                                .unwrap(),
                            );
                        let _ = request.respond(response);
                        continue;
                    }
                }
                let response = Response::from_string("Screenshot unavailable")
                    .with_status_code(StatusCode(503))
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            if (path.starts_with("/api/cover-art.png") || path.starts_with("/api/cover-art.bmp"))
                && method == "GET"
            {
                let use_png = url.starts_with("/api/cover-art.png");
                let id = url.split("id=").nth(1).and_then(|value| {
                    value
                        .chars()
                        .take_while(|ch| ch.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i64>()
                        .ok()
                });

                if let Some(id) = id {
                    if let Ok(db) = crate::database::Db::open(&self.db_path) {
                        if let Ok(Some(track)) = db.get_track_by_id(id) {
                            if let Some(art) = crate::media::cached_cover_art_for_track(&db, &track)
                            {
                                let mut response_bytes = Vec::new();
                                let write_ok = if use_png {
                                    write_cover_art_png(&mut response_bytes, &art)
                                } else {
                                    write_cover_art_bmp(&mut response_bytes, &art)
                                };
                                if write_ok.is_ok() {
                                    let content_type =
                                        if use_png { "image/png" } else { "image/bmp" };
                                    let response = Response::from_data(response_bytes)
                                        .with_header(
                                            Header::from_bytes(
                                                &b"Content-Type"[..],
                                                content_type.as_bytes(),
                                            )
                                            .unwrap(),
                                        )
                                        .with_header(
                                            Header::from_bytes(
                                                &b"Access-Control-Allow-Origin"[..],
                                                &b"*"[..],
                                            )
                                            .unwrap(),
                                        )
                                        .with_header(
                                            Header::from_bytes(
                                                &b"Cache-Control"[..],
                                                &b"public, max-age=3600"[..],
                                            )
                                            .unwrap(),
                                        );
                                    let _ = request.respond(response);
                                    continue;
                                }
                            }
                        }
                    }
                }

                let response = Response::from_string("Cover art unavailable")
                    .with_status_code(StatusCode(404))
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            if path.starts_with("/api/input") && method == "POST" {
                let key_str = url.split("key=").nth(1).unwrap_or("");
                let button = match key_str {
                    "Confirm" => Some(Button::Confirm),
                    "Back" => Some(Button::Back),
                    "Up" => Some(Button::Up),
                    "Down" => Some(Button::Down),
                    "Left" => Some(Button::Left),
                    "Right" => Some(Button::Right),
                    "Context" => Some(Button::Context),
                    "Home" => Some(Button::Home),
                    "Menu" => Some(Button::Menu),
                    "Sleep" => Some(Button::Sleep),
                    "VolumeUp" => Some(Button::VolumeUp),
                    "VolumeDown" => Some(Button::VolumeDown),
                    _ => None,
                };
                if let Some(b) = button {
                    let _ = self.tx.send(ServerEvent::Button(b));
                    let response = Response::from_string("{\"status\":\"ok\"}").with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                    let _ = request.respond(response);
                } else {
                    let response = Response::from_string("{\"status\":\"invalid_key\"}")
                        .with_status_code(StatusCode(400))
                        .with_header(
                            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                                .unwrap(),
                        );
                    let _ = request.respond(response);
                }
                continue;
            }

            if path == "/api/sync-usb" && method == "POST" {
                let _ = self.tx.send(ServerEvent::UsbSync);
                let response = Response::from_string("{\"status\":\"ok\"}")
                    .with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                    )
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/updates/install" && method == "POST" {
                let response = match crate::usb::install_pending_update() {
                    Ok(Some(path)) => json_response(&serde_json::json!({
                        "status": "ok",
                        "installed_update": path.display().to_string(),
                        "message": "Installed staged MusicOS update."
                    })),
                    Ok(None) => json_response(&serde_json::json!({
                        "status": "no_update",
                        "installed_update": null,
                        "message": "No staged MusicOS update was found."
                    })),
                    Err(error) => json_response_status(
                        &serde_json::json!({
                            "status": "error",
                            "message": error.to_string()
                        }),
                        StatusCode(500),
                    ),
                };
                let _ = request.respond(response);
                continue;
            }

            if path.starts_with("/api/pointer") && method == "POST" {
                let event = if url.contains("action=tap") {
                    Some(InputEvent::Tap)
                } else if url.contains("action=swipe") {
                    let dir_str = url.split("dir=").nth(1).unwrap_or("");
                    match dir_str {
                        "up" => Some(InputEvent::Swipe(SwipeDirection::Up)),
                        "down" => Some(InputEvent::Swipe(SwipeDirection::Down)),
                        "left" => Some(InputEvent::Swipe(SwipeDirection::Left)),
                        "right" => Some(InputEvent::Swipe(SwipeDirection::Right)),
                        _ => None,
                    }
                } else {
                    None
                };

                if let Some(ev) = event {
                    let _ = self.tx.send(ServerEvent::Pointer(ev));
                    let response = Response::from_string("{\"status\":\"ok\"}").with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                    let _ = request.respond(response);
                } else {
                    let response = Response::from_string("{\"status\":\"invalid_action\"}")
                        .with_status_code(StatusCode(400))
                        .with_header(
                            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                                .unwrap(),
                        );
                    let _ = request.respond(response);
                }
                continue;
            }

            if path == "/api/control" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                if let Ok(cmd) = serde_json::from_str::<ControlCommand>(&body) {
                    let command_name = serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|value| {
                            value
                                .get("command")
                                .and_then(|value| value.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| "unknown".to_string());
                    let accepted = self.tx.send(ServerEvent::Control(cmd)).is_ok();
                    let revision = runtime_snapshot()
                        .as_ref()
                        .map(playback_revision)
                        .unwrap_or_default();
                    let response = if accepted {
                        json_response_status(
                            &serde_json::json!({
                                "status": "accepted",
                                "command": command_name,
                                "player_revision": revision
                            }),
                            StatusCode(202),
                        )
                    } else {
                        json_response_status(
                            &serde_json::json!({
                                "status": "unavailable",
                                "message": "runtime command channel is closed"
                            }),
                            StatusCode(503),
                        )
                    };
                    let _ = request.respond(response);
                } else {
                    let response = Response::from_string("{\"status\":\"invalid_command\"}")
                        .with_status_code(StatusCode(400))
                        .with_header(
                            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                                .unwrap(),
                        );
                    let _ = request.respond(response);
                }
                continue;
            }

            if path == "/api/upload" && method == "POST" {
                let headers = request.headers();
                let requested_path = header_value(headers, "x-relative-path")
                    .or_else(|| header_value(headers, "x-filename"))
                    .unwrap_or_else(|| "upload.ogg".to_string());
                let Some(relative_path) = sanitize_relative_path(&requested_path) else {
                    let response = json_response_status(
                        &serde_json::json!({"status": "invalid_path"}),
                        StatusCode(400),
                    );
                    let _ = request.respond(response);
                    continue;
                };
                let expected_sha = header_value(headers, "x-sha256").map(|value| {
                    value
                        .trim()
                        .chars()
                        .filter(|ch| ch.is_ascii_hexdigit())
                        .collect::<String>()
                        .to_ascii_lowercase()
                });
                let trigger_scan = header_value(headers, "x-trigger-scan")
                    .map(|value| value != "0")
                    .unwrap_or(true);

                let max_upload_bytes = std::env::var("MUSICOS_MAX_UPLOAD_BYTES")
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(DEFAULT_MAX_UPLOAD_BYTES);
                if header_value(headers, "x-file-size")
                    .and_then(|value| value.parse::<u64>().ok())
                    .is_some_and(|size| size > max_upload_bytes)
                {
                    let response = json_response_status(
                        &serde_json::json!({"status": "error", "message": "upload too large"}),
                        StatusCode(413),
                    );
                    let _ = request.respond(response);
                    continue;
                }

                let mut body = Vec::new();
                let mut limited_reader = request.as_reader().take(max_upload_bytes + 1);
                if limited_reader.read_to_end(&mut body).is_err()
                    || body.len() as u64 > max_upload_bytes
                {
                    let response = json_response_status(
                        &serde_json::json!({"status": "error", "message": "upload too large"}),
                        StatusCode(413),
                    );
                    let _ = request.respond(response);
                    continue;
                }

                let upload_dir =
                    std::env::var("MUSICOS_UPLOAD_DIR").unwrap_or_else(|_| "./music".to_string());
                let _ = std::fs::create_dir_all(&upload_dir);
                let upload_dir_canon = std::path::Path::new(&upload_dir)
                    .canonicalize()
                    .unwrap_or_else(|_| std::path::PathBuf::from(&upload_dir));
                let dest_path = upload_dir_canon.join(&relative_path);

                let is_safe = dest_path.starts_with(&upload_dir_canon);

                if !is_safe {
                    log::error!("path traversal attempt blocked: {}", relative_path);
                    push_runtime_log(
                        "upload",
                        format!("blocked unsafe upload path {}", relative_path),
                    );
                    let response = json_response_status(
                        &serde_json::json!({"status": "error", "message": "traversal attempt blocked"}),
                        StatusCode(400),
                    );
                    let _ = request.respond(response);
                    continue;
                }

                let incoming_sha = hex_sha256(&body);
                if expected_sha
                    .as_deref()
                    .is_some_and(|sha| sha.len() != 64 || sha != incoming_sha)
                {
                    push_runtime_log(
                        "upload",
                        format!("Rejected checksum mismatch {}", relative_path),
                    );
                    let response = json_response_status(
                        &serde_json::json!({"status": "error", "message": "checksum mismatch"}),
                        StatusCode(400),
                    );
                    let _ = request.respond(response);
                    continue;
                }
                let skipped = if dest_path.exists() {
                    let same_size = dest_path
                        .metadata()
                        .map(|meta| meta.len() == body.len() as u64)
                        .unwrap_or(false);
                    let same_sha = same_size
                        && file_sha256_hex(&dest_path)
                            .map(|sha| sha == incoming_sha)
                            .unwrap_or(false);
                    same_size && same_sha
                } else {
                    false
                };

                let temp_path = dest_path.with_extension(format!(
                    "{}.uploading-{}",
                    dest_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("tmp"),
                    now_unix_ms()
                ));
                let result = if skipped {
                    sync_existing_upload(&dest_path)
                } else {
                    write_upload(&temp_path, &dest_path, &body)
                };
                match result {
                    Ok(_) => {
                        if skipped {
                            push_runtime_log(
                                "upload",
                                format!("Skipped unchanged {}", relative_path),
                            );
                        } else {
                            log::info!("Uploaded file saved to {}", dest_path.display());
                            push_runtime_log("upload", format!("Saved {}", relative_path));
                            if trigger_scan {
                                let _ = self.tx.send(ServerEvent::UploadComplete);
                            }
                        }
                        let response = json_response(&serde_json::json!({
                            "status": if skipped { "skipped" } else { "ok" },
                            "path": relative_path,
                            "sha256": incoming_sha,
                        }));
                        let _ = request.respond(response);
                    }
                    Err(err) => {
                        let _ = std::fs::remove_file(&temp_path);
                        log::error!("failed to write uploaded file: {err}");
                        push_runtime_log(
                            "upload",
                            format!("failed to write {}: {err}", relative_path),
                        );
                        let response = json_response_status(
                            &serde_json::json!({"status": "error", "message": "failed to write file"}),
                            StatusCode(500),
                        );
                        let _ = request.respond(response);
                    }
                }
                continue;
            }

            if path.starts_with("/api/status") && method == "GET" {
                let body = if let Ok(db) = crate::database::Db::open(&self.db_path) {
                    let counts = db.library_counts().unwrap_or_default();
                    serde_json::json!({
                        "status": "ok",
                        "db_path": self.db_path,
                        "counts": counts,
                        "version": build_version(),
                    })
                } else {
                    serde_json::json!({
                        "status": "db_unavailable",
                        "db_path": self.db_path,
                        "version": build_version(),
                    })
                };
                let response = Response::from_string(body.to_string())
                    .with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                    )
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/playlists" && method == "GET" {
                let playlists = crate::database::Db::open(&self.db_path)
                    .and_then(|db| db.get_playlists())
                    .unwrap_or_default();
                let response = json_response(&playlists);
                let _ = request.respond(response);
                continue;
            }

            if path == "/api/playlists" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<PlaylistPayload>(&body)
                    .ok()
                    .map(|payload| payload.name.trim().to_string())
                    .filter(|name| !name.is_empty())
                {
                    Some(name) => match crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.create_playlist(&name))
                    {
                        Ok(id) => {
                            let _ = self.tx.send(ServerEvent::Control(
                                ControlCommand::CreatePlaylist { name: name.clone() },
                            ));
                            let response = json_response(&serde_json::json!({
                                "status": "ok",
                                "playlist": { "id": id, "name": name }
                            }));
                            let _ = request.respond(response);
                        }
                        Err(err) => {
                            let response = json_response_status(
                                &serde_json::json!({"status": "error", "message": err.to_string()}),
                                StatusCode(500),
                            );
                            let _ = request.respond(response);
                        }
                    },
                    None => {
                        let response = json_response_status(
                            &serde_json::json!({"status": "invalid_playlist"}),
                            StatusCode(400),
                        );
                        let _ = request.respond(response);
                    }
                }
                continue;
            }

            if let Some(playlist_id) = parse_playlist_reorder_path(&path) {
                if method == "PUT" || method == "POST" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    match serde_json::from_str::<PlaylistReorderPayload>(&body) {
                        Ok(payload) => {
                            match crate::database::Db::open(&self.db_path).and_then(|db| {
                                db.reorder_playlist_tracks(playlist_id, &payload.track_ids)
                            }) {
                                Ok(()) => {
                                    let response =
                                        json_response(&serde_json::json!({"status": "ok"}));
                                    let _ = request.respond(response);
                                }
                                Err(err) => {
                                    let response = json_response_status(
                                        &serde_json::json!({"status": "error", "message": err.to_string()}),
                                        StatusCode(500),
                                    );
                                    let _ = request.respond(response);
                                }
                            }
                        }
                        Err(_) => {
                            let response = json_response_status(
                                &serde_json::json!({"status": "invalid_reorder"}),
                                StatusCode(400),
                            );
                            let _ = request.respond(response);
                        }
                    }
                    continue;
                }
            }

            if let Some((playlist_id, track_id)) = parse_playlist_track_path(&path) {
                if method == "DELETE" {
                    match crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.remove_track_from_playlist(playlist_id, track_id))
                    {
                        Ok(()) => {
                            let _ = self.tx.send(ServerEvent::Control(
                                ControlCommand::RemoveTrackFromPlaylist {
                                    playlist_id,
                                    track_id,
                                },
                            ));
                            let response = json_response(&serde_json::json!({"status": "ok"}));
                            let _ = request.respond(response);
                        }
                        Err(err) => {
                            let response = json_response_status(
                                &serde_json::json!({"status": "error", "message": err.to_string()}),
                                StatusCode(500),
                            );
                            let _ = request.respond(response);
                        }
                    }
                    continue;
                }
            }

            if let Some(playlist_id) = parse_playlist_tracks_path(&path) {
                if method == "GET" {
                    let tracks = crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.get_playlist_tracks(playlist_id))
                        .unwrap_or_default();
                    let response = json_response(&tracks);
                    let _ = request.respond(response);
                    continue;
                }

                if method == "POST" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    match serde_json::from_str::<PlaylistTrackPayload>(&body) {
                        Ok(payload) => match crate::database::Db::open(&self.db_path)
                            .and_then(|db| db.add_track_to_playlist(playlist_id, payload.track_id))
                        {
                            Ok(()) => {
                                let _ = self.tx.send(ServerEvent::Control(
                                    ControlCommand::AddTrackToPlaylist {
                                        playlist_id,
                                        track_id: payload.track_id,
                                    },
                                ));
                                let response = json_response(&serde_json::json!({"status": "ok"}));
                                let _ = request.respond(response);
                            }
                            Err(err) => {
                                let response = json_response_status(
                                    &serde_json::json!({"status": "error", "message": err.to_string()}),
                                    StatusCode(500),
                                );
                                let _ = request.respond(response);
                            }
                        },
                        Err(_) => {
                            let response = json_response_status(
                                &serde_json::json!({"status": "invalid_track"}),
                                StatusCode(400),
                            );
                            let _ = request.respond(response);
                        }
                    }
                    continue;
                }
            }

            if let Some(playlist_id) = parse_playlist_path(&path) {
                if method == "PUT" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    match serde_json::from_str::<PlaylistPayload>(&body)
                        .ok()
                        .map(|payload| payload.name.trim().to_string())
                        .filter(|name| !name.is_empty())
                    {
                        Some(name) => match crate::database::Db::open(&self.db_path)
                            .and_then(|db| db.rename_playlist(playlist_id, &name))
                        {
                            Ok(()) => {
                                let _ = self.tx.send(ServerEvent::Control(
                                    ControlCommand::RenamePlaylist {
                                        id: playlist_id,
                                        name,
                                    },
                                ));
                                let response = json_response(&serde_json::json!({"status": "ok"}));
                                let _ = request.respond(response);
                            }
                            Err(err) => {
                                let response = json_response_status(
                                    &serde_json::json!({"status": "error", "message": err.to_string()}),
                                    StatusCode(500),
                                );
                                let _ = request.respond(response);
                            }
                        },
                        None => {
                            let response = json_response_status(
                                &serde_json::json!({"status": "invalid_playlist"}),
                                StatusCode(400),
                            );
                            let _ = request.respond(response);
                        }
                    }
                    continue;
                }

                if method == "DELETE" {
                    match crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.delete_playlist(playlist_id))
                    {
                        Ok(()) => {
                            let _ = self.tx.send(ServerEvent::Control(
                                ControlCommand::DeletePlaylist { id: playlist_id },
                            ));
                            let response = json_response(&serde_json::json!({"status": "ok"}));
                            let _ = request.respond(response);
                        }
                        Err(err) => {
                            let response = json_response_status(
                                &serde_json::json!({"status": "error", "message": err.to_string()}),
                                StatusCode(500),
                            );
                            let _ = request.respond(response);
                        }
                    }
                    continue;
                }
            }

            if path.starts_with("/api/tracks") && method == "GET" {
                if let Ok(db) = crate::database::Db::open(&self.db_path) {
                    let params = query_params(&url);
                    if params
                        .get("playlists")
                        .map(|value| value == "1")
                        .unwrap_or(false)
                        || params
                            .get("type")
                            .map(|value| value == "playlists")
                            .unwrap_or(false)
                    {
                        let playlists = db.get_playlists().unwrap_or_default();
                        let response = json_response(&playlists);
                        let _ = request.respond(response);
                        continue;
                    }
                    let tracks = query_tracks(&db, &params).unwrap_or_default();
                    let response = json_response(&tracks);
                    let _ = request.respond(response);
                    continue;
                }
                let response = Response::from_string("[]")
                    .with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                    )
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            if method == "OPTIONS" {
                let response = Response::empty(StatusCode(204))
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    )
                    .with_header(
                        Header::from_bytes(
                            &b"Access-Control-Allow-Methods"[..],
                            &b"GET, POST, PUT, PATCH, DELETE, OPTIONS"[..],
                        )
                        .unwrap(),
                    )
                    .with_header(
                        Header::from_bytes(
                            &b"Access-Control-Allow-Headers"[..],
                            &b"Content-Type, X-Filename"[..],
                        )
                        .unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            let _ = request.respond(
                Response::from_string("Not Found")
                    .with_status_code(StatusCode(404))
                    .with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    ),
            );
        }
        Ok(())
    }
}

fn respond_buffered(
    request: tiny_http::Request,
    response: Response<std::io::Cursor<Vec<u8>>>,
) -> std::io::Result<()> {
    // Keep headers and the complete JSON body in one write to avoid delayed ACKs.
    let mut bytes = Vec::new();
    response.with_chunked_threshold(usize::MAX).raw_print(
        &mut bytes,
        request.http_version().clone(),
        request.headers(),
        request.method() == &tiny_http::Method::Head,
        None,
    )?;
    let mut writer = request.into_writer();
    writer.write_all(&bytes)?;
    writer.flush()
}

fn json_response<T: Serialize>(value: &T) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response_status(value, StatusCode(200))
}

fn json_response_status<T: Serialize>(
    value: &T,
    status: StatusCode,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
}

fn header_value(headers: &[Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.field.to_string().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}

fn query_params(url: &str) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();
    let Some(query) = url.split_once('?').map(|(_, query)| query) else {
        return params;
    };
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(percent_decode(key), percent_decode(value));
    }
    params
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&value[i + 1..i + 3], 16) {
                out.push(hex);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn sanitize_filename(name: &str) -> String {
    let basename = std::path::Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("upload.ogg");
    let cleaned: String = basename
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.trim().is_empty() {
        "upload.ogg".to_string()
    } else {
        cleaned
    }
}

fn sanitize_relative_path(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for raw in path.split(['/', '\\']) {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
            continue;
        }
        parts.push(sanitize_filename(trimmed));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(std::path::MAIN_SEPARATOR_STR))
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn file_sha256_hex(path: &std::path::Path) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| hex_sha256(&bytes))
}

fn query_tracks(
    db: &crate::database::Db,
    params: &std::collections::HashMap<String, String>,
) -> Result<Vec<Track>> {
    let mut tracks = if let Some(playlist_id) = params
        .get("playlist_id")
        .and_then(|value| value.parse::<i64>().ok())
    {
        db.get_playlist_tracks(playlist_id)?
    } else {
        match params.get("filter").map(|value| value.as_str()) {
            Some("favorites") => db.get_favorites()?,
            Some("recent") => {
                let mut tracks = db.get_tracks()?;
                tracks.sort_by_key(|track| {
                    (
                        std::cmp::Reverse(track.indexed_at),
                        track.title.to_ascii_lowercase(),
                    )
                });
                tracks
            }
            _ => db.get_tracks()?,
        }
    };

    if let Some(artist) = params.get("artist").filter(|value| !value.is_empty()) {
        tracks.retain(|track| track.artist.eq_ignore_ascii_case(artist));
    }
    if let Some(album) = params.get("album").filter(|value| !value.is_empty()) {
        tracks.retain(|track| track.album.eq_ignore_ascii_case(album));
    }
    if let Some(query) = params
        .get("query")
        .or_else(|| params.get("q"))
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
    {
        tracks.retain(|track| {
            [
                track.title.as_str(),
                track.artist.as_str(),
                track.album.as_str(),
                track.path.as_str(),
            ]
            .iter()
            .any(|value| value.to_ascii_lowercase().contains(&query))
        });
    }

    match params.get("sort").map(|value| value.as_str()) {
        Some("artist") => tracks.sort_by_key(|track| {
            (
                track.artist.to_ascii_lowercase(),
                track.album.to_ascii_lowercase(),
                track.track_number,
                track.title.to_ascii_lowercase(),
            )
        }),
        Some("album") => tracks.sort_by_key(|track| {
            (
                track.album.to_ascii_lowercase(),
                track.track_number,
                track.title.to_ascii_lowercase(),
            )
        }),
        Some("recent") => tracks.sort_by_key(|track| {
            (
                std::cmp::Reverse(track.indexed_at),
                track.title.to_ascii_lowercase(),
            )
        }),
        Some("plays") | Some("play_count") => tracks.sort_by_key(|track| {
            (
                std::cmp::Reverse(track.play_count),
                track.title.to_ascii_lowercase(),
            )
        }),
        _ => {}
    }

    let offset = params
        .get("offset")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
        .min(tracks.len());
    let limit = params
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .map(|limit| limit.min(500))
        .unwrap_or_else(|| tracks.len().saturating_sub(offset));

    Ok(tracks.into_iter().skip(offset).take(limit).collect())
}

fn parse_playlist_path(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/api/playlists/")?;
    if rest.contains('/') {
        return None;
    }
    rest.parse().ok()
}

fn parse_library_source_path(path: &str) -> Option<(i64, Option<String>)> {
    let rest = path.strip_prefix("/api/library/sources/")?;
    let mut parts = rest.split('/');
    let id = parts.next()?.parse().ok()?;
    let action = parts.next().map(str::to_string);
    if parts.next().is_some() {
        return None;
    }
    Some((id, action))
}

fn parse_playlist_tracks_path(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/api/playlists/")?;
    let id = rest.strip_suffix("/tracks")?;
    id.parse().ok()
}

fn parse_playlist_reorder_path(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/api/playlists/")?;
    let id = rest.strip_suffix("/tracks/reorder")?;
    id.parse().ok()
}

fn parse_playlist_track_path(path: &str) -> Option<(i64, i64)> {
    let rest = path.strip_prefix("/api/playlists/")?;
    let (playlist_id, tail) = rest.split_once("/tracks/")?;
    Some((playlist_id.parse().ok()?, tail.parse().ok()?))
}

fn parse_mix_tracks_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/api/mixes/")?;
    let id = rest.strip_suffix("/tracks")?;
    let cleaned: String = id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_library_source_path, parse_mix_tracks_path, parse_playlist_path,
        parse_playlist_reorder_path, parse_playlist_track_path, parse_playlist_tracks_path,
        query_params, query_tracks, reconcile_playlist_snapshots, sanitize_filename,
        sanitize_relative_path, PlaylistSyncSnapshot,
    };
    use crate::database::Db;

    #[test]
    fn buffered_json_preserves_bodies_status_headers_and_keep_alive() {
        use std::io::{BufRead, Read, Write};
        use std::net::TcpStream;

        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = server.server_addr().to_ip().unwrap();
        let cases = vec![
            (200, serde_json::json!([])),
            (200, serde_json::json!({"title": "Musique ♫"})),
            (500, serde_json::json!({"status": "error", "message": "unavailable"})),
            (200, serde_json::json!({"items": "x".repeat(40_000)})),
        ];
        let responses = cases.clone();
        let worker = std::thread::spawn(move || {
            for (status, value) in responses {
                let request = server.recv().unwrap();
                super::respond_buffered(
                    request,
                    super::json_response_status(&value, tiny_http::StatusCode(status)),
                ).unwrap();
            }
        });
        let mut stream = std::io::BufReader::new(TcpStream::connect(address).unwrap());
        stream.get_ref().set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        for (status, value) in cases {
            stream.get_mut().write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
            let mut line = String::new();
            stream.read_line(&mut line).unwrap();
            assert!(line.starts_with(&format!("HTTP/1.1 {status} ")));
            let mut headers = String::new();
            loop {
                line.clear();
                stream.read_line(&mut line).unwrap();
                if line == "\r\n" { break; }
                assert!(!line.is_empty());
                headers.push_str(&line.to_ascii_lowercase());
            }
            assert!(headers.contains("content-type: application/json\r\n"));
            assert!(headers.contains("access-control-allow-origin: *\r\n"));
            assert!(!headers.contains("transfer-encoding:"));
            let length: usize = headers.lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .unwrap().trim().parse().unwrap();
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            assert_eq!(serde_json::from_slice::<serde_json::Value>(&body).unwrap(), value);
        }
        worker.join().unwrap();
    }

    #[test]
    fn upload_filenames_are_sanitized() {
        assert_eq!(sanitize_filename("../evil.mp3"), "evil.mp3");
        assert_eq!(sanitize_filename("artist/song?.wav"), "song_.wav");
        assert_eq!(sanitize_filename(""), "upload.ogg");
    }

    #[test]
    fn upload_relative_paths_are_sanitized_by_component() {
        let expected_nested =
            ["Artist", "Album", "01 - Song_.wav"].join(&std::path::MAIN_SEPARATOR.to_string());
        assert_eq!(
            sanitize_relative_path("Artist/Album/01 - Song?.wav").as_deref(),
            Some(expected_nested.as_str())
        );
        let expected = ["Artist", "Song.mp3"].join(&std::path::MAIN_SEPARATOR.to_string());
        assert_eq!(
            sanitize_relative_path("../Artist/../../Song.mp3").as_deref(),
            Some(expected.as_str())
        );
        assert!(sanitize_relative_path("../..").is_none());
    }

    #[test]
    fn query_params_decode_basic_values() {
        let params = query_params("/api/tracks?query=hello+world&artist=TAS%201000");
        assert_eq!(params.get("query").map(String::as_str), Some("hello world"));
        assert_eq!(params.get("artist").map(String::as_str), Some("TAS 1000"));
    }

    #[test]
    fn playlist_api_paths_parse_ids() {
        assert_eq!(parse_playlist_path("/api/playlists/12"), Some(12));
        assert_eq!(
            parse_playlist_tracks_path("/api/playlists/12/tracks"),
            Some(12)
        );
        assert_eq!(
            parse_playlist_track_path("/api/playlists/12/tracks/34"),
            Some((12, 34))
        );
        assert_eq!(
            parse_playlist_reorder_path("/api/playlists/12/tracks/reorder"),
            Some(12)
        );
        assert_eq!(parse_playlist_path("/api/playlists/12/tracks"), None);
        assert_eq!(
            parse_library_source_path("/api/library/sources/7"),
            Some((7, None))
        );
        assert_eq!(
            parse_library_source_path("/api/library/sources/7/rescan"),
            Some((7, Some("rescan".to_string())))
        );
        assert_eq!(parse_library_source_path("/api/library/sources/nope"), None);
    }

    #[test]
    fn mix_api_paths_parse_ids() {
        assert_eq!(
            parse_mix_tracks_path("/api/mixes/recently_added/tracks").as_deref(),
            Some("recently_added")
        );
        assert_eq!(
            parse_mix_tracks_path("/api/mixes/../tracks").as_deref(),
            None
        );
        assert_eq!(parse_mix_tracks_path("/api/mixes/recently_added"), None);
    }

    #[test]
    fn track_query_filters_and_limits_results() {
        let root = std::env::temp_dir().join(format!("musicos-track-query-{}", std::process::id()));
        let db_path = root.join("media.sqlite3");
        std::fs::create_dir_all(&root).unwrap();
        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let first = db
            .insert_track("/music/alpha.wav", "Alpha", "One", "Blue", 120, 1, 10)
            .unwrap();
        let second = db
            .insert_track("/music/beta.wav", "Beta", "Two", "Red", 140, 1, 20)
            .unwrap();
        db.toggle_favorite(second).unwrap();
        let playlist = db.create_playlist("Road").unwrap();
        db.add_track_to_playlist(playlist, first).unwrap();

        let mut params = std::collections::HashMap::new();
        params.insert("query".to_string(), "red".to_string());
        let tracks = query_tracks(&db, &params).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, second);

        params.clear();
        params.insert("filter".to_string(), "favorites".to_string());
        let tracks = query_tracks(&db, &params).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, second);

        params.clear();
        params.insert("playlist_id".to_string(), playlist.to_string());
        let tracks = query_tracks(&db, &params).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, first);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn playlist_reconciliation_merges_safe_edits_and_reports_order_conflicts() {
        let base = PlaylistSyncSnapshot {
            stable_id: "playlist-road".to_string(),
            name: "Road".to_string(),
            revision: 1,
            track_ids: vec!["a".to_string(), "b".to_string()],
            deleted: false,
        };
        let desktop = PlaylistSyncSnapshot {
            name: "Road trip".to_string(),
            revision: 2,
            track_ids: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ..base.clone()
        };
        let device = PlaylistSyncSnapshot {
            revision: 3,
            track_ids: vec!["a".to_string(), "b".to_string(), "d".to_string()],
            ..base.clone()
        };
        let merged = reconcile_playlist_snapshots(&base, &desktop, &device);
        assert_eq!(merged.merged.name, "Road trip");
        assert_eq!(merged.merged.track_ids, vec!["a", "b", "c", "d"]);
        assert!(merged.conflicts.is_empty());

        let reordered_desktop = PlaylistSyncSnapshot {
            track_ids: vec!["b".to_string(), "a".to_string()],
            name: "Desktop road".to_string(),
            ..base.clone()
        };
        let reordered_device = PlaylistSyncSnapshot {
            track_ids: vec!["b".to_string(), "a".to_string()],
            name: "Device road".to_string(),
            ..base.clone()
        };
        let result = reconcile_playlist_snapshots(&base, &reordered_desktop, &reordered_device);
        assert!(result
            .conflicts
            .iter()
            .any(|conflict| conflict.field == "name"));

        let deleted_desktop = PlaylistSyncSnapshot {
            deleted: true,
            revision: 4,
            ..base.clone()
        };
        let edited_device = PlaylistSyncSnapshot {
            name: "Road edits".to_string(),
            revision: 4,
            ..base.clone()
        };
        let result = reconcile_playlist_snapshots(&base, &deleted_desktop, &edited_device);
        assert!(result
            .conflicts
            .iter()
            .any(|conflict| conflict.field == "delete-versus-edit"));
    }
}

fn write_screenshot_bmp<W: Write>(
    w: &mut W,
    bgra: &[u8],
    width: usize,
    height: usize,
) -> Result<()> {
    write_bmp(w, width, height, bgra)
}

fn write_cover_art_bmp<W: Write>(w: &mut W, art: &renderer::CoverArt) -> Result<()> {
    let mut bgra = Vec::with_capacity(art.pixels.len() * 4);
    for color in &art.pixels {
        bgra.push(color.b);
        bgra.push(color.g);
        bgra.push(color.r);
        bgra.push(255);
    }
    write_bmp(w, art.width as usize, art.height as usize, &bgra)
}

fn write_upload(
    temp_path: &std::path::Path,
    dest_path: &std::path::Path,
    body: &[u8],
) -> std::io::Result<()> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    {
        let mut file = std::fs::File::create(temp_path)?;
        file.write_all(body)?;
        if cfg!(all(target_os = "linux", feature = "pid1")) {
            file.sync_all()?;
        }
    }
    #[cfg(windows)]
    if dest_path.exists() {
        std::fs::remove_file(dest_path)?;
    }
    std::fs::rename(temp_path, dest_path)?;
    sync_upload_directories(dest_path)
}

fn sync_existing_upload(path: &std::path::Path) -> std::io::Result<()> {
    if cfg!(all(target_os = "linux", feature = "pid1")) {
        std::fs::File::open(path)?.sync_all()?;
    }
    sync_upload_directories(path)
}

fn sync_upload_directories(_path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(all(target_os = "linux", feature = "pid1"))]
    if let Some(parent) = _path.parent() {
        use std::os::unix::fs::MetadataExt;

        // A nested upload can have created more than just its immediate parent.
        let directory = parent.canonicalize()?;
        let device = directory.metadata()?.dev();
        for ancestor in directory.ancestors() {
            if ancestor.metadata()?.dev() != device {
                break;
            }
            std::fs::File::open(ancestor)?.sync_all()?;
        }
    }
    Ok(())
}

fn write_bmp<W: Write>(w: &mut W, width: usize, height: usize, bgra: &[u8]) -> Result<()> {
    let pixel_bytes = width * height * 4;
    let file_size = 14 + 40 + pixel_bytes;

    w.write_all(b"BM")?;
    w.write_all(&(file_size as u32).to_le_bytes())?;
    w.write_all(&[0; 4])?;
    w.write_all(&(54_u32).to_le_bytes())?;

    w.write_all(&(40_u32).to_le_bytes())?;
    w.write_all(&(width as i32).to_le_bytes())?;
    w.write_all(&(-(height as i32)).to_le_bytes())?;
    w.write_all(&(1_u16).to_le_bytes())?;
    w.write_all(&(32_u16).to_le_bytes())?;
    w.write_all(&(0_u32).to_le_bytes())?;
    w.write_all(&(pixel_bytes as u32).to_le_bytes())?;
    w.write_all(&(2835_i32).to_le_bytes())?;
    w.write_all(&(2835_i32).to_le_bytes())?;
    w.write_all(&(0_u32).to_le_bytes())?;
    w.write_all(&(0_u32).to_le_bytes())?;

    w.write_all(bgra)?;
    Ok(())
}

fn write_screenshot_png<W: Write>(
    w: &mut W,
    bgra: &[u8],
    width: usize,
    height: usize,
) -> Result<()> {
    let len = bgra.len();
    let mut rgba = vec![0u8; len];
    for i in (0..len).step_by(4) {
        if i + 3 < len {
            rgba[i] = bgra[i + 2]; // R
            rgba[i + 1] = bgra[i + 1]; // G
            rgba[i + 2] = bgra[i]; // B
            rgba[i + 3] = bgra[i + 3]; // A
        }
    }
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::write_buffer_with_format(
        &mut cursor,
        &rgba,
        width as u32,
        height as u32,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;
    w.write_all(cursor.get_ref())?;
    Ok(())
}

fn write_cover_art_png<W: Write>(w: &mut W, art: &renderer::CoverArt) -> Result<()> {
    let mut rgba = Vec::with_capacity(art.pixels.len() * 4);
    for color in &art.pixels {
        rgba.push(color.r);
        rgba.push(color.g);
        rgba.push(color.b);
        rgba.push(255);
    }
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::write_buffer_with_format(
        &mut cursor,
        &rgba,
        art.width,
        art.height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;
    w.write_all(cursor.get_ref())?;
    Ok(())
}
