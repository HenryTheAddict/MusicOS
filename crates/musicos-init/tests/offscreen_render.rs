use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::input::{Button, InputEvent};
use musicos_init::preferences::MotionMode;
use musicos_init::renderer::{Canvas, Color, CoverArt, Resolution};
use musicos_init::ui::{legend_bar, status_bar, theme::ThemeName};

fn render_frame(app: &mut MusicOs, seconds: f32) -> Vec<u8> {
    let mut buffer = vec![0; app.resolution.width as usize * app.resolution.height as usize * 4];
    app.draw(
        &mut Canvas::new(app.resolution, &mut buffer).unwrap(),
        seconds,
    );
    buffer
}

#[test]
fn stepped_navigation_moves_focus_and_settles_both_slide_directions() {
    let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, ":memory:");
    app.motion_mode = MotionMode::Balanced;
    app.force_screen_for_snapshot(Screen::Home);
    render_frame(&mut app, 0.0);
    let old_y = app.outline_y;
    app.handle_input(InputEvent::Button(Button::Down));
    render_frame(&mut app, 0.016);
    assert!(app.outline_y > old_y && app.outline_y < old_y + 112.0);
    app.handle_input(InputEvent::Button(Button::Up));
    for frame in 2..32 {
        render_frame(&mut app, frame as f32 / 60.0);
    }
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::AlbumsList);
    let push = render_frame(&mut app, 0.55);
    let mut settled = Vec::new();
    for frame in 34..55 {
        settled = render_frame(&mut app, frame as f32 / 60.0);
    }
    let body = 32 * 640 * 4..439 * 640 * 4;
    let click = 460 * 640 * 4..478 * 640 * 4;
    assert_ne!(push[body.clone()], settled[body.clone()]);
    assert_ne!(push[click.clone()], settled[click]);

    app.handle_input(InputEvent::Button(Button::Back));
    assert_eq!(app.screen, Screen::Home);
    let pop = render_frame(&mut app, 0.95);
    for frame in 58..79 {
        settled = render_frame(&mut app, frame as f32 / 60.0);
    }
    let pixel = |x: usize, y: usize| (y * 640 + x) * 4;
    let right_edge = pixel(620, 100);
    assert_ne!(
        pop[right_edge..right_edge + 3],
        settled[right_edge..right_edge + 3]
    );

    app.motion_mode = MotionMode::Reduced;
    app.handle_input(InputEvent::Button(Button::Confirm));
    let immediate = render_frame(&mut app, 1.4);
    let later = render_frame(&mut app, 1.6);
    assert_eq!(immediate[body.clone()], later[body]);
    assert_eq!(app.screen_transition, 1.0);
}

#[test]
fn stepped_list_edge_feedback_moves_pixels_without_changing_selection() {
    let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, ":memory:");
    app.db
        .insert_track("test.ogg", "Song", "Artist", "Album", 120, 1, 1)
        .unwrap();
    app.tracks_queue = app.db.get_tracks().unwrap();
    app.force_screen_for_snapshot(Screen::AlbumTracksList);
    app.motion_mode = MotionMode::Balanced;
    let resting = render_frame(&mut app, 0.0);
    assert!(app.handle_input(InputEvent::Button(Button::Up)));
    render_frame(&mut app, 0.03);
    let bounce = render_frame(&mut app, 0.06);
    let body = 32 * 640 * 4..439 * 640 * 4;
    assert_ne!(bounce[body.clone()], resting[body.clone()]);
    assert_eq!(app.selected_index, 0);
    app.motion_mode = MotionMode::Reduced;
    let reduced = render_frame(&mut app, 0.09);
    assert!(!app.handle_input(InputEvent::Button(Button::Up)));
    let after = render_frame(&mut app, 0.12);
    assert_eq!(after[body.clone()], reduced[body]);
}

