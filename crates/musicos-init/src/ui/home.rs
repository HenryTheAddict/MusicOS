use std::collections::BTreeMap;
use std::path::Path;

use crate::app::{Focus, HomePane};
use crate::audio::PlaybackStatus;
use crate::database::{LibraryCounts, Playlist, Track};
use crate::renderer::{Canvas, CoverArt};
use crate::ui::nowplaying::draw_cover_art;
use crate::ui::{
    accent, assets_dir, control, draw_figma_background, figma_list_row, ink, ink_secondary,
    on_accent, register_focus, rr, selected_surface, shadow_rr, surface, text_fit, FontWeight,
};

pub(crate) struct AlbumPreview {
    title: String,
    cover: Option<CoverArt>,
}

pub(crate) fn album_previews(tracks: &[Track]) -> Vec<AlbumPreview> {
    let mut albums: BTreeMap<&str, &Track> = BTreeMap::new();
    for track in tracks.iter().filter(|track| !track.album.is_empty()) {
        let entry = albums.entry(track.album.as_str()).or_insert(track);
        if track.indexed_at > entry.indexed_at
            || (track.indexed_at == entry.indexed_at && entry.cover_art_path.is_none())
        {
            *entry = track;
        }
    }
    let mut albums: Vec<_> = albums.into_values().collect();
    albums.sort_by_key(|track| std::cmp::Reverse(track.indexed_at));
    albums
        .into_iter()
        .take(6)
        .map(|track| AlbumPreview {
            title: track.album.clone(),
            cover: track
                .cover_art_path
                .as_deref()
                .and_then(|path| crate::media::load_cover_art_file(Path::new(path))),
        })
        .collect()
}

