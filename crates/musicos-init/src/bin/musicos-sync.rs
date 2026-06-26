use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use musicos_init::database::{LibraryCounts, Track};
use musicos_init::renderer::{Canvas, Color, Resolution};
use musicos_init::ui::{self, FontWeight};
use serde::Deserialize;
use walkdir::WalkDir;

const WIDTH: usize = 1280;
const HEIGHT: usize = 960;
const MIRROR_X: f32 = 608.0;
const MIRROR_Y: f32 = 108.0;
const MIRROR_W: f32 = 592.0;
const MIRROR_H: f32 = 440.0;

fn main() -> Result<()> {
    let target = HttpTarget::from_env()?;
    let upload_dir = std::env::var("MUSICOS_SYNC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./music"));
    let mut app = SyncApp::new(target, upload_dir);
    app.refresh_all();

    let mut window = Window::new(
        "MusicOS Sync",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(30);

    let mut bgra = vec![0_u8; WIDTH * HEIGHT * 4];
    let mut pixels = vec![0_u32; WIDTH * HEIGHT];
    let started = Instant::now();
    let mut last_refresh = Instant::now();
    let mut mouse_down_at: Option<(f32, f32)> = None;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for key in window.get_keys_pressed(KeyRepeat::No) {
            app.handle_key(key);
        }

        if window.get_mouse_down(MouseButton::Left) {
            if mouse_down_at.is_none() {
                mouse_down_at = window.get_mouse_pos(MouseMode::Clamp);
            }
        } else if let Some((sx, sy)) = mouse_down_at.take() {
            if let Some((ex, ey)) = window.get_mouse_pos(MouseMode::Clamp) {
                app.handle_pointer(sx, sy, ex, ey);
            }
        }

        if last_refresh.elapsed() >= Duration::from_millis(1500) {
            app.refresh_mirror();
            app.refresh_status();
            last_refresh = Instant::now();
        }

        draw(&app, started.elapsed().as_secs_f32(), &mut bgra)?;
        bgra_to_minifb(&bgra, &mut pixels);
        window.update_with_buffer(&pixels, WIDTH, HEIGHT)?;
    }

    Ok(())
}

struct SyncApp {
    target: HttpTarget,
    upload_dir: PathBuf,
    tracks: Vec<Track>,
    counts: LibraryCounts,
    mirror: Option<BmpFrame>,
    status: String,
    action_status: String,
    last_upload: String,
    theme_index: usize,
    language_index: usize,
    volume: f32,
}

impl SyncApp {
    fn new(target: HttpTarget, upload_dir: PathBuf) -> Self {
        Self {
            target,
            upload_dir,
            tracks: Vec::new(),
            counts: LibraryCounts::default(),
            mirror: None,
            status: "Starting...".to_string(),
            action_status: "Ready".to_string(),
            last_upload: "Press U to sync the folder".to_string(),
            theme_index: 0,
            language_index: 0,
            volume: 1.0,
        }
    }

