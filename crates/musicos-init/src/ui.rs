pub mod apps;
pub mod globe;
pub mod home;
pub mod localization;
pub mod music;
pub mod nowplaying;
pub mod screens;
pub mod settings;
pub mod theme;
pub mod transitions;

use crate::renderer::{Canvas, Color};

#[derive(Debug, Clone, Copy)]
pub enum FontWeight {
    Regular,
    Bold,
}

pub fn draw_figma_background(canvas: &mut Canvas<'_>) {
    let accent = canvas.theme_top_color.unwrap_or(Color::rgb(63, 143, 229));
    let top = accent.mix(Color::WHITE, 0.92);
    let bottom = accent.mix(Color::WHITE, 0.72);
    canvas.vertical_gradient(top, bottom);
}

pub fn animated_focus_blue(seconds: f32, phase: f32) -> Color {
    let pulse = transitions::pulse01(seconds, 3.2, phase);
    Color::rgb(35, 108, 220).mix(Color::rgb(82, 154, 230), 0.08 + pulse * 0.08)
}

pub fn focus_halo(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    seconds: f32,
    phase: f32,
) {
    let accent = canvas.theme_top_color.unwrap_or(Color::rgb(63, 143, 229));
    let pulse = transitions::pulse01(seconds, 3.6, phase);
    let spread = 2.0 + pulse * 3.5;
    let alpha = (18.0 + pulse * 24.0).round() as u8;
    rr_stroke(
        canvas,
        x - spread,
        y - spread,
        w + spread * 2.0,
        h + spread * 2.0,
        r + spread,
        3.0 + pulse * 1.5,
        accent.with_alpha(alpha),
    );
    rr_stroke(
        canvas,
        x - spread * 0.45,
        y - spread * 0.45,
        w + spread * 0.9,
        h + spread * 0.9,
        r + spread * 0.45,
        2.0,
        Color::rgba(255, 255, 255, 32),
    );
}

pub fn draw_globe(canvas: &mut Canvas<'_>, cx: f32, cy: f32, r: f32, thick: f32, color: Color) {
    let res = canvas.resolution();
    globe::draw_real_globe(
        canvas,
        res.scale_x(cx),
        res.scale_y(cy),
        r * res.scale(),
        r * (44.5 / 64.0) * res.scale(),
        thick * res.scale(),
        color,
    );
}

pub fn button(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    fill: Color,
    text: Color,
    seconds: f32,
    selected: bool,
) {
    if selected {
        focus_halo(canvas, x, y, w, h, 86.0, seconds, 0.0);
    }
    shadow_rr(canvas, x, y, w, h, 86.0, 4.0, 12.0);

    if fill.r == 255 && fill.g == 255 && fill.b == 255 && fill.a < 255 {
        crate::ui::liquid_glass(canvas, x, y, w, h, 86.0, fill.a);
    } else if selected && fill.b > 150 && fill.r < 90 {
        rr(canvas, x, y, w, h, 86.0, animated_focus_blue(seconds, 0.0));
    } else {
        rr(canvas, x, y, w, h, 86.0, fill);
    }

    if selected {
        canvas.register_selected(x, y, w, h, 86.0, 8.0);
    }

    canvas.text_centered(
        label,
        x + w / 2.0,
        y + 45.0,
        80.0,
        text,
        FontWeight::Regular,
    );
}

pub fn panel_gradient(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    top: Color,
    bottom: Color,
) {
    shadow_rr(canvas, x, y, w, h, r, 8.0, 8.0);
    rr_gradient(canvas, x, y, w, h, r, top, bottom);
    liquid_glass(canvas, x, y, w, h, r, 18);
}

pub fn figma_split_panels(
    canvas: &mut Canvas<'_>,
    theme: theme::ThemeName,
    left_w: f32,
    right_x: f32,
    right_w: f32,
) {
    draw_figma_background(canvas);
    panel_gradient(
        canvas,
        20.0,
        32.0,
        left_w,
        896.0,
        128.0,
        Color::rgb(178, 246, 228),
        Color::WHITE,
    );
    let (top_color, bottom_color) =
        if let (Some(top), Some(bottom)) = (canvas.theme_top_color, canvas.theme_bottom_color) {
            (top, bottom)
        } else {
            theme.gradient_colors()
        };
    panel_gradient(
        canvas,
        right_x,
        32.0,
        right_w,
        896.0,
        128.0,
        top_color,
        bottom_color,
    );
}

