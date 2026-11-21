use crate::app::Focus;
use crate::database::LibraryCounts;
use crate::renderer::{Canvas, Resolution};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{text_fit, FontWeight};
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
    crate::ui::page_heading(
        canvas,
        "Library",
        &format!(
            "{} tracks · {} albums · {} playlists · {} pins",
            counts.tracks, counts.albums, counts.playlists, counts.pins
        ),
    );

    let _ = theme;
    let items = [
        (Focus::AppsMusic, "Music", "Library", "Music.svg", 92.0),
        (
            Focus::AppsNowPlaying,
            "Now Playing",
            "Player",
            "NowPlaying.svg",
            250.0,
        ),
        (
            Focus::AppsSettings,
            "Settings",
            "Device",
            "settings.svg",
            408.0,
        ),
        (Focus::AppsUsbSync, "USB Sync", "Import", "Apps.svg", 566.0),
    ];

    for (index, (item_focus, label, subtitle, icon, y)) in items.into_iter().enumerate() {
        let row_t = ease_out_back(stagger(transition_t, index, 0.06, 0.78));
        let selected = focus == item_focus;
        let x = 44.0 - (1.0 - row_t) * 26.0;
        let subtitle = if item_focus == Focus::AppsUsbSync {
            usb_status.message.as_str()
        } else {
            subtitle
        };
        app_row(
            canvas,
            x,
            y + 120.0,
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
    let w = 1192.0;
    let h = 116.0;
    let _ = (seconds, phase);
    crate::ui::compact_row(canvas, x, y, w, h, selected);

    let base_path = crate::ui::assets_dir();
    let path = format!("{}/icons/appicons/{}", base_path, icon_filename);
    canvas.draw_svg(&path, x + 28.0, y + 20.0, 76.0, 76.0);

    text_fit(
        canvas,
        label,
        x + 130.0,
        y + 11.0,
        980.0,
        42.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        subtitle,
        x + 130.0,
        y + 64.0,
        980.0,
        30.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
    canvas.text_right(
        "›",
        x + w - 24.0,
        y + 36.0,
        42.0,
        crate::ui::ink_secondary(canvas),
        FontWeight::Bold,
    );
}

pub fn draw_about(
    canvas: &mut Canvas<'_>,
    _focus: Focus,
    seconds: f32,
    _theme: ThemeName,
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
    let display = format!("{}x{}", resolution.width, resolution.height);
    let queue = format!("{queue_len} queued");
    let library = format!("{} songs / {} albums", counts.tracks, counts.albums);
    let playlists = format!(
        "{} playlists / {} favorites",
        counts.playlists, counts.favorites
    );
    let usb = format!("{} indexed", usb_status.media_files_indexed);
    let rows = [
        ("Device", device_name),
        ("User", user_name),
        ("Display", display.as_str()),
        ("Library", library.as_str()),
        ("Playlists", playlists.as_str()),
        ("Queue", queue.as_str()),
        ("Audio", audio_label),
        ("Scan", scan_status),
        ("Battery", battery_status),
        ("USB Media", usb.as_str()),
        ("Audio test", audio_test_status),
        ("Library data", db_path),
        ("Settings data", settings_path),
    ];

    for (index, (label, value)) in rows.into_iter().enumerate() {
        status_row(
            canvas,
            44.0,
            82.0 + index as f32 * 59.0,
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
    let x = x - (1.0 - row_t) * 38.0;
    let _ = seconds;
    crate::ui::compact_row(canvas, x, y, 1192.0, 56.0, false);
    canvas.text(
        label,
        x + 32.0,
        y + 10.0,
        32.0,
        crate::ui::ink(canvas),
        FontWeight::Bold,
    );
    text_fit(
        canvas,
        value,
        x + 303.0,
        y + 12.0,
        856.0,
        32.0,
        crate::ui::ink(canvas),
        FontWeight::Regular,
    );
}
