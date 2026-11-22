use std::collections::BTreeMap;
use std::path::Path;

use crate::app::{Focus, HomePane};
use crate::audio::PlaybackStatus;
use crate::database::{LibraryCounts, Playlist, Track};
use crate::renderer::{Canvas, Color, CoverArt};
use crate::ui::nowplaying::draw_cover_art;
use crate::ui::{
    accent, assets_dir, control, figma_list_row, ink, ink_secondary, moving_selected_surface,
    on_accent, register_focus, rr, shadow_rr, surface, text_fit, FontWeight,
};

pub(crate) struct AlbumPreview {
    cover: Option<CoverArt>,
}

pub(crate) fn artist_preview(tracks: &[Track]) -> Option<CoverArt> {
    let root = Path::new(&assets_dir()).join("artists");
    let mut portraits = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("png"))
        .collect::<Vec<_>>();
    portraits.sort();
    portraits.into_iter().find_map(|path| {
        let name = path.file_stem()?.to_str()?;
        tracks
            .iter()
            .any(|track| track.artist.eq_ignore_ascii_case(name))
            .then(|| crate::media::load_cover_art_thumbnail(&path, 384))
            .flatten()
    })
}

pub(crate) fn album_previews(tracks: &[Track]) -> (Vec<AlbumPreview>, usize) {
    let recent_cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |age| age.as_secs().saturating_sub(7 * 24 * 60 * 60))
        as i64;
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
    let recent = albums
        .iter()
        .take_while(|track| track.indexed_at >= recent_cutoff)
        .count();
    let previews = albums
        .into_iter()
        .take(4)
        .map(|track| AlbumPreview {
            cover: track
                .cover_art_path
                .as_deref()
                .and_then(|path| crate::media::load_cover_art_thumbnail(Path::new(path), 384)),
        })
        .collect();
    (previews, recent)
}

pub(crate) fn album_mosaic(previews: &[AlbumPreview]) -> Option<CoverArt> {
    const CELL: usize = 144;
    const SIDE: usize = 2 * CELL;
    if previews.iter().all(|album| album.cover.is_none()) {
        return None;
    }
    let mut pixels = vec![Color::rgb(111, 92, 83); SIDE * SIDE];
    for (index, preview) in previews.iter().enumerate() {
        let Some(cover) = &preview.cover else {
            continue;
        };
        let left = index % 2 * CELL;
        let top = index / 2 * CELL;
        let size = cover.width.min(cover.height) as usize;
        let offset_x = (cover.width as usize - size) / 2;
        let offset_y = (cover.height as usize - size) / 2;
        for y in 0..CELL {
            let sy = offset_y + y * size / CELL;
            for x in 0..CELL {
                let sx = offset_x + x * size / CELL;
                pixels[(top + y) * SIDE + left + x] = cover.pixels[sy * cover.width as usize + sx];
            }
        }
    }
    Some(CoverArt {
        width: SIDE as u32,
        height: SIDE as u32,
        pixels,
    })
}