pub fn figma_list_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    selected: bool,
    seconds: f32,
) {
    if selected {
        focus_halo(canvas, x, y, w, h, h / 2.0, seconds, 0.8);
        shadow_rr(canvas, x, y, w, h, h / 2.0, 4.0, 12.0);
        rr(canvas, x, y, w, h, h / 2.0, Color::WHITE);
        canvas.register_selected(x, y, w, h, h / 2.0, 8.0);
    } else {
        let faint_h = h + 4.0;
        rr(
            canvas,
            x - 2.0,
            y,
            w + 4.0,
            faint_h,
            faint_h / 2.0,
            Color::rgba(255, 255, 255, 34),
        );
    }
}

pub fn text_fit(
    canvas: &mut Canvas<'_>,
    text: &str,
    x: f32,
    y: f32,
    max_w: f32,
    size: f32,
    color: Color,
    weight: FontWeight,
) {
    let fitted = fitted_text(canvas, text, max_w, size, weight);
    canvas.text(&fitted, x, y, size, color, weight);
}

pub fn text_fit_centered(
    canvas: &mut Canvas<'_>,
    text: &str,
    cx: f32,
    y: f32,
    max_w: f32,
    size: f32,
    color: Color,
    weight: FontWeight,
) {
    let fitted = fitted_text(canvas, text, max_w, size, weight);
    canvas.text_centered(&fitted, cx, y, size, color, weight);
}

fn fitted_text(
    canvas: &Canvas<'_>,
    text: &str,
    max_w: f32,
    size: f32,
    weight: FontWeight,
) -> String {
    if canvas.text_width(text, size, weight) <= max_w {
        return text.to_string();
    }

    let suffix = "...";
    let suffix_w = canvas.text_width(suffix, size, weight);
    if suffix_w >= max_w {
        return suffix.to_string();
    }

    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        let candidate = format!("{out}{suffix}");
        if canvas.text_width(&candidate, size, weight) > max_w {
            out.pop();
            break;
        }
    }
    format!("{out}{suffix}")
}

pub fn icon_tile(canvas: &mut Canvas<'_>, x: f32, y: f32, kind: &str, selected: bool) {
    let base_path = std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());

    let filename = if selected {
        match kind {
            "apps" => "AppsSelected.svg",
            "music" => "MusicSelected.svg",
            "settings" => "settingsSelected.svg",
            _ => "NowPlayingSelected.svg",
        }
    } else {
        match kind {
            "apps" => "Apps.svg",
            "music" => "Music.svg",
            "settings" => "settings.svg",
            _ => "NowPlaying.svg",
        }
    };

    let path = format!("{}/icons/appicons/{}", base_path, filename);
    canvas.draw_svg(&path, x - 8.0, y - 8.0, 272.0, 272.0);
}

pub fn centered_lines(
    canvas: &mut Canvas<'_>,
    lines: &[&str],
    cx: f32,
    y: f32,
    size: f32,
    line_height: f32,
    color: Color,
    weight: FontWeight,
) {
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        canvas.text_centered(
            line,
            cx,
            y + line_height * index as f32,
            size,
            color,
            weight,
        );
    }
}

pub fn rr(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let res = canvas.resolution();
    canvas.rounded_rect(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        color,
    );
}

pub fn rr_gradient(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    top: Color,
    bottom: Color,
) {
    let res = canvas.resolution();
    canvas.rounded_rect_vertical_gradient(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        top,
        bottom,
    );
}

pub fn rr_three_gradient(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    top: Color,
    middle: Color,
    bottom: Color,
) {
    let res = canvas.resolution();
    canvas.rounded_rect_three_stop_vertical_gradient(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        top,
        middle,
        bottom,
        0.5,
    );
}

pub fn rr_stroke(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    thickness: f32,
    color: Color,
) {
    let res = canvas.resolution();
    canvas.rounded_rect_stroke(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        thickness * res.scale(),
        color,
    );
}

pub fn shadow_rr(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    offset_y: f32,
    blur: f32,
) {
    let res = canvas.resolution();
    canvas.shadow_rounded_rect(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        Color::rgba(0, 0, 0, 42),
        offset_y * res.scale(),
        blur * res.scale(),
    );
}

pub fn liquid_glass(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, r: f32, opacity: u8) {
    let res = canvas.resolution();
    canvas.liquid_glass_rounded_rect(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        opacity,
    );
}

pub fn inner_shadow(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    shadow_size: f32,
) {
    let res = canvas.resolution();
    canvas.inner_shadow_rounded_rect(
        res.scale_x(x),
        res.scale_y(y),
        w * res.scale(),
        h * res.scale(),
        r * res.scale(),
        shadow_size * res.scale(),
    );
}