#[test]
fn stepped_track_change_blends_the_old_and_new_covers() {
    let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, ":memory:");
    app.motion_mode = MotionMode::Balanced;
    app.force_screen_for_snapshot(Screen::NowPlaying);
    for title in ["First track", "Next track"] {
        app.db
            .insert_track(title, title, "Artist", "Album", 120, 1, 1)
            .unwrap();
    }
    let tracks = app.db.get_tracks().unwrap();
    let cover = |color| CoverArt {
        width: 1,
        height: 1,
        pixels: vec![color],
    };
    app.player.restore_queue_paused(tracks.clone(), 0, 0.0);
    app.current_track_id = Some(tracks[0].id);
    app.current_cover_art = Some(cover(Color::rgb(255, 0, 0)));
    app.cover_art_opacity = 1.0;
    let pixel = (120 * 640 + 100) * 4;
    let first = render_frame(&mut app, 0.0);
    assert_eq!(&first[pixel..pixel + 3], &[0, 0, 255]);

    app.player.restore_queue_paused(tracks, 1, 0.0);
    app.tick();
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Some(cover(Color::rgb(0, 0, 255)))).unwrap();
    app.cover_art_rx = Some(rx);
    app.tick();
    let blended = render_frame(&mut app, 0.08);
    assert!((1..255).contains(&blended[pixel]));
    assert!((1..255).contains(&blended[pixel + 2]));
    let mut settled = Vec::new();
    for frame in 2..7 {
        settled = render_frame(&mut app, frame as f32 * 0.08);
    }
    assert_eq!(&settled[pixel..pixel + 3], &[255, 0, 0]);
}

#[test]
fn artwork_rounding_blends_the_corner_without_fading_the_center() {
    let resolution = Resolution::VGA_4_3;
    let width = resolution.width as usize;
    let mut buffer = vec![0; width * resolution.height as usize * 4];
    let mut canvas = Canvas::new(resolution, &mut buffer).unwrap();
    let image = CoverArt {
        width: 1,
        height: 1,
        pixels: vec![Color::WHITE],
    };
    canvas.clear(Color::BLACK);
    canvas.draw_image_rounded(&image, 0.0, 0.0, 40.0, 40.0, 20.0, 1.0);
    let pixel = |x: usize, y: usize| buffer[(y * width + x) * 4];
    assert_eq!(pixel(0, 0), 0);
    assert!((1..255).contains(&pixel(3, 2)));
    assert_eq!(pixel(10, 10), 255);
}

#[test]
fn scales_chrome_to_the_framebuffer_without_covering_content() {
    for resolution in [Resolution::HD_4_3, Resolution::VGA_4_3] {
        let width = resolution.width as usize;
        let mut buffer = vec![0; width * resolution.height as usize * 4];
        let mut canvas = Canvas::new(resolution, &mut buffer).unwrap();
        canvas.clear(Color::BLACK);
        status_bar(&mut canvas, "", "", "", ThemeName::Blue, false);
        legend_bar(&mut canvas, "");
        let pixel = |y: usize| &buffer[(y * width + 8) * 4..][..3];
        let header_bottom = resolution.scale_y(64.0) as usize;
        let footer_top = resolution.scale_y(918.0) as usize;
        assert_ne!(pixel(header_bottom - 2), [0; 3]);
        assert_eq!(pixel(header_bottom + 2), [0; 3]);
        assert_eq!(pixel(footer_top - 2), [0; 3]);
        assert_ne!(pixel(footer_top + 2), [0; 3]);
    }
}

#[test]
fn home_uses_warm_light_and_dark_shells_with_a_separate_preview_surface() {
    let resolution = Resolution::VGA_4_3;
    let mut app = MusicOs::with_db_path(resolution, ":memory:");
    app.force_screen_for_snapshot(Screen::Home);
    for (theme, shell, preview) in [
        (ThemeName::Seaglass, [216, 226, 240], [234, 240, 249]),
        (ThemeName::Dark, [21, 22, 26], [31, 33, 38]),
    ] {
        app.active_theme = theme;
        let size = resolution.width as usize * resolution.height as usize * 4;
        let mut buffer = vec![0; size];
        let mut canvas = Canvas::new(resolution, &mut buffer).unwrap();
        app.draw(&mut canvas, 0.75);
        let pixel = |x: usize, y: usize| &buffer[(y * resolution.width as usize + x) * 4..][..3];
        assert_eq!(pixel(12, 425), shell);
        assert_eq!(pixel(500, 400), preview);
    }
}