    fn handle_key(&mut self, key: Key) {
        let result = match key {
            Key::R => {
                self.refresh_all();
                self.action_status = "Refreshed device state".to_string();
                Ok(())
            }
            Key::U => self.upload_folder(),
            Key::S => self.trigger_usb_sync(),
            Key::Up => self.send_button("Up"),
            Key::Down => self.send_button("Down"),
            Key::Left => self.send_button("Left"),
            Key::Right => self.send_button("Right"),
            Key::Enter | Key::Space => self.send_button("Confirm"),
            Key::Backspace | Key::B => self.send_button("Back"),
            Key::C => self.send_button("Context"),
            Key::H => self.send_button("Home"),
            Key::Key1 => self.play_track_hotkey(0),
            Key::Key2 => self.play_track_hotkey(1),
            Key::Key3 => self.play_track_hotkey(2),
            Key::Key4 => self.play_track_hotkey(3),
            Key::P => self.post_json("/api/control", r#"{"command":"play_pause"}"#),
            Key::N => self.post_json("/api/control", r#"{"command":"next"}"#),
            Key::V => self.post_json("/api/control", r#"{"command":"prev"}"#),
            Key::T => self.cycle_theme(),
            Key::L => self.cycle_language(),
            Key::Equal | Key::NumPadPlus => self.adjust_volume(0.1),
            Key::Minus | Key::NumPadMinus => self.adjust_volume(-0.1),
            _ => Ok(()),
        };

        if let Err(err) = result {
            self.action_status = format!("Command failed: {err:#}");
        } else {
            self.refresh_mirror();
            self.refresh_library();
            self.refresh_status();
        }
    }

    fn handle_pointer(&mut self, sx: f32, sy: f32, ex: f32, ey: f32) {
        let result = if point_in_rect(sx, sy, MIRROR_X, MIRROR_Y, MIRROR_W, MIRROR_H)
            || point_in_rect(ex, ey, MIRROR_X, MIRROR_Y, MIRROR_W, MIRROR_H)
        {
            self.send_pointer(sx, sy, ex, ey)
        } else if point_in_rect(ex, ey, 70.0, 442.0, 180.0, 54.0) {
            self.refresh_all();
            self.action_status = "Refreshed device state".to_string();
            Ok(())
        } else if point_in_rect(ex, ey, 290.0, 442.0, 180.0, 54.0) {
            self.upload_folder()
        } else if point_in_rect(ex, ey, 70.0, 510.0, 180.0, 54.0) {
            self.trigger_usb_sync()
        } else if point_in_rect(ex, ey, 290.0, 510.0, 180.0, 54.0) {
            self.send_button("Home")
        } else if point_in_rect(ex, ey, 70.0, 578.0, 180.0, 54.0) {
            self.send_button("Back")
        } else if point_in_rect(ex, ey, 290.0, 578.0, 180.0, 54.0) {
            self.send_button("Context")
        } else if point_in_rect(ex, ey, 640.0, 580.0, 140.0, 54.0) {
            self.post_json("/api/control", r#"{"command":"prev"}"#)
        } else if point_in_rect(ex, ey, 796.0, 580.0, 160.0, 54.0) {
            self.post_json("/api/control", r#"{"command":"play_pause"}"#)
        } else if point_in_rect(ex, ey, 972.0, 580.0, 140.0, 54.0) {
            self.post_json("/api/control", r#"{"command":"next"}"#)
        } else if let Some(index) = track_index_at(ex, ey, self.tracks.len()) {
            self.play_track_hotkey(index)
        } else {
            Ok(())
        };

        if let Err(err) = result {
            self.action_status = format!("Command failed: {err:#}");
        } else {
            self.refresh_mirror();
            self.refresh_library();
            self.refresh_status();
        }
    }

    fn refresh_all(&mut self) {
        self.refresh_status();
        self.refresh_library();
        self.refresh_mirror();
    }

    fn refresh_status(&mut self) {
        match self.target.get("/api/status") {
            Ok(body) => match serde_json::from_slice::<StatusPayload>(&body) {
                Ok(payload) => {
                    self.counts = payload.counts.unwrap_or_default();
                    self.status = match payload.db_path {
                        Some(path) => format!("Connected to {} ({path})", self.target.label()),
                        None => format!("Connected to {}", self.target.label()),
                    };
                }
                Err(_) => self.status = format!("Connected to {}", self.target.label()),
            },
            Err(err) => self.status = format!("Offline: {err:#}"),
        }
    }

    fn refresh_library(&mut self) {
        match self.target.get("/api/tracks") {
            Ok(body) => match serde_json::from_slice::<Vec<Track>>(&body) {
                Ok(tracks) => self.tracks = tracks,
                Err(err) => self.status = format!("Track JSON failed: {err}"),
            },
            Err(err) => self.status = format!("Library fetch failed: {err:#}"),
        }
    }

    fn refresh_mirror(&mut self) {
        match self.target.get("/api/screenshot.bmp") {
            Ok(body) => match decode_bmp(&body) {
                Ok(frame) => self.mirror = Some(frame),
                Err(err) => self.status = format!("Mirror decode failed: {err:#}"),
            },
            Err(err) => self.status = format!("Mirror fetch failed: {err:#}"),
        }
    }

    fn send_button(&self, key: &str) -> Result<()> {
        self.post_empty(&format!("/api/input?key={key}"))
    }

    fn send_pointer(&mut self, sx: f32, sy: f32, ex: f32, ey: f32) -> Result<()> {
        let dx = ex - sx;
        let dy = ey - sy;
        if dx.abs().max(dy.abs()) < 36.0 {
            self.action_status = "Mirror tap".to_string();
            return self.post_empty("/api/pointer?action=tap");
        }
        let dir = if dx.abs() > dy.abs() {
            if dx > 0.0 {
                "right"
            } else {
                "left"
            }
        } else if dy > 0.0 {
            "down"
        } else {
            "up"
        };
        self.action_status = format!("Mirror swipe {dir}");
        self.post_empty(&format!("/api/pointer?action=swipe&dir={dir}"))
    }

    fn post_empty(&self, path: &str) -> Result<()> {
        self.target.post(path, Vec::new(), Vec::new()).map(|_| ())
    }

    fn post_json(&self, path: &str, body: &str) -> Result<()> {
        self.target
            .post(
                path,
                body.as_bytes().to_vec(),
                vec![("Content-Type".to_string(), "application/json".to_string())],
            )
            .map(|_| ())
    }

    fn trigger_usb_sync(&mut self) -> Result<()> {
        self.post_empty("/api/sync-usb")?;
        self.action_status = "Requested device USB sync".to_string();
        Ok(())
    }

    fn play_track_hotkey(&mut self, index: usize) -> Result<()> {
        let Some(track) = self.tracks.get(index) else {
            self.action_status = format!("No track at hotkey {}", index + 1);
            return Ok(());
        };
        let body = serde_json::json!({
            "command": "play_track",
            "id": track.id,
        })
        .to_string();
        self.post_json("/api/control", &body)?;
        self.action_status = format!("Playing {}", track.title);
        Ok(())
    }

    fn cycle_theme(&mut self) -> Result<()> {
        const THEMES: [&str; 4] = ["blue", "red", "green", "purple"];
        self.theme_index = (self.theme_index + 1) % THEMES.len();
        let name = THEMES[self.theme_index];
        let body = serde_json::json!({
            "command": "set_theme",
            "name": name,
        })
        .to_string();
        self.post_json("/api/control", &body)?;
        self.action_status = format!("Theme set to {name}");
        Ok(())
    }

    fn cycle_language(&mut self) -> Result<()> {
        const LANGUAGES: [&str; 5] = ["en", "es", "de", "ja", "zh"];
        self.language_index = (self.language_index + 1) % LANGUAGES.len();
        let name = LANGUAGES[self.language_index];
        let body = serde_json::json!({
            "command": "set_language",
            "name": name,
        })
        .to_string();
        self.post_json("/api/control", &body)?;
        self.action_status = format!("Language set to {name}");
        Ok(())
    }

    fn adjust_volume(&mut self, delta: f32) -> Result<()> {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        let body = serde_json::json!({
            "command": "volume",
            "value": self.volume,
        })
        .to_string();
        self.post_json("/api/control", &body)?;
        self.action_status = format!("Volume {}%", (self.volume * 100.0).round() as i32);
        Ok(())
    }

    fn upload_folder(&mut self) -> Result<()> {
        if !self.upload_dir.exists() {
            self.last_upload = format!("Folder not found: {}", self.upload_dir.display());
            self.action_status = self.last_upload.clone();
            return Ok(());
        }

        let mut uploaded = 0;
        let mut skipped = 0;
        for entry in WalkDir::new(&self.upload_dir).into_iter().flatten() {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_audio(path) {
                continue;
            }

            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                skipped += 1;
                continue;
            };
            let body =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            self.target.post(
                "/api/upload",
                body,
                vec![("X-Filename".to_string(), name.to_string())],
            )?;
            uploaded += 1;
        }

        self.last_upload = format!(
            "Uploaded {uploaded} file(s) from {}",
            self.upload_dir.display()
        );
        if skipped > 0 {
            self.last_upload.push_str(&format!("; skipped {skipped}"));
        }
        self.action_status = self.last_upload.clone();
        self.refresh_all();
        Ok(())
    }
}

#[derive(Deserialize)]
struct StatusPayload {
    counts: Option<LibraryCounts>,
    db_path: Option<String>,
}

#[derive(Clone)]
struct HttpTarget {
    host: String,
    port: u16,
}

impl HttpTarget {
    fn from_env() -> Result<Self> {
        let raw = std::env::var("MUSICOS_SYNC_TARGET")
            .unwrap_or_else(|_| "http://127.0.0.1:18080".to_string());
        Self::parse(&raw)
    }

    fn parse(raw: &str) -> Result<Self> {
        let without_scheme = raw
            .strip_prefix("http://")
            .or_else(|| raw.strip_prefix("https://"))
            .unwrap_or(raw);
        let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_string(), port.parse()?),
            None => (authority.to_string(), 80),
        };
        if host.is_empty() {
            return Err(anyhow!("empty target host"));
        }
        Ok(Self { host, port })
    }

    fn label(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    fn get(&self, path: &str) -> Result<Vec<u8>> {
        self.request("GET", path, Vec::new(), Vec::new())
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
        stream.set_read_timeout(Some(Duration::from_secs(4)))?;
        stream.set_write_timeout(Some(Duration::from_secs(4)))?;

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
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("bad HTTP response"))?;
    let header = std::str::from_utf8(&response[..header_end])?;
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(anyhow!("HTTP {status}"));
    }
    Ok(response[header_end + 4..].to_vec())
}

#[derive(Clone)]
struct BmpFrame {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

fn decode_bmp(bytes: &[u8]) -> Result<BmpFrame> {
    if bytes.len() < 54 || &bytes[0..2] != b"BM" {
        return Err(anyhow!("not a BMP"));
    }
    let pixel_offset = u32::from_le_bytes(bytes[10..14].try_into()?) as usize;
    let width = i32::from_le_bytes(bytes[18..22].try_into()?);
    let height_raw = i32::from_le_bytes(bytes[22..26].try_into()?);
    let bpp = u16::from_le_bytes(bytes[28..30].try_into()?);
    if width <= 0 || height_raw == 0 || bpp != 32 {
        return Err(anyhow!("unsupported BMP format"));
    }

    let width = width as usize;
    let height = height_raw.unsigned_abs() as usize;
    let top_down = height_raw < 0;
    let row_stride = width * 4;
    if bytes.len() < pixel_offset + row_stride * height {
        return Err(anyhow!("truncated BMP"));
    }

    let mut pixels = vec![Color::BLACK; width * height];
    for y in 0..height {
        let src_y = if top_down { y } else { height - 1 - y };
        let row = pixel_offset + src_y * row_stride;
        for x in 0..width {
            let offset = row + x * 4;
            let b = bytes[offset];
            let g = bytes[offset + 1];
            let r = bytes[offset + 2];
            pixels[y * width + x] = Color::rgb(r, g, b);
        }
    }
    Ok(BmpFrame {
        width,
        height,
        pixels,
    })
}

fn draw(app: &SyncApp, seconds: f32, bgra: &mut [u8]) -> Result<()> {
    let resolution = Resolution {
        width: WIDTH as u32,
        height: HEIGHT as u32,
    };
    let mut canvas = Canvas::new(resolution, bgra)?;
    ui::draw_figma_background(&mut canvas);
    ui::panel_gradient(
        &mut canvas,
        20.0,
        32.0,
        500.0,
        896.0,
        128.0,
        Color::rgb(178, 246, 228),
        Color::WHITE,
    );
    ui::panel_gradient(
        &mut canvas,
        548.0,
        32.0,
        712.0,
        896.0,
        128.0,
        Color::WHITE,
        Color::rgb(233, 243, 255),
    );

    canvas.text_centered(
        "MusicOS Sync",
        270.0,
        70.0,
        72.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    ui::text_fit_centered(
        &mut canvas,
        &app.status,
        270.0,
        160.0,
        420.0,
        28.0,
        Color::BLACK,
        FontWeight::Regular,
    );
    ui::text_fit_centered(
        &mut canvas,
        &app.action_status,
        270.0,
        198.0,
        420.0,
        28.0,
        Color::BLACK,
        FontWeight::Bold,
    );

    stat(&mut canvas, "Tracks", app.counts.tracks, 250.0);
    stat(&mut canvas, "Albums", app.counts.albums, 300.0);
    stat(&mut canvas, "Playlists", app.counts.playlists, 350.0);
    stat(&mut canvas, "Pins", app.counts.pins, 400.0);

    canvas.text_centered(
        "Device Controls",
        270.0,
        410.0,
        34.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    sync_button(&mut canvas, 70.0, 442.0, 180.0, "Refresh");
    sync_button(&mut canvas, 290.0, 442.0, 180.0, "Upload");
    sync_button(&mut canvas, 70.0, 510.0, 180.0, "USB Sync");
    sync_button(&mut canvas, 290.0, 510.0, 180.0, "Home");
    sync_button(&mut canvas, 70.0, 578.0, 180.0, "Back");
    sync_button(&mut canvas, 290.0, 578.0, 180.0, "Menu");

    ui::centered_lines(
        &mut canvas,
        &["Mirror: click to tap, drag to swipe", "Keys still work"],
        270.0,
        652.0,
        26.0,
        34.0,
        Color::BLACK,
        FontWeight::Bold,
    );

    ui::text_fit_centered(
        &mut canvas,
        &app.last_upload,
        270.0,
        760.0,
        420.0,
        28.0,
        Color::BLACK,
        FontWeight::Regular,
    );

    let pulse = (seconds * 2.0).sin() * 0.5 + 0.5;
    let mirror_alpha = (55.0 + pulse * 35.0) as u8;
    ui::liquid_glass(&mut canvas, 596.0, 96.0, 616.0, 472.0, 32.0, mirror_alpha);
    ui::shadow_rr(&mut canvas, 600.0, 100.0, 608.0, 456.0, 28.0, 8.0, 20.0);
    ui::rr(&mut canvas, 600.0, 100.0, 608.0, 456.0, 28.0, Color::BLACK);

    if let Some(frame) = &app.mirror {
        draw_frame(&mut canvas, frame, MIRROR_X, MIRROR_Y, MIRROR_W, MIRROR_H);
    } else {
        canvas.text_centered(
            "No mirror yet",
            904.0,
            300.0,
            48.0,
            Color::WHITE,
            FontWeight::Bold,
        );
    }

    canvas.text_centered(
        "Library",
        904.0,
        610.0,
        56.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    sync_button(&mut canvas, 640.0, 580.0, 140.0, "Prev");
    sync_button(&mut canvas, 796.0, 580.0, 160.0, "Play");
    sync_button(&mut canvas, 972.0, 580.0, 140.0, "Next");

    if app.tracks.is_empty() {
        canvas.text_centered(
            "No tracks indexed",
            904.0,
            700.0,
            42.0,
            Color::BLACK,
            FontWeight::Regular,
        );
    } else {
        for (idx, track) in app.tracks.iter().take(4).enumerate() {
            let y = 675.0 + idx as f32 * 58.0;
            ui::liquid_glass(&mut canvas, 640.0, y, 528.0, 46.0, 23.0, 44);
            let title = format!("{}. {}", idx + 1, track.title);
            ui::text_fit(
                &mut canvas,
                &title,
                664.0,
                y + 8.0,
                330.0,
                28.0,
                Color::BLACK,
                FontWeight::Bold,
            );
            ui::text_fit(
                &mut canvas,
                &track.artist,
                1000.0,
                y + 10.0,
                140.0,
                24.0,
                Color::rgb(40, 40, 40),
                FontWeight::Regular,
            );
        }
    }

    Ok(())
}

fn stat(canvas: &mut Canvas<'_>, label: &str, value: i64, y: f32) {
    canvas.text(label, 100.0, y, 34.0, Color::BLACK, FontWeight::Bold);
    canvas.text_right(
        &value.to_string(),
        440.0,
        y,
        34.0,
        Color::BLACK,
        FontWeight::Bold,
    );
}

fn sync_button(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, label: &str) {
    ui::shadow_rr(canvas, x, y, w, 54.0, 27.0, 3.0, 8.0);
    ui::liquid_glass(canvas, x, y, w, 54.0, 27.0, 155);
    ui::rr_stroke(
        canvas,
        x,
        y,
        w,
        54.0,
        27.0,
        2.0,
        Color::rgba(255, 255, 255, 160),
    );
    canvas.text_centered(
        label,
        x + w / 2.0,
        y + 12.0,
        30.0,
        Color::BLACK,
        FontWeight::Bold,
    );
}

fn draw_frame(canvas: &mut Canvas<'_>, frame: &BmpFrame, x: f32, y: f32, w: f32, h: f32) {
    let scale = (w / frame.width as f32).min(h / frame.height as f32);
    let draw_w = (frame.width as f32 * scale).round() as i32;
    let draw_h = (frame.height as f32 * scale).round() as i32;
    let x0 = (x + (w - draw_w as f32) / 2.0).round() as i32;
    let y0 = (y + (h - draw_h as f32) / 2.0).round() as i32;

    for dy in 0..draw_h {
        let sy = (dy as f32 / scale).floor() as usize;
        for dx in 0..draw_w {
            let sx = (dx as f32 / scale).floor() as usize;
            let color =
                frame.pixels[sy.min(frame.height - 1) * frame.width + sx.min(frame.width - 1)];
            canvas.put_pixel(x0 + dx, y0 + dy, color);
        }
    }
}

fn bgra_to_minifb(src: &[u8], dst: &mut [u32]) {
    for (pixel, bgra) in dst.iter_mut().zip(src.chunks_exact(4)) {
        let b = bgra[0] as u32;
        let g = bgra[1] as u32;
        let r = bgra[2] as u32;
        *pixel = (r << 16) | (g << 8) | b;
    }
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg"
            )
        })
        .unwrap_or(false)
}

fn point_in_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    px >= x && px <= x + w && py >= y && py <= y + h
}

fn track_index_at(x: f32, y: f32, track_count: usize) -> Option<usize> {
    if !(640.0..=1168.0).contains(&x) {
        return None;
    }
    for index in 0..track_count.min(4) {
        let row_y = 675.0 + index as f32 * 58.0;
        if point_in_rect(x, y, 640.0, row_y, 528.0, 46.0) {
            return Some(index);
        }
    }
    None
}