pub(crate) fn draw_home(
    canvas: &mut Canvas<'_>,
    focus: Focus,
    pane: HomePane,
    main_index: usize,
    counts: LibraryCounts,
    previews: &[AlbumPreview],
    recent_albums: usize,
    mosaic: Option<&CoverArt>,
    portrait: Option<&CoverArt>,
    playlists: &[Playlist],
    status: &PlaybackStatus,
    has_queue: bool,
    cover_art: Option<&CoverArt>,
) {
    crate::ui::rows_preview(canvas);
    let first = usize::from(main_index == 7);
    for (index, (label, icon, count)) in [
        ("Now Playing", "song.svg", None),
        ("Albums", "album.svg", Some(counts.albums)),
        ("Artists", "artist.svg", Some(counts.artists)),
        ("Songs", "song.svg", Some(counts.tracks)),
        ("Playlists", "playlist.svg", Some(counts.playlists)),
        ("Search", "search.svg", None),
        ("Settings", "settings.svg", None),
        ("Apps", "device.svg", None),
    ]
    .into_iter()
    .enumerate()
    .skip(first)
    .take(7)
    {
        let y = 64.0 + (index - first) as f32 * 112.0;
        let selected = pane == HomePane::Main && main_index == index;
        if selected {
            rr(
                canvas,
                20.0,
                y,
                608.0,
                108.0,
                54.0,
                moving_selected_surface(canvas, 20.0, y),
            );
            register_focus(canvas, 20.0, y, 608.0, 108.0, 54.0);
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
        rr(canvas, 40.0, y + 18.0, 72.0, 72.0, 36.0, fill);
        canvas.draw_svg_tinted(
            &format!("{}/icons/desktop/{icon}", assets_dir()),
            57.0,
            y + 35.0,
            38.0,
            38.0,
            glyph,
        );
        canvas.text(label, 136.0, y + 28.0, 43.0, ink(canvas), FontWeight::Bold);
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
        if index == 0 && status.track.is_some() {
            for (x, h) in [(565.0, 18.0), (578.0, 32.0), (591.0, 23.0)] {
                rr(canvas, x, y + 64.0 - h, 7.0, h, 3.5, accent(canvas));
            }
        }
    }

    if (pane == HomePane::Detail && matches!(main_index, 0 | 4)) || main_index >= 6 {
        draw_detail(
            canvas,
            focus,
            main_index,
            playlists,
            status.track.is_some() || has_queue,
        );
    } else if main_index == 0 {
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
    } else if matches!(main_index, 1 | 2) {
        let artist = main_index == 2;
        let (title, meta) = if artist {
            ("Artists", format!("{} artists", counts.artists))
        } else {
            (
                "Albums",
                format!("{} albums · {} new", counts.albums, recent_albums),
            )
        };
        if artist {
            rr(canvas, 684.0, 100.0, 540.0, 540.0, 42.0, control(canvas));
            album_bubble(
                canvas,
                portrait.or_else(|| previews.first().and_then(|p| p.cover.as_ref())),
                766.0,
                155.0,
                375.0,
            );
            for (index, (x, y, size)) in [
                (1068.0, 124.0, 132.0),
                (706.0, 447.0, 165.0),
                (1038.0, 475.0, 153.0),
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(album) = previews.get(index + usize::from(portrait.is_none())) {
                    album_bubble(canvas, album.cover.as_ref(), x, y, size);
                }
            }
        } else {
            shadow_rr(canvas, 684.0, 100.0, 540.0, 540.0, 42.0, 7.0, 12.0);
            draw_cover_art(canvas, mosaic, 684.0, 100.0, 540.0, 42.0, 1.0);
        }
        canvas.text(title, 684.0, 655.0, 50.0, ink(canvas), FontWeight::Bold);
        canvas.text(
            &meta,
            684.0,
            724.0,
            32.0,
            ink_secondary(canvas),
            FontWeight::Regular,
        );
        if counts.tracks > 0 {
            if pane == HomePane::Detail {
                register_focus(canvas, 1134.0, 662.0, 94.0, 94.0, 47.0);
            }
            rr(canvas, 1136.0, 664.0, 88.0, 88.0, 44.0, accent(canvas));
            canvas.draw_svg_tinted(
                &format!("{}/icons/desktop/shuffle.svg", assets_dir()),
                1158.0,
                686.0,
                44.0,
                44.0,
                on_accent(canvas),
            );
        }
    } else {
        let (title, subtitle, icon) = match main_index {
            3 => ("Songs", format!("{} songs", counts.tracks), "song"),
            4 => (
                "Playlists",
                format!("{} playlists", counts.playlists),
                "playlist",
            ),
            _ => (
                "Search",
                "Find a song, artist or album".to_string(),
                "search",
            ),
        };
        rr(canvas, 684.0, 100.0, 540.0, 540.0, 42.0, control(canvas));
        canvas.draw_svg_tinted(
            &format!("{}/icons/desktop/{icon}.svg", assets_dir()),
            866.0,
            282.0,
            176.0,
            176.0,
            accent(canvas),
        );
        canvas.text(title, 684.0, 655.0, 50.0, ink(canvas), FontWeight::Bold);
        canvas.text(
            &subtitle,
            684.0,
            724.0,
            32.0,
            ink_secondary(canvas),
            FontWeight::Regular,
        );
    }
}

fn album_bubble(canvas: &mut Canvas<'_>, cover: Option<&CoverArt>, x: f32, y: f32, size: f32) {
    rr(canvas, x, y, size, size, size / 2.0, surface(canvas));
    if let Some(cover) = cover {
        canvas.draw_image_rounded(
            cover,
            x + 7.0,
            y + 7.0,
            size - 14.0,
            size - 14.0,
            size / 2.0,
            1.0,
        );
    } else {
        canvas.draw_svg_tinted(
            &format!("{}/icons/desktop/artist.svg", assets_dir()),
            x + size * 0.25,
            y + size * 0.25,
            size * 0.5,
            size * 0.5,
            accent(canvas),
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
    has_track: bool,
) {
    let (title, subtitle) = match index {
        0 => ("Now Playing", "Quick controls"),
        4 => ("Playlists", "Quick play"),
        7 => ("Apps", "Shortcuts and utilities"),
        _ => ("Settings", "Device preferences"),
    };
    heading(canvas, title, subtitle);
    let count = match index {
        0 if !has_track => 0,
        4 => playlists.len().min(4),
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
            0 => ["Play / pause", "Previous track", "Next track", "Open queue"][row],
            4 => &playlists[row].name,
            7 => ["Music", "Now Playing", "USB Sync", "Utilities"][row],
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
