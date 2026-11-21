use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::renderer::{Canvas, Color, Resolution};
use musicos_init::ui::{legend_bar, status_bar, theme::ThemeName};

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
