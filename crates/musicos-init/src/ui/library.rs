use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use crate::database::Track;
use crate::renderer::{Canvas, CoverArt};
use crate::ui::{
    accent, assets_dir, control, draw_art_wash, draw_figma_background, ink, ink_secondary,
    moving_selected_surface, register_focus, rr, surface, text_fit, FontWeight,
};

pub(crate) const COLUMNS: usize = 3;

#[derive(Default)]
struct Album {
    artist: String,
    cover: Option<String>,
}

#[derive(Default)]
struct Artist {
    albums: BTreeSet<String>,
    songs: usize,
    cover: Option<String>,
}

type ArtCache = RefCell<VecDeque<(String, Option<CoverArt>)>>;

#[derive(Default)]
pub(crate) struct LibraryView {
    albums: BTreeMap<String, Album>,
    artist_albums: BTreeMap<String, BTreeMap<String, Album>>,
    artists: BTreeMap<String, Artist>,
    art: ArtCache,
}

impl LibraryView {
    pub(crate) fn update(&mut self, tracks: &[Track]) {
        self.albums.clear();
        self.artist_albums.clear();
        self.artists.clear();
        self.art.get_mut().clear();
        for track in tracks {
            let artist = self.artists.entry(track.artist.clone()).or_default();
            artist.songs += 1;
            if !track.album.is_empty() {
                artist.albums.insert(track.album.clone());
            }
            if artist.cover.is_none() {
                artist.cover.clone_from(&track.cover_art_path);
            }
            for album in [
                self.albums.entry(track.album.clone()).or_default(),
                self.artist_albums
                    .entry(track.artist.clone())
                    .or_default()
                    .entry(track.album.clone())
                    .or_default(),
            ] {
                if album.artist.is_empty()
                    || album.cover.is_none() && track.cover_art_path.is_some()
                {
                    album.artist.clone_from(&track.artist);
                    album.cover.clone_from(&track.cover_art_path);
                }
            }
        }
        for (name, portrait) in super::home::artist_portraits(tracks) {
            if let Some(artist) = self.artists.get_mut(&name) {
                artist.cover = Some(portrait);
            }
        }
    }

    fn with_art(&self, path: &str, draw: impl FnOnce(&CoverArt)) -> bool {
        let mut cache = self.art.borrow_mut();
        let entry = if let Some(index) = cache.iter().position(|(key, _)| key == path) {
            cache.remove(index).unwrap()
        } else {
            (
                path.to_owned(),
                crate::media::load_cover_art_thumbnail(Path::new(path), 384),
            )
        };
        let found = if let Some(cover) = &entry.1 {
            draw(cover);
            true
        } else {
            false
        };
        cache.push_front(entry);
        cache.truncate(16);
        found
    }

    pub(crate) fn artwork(
        &self,
        canvas: &mut Canvas<'_>,
        path: Option<&str>,
        x: f32,
        y: f32,
        size: f32,
        portrait: bool,
    ) {
        let radius = if portrait {
            size / 2.0
        } else {
            (size / 6.0).min(32.0)
        };
        if path.is_some_and(|path| {
            self.with_art(path, |cover| {
                canvas.draw_image_rounded(cover, x, y, size, size, radius, 1.0);
            })
        }) {
            return;
        }
        rr(canvas, x, y, size, size, radius, control(canvas));
        let icon = if portrait { "artist" } else { "album" };
        canvas.draw_svg_tinted(
            &format!("{}/icons/desktop/{icon}.svg", assets_dir()),
            x + size * 0.26,
            y + size * 0.26,
            size * 0.48,
            size * 0.48,
            accent(canvas),
        );
    }