pub fn line(
    canvas: &mut Canvas<'_>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: Color,
) {
    let res = canvas.resolution();
    canvas.line(
        res.scale_x(x0),
        res.scale_y(y0),
        res.scale_x(x1),
        res.scale_y(y1),
        thickness * res.scale(),
        color,
    );
}

pub fn ellipse(
    canvas: &mut Canvas<'_>,
    cx: f32,
    cy: f32,
    r: f32,
    color: Color,
    thickness: f32,
    fill: Option<Color>,
) {
    let res = canvas.resolution();
    if let Some(fill) = fill {
        canvas.circle(res.scale_x(cx), res.scale_y(cy), r * res.scale(), fill);
    }
    canvas.circle_stroke(
        res.scale_x(cx),
        res.scale_y(cy),
        r * res.scale(),
        thickness * res.scale(),
        color,
    );
}

pub fn draw_context_menu(
    canvas: &mut Canvas<'_>,
    focus: crate::app::Focus,
    theme: theme::ThemeName,
    title: &str,
    is_pinned: bool,
    transition_t: f32,
    seconds: f32,
) {
    let transition_t = transitions::ease_out_cubic(transition_t);
    let menu_t = transitions::ease_out_back(transition_t);
    let (top, bottom) = theme.gradient_colors();
    canvas.rect(0.0, 0.0, 1280.0, 960.0, Color::rgba(0, 0, 0, (transition_t * 110.0) as u8));
    let modal_w = 560.0;
    let modal_h = 896.0;
    let modal_x = 1280.0 - menu_t * 580.0; // Slides from 1280 (closed) to 700 (fully open with 20px right margin)
    let modal_y = 32.0;

    shadow_rr(canvas, modal_x, modal_y, modal_w, modal_h, 58.0, 10.0, 20.0);
    panel_gradient(
        canvas,
        modal_x,
        modal_y,
        modal_w,
        modal_h,
        58.0,
        Color::WHITE.mix(top, 0.12),
        Color::WHITE.mix(bottom, 0.22),
    );
    liquid_glass(canvas, modal_x, modal_y, modal_w, modal_h, 58.0, 42);

    canvas.text_centered(
        "Options",
        modal_x + modal_w / 2.0,
        modal_y + 42.0,
        54.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    text_fit_centered(
        canvas,
        title,
        modal_x + modal_w / 2.0,
        modal_y + 104.0,
        480.0,
        30.0,
        Color::rgba(0, 0, 0, 185),
        FontWeight::Bold,
    );

    let pin_label = if is_pinned { "Unpin" } else { "Pin" };
    let rows = [
        (crate::app::Focus::ContextPlay, "Play", 180.0, false),
        (crate::app::Focus::ContextPin, pin_label, 276.0, false),
        (
            crate::app::Focus::ContextAddToPlaylist,
            "Add to Playlist",
            372.0,
            false,
        ),
        (crate::app::Focus::ContextShuffle, "Shuffle", 468.0, false),
        (crate::app::Focus::ContextDelete, "Delete", 564.0, true),
        (crate::app::Focus::ContextCancel, "Cancel", 660.0, false),
    ];

    for (index, (row_focus, label, rel_y, danger)) in rows.into_iter().enumerate() {
        let row_t =
            transitions::ease_out_back(transitions::stagger(transition_t, index, 0.04, 0.78));
        let selected = focus == row_focus;
        let x = modal_x + 40.0;
        let y = modal_y + rel_y + (1.0 - row_t) * 34.0;
        let w = modal_w - 80.0;
        let h = 80.0;
        if selected {
            focus_halo(canvas, x, y, w, h, 24.0, seconds, index as f32 * 0.35);
            let fill = if danger {
                Color::rgb(255, 54, 54)
            } else {
                top.mix(
                    Color::WHITE,
                    transitions::pulse01(seconds, 3.2, index as f32 * 0.35) * 0.14,
                )
            };
            rr(canvas, x, y, w, h, 24.0, fill);
            rr_stroke(canvas, x, y, w, h, 24.0, 5.0, Color::BLACK);
            canvas.register_selected(x, y, w, h, 24.0, 6.0);
        } else {
            let fill = if danger {
                Color::rgba(255, 255, 255, 176)
            } else {
                Color::rgba(255, 255, 255, 142)
            };
            rr(canvas, x, y, w, h, 24.0, fill);
            liquid_glass(canvas, x, y, w, h, 24.0, 96);
            rr_stroke(canvas, x, y, w, h, 24.0, 2.0, Color::rgba(0, 0, 0, 70));
        }

        canvas.text_centered(
            label,
            x + w / 2.0,
            y + 12.0,
            46.0,
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Bold,
        );
    }
}
