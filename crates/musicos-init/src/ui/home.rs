use crate::app::Focus;
use crate::audio::PlaybackStatus;
use crate::database::{Playlist, Track};
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_back, ease_out_cubic, stagger};
use crate::ui::{
    focus_halo, icon_tile, inner_shadow, liquid_glass, panel_gradient, rr, shadow_rr, FontWeight,
};

pub fn draw_home(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    seconds: f32,
    lang: Language,
    theme: ThemeName,
    playlists: &[Playlist],
    tracks: &[Track],
    status: &PlaybackStatus,
    cover_art: Option<&crate::renderer::CoverArt>,
    battery_text: &str,
    time_str: &str,
    transition_t: f32,
) {
    let panel_t = ease_out_cubic(transition_t);
    crate::ui::draw_figma_background(canvas);
    panel_gradient(
        canvas,
        20.0,
        32.0,
        588.0,
        896.0,
        128.0,
        Color::rgb(178, 246, 228),
        Color::WHITE,
    );

    canvas.text_centered(time_str, 314.0, 16.0, 128.0, Color::BLACK, FontWeight::Bold);

    canvas.text_centered(
        battery_text,
        314.0,
        166.0,
        32.0,
        Color::BLACK,
        FontWeight::Bold,
    );

    match focus {
        Focus::HomeNowPlaying => {
            draw_now_playing_side(canvas, status, tracks, cover_art, panel_t, seconds)
        }
        Focus::HomeApps => draw_apps_side(canvas, panel_t, seconds),
        Focus::HomeSettings => draw_settings_side(canvas, panel_t, seconds),
        _ => draw_music_side(canvas, playlists, tracks, focus, panel_t, seconds),
    }

    canvas.text_centered(
        lang.translate("context_help"),
        314.0,
        830.0,
        28.0,
        Color::BLACK,
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
        640.0,
        32.0,
        620.0,
        896.0,
        128.0,
        top_color,
        bottom_color,
    );

    let tiles = [
        (678.0, 121.0, "apps", matches!(focus, Focus::HomeApps)),
        (966.0, 121.0, "music", matches!(focus, Focus::HomeMusic)),
        (
            678.0,
            409.0,
            "settings",
            matches!(focus, Focus::HomeSettings),
        ),
        (966.0, 409.0, "now", matches!(focus, Focus::HomeNowPlaying)),
    ];
    for (i, (x, y, kind, selected)) in tiles.into_iter().enumerate() {
        let tile_t = ease_out_back(stagger(panel_t, i, 0.06, 0.82));
        let inset = (1.0 - tile_t) * 22.0;
        if selected {
            focus_halo(
                canvas,
                x - 8.0 + inset,
                y - 8.0 + inset,
                272.0,
                272.0,
                68.0,
                seconds,
                i as f32 * 0.4,
            );
        }
        icon_tile(canvas, x + inset, y + inset, kind, selected);
    }
}

fn draw_music_side(
    canvas: &mut Canvas<'_>,
    playlists: &[Playlist],
    tracks: &[Track],
    focus: Focus,
    panel_t: f32,
    seconds: f32,
) {
    let y_coords = [218.0, 380.0, 542.0, 704.0];
    for (i, &y) in y_coords.iter().enumerate() {
        let row_y = y + (1.0 - ease_out_cubic(stagger(panel_t, i, 0.08, 0.82))) * 44.0;
        let selected = matches!(
            (i, focus),
            (0, Focus::HomePlaylist1)
                | (1, Focus::HomePlaylist2)
                | (2, Focus::HomePlaylist3)
                | (3, Focus::HomePlaylist4)
        );
        if let Some(playlist) = playlists.get(i) {
            playlist_row(
                canvas,
                44.0,
                row_y,
                &playlist.name,
                "Playlist",
                selected,
                seconds,
                i as f32 * 0.35,
            );
        } else if let Some(track) = tracks.get(i) {
            playlist_row(
                canvas,
                44.0,
                row_y,
                &track.title,
                &track.artist,
                selected,
                seconds,
                i as f32 * 0.35,
            );
        }
    }
}

