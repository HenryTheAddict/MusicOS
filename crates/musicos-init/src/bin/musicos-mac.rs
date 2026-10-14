use std::time::Instant;

use anyhow::Result;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use musicos_init::app::{LibraryScanPolicy, MusicOs, MusicOsOptions};
use musicos_init::input::{Button, InputEvent, SwipeDirection};
use musicos_init::renderer::{Canvas, Color, Resolution};
use musicos_init::server::{CompanionServer, ServerEvent};
use musicos_init::ui::FontWeight;

const CONTROLS_BASE_HEIGHT: usize = 160;

#[derive(Clone, Copy, Debug)]
struct ControllerButton {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    button: Button,
    label: &'static str,
    sublabel: &'static str,
}

fn get_controller_buttons(scale: f32) -> Vec<ControllerButton> {
    let s = |v: f32| v * scale;
    vec![
        // D-Pad
        ControllerButton { x: s(76.0), y: s(30.0), w: s(40.0), h: s(34.0), button: Button::Up, label: "▲", sublabel: "W/↑" },
        ControllerButton { x: s(34.0), y: s(66.0), w: s(40.0), h: s(34.0), button: Button::Left, label: "◀", sublabel: "A/←" },
        ControllerButton { x: s(76.0), y: s(66.0), w: s(40.0), h: s(34.0), button: Button::Confirm, label: "OK", sublabel: "↵" },
        ControllerButton { x: s(118.0), y: s(66.0), w: s(40.0), h: s(34.0), button: Button::Right, label: "▶", sublabel: "D/→" },
        ControllerButton { x: s(76.0), y: s(102.0), w: s(40.0), h: s(34.0), button: Button::Down, label: "▼", sublabel: "S/↓" },
        // Actions
        ControllerButton { x: s(184.0), y: s(42.0), w: s(90.0), h: s(40.0), button: Button::Back, label: "◂ Back", sublabel: "K / ⌫" },
        ControllerButton { x: s(284.0), y: s(42.0), w: s(90.0), h: s(40.0), button: Button::Home, label: "⌂ Home", sublabel: "I / Y" },
        ControllerButton { x: s(184.0), y: s(92.0), w: s(90.0), h: s(40.0), button: Button::Menu, label: "☰ Menu", sublabel: "M" },
        ControllerButton { x: s(284.0), y: s(92.0), w: s(90.0), h: s(40.0), button: Button::Context, label: "⚙ Opts", sublabel: "U / X" },
        // Playback & Volume
        ControllerButton { x: s(404.0), y: s(42.0), w: s(68.0), h: s(40.0), button: Button::PreviousTrack, label: "⏮ Prev", sublabel: "," },
        ControllerButton { x: s(480.0), y: s(42.0), w: s(72.0), h: s(40.0), button: Button::PlayPause, label: "⏯ Play", sublabel: "Space" },
        ControllerButton { x: s(560.0), y: s(42.0), w: s(68.0), h: s(40.0), button: Button::NextTrack, label: "Next ⏭", sublabel: "." },
        ControllerButton { x: s(404.0), y: s(92.0), w: s(70.0), h: s(40.0), button: Button::VolumeDown, label: "🔉 -", sublabel: "-" },
        ControllerButton { x: s(482.0), y: s(92.0), w: s(70.0), h: s(40.0), button: Button::VolumeUp, label: "🔊 +", sublabel: "+" },
        ControllerButton { x: s(560.0), y: s(92.0), w: s(68.0), h: s(40.0), button: Button::Sleep, label: "⏻ Sleep", sublabel: "P" },
    ]
}

fn find_controller_button(x: f32, y: f32, scale: f32) -> Option<Button> {
    for btn in get_controller_buttons(scale) {
        if x >= btn.x && x <= btn.x + btn.w && y >= btn.y && y <= btn.y + btn.h {
            return Some(btn.button);
        }
    }
    None
}

