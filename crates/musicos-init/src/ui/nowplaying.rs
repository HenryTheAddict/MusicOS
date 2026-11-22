#![allow(dead_code)]

use crate::app::Focus;
use crate::audio::{LoopMode, PlaybackStatus};
use crate::database::Track;
use crate::renderer::{Canvas, Color, CoverArt};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::{register_focus, text_fit, FontWeight};

pub fn draw_now_playing(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    _seconds: f32,
    _lang: Language,
    theme: ThemeName,
    status: &PlaybackStatus,
    queue: &[Track],
    cover_art: Option<&CoverArt>,
    previous_cover_art: Option<&CoverArt>,
    cover_art_opacity: f32,
    displayed_position: f32,
) {
    crate::ui::draw_figma_background(canvas);
    let _ = theme;
    let accent = crate::ui::accent(canvas);
    let Some(ref track) = status.track else {
        crate::ui::page_heading(canvas, "Now Playing", "Choose a song from Music");
        return;
    };
    if let Some(art) = cover_art.or(previous_cover_art) {
        crate::ui::draw_art_wash(canvas, art, 680.0);
    }

    let art_x = 48.0;
    let art_y = 132.0;
    let art_size = 416.0;
    if let Some(previous) = previous_cover_art {
        if let Some(art) = cover_art {
            canvas.draw_image_rounded(previous, art_x, art_y, art_size, art_size, 24.0, 1.0);
            canvas.draw_image_rounded(
                art,
                art_x,
                art_y,
                art_size,
                art_size,
                24.0,
                cover_art_opacity,
            );
        } else {
            draw_cover_art(canvas, None, art_x, art_y, art_size, 24.0, 1.0);
            canvas.draw_image_rounded(
                previous,
                art_x,
                art_y,
                art_size,
                art_size,
                24.0,
                1.0 - cover_art_opacity,
            );
        }
    } else {
        draw_cover_art(
            canvas,
            cover_art,
            art_x,
            art_y,
            art_size,
            24.0,
            cover_art_opacity,
        );
    }
    text_fit(
        canvas,
        &track.title,
        518.0,
        147.0,
        694.0,
        62.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        &format!("{} · {}", track.artist, track.album),
        518.0,
        241.0,
        694.0,
        36.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Regular,
    );
    if let Some(error) = status.last_error.as_deref() {
        crate::ui::rr(
            canvas,
            518.0,
            326.0,
            694.0,
            64.0,
            5.0,
            Color::rgb(168, 51, 59),
        );
        text_fit(
            canvas,
            error,
            534.0,
            339.0,
            662.0,
            36.0,
            Color::WHITE,
            FontWeight::Bold,
        );
    } else if let Some(next_track) = status.queue_index.and_then(|index| queue.get(index + 1)) {
        text_fit(
            canvas,
            "UP NEXT",
            518.0,
            338.0,
            694.0,
            30.0,
            crate::ui::ink_secondary(canvas),
            FontWeight::Bold,
        );
        text_fit(
            canvas,
            &next_track.title,
            518.0,
            377.0,
            694.0,
            36.0,
            crate::ui::ink(canvas),
            FontWeight::Bold,
        );
    }

    let slider_x = 48.0;
    let slider_y = 586.0;
    let slider_w = 1184.0;
    let ratio = if track.duration > 0 {
        (displayed_position / track.duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    crate::ui::rr(
        canvas,
        slider_x,
        slider_y,
        slider_w,
        12.0,
        6.0,
        crate::ui::control(canvas),
    );
    crate::ui::rr(
        canvas,
        slider_x,
        slider_y,
        slider_w * ratio,
        12.0,
        6.0,
        accent,
    );
    canvas.text(
        &format_time(status.position),
        slider_x,
        611.0,
        30.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
    canvas.text_right(
        &format!(
            "-{}",
            format_time((track.duration as f32 - status.position).max(0.0))
        ),
        slider_x + slider_w,
        611.0,
        30.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );

    let center_x = 640.0;
    let controls_y = 716.0;
    draw_player_icon(
        canvas,
        center_x - 170.0,
        controls_y,
        "prev",
        focus == Focus::Artists,
        status,
        60.0,
    );
    draw_player_icon(
        canvas,
        center_x,
        controls_y - 12.0,
        "play",
        focus == Focus::Albums,
        status,
        68.0,
    );
    draw_player_icon(
        canvas,
        center_x + 170.0,
        controls_y,
        "next",
        focus == Focus::Playlists,
        status,
        60.0,
    );

    draw_player_icon(
        canvas,
        300.0,
        726.0,
        "loop",
        focus == Focus::Songs,
        status,
        53.0,
    );
    draw_heart(
        canvas,
        1142.0,
        722.0,
        track.favorited,
        focus == Focus::Favorites,
    );
    draw_player_icon(
        canvas,
        980.0,
        726.0,
        "shuffle",
        focus == Focus::History,
        status,
        53.0,
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
        if opacity < 1.0 {
            crate::ui::rr(canvas, x, y, size, size, radius, crate::ui::control(canvas));
        }
        canvas.draw_image_rounded(art, x, y, size, size, radius, opacity);
        return;
    }
    crate::ui::rr(canvas, x, y, size, size, radius, crate::ui::control(canvas));
    let icon = size * 0.38;
    let inset = (size - icon) / 2.0;
    canvas.draw_svg_tinted(
        &format!("{}/icons/desktop/album.svg", crate::ui::assets_dir()),
        x + inset,
        y + inset,
        icon,
        icon,
        crate::ui::ink_secondary(canvas),
    );
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
    size: f32,
) {
    if selected {
        let ring = size + 28.0;
        register_focus(
            canvas,
            cx - ring / 2.0,
            cy - ring / 2.0,
            ring,
            ring,
            ring / 2.0,
        );
    }

    if kind == "play" {
        let diameter = size + 24.0;
        crate::ui::rr(
            canvas,
            cx - diameter / 2.0,
            cy - diameter / 2.0,
            diameter,
            diameter,
            diameter / 2.0,
            crate::ui::ink(canvas),
        );
    }
    let filename = match kind {
        "play" if status.is_playing => "pause.svg",
        "play" => "play.svg",
        "next" => "next.svg",
        "prev" => "prev.svg",
        "shuffle" if status.shuffle => "ShuffleON.svg",
        "shuffle" => "ShuffleOFF.svg",
        "loop" => match status.loop_mode {
            LoopMode::None => "RepeatOFF.svg",
            LoopMode::All => "RepeatON.svg",
            LoopMode::One => "RepeatONE.svg",
        },
        _ => "play.svg",
    };
    let base_path = crate::ui::assets_dir();
    let path = format!("{}/icons/ui/{}", base_path, filename);
    canvas.draw_svg_tinted(
        &path,
        cx - size / 2.0,
        cy - size / 2.0,
        size,
        size,
        if kind == "play" {
            crate::ui::shell(canvas)
        } else {
            crate::ui::ink(canvas)
        },
    );
}

fn draw_heart(canvas: &mut Canvas<'_>, cx: f32, cy: f32, filled: bool, selected: bool) {
    let size = 54.0;
    if selected {
        let ring = size + 24.0;
        register_focus(
            canvas,
            cx - ring / 2.0,
            cy - ring / 2.0,
            ring,
            ring,
            ring / 2.0,
        );
    }
    let base_path = crate::ui::assets_dir();
    let filename = if filled { "hearted.svg" } else { "noheart.svg" };
    let path = format!("{}/icons/ui/{}", base_path, filename);
    canvas.draw_svg_tinted(
        &path,
        cx - size / 2.0,
        cy - size / 2.0,
        size,
        size,
        crate::ui::ink(canvas),
    );
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
