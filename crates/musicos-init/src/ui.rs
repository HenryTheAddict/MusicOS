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
pub const INK: Color = Color::rgb(18, 32, 42);
pub const INK_SECONDARY: Color = Color::rgb(71, 86, 103);

pub fn accent(canvas: &Canvas<'_>) -> Color {
    canvas
        .theme_top_color
        .unwrap_or(Color::rgb(0, 100, 223))
        .mix(Color::BLACK, 0.15)
}

pub fn surface(canvas: &Canvas<'_>) -> Color {
    Color::rgb(247, 249, 252).mix(accent(canvas), 0.04)
}

pub fn control(canvas: &Canvas<'_>) -> Color {
    Color::rgb(230, 235, 244).mix(accent(canvas), 0.12)
}

pub fn draw_figma_background(canvas: &mut Canvas<'_>) {
    canvas.clear(surface(canvas).mix(Color::BLACK, 0.06));
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
    let _ = (fill, text, seconds);
    let radius = radius.clamp(28.0, h / 2.0);
    let button_fill = if selected {
        accent(canvas)
    } else {
        control(canvas)
    };
    rr(canvas, x, y, w, h, radius, button_fill);

    if selected {
        register_focus(canvas, x, y, w, h, radius);
    }

    text_fit_centered(
        canvas,
        label,
        x + w / 2.0,
        y + (h - 48.0) / 2.0,
        w - 36.0,
        44.0,
        if selected { Color::WHITE } else { INK },
        FontWeight::Bold,
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
    rr(canvas, x, y, w, h, r.clamp(32.0, 48.0), surface(canvas));
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
    let _ = seconds;
    let radius = 28.0_f32.min(h / 2.0);
    if selected {
        rr(canvas, x, y, w, h, radius, accent(canvas));
        register_focus(canvas, x, y, w, h, radius);
    } else {
        rr(canvas, x, y, w, h, radius, control(canvas));
    }
}

pub fn register_focus(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, radius: f32) {
    canvas.register_selected(x - 4.0, y - 4.0, w + 8.0, h + 8.0, radius + 4.0, 4.0);
}

pub fn status_bar(
    canvas: &mut Canvas<'_>,
    title: &str,
    clock: &str,
    battery_text: &str,
    theme: theme::ThemeName,
    playing: bool,
) {
    rr(canvas, 0.0, 0.0, 1280.0, 64.0, 0.0, surface(canvas));
    canvas.text(clock, 20.0, 15.0, 30.0, INK_SECONDARY, FontWeight::Bold);
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
        INK,
        FontWeight::Bold,
    );

    let battery_label = battery_text
        .split_whitespace()
        .find(|part| part.contains('%'))
        .unwrap_or("—");
    rr(canvas, 1120.0, 10.0, 136.0, 44.0, 22.0, control(canvas));
    canvas.text_right(
        battery_label,
        1232.0,
        16.0,
        27.0,
        INK_SECONDARY,
        FontWeight::Bold,
    );
}

pub fn legend_bar(canvas: &mut Canvas<'_>, legend: &str) {
    rr(canvas, 0.0, 918.0, 1280.0, 42.0, 0.0, surface(canvas));
    canvas.text_centered(legend, 640.0, 928.0, 27.0, INK_SECONDARY, FontWeight::Bold);
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

pub fn liquid_glass(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, r: f32, _opacity: u8) {
    rr(canvas, x, y, w, h, r, control(canvas));
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
    let _ = theme;
    let menu_t = transitions::ease_out_cubic(transition_t);
    canvas.selected_box = None;
    rr(
        canvas,
        0.0,
        0.0,
        1280.0,
        960.0,
        0.0,
        Color::rgba(18, 32, 42, (menu_t * 134.0) as u8),
    );
    let modal_w = 584.0;
    let modal_h = 846.0;
    let modal_x = 1280.0 - menu_t * 608.0;
    let modal_y = 88.0;

    shadow_rr(canvas, modal_x, modal_y, modal_w, modal_h, 48.0, 4.0, 8.0);
    rr(
        canvas,
        modal_x,
        modal_y,
        modal_w,
        modal_h,
        48.0,
        surface(canvas),
    );

    canvas.text(
        "Options",
        modal_x + 40.0,
        modal_y + 36.0,
        48.0,
        INK,
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        title,
        modal_x + 40.0,
        modal_y + 98.0,
        modal_w - 80.0,
        32.0,
        INK_SECONDARY,
        FontWeight::Regular,
    );

    let step = (610.0 / actions.len().max(1) as f32).min(82.0);
    let row_h = step - 6.0;
    for (index, (row_focus, label, danger)) in actions.iter().enumerate() {
        let selected = focus == *row_focus;
        let x = modal_x + 40.0;
        let y = modal_y + 152.0 + index as f32 * step;
        let w = modal_w - 80.0;
        let h = row_h;
        figma_list_row(canvas, x, y, w, h, selected, seconds);
        let ink = if selected {
            Color::WHITE
        } else if *danger {
            Color::rgb(145, 33, 49)
        } else {
            INK
        };
        text_fit(
            canvas,
            label,
            x + 24.0,
            y + (h - 34.0) / 2.0,
            w - 48.0,
            33.0,
            ink,
            FontWeight::Bold,
        );
    }
    canvas.text(
        "A  Select     B  Back",
        modal_x + 40.0,
        modal_y + 788.0,
        27.0,
        INK_SECONDARY,
        FontWeight::Regular,
    );
}
