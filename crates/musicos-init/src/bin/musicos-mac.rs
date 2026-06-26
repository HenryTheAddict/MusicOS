use std::time::Instant;

use anyhow::Result;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use musicos_init::app::MusicOs;
use musicos_init::input::{Button, InputEvent, SwipeDirection};
use musicos_init::media;
use musicos_init::renderer::{Canvas, Resolution};
use musicos_init::server::CompanionServer;

fn main() -> Result<()> {
    let resolution = Resolution::from_env_or_default();
    let width = resolution.width as usize;
    let height = resolution.height as usize;
    let mut window = Window::new(
        "MusicOS Mac App Test",
        width,
        height,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);

    let db_path = std::env::var("MUSICOS_MEDIA_DB").unwrap_or_else(|_| {
        let _ = std::fs::create_dir_all("out");
        "out/mac-media.sqlite3".to_string()
    });
    let mut app = MusicOs::with_db_path(resolution, &db_path);
    let started_at = Instant::now();
    let mut bgra = vec![0_u8; width * height * 4];
    let mut pixels = vec![0_u32; width * height];
    let mut mouse_down_at: Option<(f32, f32)> = None;

    let (tx, rx) = std::sync::mpsc::channel();
    CompanionServer::start(companion_port(), tx, db_path)?;

    if let Err(err) = media::index_default_locations(&app.db) {
        log::info!("media indexing skipped: {err:#}");
    }

    let target_fps = 60.0;
    let frame_duration = std::time::Duration::from_secs_f32(1.0 / target_fps);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        app.tick();

        for event in keyboard_events(&window) {
            app.handle_input(event);
        }

        if window.get_mouse_down(MouseButton::Left) {
            if mouse_down_at.is_none() {
                mouse_down_at = window.get_mouse_pos(MouseMode::Clamp);
            }
        } else if let Some((sx, sy)) = mouse_down_at.take() {
            if let Some((ex, ey)) = window.get_mouse_pos(MouseMode::Clamp) {
                app.handle_input(pointer_event(sx, sy, ex, ey));
            }
        }

        while let Ok(event) = rx.try_recv() {
            app.handle_server_event(event);
        }

        let mut canvas = Canvas::new(resolution, &mut bgra)?;
        app.draw(&mut canvas, started_at.elapsed().as_secs_f32());
        musicos_init::renderer::update_latest_screenshot(&bgra);
        bgra_to_minifb(&bgra, &mut pixels);

        window.update_with_buffer(&pixels, width, height)?;

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    Ok(())
}

fn map_minifb_key(key: Key) -> Option<Button> {
    match key {
        Key::Enter | Key::Space | Key::J => Some(Button::Confirm),
        Key::Backspace | Key::H => Some(Button::Back),
        Key::Up | Key::W => Some(Button::Up),
        Key::Down | Key::S => Some(Button::Down),
        Key::Left | Key::A => Some(Button::Left),
        Key::Right | Key::D => Some(Button::Right),
        Key::X | Key::U => Some(Button::Context),
        Key::Y | Key::I => Some(Button::Home),
        Key::M => Some(Button::Menu),
        Key::P => Some(Button::Sleep),
        Key::Equal => Some(Button::VolumeUp),
        Key::Minus => Some(Button::VolumeDown),
        _ => None,
    }
}

fn keyboard_events(window: &Window) -> Vec<InputEvent> {
    let mut events = Vec::new();
    for key in window.get_keys_pressed(KeyRepeat::No) {
        if let Some(button) = map_minifb_key(key) {
            events.push(InputEvent::Button(button));
        } else if key == Key::Escape {
        } else {
            events.push(InputEvent::AnyButton);
        }
    }
    for key in window.get_keys_released() {
        if let Some(button) = map_minifb_key(key) {
            events.push(InputEvent::ButtonRelease(button));
        }
    }
    events
}

fn pointer_event(sx: f32, sy: f32, ex: f32, ey: f32) -> InputEvent {
    let dx = ex - sx;
    let dy = ey - sy;
    if dx.abs().max(dy.abs()) < 40.0 {
        return InputEvent::Tap;
    }
    if dx.abs() > dy.abs() {
        if dx > 0.0 {
            InputEvent::Swipe(SwipeDirection::Right)
        } else {
            InputEvent::Swipe(SwipeDirection::Left)
        }
    } else if dy > 0.0 {
        InputEvent::Swipe(SwipeDirection::Down)
    } else {
        InputEvent::Swipe(SwipeDirection::Up)
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

fn companion_port() -> u16 {
    std::env::var("MUSICOS_COMPANION_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(18080)
}