fn draw_now_playing_side(
    canvas: &mut Canvas<'_>,
    status: &PlaybackStatus,
    tracks: &[Track],
    cover_art: Option<&crate::renderer::CoverArt>,
    panel_t: f32,
    seconds: f32,
) {
    let Some(track) = &status.track else {
        for (idx, track) in tracks.iter().take(4).enumerate() {
            compact_song_row(
                canvas,
                44.0,
                218.0 + idx as f32 * 146.0,
                &track.title,
                &track.artist,
                idx,
                panel_t,
                seconds,
            );
        }
        return;
    };

    let card_y = 218.0 + (1.0 - ease_out_cubic(stagger(panel_t, 0, 0.08, 0.82))) * 44.0;
    shadow_rr(canvas, 44.0, card_y, 540.0, 240.0, 58.0, 5.0, 14.0);
    liquid_glass(canvas, 44.0, card_y, 540.0, 240.0, 58.0, 232);
    inner_shadow(canvas, 44.0, card_y, 540.0, 240.0, 58.0, 10.0);
    crate::ui::nowplaying::draw_cover_art(canvas, cover_art, 82.0, card_y + 42.0, 96.0, 18.0, 1.0);

    let (title, artist, duration, position) = (
        track.title.as_str(),
        track.artist.as_str(),
        track.duration,
        status.position,
    );
    crate::ui::text_fit(
        canvas,
        title,
        196.0,
        card_y + 48.0,
        330.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    crate::ui::text_fit(
        canvas,
        artist,
        196.0,
        card_y + 96.0,
        330.0,
        32.0,
        Color::BLACK,
        FontWeight::Regular,
    );
    let ratio = if duration > 0 {
        (position / duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    rr(canvas, 82.0, card_y + 150.0, 462.0, 10.0, 5.0, Color::BLACK);
    rr(
        canvas,
        82.0,
        card_y + 146.0,
        462.0 * ratio,
        18.0,
        9.0,
        canvas.theme_top_color.unwrap_or(Color::rgb(63, 143, 229)),
    );
    canvas.text(
        &format_time(position),
        82.0,
        card_y + 174.0,
        30.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    canvas.text_right(
        &format_time(duration as f32),
        544.0,
        card_y + 174.0,
        30.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    draw_side_icon(canvas, 228.0, card_y + 214.0, "previous.svg", 48.0);
    draw_side_icon(canvas, 314.0, card_y + 210.0, "pause.svg", 66.0);
    draw_side_icon(canvas, 400.0, card_y + 214.0, "skip.svg", 48.0);
    draw_side_icon(canvas, 490.0, card_y + 214.0, "Heart.svg", 54.0);

    for (idx, track) in tracks.iter().take(3).enumerate() {
        compact_song_row(
            canvas,
            44.0,
            512.0 + idx as f32 * 146.0,
            &track.title,
            &track.artist,
            idx,
            panel_t,
            seconds,
        );
    }
}

fn draw_apps_side(canvas: &mut Canvas<'_>, panel_t: f32, seconds: f32) {
    let rows = [
        ("Music", "library and albums", "Music.svg"),
        ("Now Playing", "big player view", "NowPlaying.svg"),
        ("Settings", "device controls", "settings.svg"),
        ("USB Sync", "import and update", "Apps.svg"),
    ];
    for (idx, (title, subtitle, icon)) in rows.into_iter().enumerate() {
        app_side_row(
            canvas,
            44.0,
            218.0 + idx as f32 * 154.0,
            title,
            subtitle,
            icon,
            idx,
            panel_t,
            seconds,
        );
    }
}

fn draw_settings_side(canvas: &mut Canvas<'_>, panel_t: f32, seconds: f32) {
    let rows = [
        ("General", "language, date, time"),
        ("Audio", "volume and playback"),
        ("Looks", "theme and brightness"),
        ("Get Update", "USB updater"),
    ];
    for (idx, (title, subtitle)) in rows.into_iter().enumerate() {
        compact_song_row(
            canvas,
            44.0,
            218.0 + idx as f32 * 154.0,
            title,
            subtitle,
            idx,
            panel_t,
            seconds,
        );
    }
}

fn compact_song_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    title: &str,
    subtitle: &str,
    index: usize,
    panel_t: f32,
    seconds: f32,
) {
    let y = y + (1.0 - ease_out_cubic(stagger(panel_t, index, 0.06, 0.82))) * 44.0;
    playlist_row(
        canvas,
        x,
        y,
        title,
        subtitle,
        false,
        seconds,
        index as f32 * 0.35,
    );
}

fn app_side_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    title: &str,
    subtitle: &str,
    icon: &str,
    index: usize,
    panel_t: f32,
    _seconds: f32,
) {
    let y = y + (1.0 - ease_out_cubic(stagger(panel_t, index, 0.06, 0.82))) * 44.0;
    shadow_rr(canvas, x, y, 540.0, 136.0, 68.0, 4.0, 12.0);
    liquid_glass(canvas, x, y, 540.0, 136.0, 68.0, 220);
    inner_shadow(canvas, x, y, 540.0, 136.0, 68.0, 12.0);
    let base_path = std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
    canvas.draw_svg(
        &format!("{}/icons/appicons/{}", base_path, icon),
        x + 36.0,
        y + 20.0,
        96.0,
        96.0,
    );
    crate::ui::text_fit(
        canvas,
        title,
        x + 154.0,
        y + 28.0,
        340.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    crate::ui::text_fit(
        canvas,
        subtitle,
        x + 154.0,
        y + 78.0,
        340.0,
        30.0,
        Color::BLACK,
        FontWeight::Regular,
    );
}

fn draw_side_icon(canvas: &mut Canvas<'_>, cx: f32, cy: f32, filename: &str, size: f32) {
    let base_path = std::env::var("MUSICOS_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
    canvas.draw_svg(
        &format!("{}/icons/nowplaying/{}", base_path, filename),
        cx - size / 2.0,
        cy - size / 2.0,
        size,
        size,
    );
}

fn playlist_row(
    canvas: &mut Canvas<'_>,
    x: f32,
    y: f32,
    title: &str,
    subtitle: &str,
    selected: bool,
    seconds: f32,
    phase: f32,
) {
    if selected {
        focus_halo(canvas, x, y, 540.0, 136.0, 68.0, seconds, phase);
    }
    shadow_rr(canvas, x, y, 540.0, 136.0, 68.0, 4.0, 12.0);
    liquid_glass(canvas, x, y, 540.0, 136.0, 68.0, 220);
    inner_shadow(canvas, x, y, 540.0, 136.0, 68.0, 12.0);
    rr(canvas, x + 40.0, y + 20.0, 96.0, 96.0, 16.0, Color::BLACK);

    if selected {
        canvas.register_selected(x, y, 540.0, 136.0, 68.0, 8.0);
    }

    crate::ui::text_fit(
        canvas,
        title,
        x + 144.0,
        y + 28.0,
        350.0,
        48.0,
        Color::BLACK,
        FontWeight::Bold,
    );
    crate::ui::text_fit(
        canvas,
        subtitle,
        x + 144.0,
        y + 76.0,
        340.0,
        32.0,
        Color::BLACK,
        FontWeight::Bold,
    );
}

fn format_time(sec: f32) -> String {
    let sec = sec.max(0.0);
    let m = (sec / 60.0) as i32;
    let s = (sec % 60.0) as i32;
    format!("{m}:{s:02}")
}