fn render_controller_panel(
    canvas: &mut Canvas<'_>,
    active_button: Option<Button>,
    scale: f32,
    dark: bool,
) {
    let width = canvas.resolution().width as f32;
    let _height = canvas.resolution().height as f32;
    let bg = if dark { Color::rgb(20, 24, 32) } else { Color::rgb(240, 243, 248) };
    let border = if dark { Color::rgb(42, 50, 66) } else { Color::rgb(203, 213, 225) };
    let text = if dark { Color::rgb(241, 245, 249) } else { Color::rgb(15, 23, 42) };
    let muted = if dark { Color::rgb(148, 163, 184) } else { Color::rgb(100, 116, 139) };
    let btn_bg = if dark { Color::rgb(30, 38, 52) } else { Color::rgb(255, 255, 255) };
    let btn_border = if dark { Color::rgb(55, 65, 85) } else { Color::rgb(226, 232, 240) };
    let active_bg = Color::rgb(37, 99, 235);
    let active_border = Color::rgb(96, 165, 250);

    canvas.clear(bg);
    musicos_init::ui::rr(canvas, 0.0, 0.0, width, 2.0 * scale, 0.0, border);

    canvas.text("HARDWARE CONTROLS", 34.0 * scale, 8.0 * scale, 13.0 * scale, muted, FontWeight::Bold);
    canvas.text("Click controls or use keyboard (Arrows, Return, Space, Backspace) · No touchpad swipe needed", 175.0 * scale, 9.0 * scale, 12.0 * scale, muted, FontWeight::Regular);

    musicos_init::ui::rr(canvas, 172.0 * scale, 28.0 * scale, 1.0 * scale, 116.0 * scale, 0.0, border);
    musicos_init::ui::rr(canvas, 392.0 * scale, 28.0 * scale, 1.0 * scale, 116.0 * scale, 0.0, border);

    for btn in get_controller_buttons(scale) {
        let is_active = active_button == Some(btn.button);
        let fill = if is_active { active_bg } else { btn_bg };
        let outline = if is_active { active_border } else { btn_border };
        let label_color = if is_active { Color::WHITE } else { text };
        let sub_color = if is_active { Color::rgb(219, 234, 254) } else { muted };

        musicos_init::ui::rr(canvas, btn.x, btn.y, btn.w, btn.h, 6.0 * scale, fill);
        musicos_init::ui::rr_stroke(canvas, btn.x, btn.y, btn.w, btn.h, 6.0 * scale, 1.5 * scale, outline);

        if btn.h >= 36.0 * scale && !btn.sublabel.is_empty() {
            canvas.text(btn.label, btn.x + 8.0 * scale, btn.y + 4.0 * scale, 14.0 * scale, label_color, FontWeight::Bold);
            canvas.text(btn.sublabel, btn.x + 8.0 * scale, btn.y + 20.0 * scale, 11.0 * scale, sub_color, FontWeight::Regular);
        } else {
            canvas.text(btn.label, btn.x + 10.0 * scale, btn.y + 7.0 * scale, 15.0 * scale, label_color, FontWeight::Bold);
        }
    }
}

