use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{ensure, Result};
use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::input::{Button, InputEvent};
use musicos_init::renderer::{Canvas, Resolution};
use musicos_init::ui::theme::ThemeName;

fn main() -> Result<()> {
    let resolution = Resolution::from_env_or_default();
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out/musicos-snapshot.bmp".to_string());
    let mut bgra = vec![0_u8; resolution.width as usize * resolution.height as usize * 4];
    let snapshot_screen = std::env::var("MUSICOS_SNAPSHOT_SCREEN");
    let mut app = if snapshot_screen.is_ok() {
        MusicOs::with_db_path(resolution, ":memory:")
    } else {
        MusicOs::new(resolution)
    };
    if let Ok(theme) = std::env::var("MUSICOS_SNAPSHOT_THEME") {
        app.active_theme = ThemeName::from_str(&theme);
    }
    if snapshot_screen.as_deref() == Ok("motion") {
        return write_motion_frames(app, Path::new(&output), &mut bgra);
    }
    if matches!(
        snapshot_screen.as_deref(),
        Ok("home"
            | "home-albums"
            | "home-artists"
            | "home-nowplaying"
            | "music"
            | "albums"
            | "album"
            | "artists"
            | "artist-albums"
            | "album-tracks"
            | "songs"
            | "nowplaying-track"
            | "queue"
            | "playlists"
            | "playlist-tracks"
            | "search"
            | "search-results"
            | "search-empty"
            | "context"
            | "about"
            | "status"
            | "wifi-sync"
            | "volume-hud"
            | "volume_hud"
            | "brightness-hud"
            | "brightness_hud"
            | "rating")
    ) {
        seed_library(&mut app)?;
    }
    match snapshot_screen.as_deref() {
        Ok("first") => app.force_screen_for_snapshot(Screen::First),
        Ok("welcome") | Ok("hello") => app.force_screen_for_snapshot(Screen::Welcome),
        Ok("theme") => app.force_screen_for_snapshot(Screen::Theme),
        Ok("datetime") | Ok("date-time") | Ok("time") => {
            app.force_screen_for_snapshot(Screen::DateTimeSetup)
        }
        Ok("home") => app.force_screen_for_snapshot(Screen::Home),
        Ok("home-albums" | "home-nowplaying" | "home-artists") => {
            app.force_screen_for_snapshot(Screen::Home);
            match snapshot_screen.as_deref() {
                Ok("home-nowplaying") => {
                    app.handle_input(InputEvent::Button(Button::Up));
                }
                Ok("home-artists") => {
                    app.handle_input(InputEvent::Button(Button::Down));
                }
                _ => {}
            }
        }
        Ok("music") => app.force_screen_for_snapshot(Screen::MusicApp),
        Ok("songs" | "albums" | "album" | "artists" | "artist-albums" | "album-tracks") => {
            match snapshot_screen.as_deref() {
                Ok("songs") => {
                    app.tracks_queue = app.db.get_tracks()?;
                    app.force_screen_for_snapshot(Screen::SongsList);
                }
                Ok("artists" | "artist-albums") => {
                    app.force_screen_for_snapshot(Screen::ArtistsList);
                    if snapshot_screen.as_deref() == Ok("artist-albums") {
                        app.selected_index = app
                            .artists_list
                            .iter()
                            .position(|name| name == "Tame Impala")
                            .unwrap_or(0);
                        app.handle_input(InputEvent::Button(Button::Confirm));
                    }
                }
                _ => {
                    app.force_screen_for_snapshot(Screen::AlbumsList);
                    if snapshot_screen.as_deref() == Ok("album-tracks") {
                        app.selected_index = app
                            .albums_list
                            .iter()
                            .position(|title| title == "The Slow Rush")
                            .unwrap_or(0);
                        app.handle_input(InputEvent::Button(Button::Confirm));
                    }
                }
            }
            app.screen_transition = 1.0;
        }
        Ok("queue") => {
            app.tracks_queue = app.player.queue_tracks();
            app.force_screen_for_snapshot(Screen::Queue);
        }
        Ok("playlists") => app.force_screen_for_snapshot(Screen::PlaylistsList),
        Ok("playlist-tracks") => {
            app.force_screen_for_snapshot(Screen::PlaylistsList);
            app.handle_input(InputEvent::Button(Button::Confirm));
        }
        Ok("search" | "search-results" | "search-empty") => {
            app.search_query = if snapshot_screen.as_deref() == Ok("search-empty") {
                "Unmatched"
            } else {
                "Tame"
            }
            .into();
            app.search_results = app.db.search_tracks(&app.search_query, 100)?;
            app.force_screen_for_snapshot(Screen::Search);
            if snapshot_screen.as_deref() != Ok("search") {
                app.focus = Focus::SearchResults;
            }
        }
        Ok("rating") => {
            app.pending_rating = 3;
            app.force_screen_for_snapshot(Screen::RatingPicker);
        }
        Ok("context") => {
            app.force_screen_for_snapshot(Screen::ContextMenu);
            app.previous_screen = Screen::AlbumsList;
            app.previous_focus = Focus::Albums;
            app.focus = Focus::ContextPlay;
        }
        Ok("language") => app.force_screen_for_snapshot(Screen::Language),
        Ok("test") => app.force_screen_for_snapshot(Screen::TestComplete),
        Ok("settings") => app.force_screen_for_snapshot(Screen::Settings),
        Ok("settings-detail") | Ok("settings_detail") | Ok("general") => {
            app.force_screen_for_snapshot(Screen::SettingsDetail)
        }
        Ok("apps") => app.force_screen_for_snapshot(Screen::Apps),
        Ok("about") | Ok("status") => app.force_screen_for_snapshot(Screen::About),
        Ok("now") | Ok("nowplaying") => app.force_screen_for_snapshot(Screen::NowPlaying),
        Ok("nowplaying-track") => {
            app.motion_mode = musicos_init::preferences::MotionMode::Reduced;
            app.force_screen_for_snapshot(Screen::NowPlaying);
        }
        Ok("wifi-sync") => {
            app.force_screen_for_snapshot(Screen::Home);
            app.wifi_sync_overlay = true;
            app.wifi_sync_ip = "192.0.2.1".into();
        }
        Ok("volume-hud") | Ok("volume_hud") => {
            app.force_screen_for_snapshot(Screen::Home);
            app.hud_type = musicos_init::app::HudType::Volume;
            app.hud_t = 1.0;
            app.hud_timer = 2.0;
        }
        Ok("brightness-hud") | Ok("brightness_hud") => {
            app.force_screen_for_snapshot(Screen::Home);
            app.hud_type = musicos_init::app::HudType::Brightness;
            app.hud_t = 1.0;
            app.hud_timer = 2.0;
        }
        _ => {}
    }
    if let Ok(value) = std::env::var("MUSICOS_SNAPSHOT_TRANSITION") {
        if let Ok(progress) = value.parse::<f32>() {
            app.screen_transition = progress.clamp(0.0, 1.0);
        }
    }
    let mut canvas = Canvas::new(resolution, &mut bgra)?;
    let seconds = std::env::var("MUSICOS_SNAPSHOT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.75);
    app.draw(&mut canvas, seconds);
    write_bmp(&output, resolution, &bgra)?;
    println!("{output}");
    Ok(())
}

fn write_motion_frames(mut app: MusicOs, output: &Path, bgra: &mut [u8]) -> Result<()> {
    app.motion_mode = musicos_init::preferences::MotionMode::Balanced;
    seed_library(&mut app)?;
    app.force_screen_for_snapshot(Screen::Home);
    std::fs::create_dir_all(output)?;
    let mut render_times = Vec::new();
    for frame in 0..168 {
        let action = match frame {
            15 => Some((Button::Up, Screen::Home)),
            27 => Some((Button::Down, Screen::Home)),
            42 => Some((Button::Confirm, Screen::AlbumsList)),
            60 => Some((Button::Right, Screen::AlbumsList)),
            68 => Some((Button::Up, Screen::AlbumsList)),
            78 => Some((Button::Confirm, Screen::AlbumTracksList)),
            108 => Some((Button::Confirm, Screen::NowPlaying)),
            144 => Some((Button::Back, Screen::AlbumTracksList)),
            _ => None,
        };
        if let Some((button, screen)) = action {
            app.handle_input(InputEvent::Button(button));
            ensure!(app.screen == screen, "unexpected screen at frame {frame}");
            if screen == Screen::NowPlaying {
                app.tick();
                let art = app.player.get_current_track().and_then(|track| {
                    track
                        .cover_art_path
                        .as_deref()
                        .and_then(|path| musicos_init::media::load_cover_art_file(Path::new(path)))
                });
                let (tx, rx) = std::sync::mpsc::channel();
                tx.send(art)?;
                app.cover_art_rx = Some(rx);
                app.tick();
            }
        }
        if app.screen == Screen::NowPlaying {
            app.player.seek(32.0 + (frame - 108) as f32 / 30.0);
        }
        let start = std::time::Instant::now();
        app.draw(&mut Canvas::new(app.resolution, bgra)?, frame as f32 / 30.0);
        render_times.push(start.elapsed().as_secs_f64() * 1000.0);
        let path = output.join(format!("{frame:04}.bmp"));
        write_bmp(&path.to_string_lossy(), app.resolution, bgra)?;
    }
    render_times.sort_by(f64::total_cmp);
    let p95 = render_times[(render_times.len() * 95).div_ceil(100) - 1];
    println!("168 frames at 30 fps, offscreen frame p95: {p95:.2} ms");
    Ok(())
}

#[derive(serde::Deserialize)]
struct SnapshotAlbum {
    key: String,
    artist: String,
    title: String,
    bundled_cover: Option<String>,
    tracks: Vec<(String, i32, i32, i32)>,
}

fn seed_library(app: &mut MusicOs) -> Result<()> {
    let albums: Vec<SnapshotAlbum> = serde_json::from_str(include_str!("snapshot-library.json"))?;
    let assets = musicos_init::ui::assets_dir();
    let covers = Path::new(&assets).join("covers");
    let covers = if covers.is_dir() {
        covers
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/covers")
    };
    let local = std::env::var_os("MUSICOS_SNAPSHOT_ART_DIR").map(std::path::PathBuf::from);
    let mut playback_ids = Vec::new();
    let mut ids = Vec::new();
    for album in &albums {
        let artwork = local
            .as_ref()
            .map(|root| root.join(format!("{}.jpg", album.key)))
            .filter(|path| path.is_file())
            .or_else(|| album.bundled_cover.as_ref().map(|file| covers.join(file)));
        let mut album_ids = Vec::new();
        for (title, duration, disc, number) in &album.tracks {
            let id = app.db.insert_track_with_number(
                &format!("/fixture/{}/{disc}-{number}.flac", album.key),
                title,
                &album.artist,
                &album.title,
                *number,
                *duration,
                1,
                1,
            )?;
            if let Some(path) = &artwork {
                app.db.set_track_cover_art(id, path.to_str(), 1)?;
            }
            if *number == 2 {
                app.db.toggle_favorite(id)?;
            }
            album_ids.push(id);
        }
        if album.key == "the-slow-rush" {
            playback_ids.clone_from(&album_ids);
        }
        ids.push(album_ids);
    }

    for (name, offset) in [("Evening", 0), ("On Repeat", 1), ("Weekend Mix", 2)] {
        let playlist = app.db.create_playlist(name)?;
        let tracks: Vec<_> = ids
            .iter()
            .filter_map(|album| album.get(offset).copied())
            .collect();
        app.db.add_tracks_to_playlist(playlist, &tracks)?;
    }
    app.refresh_library();

    let queue = playback_ids
        .into_iter()
        .map(|id| app.db.get_track_by_id(id))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if let Some(track) = queue.first() {
        app.current_cover_art = track
            .cover_art_path
            .as_deref()
            .and_then(|path| musicos_init::media::load_cover_art_file(Path::new(path)));
        app.current_track_id = Some(track.id);
        app.player.restore_queue_paused(queue, 0, 118.0);
    }
    Ok(())
}

fn write_bmp(path: &str, resolution: Resolution, bgra: &[u8]) -> Result<()> {
    let width = resolution.width as usize;
    let height = resolution.height as usize;
    let row_bytes = width * 4;
    let pixel_bytes = row_bytes * height;
    let file_size = 14 + 40 + pixel_bytes;
    let mut file = BufWriter::new(File::create(path)?);

    file.write_all(b"BM")?;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(&[0; 4])?;
    file.write_all(&(54_u32).to_le_bytes())?;

    file.write_all(&(40_u32).to_le_bytes())?;
    file.write_all(&(resolution.width as i32).to_le_bytes())?;
    file.write_all(&(-(resolution.height as i32)).to_le_bytes())?;
    file.write_all(&(1_u16).to_le_bytes())?;
    file.write_all(&(32_u16).to_le_bytes())?;
    file.write_all(&(0_u32).to_le_bytes())?;
    file.write_all(&(pixel_bytes as u32).to_le_bytes())?;
    file.write_all(&(2835_i32).to_le_bytes())?;
    file.write_all(&(2835_i32).to_le_bytes())?;
    file.write_all(&(0_u32).to_le_bytes())?;
    file.write_all(&(0_u32).to_le_bytes())?;

    file.write_all(bgra)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_library_uses_the_real_desktop_album_catalog() -> Result<()> {
        let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, ":memory:");
        seed_library(&mut app)?;
        let tracks = app.db.get_tracks()?;
        assert_eq!(tracks.len(), 160);
        assert_eq!(app.albums_list.len(), 11);
        assert_eq!(
            app.artists_list,
            [
                "Coldplay",
                "Daft Punk",
                "Frank Ocean",
                "Gorillaz",
                "M83",
                "Tame Impala"
            ]
        );
        assert_eq!(app.playlists_list.len(), 3);
        assert!(tracks.iter().any(|track| track.artist == "Daft Punk"
            && track.title == "One More Time"
            && track.cover_art_path.is_some()));
        if std::env::var_os("MUSICOS_SNAPSHOT_ART_DIR").is_some() {
            assert!(tracks.iter().all(|track| track.cover_art_path.is_some()));
        }
        assert!(app.current_cover_art.is_some());
        Ok(())
    }
}
