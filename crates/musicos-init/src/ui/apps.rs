use crate::app::Focus;
use crate::database::LibraryCounts;
use crate::renderer::{Canvas, Color, Resolution};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{focus_halo, panel_gradient, rr_stroke, text_fit, FontWeight};
use crate::usb::UsbStatus;

pub fn draw_apps(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    _lang: Language,
    theme: ThemeName,
    counts: LibraryCounts,
    usb_status: &UsbStatus,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);

    crate::ui::shadow_rr(canvas, 31.0, 37.0, 370.0, 886.0, 100.0, 8.0, 16.0);
    crate::ui::rr(
        canvas,
        31.0,
        37.0,
        370.0,
        886.0,
        100.0,
        Color::rgb(178, 246, 228),
    );
    crate::ui::liquid_glass(canvas, 31.0, 37.0, 370.0, 886.0, 100.0, 32);

    canvas.text_centered("Apps", 216.0, 120.0, 96.0, Color::BLACK, FontWeight::Bold);

    let stat_y = 280.0;
    canvas.text_centered(
        &format!("Tracks: {}", counts.tracks),
        216.0,
        stat_y,
        32.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_centered(
        &format!("Albums: {}", counts.albums),
        216.0,
        stat_y + 60.0,
        32.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_centered(
        &format!("Playlists: {}", counts.playlists),
        216.0,
        stat_y + 120.0,
        32.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_centered(
        &format!("Pins: {}", counts.pins),
        216.0,
        stat_y + 180.0,
        32.0,
        Color::BLACK,
        FontWeight::Bold,
    );

    canvas.text_centered(
        "USB Sync",
        216.0,
        stat_y + 300.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_centered(
        &usb_status.message,
        216.0,
        stat_y + 360.0,
        24.0,
        Color::BLACK,
        FontWeight::Regular,
    );
    canvas.text_centered(
        &format!("{} mounted", usb_status.roots_checked.len()),
        216.0,
        stat_y + 400.0,
        24.0,
        Color::BLACK,
        FontWeight::Regular,
    );

    let (theme_top, theme_bottom) = theme.gradient_colors();
    crate::ui::shadow_rr(canvas, 427.0, 37.0, 822.0, 886.0, 100.0, 8.0, 16.0);
    panel_gradient(
        canvas,
        427.0,
        37.0,
        822.0,
        886.0,
        100.0,
        theme_top,
        theme_bottom,
    );
    crate::ui::liquid_glass(canvas, 427.0, 37.0, 822.0, 886.0, 100.0, 28);

    let items = [
        (Focus::AppsMusic, "Music", "Library", "Music.svg", 92.0),
        (
            Focus::AppsNowPlaying,
            "Now Playing",
            "Player",
            "NowPlaying.svg",
            248.0,
        ),
        (
            Focus::AppsSettings,
            "Settings",
            "Device",
            "settings.svg",
            404.0,
        ),
        (Focus::AppsUsbSync, "USB Sync", "Import", "Apps.svg", 560.0),
    ];

    for (index, (item_focus, label, subtitle, icon, y)) in items.into_iter().enumerate() {
        let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.78));
        let selected = focus == item_focus;
        let x = 487.0 + (1.0 - row_t) * 72.0;
        let subtitle = if item_focus == Focus::AppsUsbSync {
            usb_status.message.as_str()
        } else {
            subtitle
        };
        app_row(
            canvas,
            x,
            y,
            label,
            subtitle,
            icon,
            selected,
            seconds,
            index as f32 * 0.35,
        );
    }
}

fn app_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    label: &str,
    subtitle: &str,
    icon_filename: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    let w = 702.0;
    let h = 138.0;
    if selected {
        focus_halo(canvas, x, y, w, h, 69.0, seconds, phase);
        crate::ui::shadow_rr(canvas, x, y, w, h, 69.0, 4.0, 12.0);
        crate::ui::liquid_glass(canvas, x, y, w, h, 69.0, 204);
        canvas.register_selected(x, y, w, h, 69.0, 8.0);
    } else {
        crate::ui::liquid_glass(canvas, x, y, w, h, 69.0, 54);
    }

    let base_path = std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
    let path = format!("{}/icons/appicons/{}", base_path, icon_filename);
    canvas.draw_svg(&path, x + 28.0, y + 16.0, 106.0, 106.0);

    text_fit(
        canvas,
        label,
        x + 160.0,
        y + 22.0,
        480.0,
        54.0,
        Color::BLACK,
        FontWeight::Regular,
    );
    text_fit(
        canvas,
        subtitle,
        x + 160.0,
        y + 80.0,
        480.0,
        30.0,
        Color::rgba(0, 0, 0, 178),
        FontWeight::Bold,
    );
}