pub(crate) fn draw_home(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    pane: HomePane,
    main_index: usize,
    counts: LibraryCounts,
    previews: &[AlbumPreview],
    playlists: &[Playlist],
    tracks: &[Track],
    status: &PlaybackStatus,
    has_queue: bool,
    cover_art: Option<&CoverArt>,
) {
    draw_figma_background(canvas);
    rr(canvas, 652.0, 68.0, 604.0, 797.0, 60.0, surface(canvas));
    for (index, (label, icon, count)) in [
        ("Music", "music.svg", Some(counts.tracks)),
        ("Now Playing", "play.svg", None),
        ("Apps", "device.svg", None),
        ("Settings", "settings.svg", None),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 70.0 + index as f32 * 112.0;
        let selected = pane == HomePane::Main && main_index == index;
        if selected {
            rr(canvas, 24.0, y, 600.0, 98.0, 49.0, selected_surface(canvas));
            register_focus(canvas, 24.0, y, 600.0, 98.0, 49.0);
        }
        let fill = if selected {
            accent(canvas)
        } else {
            control(canvas)
        };
        let glyph = if selected {
            on_accent(canvas)
        } else {
            accent(canvas)
        };
        rr(canvas, 40.0, y + 13.0, 72.0, 72.0, 36.0, fill);
        canvas.draw_svg_tinted(
            &format!("{}/icons/desktop/{icon}", assets_dir()),
            57.0,
            y + 30.0,
            38.0,
            38.0,
            glyph,
        );
        canvas.text(
            label,
            136.0,
            y + 23.0,
            43.0,
            ink(canvas),
            if selected {
                FontWeight::Bold
            } else {
                FontWeight::Regular
            },
        );
        if let Some(count) = count {
            canvas.text_right(
                &count.to_string(),
                598.0,
                y + 34.0,
                32.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
        }
        if index == 1 && status.is_playing {
            for (x, h) in [(565.0, 18.0), (578.0, 32.0), (591.0, 23.0)] {
                rr(canvas, x, y + 64.0 - h, 7.0, h, 3.5, accent(canvas));
            }
        }
    }

    if pane == HomePane::Detail || main_index >= 2 {
        draw_detail(
            canvas,
            focus,
            main_index,
            playlists,
            tracks,
            status.track.is_some() || has_queue,
        );
    } else if main_index == 1 {
        if let Some(track) = &status.track {
            shadow_rr(canvas, 685.0, 100.0, 540.0, 540.0, 42.0, 7.0, 12.0);
            draw_cover_art(canvas, cover_art, 685.0, 100.0, 540.0, 42.0, 1.0);
            text_fit(
                canvas,
                &track.title,
                685.0,
                655.0,
                540.0,
                50.0,
                ink(canvas),
                FontWeight::Bold,
            );
            text_fit(
                canvas,
                &format!("{} · {}", track.artist, track.album),
                685.0,
                726.0,
                540.0,
                32.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
            let duration = track.duration.max(0) as f32;
            let position = status.position.clamp(0.0, duration);
            rr(canvas, 685.0, 785.0, 540.0, 14.0, 7.0, control(canvas));
            if duration > 0.0 {
                rr(
                    canvas,
                    685.0,
                    785.0,
                    540.0 * position / duration,
                    14.0,
                    7.0,
                    accent(canvas),
                );
            }
            canvas.text(
                &format_time(position),
                685.0,
                807.0,
                29.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
            canvas.text_right(
                &format!("-{}", format_time(duration - position)),
                1225.0,
                807.0,
                29.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
        } else {
            heading(canvas, "Now Playing", "No song selected");
        }
    } else {
        heading(
            canvas,
            "Albums",
            &format!("{} albums · recently added", counts.albums),
        );
        let icon_path = format!("{}/icons/desktop/album.svg", assets_dir());
        for (index, album) in previews.iter().enumerate() {
            let x = 688.0 + (index % 3) as f32 * 183.0;
            let y = 230.0 + (index / 3) as f32 * 181.0;
            if let Some(cover) = &album.cover {
                draw_cover_art(canvas, Some(cover), x, y, 164.0, 28.0, 1.0);
            } else {
                rr(canvas, x, y, 164.0, 164.0, 28.0, control(canvas));
                canvas.draw_svg_tinted(&icon_path, x + 50.0, y + 32.0, 64.0, 64.0, accent(canvas));
                text_fit(
                    canvas,
                    &album.title,
                    x + 12.0,
                    y + 112.0,
                    140.0,
                    25.0,
                    ink(canvas),
                    FontWeight::Bold,
                );
            }
        }
        if previews.is_empty() {
            text_fit(
                canvas,
                "Import songs from Settings to get started.",
                688.0,
                270.0,
                540.0,
                31.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
        }
        rr(canvas, 688.0, 614.0, 254.0, 76.0, 38.0, accent(canvas));
        canvas.text_centered(
            "Open Music",
            815.0,
            633.0,
            33.0,
            on_accent(canvas),
            FontWeight::Bold,
        );
    }
}

fn heading(canvas: &mut Canvas<'_>, title: &str, subtitle: &str) {
    canvas.text(title, 688.0, 104.0, 50.0, ink(canvas), FontWeight::Bold);
    text_fit(
        canvas,
        subtitle,
        688.0,
        166.0,
        532.0,
        32.0,
        ink_secondary(canvas),
        FontWeight::Regular,
    );
}

fn draw_detail(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    index: usize,
    playlists: &[Playlist],
    tracks: &[Track],
    has_track: bool,
) {
    let (title, subtitle) = match index {
        0 => ("Music", "Playlists and tracks"),
        1 => ("Now Playing", "Quick controls"),
        2 => ("Apps", "Shortcuts and utilities"),
        _ => ("Settings", "Device preferences"),
    };
    heading(canvas, title, subtitle);
    let count = match index {
        0 => tracks.len().max(playlists.len()).min(4),
        1 if !has_track && tracks.is_empty() => 0,
        _ => 4,
    };
    for (row, detail_focus) in [
        Focus::HomePlaylist1,
        Focus::HomePlaylist2,
        Focus::HomePlaylist3,
        Focus::HomePlaylist4,
    ]
    .into_iter()
    .enumerate()
    .take(count)
    {
        let label = match index {
            0 => playlists
                .get(row)
                .map(|p| p.name.as_str())
                .or_else(|| tracks.get(row).map(|t| t.title.as_str()))
                .unwrap_or("No item"),
            1 => ["Play / pause", "Previous track", "Next track", "Open queue"][row],
            2 => ["Music", "Now Playing", "USB Sync", "Utilities"][row],
            _ => ["General", "Audio", "Appearance", "Updates"][row],
        };
        let y = 236.0 + row as f32 * 116.0;
        let selected = focus == detail_focus;
        figma_list_row(canvas, 688.0, y, 530.0, 88.0, selected, 0.0);
        text_fit(
            canvas,
            label,
            716.0,
            y + 20.0,
            465.0,
            39.0,
            ink(canvas),
            FontWeight::Bold,
        );
    }
}

fn format_time(position: f32) -> String {
    let seconds = position as u32;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
