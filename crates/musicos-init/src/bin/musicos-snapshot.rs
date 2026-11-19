use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::database::Track;
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
    let mut app = if matches!(
        snapshot_screen.as_deref(),
        Ok("home-albums"
            | "home-artists"
            | "home-nowplaying"
            | "albums"
            | "album"
            | "artists"
            | "artist-albums"
            | "album-tracks"
            | "nowplaying-track"
            | "songs")
    ) {
        MusicOs::with_db_path(resolution, ":memory:")
    } else {
        MusicOs::new(resolution)
    };
    if let Ok(theme) = std::env::var("MUSICOS_SNAPSHOT_THEME") {
        app.active_theme = ThemeName::from_str(&theme);
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
            seed_home_library(&mut app)?;
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
            seed_home_library(&mut app)?;
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
            seed_demo_queue(&mut app);
            app.force_screen_for_snapshot(Screen::Queue);
        }
        Ok("playlists") => app.force_screen_for_snapshot(Screen::PlaylistsList),
        Ok("playlist-tracks") => {
            seed_demo_queue(&mut app);
            app.force_screen_for_snapshot(Screen::PlaylistTracksList);
        }
        Ok("search") => {
            seed_demo_queue(&mut app);
            app.search_query = "Tanger".into();
            app.search_results = app.tracks_queue.clone();
            app.force_screen_for_snapshot(Screen::Search);
        }
        Ok("search-results") | Ok("search-empty") => {
            seed_demo_queue(&mut app);
            app.search_query = "Tanger".into();
            if std::env::var("MUSICOS_SNAPSHOT_SCREEN").as_deref() == Ok("search-results") {
                app.search_results = app.tracks_queue.clone();
            }
            app.force_screen_for_snapshot(Screen::Search);
            app.focus = Focus::SearchResults;
        }
        Ok("rating") => {
            app.pending_rating = 3;
            app.force_screen_for_snapshot(Screen::RatingPicker);
        }
        Ok("context") => {
            seed_demo_albums(&mut app);
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
            seed_home_library(&mut app)?;
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

fn seed_home_library(app: &mut MusicOs) -> Result<()> {
    let assets = musicos_init::ui::assets_dir();
    for (title, artist, album, artwork) in [
        (
            "One More Year",
            "Tame Impala",
            "The Slow Rush",
            "SlowRush.png",
        ),
        ("Let It Happen", "Tame Impala", "Currents", "Currents.png"),
        ("Feel Good Inc.", "Gorillaz", "Demon Days", "DemonDays.png"),
        (
            "Radioactive",
            "Imagine Dragons",
            "Night Visions",
            "NightVisions.png",
        ),
        ("Pink", "Luna", "Flamingo", "Flamingo.png"),
        (
            "Midnight City",
            "M83",
            "Hurry Up, We're Dreaming",
            "Sea.png",
        ),
    ] {
        let id = app.db.insert_track_with_number(
            &format!("/music/{artwork}.ogg"),
            title,
            artist,
            album,
            1,
            327,
            1,
            1,
        )?;
        let path = Path::new(&assets).join("covers").join(artwork);
        app.db.set_track_cover_art(id, path.to_str(), 1)?;
    }
    for (number, title, duration) in [
        (2, "Instant Destiny", 193),
        (3, "Borderline", 238),
        (4, "Posthumous Forgiveness", 367),
        (5, "Breathe Deeper", 372),
    ] {
        let id = app.db.insert_track_with_number(
            &format!("/music/slow-rush-{number}.ogg"),
            title,
            "Tame Impala",
            "The Slow Rush",
            number,
            duration,
            1,
            1,
        )?;
        app.db.set_track_cover_art(
            id,
            Path::new(&assets).join("covers/SlowRush.png").to_str(),
            1,
        )?;
    }
    app.refresh_library();
    let tracks = app.db.get_tracks()?;
    if let Some((index, track)) = tracks
        .iter()
        .enumerate()
        .find(|(_, track)| track.title == "One More Year")
    {
        app.current_cover_art = track
            .cover_art_path
            .as_deref()
            .and_then(|path| musicos_init::media::load_cover_art_file(Path::new(path)));
        app.player.restore_queue_paused(tracks, index, 118.0);
    }
    Ok(())
}

fn seed_demo_albums(app: &mut MusicOs) {
    app.albums_list = vec![
        "mr henry and the stink store vol 2".to_string(),
        "bad album".to_string(),
        "goop eating".to_string(),
    ];
    app.selected_index = 0;
}

fn seed_demo_queue(app: &mut MusicOs) {
    app.tracks_queue = vec![
        Track {
            id: 1,
            path: "/music/alpha.ogg".to_string(),
            title: "Alpha".to_string(),
            artist: "Tanger".to_string(),
            album: "Glow".to_string(),
            track_number: 1,
            duration: 120,
            file_size: 0,
            modified_at: 0,
            indexed_at: 0,
            cover_art_path: None,
            cover_art_updated_at: None,
            cover_art_source_modified_at: None,
            favorited: false,
            play_count: 0,
            last_played: None,
            rating: 4,
        },
        Track {
            id: 2,
            path: "/music/beta.ogg".to_string(),
            title: "Beta".to_string(),
            artist: "Tanger".to_string(),
            album: "Glow".to_string(),
            track_number: 2,
            duration: 140,
            file_size: 0,
            modified_at: 0,
            indexed_at: 0,
            cover_art_path: None,
            cover_art_updated_at: None,
            cover_art_source_modified_at: None,
            favorited: true,
            play_count: 0,
            last_played: None,
            rating: 0,
        },
    ];
    app.selected_index = 1;
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