    pub(crate) fn grid(
        &self,
        canvas: &mut Canvas<'_>,
        title: &str,
        items: &[String],
        selected: usize,
        mode: Grid,
    ) {
        draw_figma_background(canvas);
        let (start_y, row_height, art_size, rows) = match mode {
            Grid::ArtistAlbums => {
                let artist = self.artists.get(title);
                let cover = artist.and_then(|artist| artist.cover.as_deref());
                let subtitle = artist.map(|artist| counts(artist.albums.len(), artist.songs));
                self.hero(
                    canvas,
                    title,
                    subtitle.as_deref().unwrap_or("Artist"),
                    cover,
                    true,
                );
                canvas.text("Albums", 52.0, 325.0, 37.0, ink(canvas), FontWeight::Bold);
                (382.0, 486.0, 346.0, 1)
            }
            Grid::Albums | Grid::Artists => {
                let kind = if matches!(mode, Grid::Artists) {
                    "artist"
                } else {
                    "album"
                };
                heading(canvas, title, &plural(items.len(), kind));
                (
                    218.0,
                    326.0,
                    if matches!(mode, Grid::Artists) {
                        222.0
                    } else {
                        232.0
                    },
                    2,
                )
            }
        };
        if items.is_empty() {
            empty(
                canvas,
                "Nothing here yet",
                "Add music, then run a library scan.",
            );
            return;
        }
        let page = COLUMNS * rows;
        let start = selected.min(items.len() - 1) / page * page;
        for (cell, item) in items.iter().skip(start).take(page).enumerate() {
            let x = 43.0 + cell.rem_euclid(COLUMNS) as f32 * 405.0;
            let y = start_y + (cell / COLUMNS) as f32 * row_height;
            let width = 384.0;
            if cell + start == selected {
                rr(
                    canvas,
                    x,
                    y,
                    width,
                    row_height - 13.0,
                    20.0,
                    moving_selected_surface(canvas, x, y),
                );
                register_focus(canvas, x, y, width, row_height - 13.0, 20.0);
            }
            let (subtitle, cover) = match mode {
                Grid::Artists => self
                    .artists
                    .get(item)
                    .map_or((String::new(), None), |artist| {
                        (
                            counts(artist.albums.len(), artist.songs),
                            artist.cover.as_deref(),
                        )
                    }),
                Grid::Albums | Grid::ArtistAlbums => {
                    let album = match mode {
                        Grid::ArtistAlbums => self
                            .artist_albums
                            .get(title)
                            .and_then(|albums| albums.get(item)),
                        _ => self.albums.get(item),
                    };
                    album.map_or((String::new(), None), |album| {
                        (album.artist.clone(), album.cover.as_deref())
                    })
                }
            };
            self.artwork(
                canvas,
                cover,
                if matches!(mode, Grid::Artists) {
                    x + (width - art_size) / 2.0
                } else {
                    x + 20.0
                },
                y + 11.0,
                art_size,
                matches!(mode, Grid::Artists),
            );
            text_fit(
                canvas,
                item,
                x + 20.0,
                y + art_size + 14.0,
                width - 40.0,
                34.0,
                ink(canvas),
                FontWeight::Bold,
            );
            text_fit(
                canvas,
                &subtitle,
                x + 20.0,
                y + art_size + 49.0,
                width - 40.0,
                25.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
        }
    }

    pub(crate) fn tracks(
        &self,
        canvas: &mut Canvas<'_>,
        title: &str,
        tracks: &[Track],
        selected: usize,
        album: bool,
    ) {
        draw_figma_background(canvas);
        let start_y = if album {
            let artist = tracks.first().map_or("", |track| track.artist.as_str());
            let duration: u64 = tracks
                .iter()
                .map(|track| track.duration.max(0) as u64)
                .sum();
            let meta = format!(
                "{artist} · {} · {}",
                plural(tracks.len(), "song"),
                time(duration)
            );
            let art = tracks
                .iter()
                .find_map(|track| track.cover_art_path.as_deref());
            self.hero(canvas, title, &meta, art, false);
            332.0
        } else {
            heading(canvas, title, &plural(tracks.len(), "song"));
            222.0
        };
        if tracks.is_empty() {
            empty(
                canvas,
                "No songs yet",
                "Sync from the desktop or add music to the library.",
            );
            return;
        }
        let (title_x, album_x, time_x) = (130.0, 812.0, 1187.0);
        for (label, x) in [("#", 68.0), ("Title / Artist", title_x), ("Album", album_x)] {
            if label != "Album" || !album {
                canvas.text(
                    label,
                    x,
                    start_y,
                    29.0,
                    ink_secondary(canvas),
                    FontWeight::Bold,
                );
            }
        }
        canvas.text_right(
            "Time",
            time_x,
            start_y,
            29.0,
            ink_secondary(canvas),
            FontWeight::Bold,
        );
        let visible = if album { 6 } else { 7 };
        let start = selected.min(tracks.len() - 1) / visible * visible;
        for (index, track) in tracks.iter().enumerate().skip(start).take(visible) {
            let y = start_y + 43.0 + (index - start) as f32 * 83.0;
            crate::ui::compact_row(canvas, 43.0, y, 1194.0, 76.0, selected == index);
            let number = if album && track.track_number > 0 {
                track.track_number as usize
            } else {
                index + 1
            };
            canvas.text(
                &number.to_string(),
                68.0,
                y + 28.0,
                30.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
            let cover_width = if album { 0.0 } else { 82.0 };
            if !album {
                self.artwork(
                    canvas,
                    track.cover_art_path.as_deref(),
                    title_x,
                    y + 10.0,
                    56.0,
                    false,
                );
                text_fit(
                    canvas,
                    &track.album,
                    album_x,
                    y + 27.0,
                    274.0,
                    33.0,
                    ink_secondary(canvas),
                    FontWeight::Regular,
                );
            }
            let title_width = if album {
                910.0
            } else {
                album_x - title_x - cover_width - 28.0
            };
            text_fit(
                canvas,
                &track.title,
                title_x + cover_width,
                y + 10.0,
                title_width,
                37.0,
                ink(canvas),
                FontWeight::Bold,
            );
            let artist = if track.rating > 0 {
                format!("{} · {}/5", track.artist, track.rating)
            } else {
                track.artist.clone()
            };
            text_fit(
                canvas,
                &artist,
                title_x + cover_width,
                y + 45.0,
                title_width,
                27.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
            canvas.text_right(
                &time(track.duration.max(0) as u64),
                time_x,
                y + 26.0,
                32.0,
                ink_secondary(canvas),
                FontWeight::Regular,
            );
        }
    }

    fn hero(
        &self,
        canvas: &mut Canvas<'_>,
        title: &str,
        subtitle: &str,
        cover: Option<&str>,
        portrait: bool,
    ) {
        if let Some(path) = cover {
            self.with_art(path, |art| draw_art_wash(canvas, art, 340.0));
        }
        self.artwork(canvas, cover, 68.0, 98.0, 176.0, portrait);
        text_fit(
            canvas,
            title,
            278.0,
            112.0,
            910.0,
            66.0,
            ink(canvas),
            FontWeight::Bold,
        );
        text_fit(
            canvas,
            subtitle,
            282.0,
            209.0,
            902.0,
            36.0,
            ink_secondary(canvas),
            FontWeight::Regular,
        );
    }
}

pub(crate) enum Grid {
    Albums,
    Artists,
    ArtistAlbums,
}

fn heading(canvas: &mut Canvas<'_>, title: &str, detail: &str) {
    text_fit(
        canvas,
        title,
        50.0,
        71.0,
        1170.0,
        75.0,
        ink(canvas),
        FontWeight::Bold,
    );
    canvas.text(
        detail,
        54.0,
        167.0,
        32.0,
        ink_secondary(canvas),
        FontWeight::Regular,
    );
}

fn empty(canvas: &mut Canvas<'_>, title: &str, description: &str) {
    rr(canvas, 66.0, 380.0, 1148.0, 198.0, 45.0, surface(canvas));
    canvas.text_centered(title, 640.0, 408.0, 57.0, ink(canvas), FontWeight::Bold);
    canvas.text_centered(
        description,
        640.0,
        495.0,
        31.0,
        ink_secondary(canvas),
        FontWeight::Regular,
    );
}

fn counts(albums: usize, tracks: usize) -> String {
    format!("{} · {}", plural(albums, "album"), plural(tracks, "song"))
}

fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
