#![allow(dead_code)]

use crate::app::{Focus, HomePane};
use crate::audio::PlaybackStatus;
use crate::database::{Playlist, Track};
use crate::renderer::{Canvas, Color};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::ui::transitions::{ease_out_cubic, stagger};
use crate::ui::{
    figma_list_row, focus_halo, inner_shadow, liquid_glass, rr, shadow_rr, FontWeight,
};

pub fn draw_home(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    home_pane: HomePane,
    main_index: usize,
    seconds: f32,
    lang: Language,
    _theme: ThemeName,
    playlists: &[Playlist],
    tracks: &[Track],
    status: &PlaybackStatus,
    cover_art: Option<&crate::renderer::CoverArt>,
    battery_text: &str,
    time_str: &str,
    transition_t: f32,
) {
    let _ = (lang, battery_text, time_str, transition_t, seconds);
    crate::ui::draw_figma_background(canvas);
    rr(
        canvas,
        514.0,
        82.0,
        742.0,
        828.0,
        48.0,
        crate::ui::surface(canvas),
    );

    let menu = ["Music", "Now Playing", "Apps", "Settings"];
    let menu_icons = [
        ("Music.svg", "MusicSelected.svg"),
        ("NowPlaying.svg", "NowPlayingSelected.svg"),
        ("Apps.svg", "AppsSelected.svg"),
        ("settings.svg", "settingsSelected.svg"),
    ];
    let icon_base = crate::ui::assets_dir();
    for (index, label) in menu.into_iter().enumerate() {
        let y = 132.0 + index as f32 * 116.0;
        let selected = home_pane == HomePane::Main && main_index == index;
        figma_list_row(canvas, 28.0, y, 454.0, 96.0, selected, seconds);
        let icon_file = if selected {
            menu_icons[index].1
        } else {
            menu_icons[index].0
        };
        canvas.draw_svg(
            &format!("{icon_base}/icons/appicons/{icon_file}"),
            44.0,
            y + 18.0,
            60.0,
            60.0,
        );
        crate::ui::text_fit(
            canvas,
            label,
            118.0,
            y + 18.0,
            290.0,
            46.0,
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Bold,
        );
        canvas.text_right(
            "›",
            452.0,
            y + 20.0,
            52.0,
            if selected {
                Color::WHITE
            } else {
                Color::rgb(103, 108, 114)
            },
            FontWeight::Bold,
        );
    }

    let (heading, description) = match main_index {
        0 => ("Music", "Recently added tracks and playlists"),
        1 => ("Now Playing", "Playback and queue controls"),
        2 => ("Extras", "MusicOS shortcuts and utilities"),
        _ => ("Settings", "Device preferences"),
    };
    canvas.text(
        heading,
        556.0,
        94.0,
        40.0,
        Color::rgb(54, 58, 64),
        FontWeight::Bold,
    );
    canvas.text(
        description,
        556.0,
        144.0,
        28.0,
        Color::rgb(103, 108, 114),
        FontWeight::Regular,
    );

    if main_index == 0 {
        let art_x = 814.0;
        let art_y = 190.0;
        let art_size = 210.0;
        shadow_rr(canvas, art_x, art_y, art_size, art_size, 8.0, 3.0, 8.0);
        crate::ui::nowplaying::draw_cover_art(canvas, cover_art, art_x, art_y, art_size, 8.0, 1.0);
        if let Some(track) = status.track.as_ref().or_else(|| tracks.first()) {
            crate::ui::text_fit_centered(
                canvas,
                &track.title,
                920.0,
                420.0,
                610.0,
                38.0,
                Color::BLACK,
                FontWeight::Bold,
            );
            crate::ui::text_fit_centered(
                canvas,
                &track.artist,
                920.0,
                462.0,
                610.0,
                27.0,
                Color::rgb(94, 99, 105),
                FontWeight::Regular,
            );
        }
    }

    let detail_base_y = if main_index == 0 { 532.0 } else { 216.0 };
    if main_index != 0 {
        let detail_caption = match main_index {
            1 => "Quick controls",
            2 => "Shortcuts",
            _ => "Categories",
        };
        canvas.text(
            detail_caption,
            556.0,
            detail_base_y - 36.0,
            22.0,
            Color::rgb(131, 136, 142),
            FontWeight::Bold,
        );
    }
    let detail_count = if main_index == 0 {
        tracks.len().max(playlists.len()).min(4)
    } else {
        4
    };
    for index in 0..detail_count {
        let y = detail_base_y + index as f32 * 84.0;
        let selected = matches!(
            (index, focus),
            (0, Focus::HomePlaylist1)
                | (1, Focus::HomePlaylist2)
                | (2, Focus::HomePlaylist3)
                | (3, Focus::HomePlaylist4)
        );
        let name = match main_index {
            0 => playlists
                .get(index)
                .map(|playlist| playlist.name.as_str())
                .or_else(|| tracks.get(index).map(|track| track.title.as_str()))
                .unwrap_or("No item"),
            1 => ["Play / pause", "Previous track", "Next track", "Open queue"][index],
            2 => ["Music", "Now Playing", "USB Sync", "Utilities"][index],
            _ => ["General", "Audio", "Appearance", "Updates"][index],
        };
        figma_list_row(canvas, 548.0, y, 694.0, 66.0, selected, seconds);
        crate::ui::text_fit(
            canvas,
            name,
            568.0,
            y + 14.0,
            610.0,
            34.0,
            if selected { Color::WHITE } else { Color::BLACK },
            FontWeight::Bold,
        );
        canvas.text_right(
            "›",
            1210.0,
            y + 12.0,
            38.0,
            if selected {
                Color::WHITE
            } else {
                Color::rgb(103, 108, 114)
            },
            FontWeight::Bold,
        );
    }

    if main_index == 0 && tracks.is_empty() && playlists.is_empty() {
        crate::ui::text_fit_centered(
            canvas,
            "No music yet",
            885.0,
            444.0,
            640.0,
            42.0,
            crate::ui::INK,
            FontWeight::Bold,
        );
        crate::ui::text_fit_centered(
            canvas,
            "Import songs from Settings to get started.",
            885.0,
            508.0,
            640.0,
            29.0,
            crate::ui::INK_SECONDARY,
            FontWeight::Regular,
        );
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
    if playlists.is_empty() && tracks.is_empty() {
        liquid_glass(canvas, 44.0, 286.0, 540.0, 250.0, 32.0, 210);
        canvas.text_centered(
            "No music yet",
            314.0,
            338.0,
            64.0,
            Color::BLACK,
            FontWeight::Bold,
        );
        crate::ui::text_fit_centered(
            canvas,
            "Sync from the desktop to get started.",
            314.0,
            432.0,
            450.0,
            30.0,
            Color::rgba(0, 0, 0, 165),
            FontWeight::Regular,
        );
        return;
    }

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
    shadow_rr(canvas, 44.0, card_y, 540.0, 240.0, 32.0, 5.0, 14.0);
    liquid_glass(canvas, 44.0, card_y, 540.0, 240.0, 32.0, 232);
    inner_shadow(canvas, 44.0, card_y, 540.0, 240.0, 32.0, 10.0);
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
    draw_side_icon(canvas, 228.0, card_y + 194.0, "previous.svg", 48.0);
    draw_side_icon(canvas, 314.0, card_y + 190.0, "pause.svg", 66.0);
    draw_side_icon(canvas, 400.0, card_y + 194.0, "skip.svg", 48.0);
    draw_side_icon(canvas, 490.0, card_y + 194.0, "Heart.svg", 54.0);

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
    shadow_rr(canvas, x, y, 540.0, 136.0, 24.0, 4.0, 12.0);
    liquid_glass(canvas, x, y, 540.0, 136.0, 24.0, 220);
    inner_shadow(canvas, x, y, 540.0, 136.0, 24.0, 12.0);
    let base_path = crate::ui::assets_dir();
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
    let base_path = crate::ui::assets_dir();
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
        focus_halo(canvas, x, y, 540.0, 136.0, 24.0, seconds, phase);
    }
    shadow_rr(canvas, x, y, 540.0, 136.0, 24.0, 4.0, 12.0);
    liquid_glass(canvas, x, y, 540.0, 136.0, 24.0, 220);
    inner_shadow(canvas, x, y, 540.0, 136.0, 24.0, 12.0);
    rr(canvas, x + 40.0, y + 20.0, 96.0, 96.0, 16.0, Color::BLACK);

    if selected {
        canvas.register_selected(x, y, 540.0, 136.0, 24.0, 8.0);
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
