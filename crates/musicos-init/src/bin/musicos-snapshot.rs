use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Result;
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

fn seed_library(app: &mut MusicOs) -> Result<()> {
    const FEATURED: [(&str, &str, &str, &[&str]); 6] = [
        (
            "Tame Impala",
            "The Slow Rush",
            "SlowRush.png",
            &[
                "One More Year",
                "Instant Destiny",
                "Borderline",
                "Posthumous Forgiveness",
                "Breathe Deeper",
            ],
        ),
        (
            "Tame Impala",
            "Currents",
            "Currents.png",
            &["Let It Happen", "Nangs", "The Moment"],
        ),
        (
            "Gorillaz",
            "Demon Days",
            "DemonDays.png",
            &[
                "Intro",
                "Last Living Souls",
                "Kids with Guns",
                "Feel Good Inc.",
            ],
        ),
        (
            "Imagine Dragons",
            "Night Visions",
            "NightVisions.png",
            &["Radioactive", "Tiptoe", "It's Time"],
        ),
        ("Luna", "Flamingo", "Flamingo.png", &["Pink", "Blue Hour"]),
        (
            "M83",
            "Hurry Up, We're Dreaming",
            "Sea.png",
            &["Intro", "Midnight City", "Reunion"],
        ),
    ];
    const ARTISTS: [&str; 33] = [
        "Aerial Rooms",
        "Amber Arcade",
        "Atlas Bloom",
        "Blue Hours",
        "Caldera",
        "Cedar Sound",
        "Common Weather",
        "Dawn Assembly",
        "Delta Coast",
        "Ember Field",
        "Far Meridian",
        "Field Notes",
        "Glass Atlas",
        "Harborlight",
        "Inland Current",
        "Juniper",
        "Kite Lines",
        "Low Sun",
        "Mallow",
        "Mirror District",
        "North Transit",
        "Orchard Radio",
        "Paper Kites",
        "Quiet Avenue",
        "River Index",
        "Salt Circuit",
        "Soft Signal",
        "Still Summer",
        "The Night Ferry",
        "Thread Count",
        "Valley Echo",
        "Westward",
        "Wild Static",
    ];
    const TITLES: [&str; 16] = [
        "Low Water",
        "Mirror Lake",
        "Night Window",
        "Open Road",
        "Paper Lanterns",
        "Quiet Company",
        "Room Tone",
        "Salt Lines",
        "Still Frames",
        "Telephone Wires",
        "The Long Way",
        "Tidal Pool",
        "Under Glass",
        "Velvet Sky",
        "Waterline",
        "Winter Light",
    ];
    let assets = musicos_init::ui::assets_dir();
    let assets = Path::new(&assets);
    let cover_dir = if assets.is_dir() {
        assets.join("covers")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/covers")
    };
    let local = std::env::var_os("MUSICOS_SNAPSHOT_ART_DIR").map(std::path::PathBuf::from);
    let fallback = [
        "Apart.png",
        "Baltimore.png",
        "Beach.png",
        "Cave.png",
        "Clouds.png",
        "Crowd.png",
        "Hills.png",
        "Lake.png",
        "Overcast.png",
        "Pier.png",
        "Rocks.png",
        "Sand.png",
        "Towers.png",
        "Vacate.png",
        "sky.png",
        "skate.png",
    ];
    let mut ids = Vec::new();
    for index in 0..64 {
        let featured = FEATURED.get(index);
        let artist = featured.map_or_else(
            || ARTISTS[(index - FEATURED.len()) % ARTISTS.len()],
            |entry| entry.0,
        );
        let album = featured.map_or_else(
            || {
                let adjective = [
                    "Northern", "Open", "Quiet", "Silver", "Still", "Summer", "Tidal", "Winter",
                ];
                let noun = [
                    "Arc", "Coast", "Current", "Field", "Light", "Signal", "Sky", "Water",
                ];
                format!("{} {}", adjective[index % 8], noun[index / 8])
            },
            |entry| entry.1.to_owned(),
        );
        let file = featured.map_or(fallback[index % fallback.len()], |entry| entry.2);
        let filename = if featured.is_some() {
            file.to_owned()
        } else {
            format!("{index:02}.png")
        };
        let art = local
            .as_ref()
            .map(|root| root.join(filename))
            .filter(|path| path.is_file())
            .unwrap_or_else(|| cover_dir.join(file));
        let mut album_ids = Vec::new();
        for number in 0..(12 + usize::from(index < 44)) {
            let title = featured
                .and_then(|entry| entry.3.get(number))
                .map(|title| (*title).to_owned())
                .unwrap_or_else(|| {
                    let suffixes = [
                        "at Dawn",
                        "at Dusk",
                        "in Blue",
                        "in Bloom",
                        "in June",
                        "in Motion",
                        "in the Rain",
                        "on Film",
                        "on the Coast",
                        "Under Glass",
                        "Underwater",
                        "Without Signal",
                    ];
                    format!(
                        "{} {}",
                        TITLES[(number + index * 7) % TITLES.len()],
                        suffixes[(index + number * 5) % suffixes.len()]
                    )
                });
            let duration = if index == 0 && number == 0 {
                327
            } else {
                158 + (index * 43 + number * 31) % 191
            };
            let id = app.db.insert_track_with_number(
                &format!("/music/{index:02}/{:02}.ogg", number + 1),
                &title,
                artist,
                &album,
                number as i32 + 1,
                duration as i32,
                1,
                1,
            )?;
            app.db.set_track_cover_art(id, art.to_str(), 1)?;
            if (index + number) % 29 == 0 {
                app.db.toggle_favorite(id)?;
            }
            if (index + number) % 17 == 3 {
                app.db.set_track_rating(id, 4)?;
            }
            album_ids.push(id);
        }
        ids.push(album_ids);
    }
    for (index, name) in [
        "After Hours",
        "Coastline",
        "Daybreak",
        "Deep Focus",
        "Headphones On",
        "Late Night Drive",
        "New Finds",
        "On Repeat",
        "Weekend Mix",
    ]
    .into_iter()
    .enumerate()
    {
        let id = app.db.create_playlist(name)?;
        let tracks = (0..18 + index)
            .map(|row| ids[(row * 7 + index + 1) % ids.len()][row % 12])
            .collect::<Vec<_>>();
        app.db.add_tracks_to_playlist(id, &tracks)?;
    }
    app.refresh_library();
    let tracks = app.db.get_tracks()?;
    if let Some(track) = tracks.iter().find(|track| track.id == ids[0][0]) {
        app.current_cover_art = track
            .cover_art_path
            .as_deref()
            .and_then(|path| musicos_init::media::load_cover_art_file(Path::new(path)));
        let mut queue = vec![track.clone()];
        queue.extend(app.db.get_playlist_tracks(app.playlists_list[0].id)?);
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
    fn snapshot_library_has_album_art_throughout_the_catalog() -> Result<()> {
        let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, ":memory:");
        seed_library(&mut app)?;
        let tracks = app.db.get_tracks()?;
        assert_eq!(tracks.len(), 812);
        assert_eq!(app.albums_list.len(), 64);
        assert_eq!(app.artists_list.len(), 38);
        assert_eq!(app.playlists_list.len(), 9);
        assert!(tracks.iter().all(|track| track.cover_art_path.is_some()));
        assert!(app.current_cover_art.is_some());
        Ok(())
    }
}