fn main() -> Result<()> {
    let resolution = Resolution::from_env_or_default();
    let screen_width = resolution.width as usize;
    let screen_height = resolution.height as usize;
    let scale = screen_width as f32 / 640.0;
    let controls_height = (CONTROLS_BASE_HEIGHT as f32 * scale).round() as usize;
    let window_width = screen_width;
    let window_height = screen_height + controls_height;

    let mut window = Window::new(
        "MusicOS Mac App Test - Controls Active",
        window_width,
        window_height,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;
    window.set_position(56, 72);
    window.set_target_fps(60);

    let mut controls_window: Option<Window> = None;
    let controls_pixels = render_controls_reference(system_uses_dark_appearance())?;

    let db_path = std::env::var("MUSICOS_MEDIA_DB").unwrap_or_else(|_| {
        let _ = std::fs::create_dir_all("out");
        "out/mac-media.sqlite3".to_string()
    });
    let mut app = MusicOs::with_options(
        resolution,
        MusicOsOptions {
            db_path: db_path.clone().into(),
            scan_policy: LibraryScanPolicy::ReadOnlyPreview,
        },
    );
    let started_at = Instant::now();
    let mut bgra = vec![0_u8; screen_width * screen_height * 4];
    let mut controller_bgra = vec![0_u8; window_width * controls_height * 4];
    let mut pixels = vec![0_u32; window_width * window_height];
    let mut active_button: Option<Button> = None;
    let mut screen_clicked = false;

    let (tx, rx) = std::sync::mpsc::channel();
    CompanionServer::start(companion_port(), tx, db_path)?;

    let controls_resolution = Resolution {
        width: window_width as u32,
        height: controls_height as u32,
    };

    while window.is_open() && !window.is_key_down(Key::Q) {
        let frame_start = Instant::now();

        app.tick();

        let (events, show_controls) = keyboard_events(&window);
        for event in events {
            app.handle_input(event);
        }
        if show_controls && controls_window.is_none() {
            controls_window = Some(create_controls_window(window_width)?);
        }

        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let mouse_pos = window.get_mouse_pos(MouseMode::Clamp);

        if mouse_down {
            if let Some((mx, my)) = mouse_pos {
                if my >= screen_height as f32 {
                    let local_y = my - screen_height as f32;
                    let target_btn = find_controller_button(mx, local_y, scale);
                    if target_btn != active_button {
                        if let Some(old_btn) = active_button.take() {
                            app.handle_input(InputEvent::ButtonRelease(old_btn));
                        }
                        if let Some(new_btn) = target_btn {
                            active_button = Some(new_btn);
                            app.handle_input(InputEvent::Button(new_btn));
                        }
                    }
                } else if !screen_clicked {
                    screen_clicked = true;
                    app.handle_input(InputEvent::Tap);
                }
            }
        } else {
            if let Some(released_btn) = active_button.take() {
                app.handle_input(InputEvent::ButtonRelease(released_btn));
            }
            screen_clicked = false;
        }

        while let Ok(event) = rx.try_recv() {
            if matches!(event, ServerEvent::ShowControls) {
                controls_window = Some(create_controls_window(window_width)?);
            } else {
                app.handle_server_event(event);
            }
        }

        let mut canvas = Canvas::new(resolution, &mut bgra)?;
        app.draw(&mut canvas, started_at.elapsed().as_secs_f32());
        app.set_last_frame_ms(frame_start.elapsed().as_secs_f32() * 1000.0);
        musicos_init::renderer::update_latest_screenshot(&bgra, resolution);
        bgra_to_minifb(&bgra, &mut pixels[0..screen_width * screen_height]);

        let dark = system_uses_dark_appearance();
        let mut ctrl_canvas = Canvas::new(controls_resolution, &mut controller_bgra)?;
        render_controller_panel(&mut ctrl_canvas, active_button, scale, dark);
        bgra_to_minifb(&controller_bgra, &mut pixels[screen_width * screen_height..]);

        window.update_with_buffer(&pixels, window_width, window_height)?;
        if let Some(help) = controls_window.as_mut() {
            if help.is_open() {
                help.update_with_buffer(&controls_pixels, 640, 480)?;
            } else {
                controls_window = None;
            }
        }
    }

    Ok(())
}

fn map_minifb_key(key: Key) -> Option<Button> {
    match key {
        Key::Enter | Key::J => Some(Button::Confirm),
        Key::Backspace | Key::K | Key::H => Some(Button::Back),
        Key::Escape => Some(Button::Back),
        Key::Up | Key::W => Some(Button::Up),
        Key::Down | Key::S => Some(Button::Down),
        Key::Left | Key::A => Some(Button::Left),
        Key::Right | Key::D => Some(Button::Right),
        Key::X | Key::U => Some(Button::Context),
        Key::Y | Key::I => Some(Button::Home),
        Key::Space => Some(Button::PlayPause),
        Key::Comma => Some(Button::PreviousTrack),
        Key::Period => Some(Button::NextTrack),
        Key::M => Some(Button::Menu),
        Key::P => Some(Button::Sleep),
        Key::Equal => Some(Button::VolumeUp),
        Key::Minus => Some(Button::VolumeDown),
        _ => None,
    }
}

fn keyboard_events(window: &Window) -> (Vec<InputEvent>, bool) {
    let mut events = Vec::new();
    let mut show_controls = false;
    for key in window.get_keys_pressed(KeyRepeat::No) {
        if let Some(button) = map_minifb_key(key) {
            events.push(InputEvent::Button(button));
        } else if key == Key::F1 {
            show_controls = true;
        } else if key == Key::F2 {
            events.push(InputEvent::WifiSync);
        }
    }
    for key in window.get_keys_pressed(KeyRepeat::Yes) {
        if matches!(
            key,
            Key::Up | Key::W | Key::Down | Key::S | Key::Left | Key::A | Key::Right | Key::D | Key::Minus | Key::Equal
        ) {
            if let Some(button) = map_minifb_key(key) {
                events.push(InputEvent::Button(button));
            }
        }
    }
    for key in window.get_keys_released() {
        if let Some(button) = map_minifb_key(key) {
            events.push(InputEvent::ButtonRelease(button));
        }
    }
    (events, show_controls)
}

fn create_controls_window(preview_width: usize) -> Result<Window> {
    let mut window = Window::new(
        "MusicOS Preview Controls",
        640,
        480,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;
    window.set_position(80 + preview_width as isize, 72);
    Ok(window)
}

fn render_controls_reference(dark: bool) -> Result<Vec<u32>> {
    let resolution = Resolution::VGA_4_3;
    let mut bgra = vec![0_u8; 640 * 480 * 4];
    let mut pixels = vec![0_u32; 640 * 480];
    {
        let mut canvas = Canvas::new(resolution, &mut bgra)?;
        let background = if dark {
            Color::rgb(17, 22, 29)
        } else {
            Color::rgb(243, 245, 248)
        };
        let surface = if dark {
            Color::rgb(29, 37, 48)
        } else {
            Color::WHITE
        };
        let text = if dark {
            Color::rgb(243, 246, 250)
        } else {
            Color::rgb(29, 38, 51)
        };
        let muted = if dark {
            Color::rgb(168, 178, 193)
        } else {
            Color::rgb(93, 104, 118)
        };
        canvas.clear(background);
        musicos_init::ui::rr(&mut canvas, 44.0, 42.0, 1192.0, 876.0, 14.0, surface);
        canvas.text(
            "MusicOS Preview Controls",
            84.0,
            76.0,
            50.0,
            text,
            FontWeight::Bold,
        );
        canvas.text(
            "The preview is optional. Desktop playback stays local.",
            84.0,
            134.0,
            25.0,
            muted,
            FontWeight::Regular,
        );

        let groups: [(&str, &[(&str, &str)]); 5] = [
            (
                "Navigation",
                &[
                    ("W / ↑", "Up"),
                    ("A / ←", "Left"),
                    ("S / ↓", "Down"),
                    ("D / →", "Right"),
                ],
            ),
            (
                "Actions",
                &[
                    ("J / Enter", "Select"),
                    ("K / Backspace", "Back"),
                    ("U / X", "Options"),
                    ("I / Y", "Home"),
                ],
            ),
            (
                "Playback",
                &[
                    ("Space", "Play / pause"),
                    (", / .", "Previous / next"),
                    ("- / =", "Volume"),
                ],
            ),
            ("Mouse", &[("Click", "Tap"), ("Click-drag", "Swipe")]),
            (
                "Preview utilities",
                &[
                    ("F1", "Show controls"),
                    ("F2", "Wi-Fi sync"),
                    ("M / P", "Menu / sleep"),
                    ("Esc", "Quit preview"),
                ],
            ),
        ];
        for (group_index, (heading, rows)) in groups.iter().enumerate() {
            let column = group_index % 2;
            let row = group_index / 2;
            let x = 84.0 + column as f32 * 566.0;
            let y = 204.0 + row as f32 * 222.0;
            canvas.text(heading, x, y, 30.0, text, FontWeight::Bold);
            for (index, (keys, action)) in rows.iter().enumerate() {
                let row_y = y + 48.0 + index as f32 * 37.0;
                canvas.text(
                    keys,
                    x,
                    row_y,
                    23.0,
                    Color::rgb(76, 141, 255),
                    FontWeight::Bold,
                );
                canvas.text(action, x + 190.0, row_y, 23.0, muted, FontWeight::Regular);
            }
        }
    }
    bgra_to_minifb(&bgra, &mut pixels);
    Ok(pixels)
}

fn system_uses_dark_appearance() -> bool {
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .eq_ignore_ascii_case("dark")
            })
            .unwrap_or(false);
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("MUSICOS_DARK_MODE")
            .map(|value| matches!(value.as_str(), "1" | "true" | "dark"))
            .unwrap_or(false)
    }
}

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_ui_jk_controls() {
        assert_eq!(map_minifb_key(Key::W), Some(Button::Up));
        assert_eq!(map_minifb_key(Key::A), Some(Button::Left));
        assert_eq!(map_minifb_key(Key::S), Some(Button::Down));
        assert_eq!(map_minifb_key(Key::D), Some(Button::Right));
        assert_eq!(map_minifb_key(Key::J), Some(Button::Confirm));
        assert_eq!(map_minifb_key(Key::K), Some(Button::Back));
        assert_eq!(map_minifb_key(Key::U), Some(Button::Context));
        assert_eq!(map_minifb_key(Key::I), Some(Button::Home));
        assert_eq!(map_minifb_key(Key::Space), Some(Button::PlayPause));
        assert_eq!(map_minifb_key(Key::Comma), Some(Button::PreviousTrack));
        assert_eq!(map_minifb_key(Key::Period), Some(Button::NextTrack));
        assert_eq!(map_minifb_key(Key::Escape), Some(Button::Back));
        assert_eq!(map_minifb_key(Key::Q), None);
    }

    #[test]
    fn finds_hardware_controller_buttons() {
        assert_eq!(find_controller_button(80.0, 40.0, 1.0), Some(Button::Up));
        assert_eq!(find_controller_button(80.0, 75.0, 1.0), Some(Button::Confirm));
        assert_eq!(find_controller_button(200.0, 50.0, 1.0), Some(Button::Back));
        assert_eq!(find_controller_button(490.0, 50.0, 1.0), Some(Button::PlayPause));
        assert_eq!(find_controller_button(0.0, 0.0, 1.0), None);
    }
}
