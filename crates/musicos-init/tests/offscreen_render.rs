use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::renderer::{Canvas, Resolution};

#[test]
fn renders_setup_screen_at_both_supported_resolutions() {
    for resolution in [Resolution::HD_4_3, Resolution::VGA_4_3] {
        let mut buffer = vec![0; resolution.width as usize * resolution.height as usize * 4];
        let mut canvas = Canvas::new(resolution, &mut buffer).expect("canvas");
        let mut app = MusicOs::new(resolution);
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