#[test]
fn non_home_screens_do_not_draw_a_fixed_preview_panel() {
    let resolution = Resolution::VGA_4_3;
    let mut app = MusicOs::with_db_path(resolution, ":memory:");
    for (theme, shell) in [
        (ThemeName::Seaglass, [216, 226, 240]),
        (ThemeName::Dark, [21, 22, 26]),
    ] {
        app.active_theme = theme;
        for screen in [
            Screen::MusicApp,
            Screen::First,
            Screen::Welcome,
            Screen::Theme,
            Screen::Language,
            Screen::DateTimeSetup,
            Screen::Settings,
            Screen::SettingsDetail,
            Screen::Apps,
            Screen::About,
            Screen::NowPlaying,
            Screen::PlaylistsList,
            Screen::PlaylistTracksList,
            Screen::Queue,
            Screen::Search,
            Screen::RatingPicker,
        ] {
            app.force_screen_for_snapshot(screen);
            let mut buffer = vec![0; resolution.width as usize * resolution.height as usize * 4];
            let mut canvas = Canvas::new(resolution, &mut buffer).unwrap();
            app.draw(&mut canvas, 0.75);
            let pixel = (410 * resolution.width as usize + 620) * 4;
            assert_eq!(buffer[pixel..pixel + 3], shell, "{screen:?} in {theme:?}");
        }
    }
}

#[test]
fn renders_setup_screen_at_both_supported_resolutions() {
    let root = tempfile::tempdir().unwrap();
    for resolution in [Resolution::HD_4_3, Resolution::VGA_4_3] {
        let mut buffer = vec![0; resolution.width as usize * resolution.height as usize * 4];
        let mut canvas = Canvas::new(resolution, &mut buffer).expect("canvas");
        let mut app =
            MusicOs::with_db_path(resolution, root.path().join("media.sqlite3").to_str().unwrap());
        app.draw(&mut canvas, 0.75);
        assert!(
            buffer.iter().any(|byte| *byte != 0),
            "rendered buffer should not be blank for {:?}",
            resolution
        );
    }
}

#[test]
fn renders_search_drilldown_and_rating_screens_at_both_resolutions() {
    for resolution in [Resolution::HD_4_3, Resolution::VGA_4_3] {
        let root = std::env::temp_dir().join(format!(
            "musicos-offscreen-features-{}-{}x{}",
            std::process::id(),
            resolution.width,
            resolution.height
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut app =
            MusicOs::with_db_path(resolution, root.join("media.sqlite3").to_str().unwrap());
        app.search_query = "A".to_string();
        app.artist_albums_list = vec!["Glow".to_string(), "Night".to_string()];
        app.active_artist = Some("Tanger".to_string());

        for (screen, focus) in [
            (Screen::Search, Focus::SearchKeyboard),
            (Screen::Search, Focus::SearchResults),
            (Screen::ArtistAlbumsList, Focus::Albums),
            (Screen::AlbumTracksList, Focus::Albums),
            (Screen::RatingPicker, Focus::RatingStars),
            (Screen::Settings, Focus::SettingsConnectivity),
            (Screen::SettingsDetail, Focus::SettingsSwitchObject),
        ] {
            let mut buffer = vec![0; resolution.width as usize * resolution.height as usize * 4];
            app.force_screen_for_snapshot(screen);
            app.focus = focus;
            let mut canvas = Canvas::new(resolution, &mut buffer).expect("canvas");
            app.draw(&mut canvas, 0.75);
            assert!(
                buffer.iter().any(|byte| *byte != 0),
                "{screen:?} should render at {resolution:?}"
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