pub fn draw_about(
    canvas: &mut Canvas<'_>,
    _focus: Focus,
    seconds: f32,
    theme: ThemeName,
    counts: LibraryCounts,
    usb_status: &UsbStatus,
    resolution: Resolution,
    user_name: &str,
    device_name: &str,
    queue_len: usize,
    audio_label: &str,
    scan_status: &str,
    audio_test_status: &str,
    battery_status: &str,
    db_path: &str,
    settings_path: &str,
    transition_t: f32,
) {
    let transition_t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);
    panel_gradient(
        canvas,
        20.0,
        32.0,
        375.0,
        896.0,
        128.0,
        Color::rgb(178, 246, 228),
        Color::WHITE,
    );
    canvas.text_centered("About", 208.0, 60.0, 96.0, Color::BLACK, FontWeight::Bold);
    canvas.text_centered(
        "MusicOS\nDaily Driver",
        208.0,
        240.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    crate::ui::centered_lines(
        canvas,
        &[device_name, user_name, audio_test_status],
        208.0,
        430.0,
        30.0,
        48.0,
        Color::BLACK,
        FontWeight::Regular,
    );
    text_fit(
        canvas,
        db_path,
        64.0,
        670.0,
        292.0,
        26.0,
        Color::rgba(0, 0, 0, 190),
        FontWeight::Regular,
    );
    text_fit(
        canvas,
        settings_path,
        64.0,
        724.0,
        292.0,
        26.0,
        Color::rgba(0, 0, 0, 190),
        FontWeight::Regular,
    );

    let (top_color, bottom_color) =
        if let (Some(top), Some(bottom)) = (canvas.theme_top_color, canvas.theme_bottom_color) {
            (top, bottom)
        } else {
            theme.gradient_colors()
        };
    panel_gradient(
        canvas,
        427.0,
        32.0,
        832.0,
        896.0,
        128.0,
        top_color,
        bottom_color,
    );

    let display = format!("{}x{}", resolution.width, resolution.height);
    let queue = format!("{queue_len} queued");
    let library = format!("{} songs / {} albums", counts.tracks, counts.albums);
    let playlists = format!("{} playlists / {} favorites", counts.playlists, counts.favorites);
    let usb = format!("{} indexed", usb_status.media_files_indexed);
    let rows = [
        ("Display", display.as_str()),
        ("Library", library.as_str()),
        ("Playlists", playlists.as_str()),
        ("Queue", queue.as_str()),
        ("Audio", audio_label),
        ("Scan", scan_status),
        ("Battery", battery_status),
        ("USB Media", usb.as_str()),
    ];

    for (index, (label, value)) in rows.into_iter().enumerate() {
        status_row(
            canvas,
            505.0,
            78.0 + index as f32 * 100.0,
            label,
            value,
            index,
            transition_t,
            seconds,
        );
    }
}

fn status_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    label: &str,
    value: &str,
    index: usize,
    transition_t: f32,
    seconds: f32,
) {
    let row_t = ease_out_back(stagger(transition_t, index, 0.05, 0.76));
    let x = x + (1.0 - row_t) * 64.0;
    let shimmer = crate::ui::transitions::pulse01(seconds, 3.2, index as f32 * 0.28);
    crate::ui::liquid_glass(canvas, x, y, 660.0, 96.0, 48.0, 64);
    rr_stroke(
        canvas,
        x,
        y,
        660.0,
        96.0,
        48.0,
        2.0 + shimmer,
        Color::rgba(255, 255, 255, (100.0 + shimmer * 50.0).round() as u8),
    );
    canvas.text(
        label,
        x + 32.0,
        y + 26.0,
        36.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        value,
        x + 302.0,
        y + 28.0,
        322.0,
        36.0,
        Color::BLACK,
        FontWeight::Regular,
    );
}
