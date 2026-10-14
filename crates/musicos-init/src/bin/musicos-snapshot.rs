use std::fs::File;
use std::io::{BufWriter, Write};

use anyhow::Result;
use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::database::Track;
use musicos_init::renderer::{Canvas, Resolution};

fn main() -> Result<()> {
    let resolution = Resolution::from_env_or_default();
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out/musicos-snapshot.bmp".to_string());
    let mut bgra = vec![0_u8; resolution.width as usize * resolution.height as usize * 4];
    let mut app = MusicOs::new(resolution);
    match std::env::var("MUSICOS_SNAPSHOT_SCREEN").as_deref() {
        Ok("welcome") | Ok("hello") => app.force_screen_for_snapshot(Screen::Welcome),
        Ok("theme") => app.force_screen_for_snapshot(Screen::Theme),
        Ok("datetime") | Ok("date-time") | Ok("time") => {
            app.force_screen_for_snapshot(Screen::DateTimeSetup)
        }
        Ok("home") => app.force_screen_for_snapshot(Screen::Home),
        Ok("music") => app.force_screen_for_snapshot(Screen::MusicApp),
        Ok("albums") | Ok("album") => {
            seed_demo_albums(&mut app);
            app.force_screen_for_snapshot(Screen::AlbumsList);
        }
        Ok("queue") => {
            seed_demo_queue(&mut app);
            app.force_screen_for_snapshot(Screen::Queue);
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
