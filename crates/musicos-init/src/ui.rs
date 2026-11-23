pub mod apps;
pub mod globe;
pub mod home;
pub mod library;
pub mod localization;
pub mod music;
pub mod nowplaying;
pub mod screens;
pub mod settings;
pub mod theme;
pub mod transitions;

use crate::renderer::{Canvas, Color, CoverArt};

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

pub fn accent(canvas: &Canvas<'_>) -> Color {
    canvas.theme_top_color.unwrap_or(Color::rgb(171, 51, 23))
}

pub fn ink(canvas: &Canvas<'_>) -> Color {
    if canvas.dark_theme {
        Color::rgb(238, 233, 229)
    } else {
        INK
    }
}

pub fn ink_secondary(canvas: &Canvas<'_>) -> Color {
    if canvas.dark_theme {
        Color::rgb(194, 178, 169)
    } else {
        Color::rgb(78, 65, 60)
    }
}

pub fn on_accent(canvas: &Canvas<'_>) -> Color {
    if canvas.dark_theme {
        Color::rgb(35, 25, 23)
    } else {
        Color::WHITE
    }
}

pub fn surface(canvas: &Canvas<'_>) -> Color {
    if canvas.dark_theme {
        Color::rgb(38, 33, 31)
    } else {
        Color::rgb(249, 240, 234)
    }
}

pub fn control(canvas: &Canvas<'_>) -> Color {
    if canvas.dark_theme {
        surface(canvas).mix(accent(canvas), 0.13)
    } else {
        surface(canvas).mix(accent(canvas), 0.11)
    }
}

pub fn selected_surface(canvas: &Canvas<'_>) -> Color {
    surface(canvas).mix(accent(canvas), if canvas.dark_theme { 0.24 } else { 0.21 })
}

pub(crate) fn moving_selected_surface(canvas: &Canvas<'_>, x: f32, y: f32) -> Color {
    let opacity = canvas.focus_origin.map_or(1.0, |(from_x, from_y)| {
        let distance = (x - 2.0 - from_x).abs() + (y - 2.0 - from_y).abs();
        (1.0 - distance / 100.0).clamp(0.0, 1.0)
    });
    selected_surface(canvas).with_alpha((opacity * 255.0).round() as u8)
}
pub fn shell(canvas: &Canvas<'_>) -> Color {
    if canvas.dark_theme {
        Color::rgb(26, 22, 21)
    } else {
        Color::rgb(240, 226, 216)
    }
}

pub fn draw_figma_background(canvas: &mut Canvas<'_>) {
    canvas.clear(shell(canvas));
}

pub(crate) fn draw_art_wash(canvas: &mut Canvas<'_>, art: &CoverArt, height: f32) {
    if art.width == 0 || art.height == 0 {
        return;
    }
    let mut channels = [0u32; 3];
    for y in [1, 2, 3] {
        for x in [1, 2, 3] {
            let sample = art.pixels[(y * art.height / 4 * art.width + x * art.width / 4) as usize];
            for (sum, color) in channels.iter_mut().zip([sample.r, sample.g, sample.b]) {
                *sum += u32::from(color);
            }
        }
    }
    let [r, g, b] = channels.map(|channel| (channel / 9) as u8);
    let base = shell(canvas);
    let tint = base.mix(Color::rgb(r, g, b), 0.17);
    let resolution = canvas.resolution();
    let rows = resolution.scale_y(height).round() as u32;
    for y in 0..rows {
        canvas.rect(
            0.0,
            y as f32,
            resolution.width as f32,
            1.0,
            tint.mix(base, y as f32 / rows.saturating_sub(1).max(1) as f32),
        );
    }
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
        accent(canvas),
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
        if selected {
            on_accent(canvas)
        } else {
            ink(canvas)
        },
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
    let radius = (h * 0.5).min(42.0);
    if selected {
        rr(canvas, x, y, w, h, radius, selected_surface(canvas));
        register_focus(canvas, x, y, w, h, radius);
    } else {
        rr(canvas, x, y, w, h, radius, control(canvas));
    }
}

pub fn register_focus(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, radius: f32) {
    canvas.register_selected(x - 2.0, y - 2.0, w + 4.0, h + 4.0, radius + 2.0, 4.0);
}

pub(crate) fn compact_row(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, h: f32, selected: bool) {
    if selected {
        rr(
            canvas,
            x,
            y,
            w,
            h,
            12.0,
            moving_selected_surface(canvas, x, y),
        );
        register_focus(canvas, x, y, w, h, 12.0);
    } else {
        rr(
            canvas,
            x + 12.0,
            y + h - 2.0,
            w - 24.0,
            2.0,
            0.0,
            control(canvas),
        );
    }
}

