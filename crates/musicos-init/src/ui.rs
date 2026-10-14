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

pub const RADIUS_SM: f32 = 3.0;
pub const RADIUS_MD: f32 = 5.0;
pub const RADIUS_LG: f32 = 8.0;
pub const IPOD_CHROME_TOP: Color = Color::rgb(251, 251, 251);
pub const IPOD_CHROME_BOTTOM: Color = Color::rgb(202, 205, 209);
pub const IPOD_BORDER: Color = Color::rgb(126, 130, 136);
pub const IPOD_ROW: Color = Color::rgb(252, 252, 252);
pub const IPOD_ROW_ALT: Color = Color::rgb(239, 241, 243);

pub fn draw_figma_background(canvas: &mut Canvas<'_>) {
    canvas.clear(Color::rgb(244, 245, 246));
}

pub fn animated_focus_blue(seconds: f32, phase: f32) -> Color {
    let _ = (seconds, phase);
    Color::rgb(37, 112, 190)
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
    let _ = (seconds, phase);
    rr_stroke(
        canvas,
        x - 2.0,
        y - 2.0,
        w + 4.0,
        h + 4.0,
        r + 2.0,
        2.0,
        Color::rgb(24, 77, 132),
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
    button_with_radius(
        canvas, x, y, w, h, 86.0, label, fill, text, seconds, selected,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn button_with_radius(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    label: &str,
    fill: Color,
    text: Color,
    seconds: f32,
    selected: bool,
) {
    let _ = fill;
    let radius = radius.min(8.0);
    let button_fill = if selected {
        canvas
            .theme_top_color
            .unwrap_or_else(|| animated_focus_blue(seconds, 0.0))
    } else {
        IPOD_CHROME_TOP
    };
    rr_gradient(
        canvas,
        x,
        y,
        w,
        h,
        radius,
        if selected {
            button_fill.mix(Color::WHITE, 0.12)
        } else {
            Color::WHITE
        },
        if selected {
            button_fill.mix(Color::BLACK, 0.08)
        } else {
            IPOD_CHROME_BOTTOM
        },
    );
    rr_stroke(canvas, x, y, w, h, radius, 2.0, IPOD_BORDER);

    if selected {
        canvas.register_selected(x, y, w, h, radius, 8.0);
    }

    canvas.text_centered(
        label,
        x + w / 2.0,
        y + 45.0,
        80.0,
        if selected { Color::WHITE } else { text },
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
    let _ = (top, bottom);
    rr(canvas, x, y, w, h, r.min(8.0), Color::WHITE);
    rr_stroke(canvas, x, y, w, h, r.min(8.0), 2.0, IPOD_BORDER);
}

pub fn figma_split_panels(
    canvas: &mut Canvas<'_>,
    theme: theme::ThemeName,
    left_w: f32,
    right_x: f32,
    right_w: f32,
) {
    let accent = canvas
        .theme_top_color
        .unwrap_or_else(|| theme.primary_color());
    draw_figma_background(canvas);
    panel_gradient(
        canvas,
        12.0,
        64.0,
        left_w,
        884.0,
        RADIUS_LG,
        IPOD_ROW_ALT,
        IPOD_ROW,
    );
    panel_gradient(
        canvas,
        right_x,
        64.0,
        right_w,
        884.0,
        RADIUS_LG,
        Color::WHITE,
        accent.mix(Color::WHITE, 0.94),
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
    let radius = RADIUS_SM.min(h / 2.0);
    let accent = canvas
        .theme_top_color
        .unwrap_or_else(|| animated_focus_blue(seconds, 0.8));
    if selected {
        rr_gradient(
            canvas,
            x,
            y,
            w,
            h,
            radius,
            accent.mix(Color::WHITE, 0.14),
            accent.mix(Color::BLACK, 0.08),
        );
        rr_stroke(
            canvas,
            x,
            y,
            w,
            h,
            radius,
            1.0,
            accent.mix(Color::BLACK, 0.28),
        );
        canvas.register_selected(x, y, w, h, radius, 8.0);
    } else {
        rr(canvas, x, y, w, h, radius, IPOD_ROW);
        rr_stroke(canvas, x, y, w, h, radius, 1.0, Color::rgb(208, 211, 215));
    }
}

pub fn ipod_status_bar(
    canvas: &mut Canvas<'_>,
    title: &str,
    clock: &str,
    battery_text: &str,
    theme: theme::ThemeName,
    playing: bool,
) {
    rr_gradient(
        canvas,
        0.0,
        0.0,
        1280.0,
        64.0,
        0.0,
        IPOD_CHROME_TOP,
        IPOD_CHROME_BOTTOM,
    );
    line(canvas, 0.0, 63.0, 1280.0, 63.0, 2.0, IPOD_BORDER);
    canvas.text(
        clock,
        20.0,
        15.0,
        30.0,
        Color::rgb(38, 40, 44),
        FontWeight::Bold,
    );
    if playing {
        canvas.text(
            "▶",
            168.0,
            14.0,
            28.0,
            theme.primary_color(),
            FontWeight::Bold,
        );
    }
    text_fit_centered(
        canvas,
        title,
        640.0,
        9.0,
        650.0,
        40.0,
        Color::rgb(32, 34, 38),
        FontWeight::Bold,
    );

    let battery_x = 1158.0;
    rr(canvas, battery_x, 17.0, 86.0, 32.0, 5.0, Color::WHITE);
    rr_stroke(
        canvas,
        battery_x,
        17.0,
        86.0,
        32.0,
        5.0,
        2.0,
        Color::rgb(52, 55, 60),
    );
    rr(
        canvas,
        battery_x + 86.0,
        25.0,
        7.0,
        16.0,
        2.0,
        Color::rgb(52, 55, 60),
    );
    rr(
        canvas,
        battery_x + 5.0,
        22.0,
        62.0,
        22.0,
        2.0,
        theme.primary_color(),
    );
    let battery_label = battery_text
        .split_whitespace()
        .find(|part| part.contains('%'))
        .unwrap_or("--");
    canvas.text_right(
        battery_label,
        1144.0,
        18.0,
        23.0,
        Color::rgb(52, 55, 60),
        FontWeight::Bold,
    );
}

pub fn ipod_scrollbar(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    h: f32,
    visible: usize,
    total: usize,
    selected: usize,
) {
    if total <= visible || total == 0 {
        return;
    }
    rr(canvas, x, y, 8.0, h, 4.0, Color::rgb(220, 223, 226));
    let thumb_h = (h * visible as f32 / total as f32).clamp(42.0, h);
    let max_offset = (h - thumb_h).max(0.0);
    let ratio = selected as f32 / total.saturating_sub(1).max(1) as f32;
    rr(
        canvas,
        x,
        y + max_offset * ratio,
        8.0,
        thumb_h,
        4.0,
        Color::rgb(105, 110, 116),
    );
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

pub fn assets_dir() -> String {
    std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| {
        let installed = "/usr/share/musicos/assets";
        if std::path::Path::new(installed).is_dir() {
            installed.to_string()
        } else {
            "assets".to_string()
        }
    })
}

pub fn icon_tile(canvas: &mut Canvas<'_>, x: f32, y: f32, kind: &str, selected: bool) {
    let base_path = assets_dir();

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
    let _ = blur;
    let offset = offset_y.clamp(1.0, 3.0);
    rr(
        canvas,
        x + offset,
        y + offset,
        w,
        h,
        r,
        Color::rgba(54, 60, 68, 52),
    );
}

pub fn liquid_glass(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, r: f32, opacity: u8) {
    let fill = Color::rgba(255, 255, 255, opacity);
    rr(canvas, x, y, w, h, r.min(8.0), fill);
    rr_stroke(
        canvas,
        x,
        y,
        w,
        h,
        r.min(8.0),
        1.0,
        Color::rgb(172, 177, 184),
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
    let _ = shadow_size;
    rr_stroke(
        canvas,
        x + 1.0,
        y + 1.0,
        w - 2.0,
        h - 2.0,
        r.min(7.0),
        1.0,
        Color::rgba(255, 255, 255, 170),
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
    actions: &[(crate::app::Focus, String, bool)],
    transition_t: f32,
    seconds: f32,
) {
    let transition_t = transitions::ease_out_cubic(transition_t);
    let menu_t = transitions::ease_out_back(transition_t);
    let (top, bottom) = theme.gradient_colors();
    canvas.rect(
        0.0,
        0.0,
        1280.0,
        960.0,
        Color::rgba(0, 0, 0, (transition_t * 110.0) as u8),
    );
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

    let step = (650.0 / actions.len().max(1) as f32).min(82.0);
    let row_h = (step - 8.0).max(54.0);
    for (index, (row_focus, label, danger)) in actions.iter().enumerate() {
        let row_t =
            transitions::ease_out_back(transitions::stagger(transition_t, index, 0.04, 0.78));
        let selected = focus == *row_focus;
        let x = modal_x + 40.0;
        let y = modal_y + 170.0 + index as f32 * step + (1.0 - row_t) * 34.0;
        let w = modal_w - 80.0;
        let h = row_h;
        if selected {
            focus_halo(canvas, x, y, w, h, 24.0, seconds, index as f32 * 0.35);
            let fill = if *danger {
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
            let fill = if *danger {
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
            y + (h - 42.0) / 2.0,
            38.0,
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Bold,
        );
    }
}
