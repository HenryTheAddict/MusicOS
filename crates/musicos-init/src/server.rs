use std::io::Write;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::thread;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Response, Server, StatusCode};

use crate::audio::PlaybackStatus;
use crate::database::{LibraryCounts, Track};
use crate::input::{Button, InputEvent, SwipeDirection};
use crate::preferences::Preferences;
use crate::renderer;

pub enum ServerEvent {
    Button(Button),
    Pointer(InputEvent),
    Control(ControlCommand),
    UploadComplete,
    UsbSync,
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
    pub first_run_audio_test_complete: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ControlCommand {
    PlayPause,
    Next,
    Prev,
    Volume { value: f32 },
    Brightness { value: u8 },
    Seek { value: f32 },
    Shuffle,
    Loop,
    PlayTrack { id: i64 },
    PlayPlaylist { id: i64, start_index: usize },
    SetTheme { name: String },
    SetLanguage { name: String },
    ToggleFavorite { id: i64 },
    DeletePlaylist { id: i64 },
    RenamePlaylist { id: i64, name: String },
    CreatePlaylist { name: String },
    AddTrackToPlaylist { playlist_id: i64, track_id: i64 },
    RemoveTrackFromPlaylist { playlist_id: i64, track_id: i64 },
    SetSettings { settings: SettingsUpdate },
    ResetSettings,
    ScanLibrary,
    AudioTest,
    QueueAdd { id: i64, position: Option<String> },
    QueueRemove { index: usize },
    QueueMove { from: usize, to: usize },
    QueueClear,
    CleanupMissingFiles,
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
    pub usb_status: String,
    pub battery_status: String,
}

static RUNTIME_SNAPSHOT: OnceLock<Mutex<Option<RuntimeSnapshot>>> = OnceLock::new();

pub fn publish_runtime_snapshot(snapshot: RuntimeSnapshot) {
    let lock = RUNTIME_SNAPSHOT.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = lock.lock() {
        *current = Some(snapshot);
    }
}

fn runtime_snapshot() -> Option<RuntimeSnapshot> {
    RUNTIME_SNAPSHOT
        .get()
        .and_then(|lock| lock.lock().ok().and_then(|snapshot| snapshot.clone()))
}

pub struct CompanionServer {
    server: Server,
    tx: Sender<ServerEvent>,
    db_path: String,
}

impl CompanionServer {
    pub fn start(port: u16, tx: Sender<ServerEvent>, db_path: String) -> Result<()> {
        let server = Server::http(format!("0.0.0.0:{port}"))
            .map_err(|e| anyhow::anyhow!("failed to start HTTP server: {e}"))?;
        log::info!("Companion HTTP server listening on port {port}");

        let companion = Self {
            server,
            tx,
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
            let method = request.method().as_str();

            if url == "/" || url == "/index.html" {
                let response = Response::from_string(
                    "{\"status\":\"ok\",\"app\":\"MusicOS API\",\"sync_app\":\"npm run sync:electron\"}",
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

            if url == "/api/settings" && method == "GET" {
                let response = json_response(&crate::preferences::load());
                let _ = request.respond(response);
                continue;
            }

            if url == "/api/settings" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                if let Ok(settings) = serde_json::from_str::<SettingsUpdate>(&body) {
                    let _ = self
                        .tx
                        .send(ServerEvent::Control(ControlCommand::SetSettings { settings }));
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

            if url == "/api/player" && method == "GET" {
                let body = if let Some(snapshot) = runtime_snapshot() {
                    serde_json::json!({
                        "status": "ok",
                        "player": snapshot.player,
                        "queue": snapshot.queue,
                        "queue_index": snapshot.queue_index,
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
                let _ = request.respond(response);
                continue;
            }

            if url == "/api/diagnostics" && method == "GET" {
                let body = if let Some(snapshot) = runtime_snapshot() {
                    serde_json::json!({
                        "status": "ok",
                        "db_path": snapshot.db_path,
                        "settings_path": snapshot.settings_path,
                        "counts": snapshot.counts,
                        "audio": crate::system::audio_diagnostics(),
                        "player": snapshot.player,
                        "scan_status": snapshot.scan_status,
                        "usb_status": snapshot.usb_status,
                        "battery_status": snapshot.battery_status,
                        "settings": snapshot.settings,
                    })
                } else {
                    let counts = crate::database::Db::open(&self.db_path)
                        .and_then(|db| db.library_counts())
                        .unwrap_or_default();
                    serde_json::json!({
                        "status": "warming_up",
                        "db_path": self.db_path,
                        "settings_path": crate::preferences::path(),
                        "counts": counts,
                        "audio": crate::system::audio_diagnostics(),
                    })
                };
                let response = json_response(&body);
                let _ = request.respond(response);
                continue;
            }

            if url.starts_with("/api/screenshot.bmp") {
                if let Some(bmp) = renderer::get_latest_screenshot() {
                    let mut response_bytes = Vec::new();
                    if write_screenshot_bmp(&mut response_bytes, &bmp).is_ok() {
                        let response = Response::from_data(response_bytes)
                            .with_header(
                                Header::from_bytes(&b"Content-Type"[..], &b"image/bmp"[..])
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

            if url.starts_with("/api/cover-art.bmp") && method == "GET" {
                let id = url
                    .split("id=")
                    .nth(1)
                    .and_then(|value| {
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
                            if let Some(art) = crate::media::cached_cover_art(&track.path) {
                                let mut response_bytes = Vec::new();
                                if write_cover_art_bmp(&mut response_bytes, &art).is_ok() {
                                    let response = Response::from_data(response_bytes)
                                        .with_header(
                                            Header::from_bytes(
                                                &b"Content-Type"[..],
                                                &b"image/bmp"[..],
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

            if url.starts_with("/api/input") && method == "POST" {
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

            if url == "/api/sync-usb" && method == "POST" {
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

            if url.starts_with("/api/pointer") && method == "POST" {
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

            if url == "/api/control" && method == "POST" {
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                if let Ok(cmd) = serde_json::from_str::<ControlCommand>(&body) {
                    let _ = self.tx.send(ServerEvent::Control(cmd));
                    let response = Response::from_string("{\"status\":\"ok\"}").with_header(
                        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    );
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

            if url == "/api/upload" && method == "POST" {
                let headers = request.headers();
                let filename = headers
                    .iter()
                    .find(|h| h.field.to_string().to_lowercase() == "x-filename")
                    .map(|h| h.value.as_str().to_string())
                    .unwrap_or_else(|| "upload.ogg".to_string());
                let filename = sanitize_filename(&filename);

                let mut body = Vec::new();
                let _ = request.as_reader().read_to_end(&mut body);

                let upload_dir = "./music";
                let _ = std::fs::create_dir_all(upload_dir);
                let dest_path = std::path::Path::new(upload_dir).join(&filename);

                match std::fs::write(&dest_path, &body) {
                    Ok(_) => {
                        log::info!("Uploaded file saved to {}", dest_path.display());
                        let _ = self.tx.send(ServerEvent::UploadComplete);
                        let response = Response::from_string("{\"status\":\"ok\"}").with_header(
                            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                                .unwrap(),
                        );
                        let _ = request.respond(response);
                    }
                    Err(err) => {
                        log::error!("failed to write uploaded file: {err}");
                        let response = Response::from_string(
                            "{\"status\":\"error\",\"message\":\"failed to write file\"}",
                        )
                        .with_status_code(StatusCode(500))
                        .with_header(
                            Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                                .unwrap(),
                        );
                        let _ = request.respond(response);
                    }
                }
                continue;
            }

            if url.starts_with("/api/status") && method == "GET" {
                let body = if let Ok(db) = crate::database::Db::open(&self.db_path) {
                    let counts = db.library_counts().unwrap_or_default();
                    serde_json::json!({
                        "status": "ok",
                        "db_path": self.db_path,
                        "counts": counts,
                    })
                } else {
                    serde_json::json!({
                        "status": "db_unavailable",
                        "db_path": self.db_path,
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

            if url.starts_with("/api/tracks") && method == "GET" {
                if let Ok(db) = crate::database::Db::open(&self.db_path) {
                    if url.contains("playlists=1") || url.contains("type=playlists") {
                        if let Ok(playlists) = db.get_playlists() {
                            if let Ok(json) = serde_json::to_string(&playlists) {
                                let response = Response::from_string(json)
                                    .with_header(
                                        Header::from_bytes(
                                            &b"Content-Type"[..],
                                            &b"application/json"[..],
                                        )
                                        .unwrap(),
                                    )
                                    .with_header(
                                        Header::from_bytes(
                                            &b"Access-Control-Allow-Origin"[..],
                                            &b"*"[..],
                                        )
                                        .unwrap(),
                                    );
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    } else if let Some(playlist_id_str) = url.split("playlist_id=").nth(1) {
                        let playlist_id_digits: String = playlist_id_str
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(playlist_id) = playlist_id_digits.parse::<i64>() {
                            if let Ok(tracks) = db.get_playlist_tracks(playlist_id) {
                                if let Ok(json) = serde_json::to_string(&tracks) {
                                    let response = Response::from_string(json)
                                        .with_header(
                                            Header::from_bytes(
                                                &b"Content-Type"[..],
                                                &b"application/json"[..],
                                            )
                                            .unwrap(),
                                        )
                                        .with_header(
                                            Header::from_bytes(
                                                &b"Access-Control-Allow-Origin"[..],
                                                &b"*"[..],
                                            )
                                            .unwrap(),
                                        );
                                    let _ = request.respond(response);
                                    continue;
                                }
                            }
                        }
                    } else {
                        if let Ok(tracks) = db.get_tracks() {
                            if let Ok(json) = serde_json::to_string(&tracks) {
                                let response = Response::from_string(json)
                                    .with_header(
                                        Header::from_bytes(
                                            &b"Content-Type"[..],
                                            &b"application/json"[..],
                                        )
                                        .unwrap(),
                                    )
                                    .with_header(
                                        Header::from_bytes(
                                            &b"Access-Control-Allow-Origin"[..],
                                            &b"*"[..],
                                        )
                                        .unwrap(),
                                    );
                                let _ = request.respond(response);
                                continue;
                            }
                        }
                    }
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
                            &b"GET, POST, OPTIONS"[..],
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

#[cfg(test)]
mod tests {
    use super::sanitize_filename;

    #[test]
    fn upload_filenames_are_sanitized() {
        assert_eq!(sanitize_filename("../evil.mp3"), "evil.mp3");
        assert_eq!(sanitize_filename("artist/song?.wav"), "song_.wav");
        assert_eq!(sanitize_filename(""), "upload.ogg");
    }
}

fn write_screenshot_bmp<W: Write>(w: &mut W, bgra: &[u8]) -> Result<()> {
    let resolution = renderer::get_latest_screenshot()
        .map(|b| {
            let len = b.len();
            if len == 1280 * 960 * 4 {
                (1280, 960)
            } else {
                (640, 480)
            }
        })
        .unwrap_or((1280, 960));

    let width = resolution.0;
    let height = resolution.1;
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