pub fn status_bar(
    canvas: &mut Canvas<'_>,
    title: &str,
    clock: &str,
    battery_text: &str,
    _theme: theme::ThemeName,
    _playing: bool,
) {
    let secondary = ink_secondary(canvas);
    let resolution = canvas.resolution();
    canvas.rect(
        resolution.scale_x(0.0),
        resolution.scale_y(0.0),
        1280.0 * resolution.scale(),
        64.0 * resolution.scale(),
        shell(canvas),
    );
    canvas.text(clock, 40.0, 11.0, 32.0, secondary, FontWeight::Regular);
    text_fit_centered(
        canvas,
        title,
        640.0,
        9.0,
        650.0,
        38.0,
        ink(canvas),
        FontWeight::Bold,
    );

    let battery = battery_text
        .split_whitespace()
        .find(|part| part.ends_with('%'));
    canvas.text_right(
        battery.unwrap_or("—"),
        1170.0,
        11.0,
        32.0,
        secondary,
        FontWeight::Regular,
    );
    rr_stroke(canvas, 1184.0, 18.0, 50.0, 24.0, 7.0, 3.0, secondary);
    rr(canvas, 1234.0, 25.0, 4.0, 10.0, 2.0, secondary);
    if let Some(percent) = battery.and_then(|label| label.trim_end_matches('%').parse::<f32>().ok())
    {
        rr(
            canvas,
            1190.0,
            24.0,
            38.0 * (percent / 100.0).clamp(0.0, 1.0),
            12.0,
            3.0,
            secondary,
        );
    }
}

pub fn legend_bar(canvas: &mut Canvas<'_>, legend: &str) {
    legend_bar_at(canvas, legend, 918.0, 0.0);
}

pub(crate) fn home_legend_bar(canvas: &mut Canvas<'_>, legend: &str) {
    legend_bar_at(canvas, legend, 878.0, 10.0);
}

pub(crate) fn rows_preview(canvas: &mut Canvas<'_>) {
    draw_figma_background(canvas);
    rr(canvas, 652.0, 68.0, 604.0, 797.0, 32.0, surface(canvas));
}

pub(crate) fn page_heading(canvas: &mut Canvas<'_>, title: &str, subtitle: &str) {
    text_fit(
        canvas,
        title,
        48.0,
        100.0,
        1184.0,
        45.0,
        ink(canvas),
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        subtitle,
        48.0,
        160.0,
        1184.0,
        30.0,
        ink_secondary(canvas),
        FontWeight::Regular,
    );
}

fn legend_bar_at(canvas: &mut Canvas<'_>, legend: &str, top: f32, inset: f32) {
    let resolution = canvas.resolution();
    canvas.rect(
        resolution.scale_x(0.0),
        resolution.scale_y(top),
        1280.0 * resolution.scale(),
        (960.0 - top) * resolution.scale(),
        shell(canvas),
    );
    let y = top + inset;
    let height = if inset > 0.0 { 48.0 } else { 38.0 };
    let mut x = 40.0;
    for action in legend.split("     ") {
        let Some((button, label)) = action.split_once("  ") else {
            continue;
        };
        let press = canvas
            .legend_press
            .filter(|(key, _)| *key == button)
            .map_or(0.0, |(_, strength)| strength);
        let button = match button {
            "A" => "CLICK",
            "B" => "BACK",
            "X" => "CONTEXT",
            _ => button,
        };
        let font_size = 30.0;
        let chip = (canvas.text_width(button, font_size, FontWeight::Bold) + 30.0).max(38.0);
        rr(
            canvas,
            x,
            y + 2.0 + 3.0 * press,
            chip,
            height,
            height / 2.0,
            control(canvas).mix(accent(canvas), 0.28 * press),
        );
        canvas.text_centered(
            button,
            x + chip * 0.5,
            y + 4.0 + 3.0 * press,
            font_size,
            ink_secondary(canvas),
            FontWeight::Bold,
        );
        canvas.text(
            label,
            x + chip + 12.0,
            y + 6.0,
            30.0,
            ink_secondary(canvas),
            FontWeight::Regular,
        );
        x += chip + 44.0 + canvas.text_width(label, 30.0, FontWeight::Regular);
    }
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
    rr(canvas, x, y, 8.0, h, 4.0, control(canvas));
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
        accent(canvas),
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
    let modal_h = 812.0;
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
        ink(canvas),
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        title,
        modal_x + 40.0,
        modal_y + 98.0,
        modal_w - 80.0,
        36.0,
        ink_secondary(canvas),
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
        let ink = if *danger { accent(canvas) } else { ink(canvas) };
        text_fit(
            canvas,
            label,
            x + 24.0,
            y + (h - 38.0) / 2.0,
            w - 48.0,
            36.0,
            ink,
            FontWeight::Bold,
        );
    }
}
