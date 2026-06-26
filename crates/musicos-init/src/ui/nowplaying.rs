use std::path::Path;
use std::sync::OnceLock;

use crate::app::Focus;
use crate::audio::{LoopMode, PlaybackStatus};
use crate::database::Track;
use crate::renderer::{Canvas, Color, CoverArt};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, pulse01};
use crate::ui::{focus_halo, text_fit, FontWeight};

static PLACEHOLDER_ART: OnceLock<Option<CoverArt>> = OnceLock::new();

pub fn draw_now_playing(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    _lang: Language,
    theme: ThemeName,
    status: &PlaybackStatus,
    queue: &[Track],
    cover_art: Option<&CoverArt>,
    cover_art_opacity: f32,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    let (top, _) =
        if let (Some(top), Some(bottom)) = (canvas.theme_top_color, canvas.theme_bottom_color) {
            (top, bottom)
        } else {
            theme.gradient_colors()
        };
    let accent = top;
    canvas.vertical_gradient(Color::WHITE, accent);
    crate::ui::liquid_glass(canvas, 0.0, 0.0, 1280.0, 960.0, 0.0, 18);

    let Some(ref track) = status.track else {
        return;
    };

    let (title, artist, album, duration, favorited) = (
        track.title.as_str(),
        track.artist.as_str(),
        track.album.as_str(),
        track.duration,
        track.favorited,
    );

    let intro = ease_out_back(transition_t);
    let art_x = 80.0 - (1.0 - intro) * 44.0;
    let art_y = 86.0;
    let art_size = 500.0;
    let art_pulse = if status.is_playing {
        1.0 + (seconds * 2.2).sin() * 0.012
    } else {
        1.0
    };
    let pulsed_size = art_size * art_pulse;
    let pulsed_x = art_x - (pulsed_size - art_size) / 2.0;
    let pulsed_y = art_y - (pulsed_size - art_size) / 2.0;

    crate::ui::shadow_rr(
        canvas,
        pulsed_x,
        pulsed_y,
        pulsed_size,
        pulsed_size,
        72.0,
        8.0,
        16.0,
    );
    draw_cover_art(
        canvas,
        cover_art,
        pulsed_x,
        pulsed_y,
        pulsed_size,
        72.0,
        cover_art_opacity,
    );

    let title_lines = wrap_lines(canvas, title, 610.0, 72.0, FontWeight::Bold, 5);
    let title_y = 78.0 + (1.0 - transition_t) * 28.0;
    for (index, line) in title_lines.iter().enumerate() {
        canvas.text(
            line,
            610.0,
            title_y + index as f32 * 82.0,
            72.0,
            Color::BLACK,
            FontWeight::Bold,
        );
    }

    outlined_text(canvas, artist, 610.0, 510.0, 64.0, Color::WHITE);
    text_fit(
        canvas,
        album,
        610.0,
        585.0,
        590.0,
        58.0,
        Color::WHITE,
        FontWeight::Bold,
    );
    if let Some(next_track) = status
        .queue_index
        .and_then(|index| queue.get(index + 1))
    {
        text_fit(
            canvas,
            &format!("Up Next: {}", next_track.title),
            610.0,
            648.0,
            590.0,
            30.0,
            Color::rgba(0, 0, 0, 190),
            FontWeight::Bold,
        );
    }

    let slider_x = 136.0;
    let slider_y = 690.0;
    let slider_w = 1008.0;
    let slider_h = 30.0;
    let ratio = if duration > 0 {
        (status.position / duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    crate::ui::rr(
        canvas,
        slider_x,
        slider_y + 8.0,
        slider_w,
        10.0,
        5.0,
        Color::BLACK,
    );
    crate::ui::rr(
        canvas,
        slider_x,
        slider_y,
        slider_w * ratio,
        slider_h,
        15.0,
        accent,
    );
    let knob_x = slider_x + slider_w * ratio;
    let res = canvas.resolution();
    canvas.circle(
        res.scale_x(knob_x),
        res.scale_y(slider_y + slider_h / 2.0),
        (16.0 + pulse01(seconds, 4.0, 0.1) * 2.0) * res.scale(),
        accent,
    );
    canvas.text(
        &format_time(status.position),
        slider_x + 16.0,
        735.0,
        34.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_right(
        &format_time(duration as f32),
        slider_x + slider_w,
        735.0,
        34.0,
        Color::BLACK,
        FontWeight::Bold,
    );

    let center_x = 640.0;
    let controls_y = 770.0;
    draw_player_icon(
        canvas,
        center_x - 160.0,
        controls_y,
        "prev",
        focus == Focus::Artists,
        status,
        seconds,
        0.0,
        88.0,
    );
    draw_player_icon(
        canvas,
        center_x,
        controls_y - 18.0,
        "play",
        focus == Focus::Albums,
        status,
        seconds,
        0.35,
        112.0,
    );
    draw_player_icon(
        canvas,
        center_x + 160.0,
        controls_y,
        "next",
        focus == Focus::Playlists,
        status,
        seconds,
        0.7,
        88.0,
    );

    let lower_y = 872.0;
    draw_player_icon(
        canvas,
        center_x - 170.0,
        lower_y,
        "loop",
        focus == Focus::Songs,
        status,
        seconds,
        1.05,
        64.0,
    );
    draw_heart(
        canvas,
        center_x,
        lower_y - 6.0,
        favorited,
        focus == Focus::Favorites,
        seconds,
    );
    draw_player_icon(
        canvas,
        center_x + 170.0,
        lower_y,
        "shuffle",
        focus == Focus::History,
        status,
        seconds,
        1.4,
        64.0,
    );
}

pub fn draw_cover_art(
    canvas: &mut Canvas<'_>,
    cover_art: Option<&CoverArt>,
    x: f32,
    y: f32,
    size: f32,
    radius: f32,
    opacity: f32,
) {
    if let Some(art) = cover_art {
        if let Some(placeholder) = placeholder_cover_art() {
            canvas.draw_image_rounded(placeholder, x, y, size, size, radius, 1.0);
        } else {
            crate::ui::rr(canvas, x, y, size, size, radius, Color::BLACK);
        }
        canvas.draw_image_rounded(art, x, y, size, size, radius, opacity);
        return;
    }

    if let Some(art) = placeholder_cover_art() {
        canvas.draw_image_rounded(art, x, y, size, size, radius, opacity.max(0.96));
        return;
    }

    crate::ui::rr(canvas, x, y, size, size, radius, Color::BLACK);
}

pub fn placeholder_cover_art() -> Option<&'static CoverArt> {
    PLACEHOLDER_ART
        .get_or_init(|| {
            let base_path =
                std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
            crate::media::load_cover_art_file(
                Path::new(&base_path)
                    .join("icons/coverartplaceholder.png")
                    .as_path(),
            )
        })
        .as_ref()
}

fn outlined_text(canvas: &mut Canvas<'_>, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let shadow = Color::rgba(80, 105, 130, 190);
    canvas.text(text, x - 2.0, y, size, shadow, FontWeight::Bold);
    canvas.text(text, x + 2.0, y, size, shadow, FontWeight::Bold);
    canvas.text(text, x, y - 2.0, size, shadow, FontWeight::Bold);
    canvas.text(text, x, y + 2.0, size, shadow, FontWeight::Bold);
    text_fit(canvas, text, x, y, 590.0, size, color, FontWeight::Bold);
}

fn draw_player_icon(
    canvas: &mut Canvas<'_>,
    cx: f32,
    cy: f32,
    kind: &str,
    selected: bool,
    status: &PlaybackStatus,
    seconds: f32,
    phase: f32,
    size: f32,
) {
    if selected {
        let ring = size + 28.0;
        focus_halo(
            canvas,
            cx - ring / 2.0,
            cy - ring / 2.0,
            ring,
            ring,
            ring / 2.0,
            seconds,
            phase,
        );
        canvas.register_selected(
            cx - ring / 2.0,
            cy - ring / 2.0,
            ring,
            ring,
            ring / 2.0,
            8.0,
        );
    }

    let filename = match kind {
        "play" if status.is_playing => "pause.svg",
        "play" => "Play.svg",
        "next" => "skip.svg",
        "prev" => "previous.svg",
        "shuffle" if status.shuffle => "shuffle-1.svg",
        "shuffle" => "shuffle.svg",
        "loop" => match status.loop_mode {
            LoopMode::None => "no loop.svg",
            LoopMode::All => "loop.svg",
            LoopMode::One => "loop1.svg",
        },
        _ => "Play.svg",
    };
    let base_path = std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
    let path = format!("{}/icons/nowplaying/{}", base_path, filename);
    canvas.draw_svg(&path, cx - size / 2.0, cy - size / 2.0, size, size);
}

fn draw_heart(
    canvas: &mut Canvas<'_>,
    cx: f32,
    cy: f32,
    filled: bool,
    selected: bool,
    seconds: f32,
) {
    let size = 96.0 + pulse01(seconds, 3.8, 0.4) * 4.0;
    if selected {
        let ring = size + 24.0;
        focus_halo(
            canvas,
            cx - ring / 2.0,
            cy - ring / 2.0,
            ring,
            ring,
            ring / 2.0,
            seconds,
            1.2,
        );
        canvas.register_selected(
            cx - ring / 2.0,
            cy - ring / 2.0,
            ring,
            ring,
            ring / 2.0,
            8.0,
        );
    }
    let base_path = std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
    let filename = if filled {
        "HeartFilled.svg"
    } else {
        "Heart.svg"
    };
    let path = format!("{}/icons/nowplaying/{}", base_path, filename);
    canvas.draw_svg(&path, cx - size / 2.0, cy - size / 2.0, size, size);
}

fn wrap_lines(
    canvas: &Canvas<'_>,
    text: &str,
    max_w: f32,
    size: f32,
    weight: FontWeight,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let next = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if canvas.text_width(&next, size, weight) <= max_w {
            current = next;
        } else {
            if !current.is_empty() {
                lines.push(current);
            }
            current = word.to_string();
        }
        if lines.len() + 1 >= max_lines {
            break;
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push("Untitled".to_string());
    }
    if canvas.text_width(lines.last().unwrap(), size, weight) > max_w {
        let last = lines.pop().unwrap();
        lines.push(fit_line(canvas, &last, max_w, size, weight));
    } else if text.split_whitespace().count() > lines.join(" ").split_whitespace().count() {
        let last = lines.pop().unwrap();
        lines.push(fit_line(canvas, &format!("{last}..."), max_w, size, weight));
    }
    lines
}

fn fit_line(canvas: &Canvas<'_>, text: &str, max_w: f32, size: f32, weight: FontWeight) -> String {
    if canvas.text_width(text, size, weight) <= max_w {
        return text.to_string();
    }
    let suffix = "...";
    let mut out = String::new();
    for ch in text.trim_end_matches('.').chars() {
        out.push(ch);
        if canvas.text_width(&format!("{out}{suffix}"), size, weight) > max_w {
            out.pop();
            break;
        }
    }
    format!("{out}{suffix}")
}

fn format_time(sec: f32) -> String {
    let sec = sec.max(0.0);
    let m = (sec / 60.0) as i32;
    let s = (sec % 60.0) as i32;
    format!("{m}:{s:02}")
}
