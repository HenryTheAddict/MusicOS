use musicos_init::app::{Focus, MusicOs, Screen};
use musicos_init::audio::EqPreset;
use musicos_init::database::Db;
use musicos_init::input::{Button, InputEvent};
use musicos_init::preferences::Preferences;
use musicos_init::renderer::Resolution;
use musicos_init::server::{ControlCommand, ServerEvent};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn scan_library_control_is_nonblocking_and_refreshes_when_done() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-async-scan-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let music_root = root.join("music");
    std::fs::create_dir_all(&music_root).expect("music dir");
    std::fs::write(
        music_root.join("Codex - Background Scanner.mp3"),
        b"minimal mp3 test payload",
    )
    .expect("song");

    std::env::set_var("MUSICOS_UPLOAD_DIR", music_root.display().to_string());
    std::env::set_var("MUSICOS_REINDEX_ALL", "1");

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    let started = Instant::now();
    assert!(app.handle_server_event(ServerEvent::Control(ControlCommand::ScanLibrary)));
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "scan command blocked the UI loop"
    );
    assert_eq!(app.scan_status, "Scanning library...");

    for _ in 0..150 {
        app.tick();
        if app.scan_status.starts_with("Scan complete:") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        app.scan_status.starts_with("Scan complete:"),
        "unexpected scan status: {}",
        app.scan_status
    );
    assert!(app.db.library_counts().unwrap().tracks >= 1);

    std::env::remove_var("MUSICOS_UPLOAD_DIR");
    std::env::remove_var("MUSICOS_REINDEX_ALL");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn apps_launcher_and_usb_sync_have_real_destinations() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-product-flow-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let usb_root = root.join("usb");
    let stage_root = root.join("stage");
    std::fs::create_dir_all(usb_root.join("musicos-update")).expect("usb update dir");
    std::fs::write(
        usb_root.join("Artist - USB Fixture.mp3"),
        b"minimal mp3 test payload",
    )
    .expect("song");
    std::fs::write(
        usb_root.join("musicos-update/musicos-init"),
        elf_update_payload(),
    )
    .expect("update");

    std::env::set_var("MUSICOS_USB_ROOTS", usb_root.display().to_string());
    std::env::set_var("MUSICOS_UPDATE_STAGING", stage_root.display().to_string());

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    app.force_screen_for_snapshot(Screen::Home);
    app.handle_input(InputEvent::Button(Button::Left));
    assert_eq!(app.focus, Focus::HomeApps);

    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::Apps);
    assert_eq!(app.focus, Focus::AppsMusic);

    app.handle_input(InputEvent::Button(Button::Down));
    app.handle_input(InputEvent::Button(Button::Down));
    app.handle_input(InputEvent::Button(Button::Down));
    assert_eq!(app.focus, Focus::AppsUsbSync);

    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::About);
    assert!(app.usb_status.media_files_indexed >= 1);
    assert!(app.usb_status.staged_update.is_some());
    assert!(stage_root.join("musicos-init.next").exists());

    std::env::remove_var("MUSICOS_USB_ROOTS");
    std::env::remove_var("MUSICOS_UPDATE_STAGING");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn settings_get_update_installs_usb_update_to_target() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-settings-update-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let usb_root = root.join("usb");
    let stage_root = root.join("stage");
    let update_target = root.join("installed/sbin/init");
    std::fs::create_dir_all(usb_root.join("MusicOS/update")).expect("usb update dir");
    std::fs::write(
        usb_root.join("MusicOS/update/musicos-init"),
        elf_update_payload(),
    )
    .expect("update");

    std::env::set_var("MUSICOS_USB_ROOTS", usb_root.display().to_string());
    std::env::set_var("MUSICOS_UPDATE_STAGING", stage_root.display().to_string());
    std::env::set_var("MUSICOS_UPDATE_TARGET", update_target.display().to_string());

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    app.force_screen_for_snapshot(Screen::Settings);
    app.focus = Focus::SettingsGetUpdate;

    app.handle_input(InputEvent::Button(Button::Confirm));

    assert_eq!(app.screen, Screen::About);
    assert!(app.usb_status.staged_update.is_some());
    assert_eq!(
        app.usb_status.installed_update.as_deref(),
        Some(update_target.to_str().unwrap())
    );
    assert_eq!(std::fs::read(&update_target).unwrap(), elf_update_payload());
    assert!(!stage_root.join("musicos-init.next").exists());

    std::env::remove_var("MUSICOS_USB_ROOTS");
    std::env::remove_var("MUSICOS_UPDATE_STAGING");
    std::env::remove_var("MUSICOS_UPDATE_TARGET");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn settings_get_update_installs_usb_system_image_update() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!(
        "musicos-settings-image-update-{}",
        std::process::id()
    ));
    let db_path = root.join("media.sqlite3");
    let usb_root = root.join("usb");
    let stage_root = root.join("stage");
    let update_target = root.join("boot/knulli");
    std::fs::create_dir_all(usb_root.join("musicos-update")).expect("usb update dir");
    std::fs::write(
        usb_root.join("musicos-update/musicos-knulli-ui.squashfs"),
        squashfs_update_payload(),
    )
    .expect("update");

    std::env::set_var("MUSICOS_USB_ROOTS", usb_root.display().to_string());
    std::env::set_var("MUSICOS_UPDATE_STAGING", stage_root.display().to_string());
    std::env::set_var(
        "MUSICOS_UPDATE_IMAGE_TARGET",
        update_target.display().to_string(),
    );

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    app.force_screen_for_snapshot(Screen::Settings);
    app.focus = Focus::SettingsGetUpdate;

    app.handle_input(InputEvent::Button(Button::Confirm));

    assert_eq!(app.screen, Screen::About);
    assert_eq!(
        app.usb_status.staged_update_kind.as_deref(),
        Some("system image")
    );
    assert_eq!(
        app.usb_status.installed_update.as_deref(),
        Some(update_target.to_str().unwrap())
    );
    assert_eq!(
        std::fs::read(&update_target).unwrap(),
        squashfs_update_payload()
    );
    assert!(!stage_root.join("musicos-knulli.next").exists());

    std::env::remove_var("MUSICOS_USB_ROOTS");
    std::env::remove_var("MUSICOS_UPDATE_STAGING");
    std::env::remove_var("MUSICOS_UPDATE_IMAGE_TARGET");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn completed_setup_saves_theme_and_skips_setup_next_boot() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-prefs-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let settings_path = root.join("settings.json");
    std::env::set_var("MUSICOS_SETTINGS_PATH", settings_path.display().to_string());

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    assert_eq!(app.screen, Screen::First);

    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Right));
    assert_eq!(app.active_theme, musicos_init::ui::theme::ThemeName::Red);
    app.handle_input(InputEvent::Button(Button::Confirm));

    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Confirm));

    let next = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    assert_eq!(next.screen, Screen::Home);
    assert_eq!(next.focus, Focus::HomeMusic);
    assert_eq!(next.active_theme, musicos_init::ui::theme::ThemeName::Red);

    std::env::remove_var("MUSICOS_SETTINGS_PATH");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn equalizer_setting_cycles_and_persists() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-eq-prefs-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let settings_path = root.join("settings.json");
    std::env::set_var("MUSICOS_SETTINGS_PATH", settings_path.display().to_string());

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    app.force_screen_for_snapshot(Screen::SettingsDetail);
    app.focus = Focus::SettingsEq;

    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.player.get_eq_preset(), EqPreset::Bass);
    app.handle_input(InputEvent::Button(Button::Right));
    assert_eq!(app.player.get_eq_preset(), EqPreset::Vocal);
    app.handle_input(InputEvent::Button(Button::Left));
    assert_eq!(app.player.get_eq_preset(), EqPreset::Bass);

    let next = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    assert_eq!(next.player.get_eq_preset(), EqPreset::Bass);

    std::env::remove_var("MUSICOS_SETTINGS_PATH");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn context_menu_actions_use_the_selected_library_item() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-context-menu-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());

    let first = app
        .db
        .insert_track("/music/alpha.ogg", "Alpha", "Tanger", "Glow", 120, 1, 1)
        .unwrap();
    let second = app
        .db
        .insert_track("/music/beta.ogg", "Beta", "Tanger", "Glow", 140, 1, 1)
        .unwrap();
    let third = app
        .db
        .insert_track("/music/gamma.ogg", "Gamma", "Tanger", "Night", 160, 1, 1)
        .unwrap();
    let playlist_id = app.db.create_playlist("Pocket Mix").unwrap();
    app.db.add_track_to_playlist(playlist_id, first).unwrap();
    app.refresh_library();

    app.force_screen_for_snapshot(Screen::SongsList);
    app.tracks_queue = app.db.get_tracks().unwrap();
    app.selected_index = app
        .tracks_queue
        .iter()
        .position(|track| track.id == second)
        .unwrap();
    app.handle_input(InputEvent::Button(Button::Context));
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::NowPlaying);
    assert_eq!(app.player.get_status().track.unwrap().id, second);

    app.force_screen_for_snapshot(Screen::SongsList);
    app.tracks_queue = app.db.get_tracks().unwrap();
    app.selected_index = app
        .tracks_queue
        .iter()
        .position(|track| track.id == second)
        .unwrap();
    app.handle_input(InputEvent::Button(Button::Context));
    app.handle_input(InputEvent::Button(Button::Down));
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert!(app
        .db
        .get_pins()
        .unwrap()
        .iter()
        .any(|pin| pin.pin_type == "track" && pin.target_id == second));

    app.force_screen_for_snapshot(Screen::AlbumsList);
    app.albums_list = app.db.get_albums().unwrap();
    app.selected_index = app
        .albums_list
        .iter()
        .position(|album| album == "Glow")
        .unwrap();
    app.handle_input(InputEvent::Button(Button::Context));
    app.handle_input(InputEvent::Button(Button::Down));
    app.handle_input(InputEvent::Button(Button::Down));
    app.handle_input(InputEvent::Button(Button::Confirm));
    let context_adds = app
        .db
        .get_playlists()
        .unwrap()
        .into_iter()
        .find(|playlist| playlist.name == "Context Adds")
        .unwrap();
    let added = app.db.get_playlist_tracks(context_adds.id).unwrap();
    assert_eq!(added.len(), 2);
    assert!(added.iter().any(|track| track.id == first));
    assert!(added.iter().any(|track| track.id == second));
    assert!(!added.iter().any(|track| track.id == third));

    app.force_screen_for_snapshot(Screen::PlaylistsList);
    app.playlists_list = app.db.get_playlists().unwrap();
    app.selected_index = app
        .playlists_list
        .iter()
        .position(|playlist| playlist.id == playlist_id)
        .unwrap();
    app.handle_input(InputEvent::Button(Button::Context));
    for _ in 0..4 {
        app.handle_input(InputEvent::Button(Button::Down));
    }
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert!(!app
        .db
        .get_playlists()
        .unwrap()
        .iter()
        .any(|playlist| playlist.id == playlist_id));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn context_menu_only_opens_for_real_targets() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-context-targets-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());

    app.force_screen_for_snapshot(Screen::MusicApp);
    app.focus = Focus::Albums;
    app.handle_input(InputEvent::Button(Button::Context));
    assert_eq!(app.screen, Screen::MusicApp);

    app.force_screen_for_snapshot(Screen::Home);
    app.focus = Focus::HomeSettings;
    app.handle_input(InputEvent::Button(Button::Context));
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.focus, Focus::HomeSettings);

    app.force_screen_for_snapshot(Screen::SongsList);
    app.tracks_queue.clear();
    app.selected_index = 0;
    app.handle_input(InputEvent::Button(Button::Context));
    assert_eq!(app.screen, Screen::SongsList);

    app.db
        .insert_track("/music/real.ogg", "Real", "Tanger", "Works", 120, 1, 1)
        .unwrap();
    app.tracks_queue = app.db.get_tracks().unwrap();
    app.force_screen_for_snapshot(Screen::Home);
    app.focus = Focus::HomeNowPlaying;
    app.handle_input(InputEvent::Button(Button::Context));
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.focus, Focus::HomePlaylist1);

    app.screen = Screen::SongsList;
    app.focus = Focus::Albums;
    app.handle_input(InputEvent::Button(Button::Context));
    assert_eq!(app.screen, Screen::ContextMenu);
    assert_eq!(app.previous_screen, Screen::SongsList);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn empty_home_does_not_create_demo_content() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-empty-home-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());

    app.refresh_library();
    assert!(app.db.get_playlists().unwrap().is_empty());

    app.force_screen_for_snapshot(Screen::Home);
    app.focus = Focus::HomePlaylist1;
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::Home);
    assert!(app.db.get_playlists().unwrap().is_empty());
    assert!(app.player.get_status().track.is_none());

    app.focus = Focus::HomeSettings;
    app.handle_input(InputEvent::Button(Button::Context));
    assert_eq!(app.focus, Focus::HomeSettings);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn album_order_and_favorites_match_music_player_expectations() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-ipod-flow-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());

    let second = app
        .db
        .insert_track_with_number("/music/02-beta.ogg", "Beta", "Tanger", "Glow", 2, 140, 1, 1)
        .unwrap();
    let first = app
        .db
        .insert_track_with_number(
            "/music/01-alpha.ogg",
            "Alpha",
            "Tanger",
            "Glow",
            1,
            120,
            1,
            1,
        )
        .unwrap();
    let other = app
        .db
        .insert_track_with_number(
            "/music/01-gamma.ogg",
            "Gamma",
            "Tanger",
            "Night",
            1,
            160,
            1,
            1,
        )
        .unwrap();

    app.force_screen_for_snapshot(Screen::AlbumsList);
    app.albums_list = app.db.get_albums().unwrap();
    app.selected_index = app
        .albums_list
        .iter()
        .position(|album| album == "Glow")
        .unwrap();
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::SongsList);
    assert_eq!(
        app.tracks_queue
            .iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
        vec![first, second]
    );
    assert!(!app.tracks_queue.iter().any(|track| track.id == other));

    app.player.set_queue(app.tracks_queue.clone(), 0);
    app.screen = Screen::NowPlaying;
    app.focus = Focus::Favorites;
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert!(app.db.get_track_by_id(first).unwrap().unwrap().favorited);
    assert!(app.player.get_status().track.unwrap().favorited);

    app.force_screen_for_snapshot(Screen::MusicApp);
    app.focus = Focus::Favorites;
    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::SongsList);
    assert_eq!(app.tracks_queue.len(), 1);
    assert_eq!(app.tracks_queue[0].id, first);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn queue_screen_uses_the_real_player_queue() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-queue-flow-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());

    let first = app
        .db
        .insert_track("/music/alpha.ogg", "Alpha", "Tanger", "Glow", 120, 1, 1)
        .unwrap();
    let second = app
        .db
        .insert_track("/music/beta.ogg", "Beta", "Tanger", "Glow", 140, 1, 1)
        .unwrap();
    let third = app
        .db
        .insert_track("/music/gamma.ogg", "Gamma", "Tanger", "Night", 160, 1, 1)
        .unwrap();

    let queue = app.db.get_tracks().unwrap();
    app.player.set_queue(queue.clone(), 1);
    app.force_screen_for_snapshot(Screen::MusicApp);
    app.focus = Focus::Queue;
    app.handle_input(InputEvent::Button(Button::Confirm));

    assert_eq!(app.screen, Screen::Queue);
    assert_eq!(app.selected_index, 1);
    assert_eq!(
        app.tracks_queue
            .iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
        vec![first, second, third]
    );

    app.handle_input(InputEvent::Button(Button::Confirm));
    assert_eq!(app.screen, Screen::NowPlaying);
    assert_eq!(app.player.get_status().track.unwrap().id, second);

    app.force_screen_for_snapshot(Screen::MusicApp);
    app.focus = Focus::Queue;
    app.handle_input(InputEvent::Button(Button::Confirm));
    app.handle_input(InputEvent::Button(Button::Back));
    assert_eq!(app.screen, Screen::MusicApp);
    assert_eq!(app.focus, Focus::Queue);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn persisted_queue_restores_paused_with_position() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-queue-restore-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let settings_path = root.join("settings.json");
    std::fs::create_dir_all(&root).unwrap();

    let db = Db::open(db_path.to_str().unwrap()).unwrap();
    let first = db
        .insert_track(
            "/music/restore-a.ogg",
            "Restore A",
            "Artist",
            "Album",
            120,
            1,
            1,
        )
        .unwrap();
    let second = db
        .insert_track(
            "/music/restore-b.ogg",
            "Restore B",
            "Artist",
            "Album",
            150,
            1,
            1,
        )
        .unwrap();
    drop(db);

    std::env::set_var("MUSICOS_SETTINGS_PATH", &settings_path);
    let mut preferences = Preferences::default();
    preferences.setup_complete = true;
    preferences.time_configured = true;
    preferences.resume_queue_track_ids = vec![first, second];
    preferences.resume_queue_index = 1;
    preferences.resume_position_seconds = 42.0;
    preferences.resume_paused = true;
    musicos_init::preferences::save(&preferences).unwrap();

    let app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    let status = app.player.get_status();

    assert_eq!(status.track.unwrap().id, second);
    assert!(!status.is_playing);
    assert_eq!(status.queue_len, 2);
    assert!((status.position - 42.0).abs() < 0.5);

    std::env::remove_var("MUSICOS_SETTINGS_PATH");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn settings_command_persists_validated_user_audio_and_library_values() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root =
        std::env::temp_dir().join(format!("musicos-settings-command-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let settings_path = root.join("settings.json");
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("MUSICOS_SETTINGS_PATH", &settings_path);

    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());
    app.handle_server_event(musicos_init::server::ServerEvent::Control(
        musicos_init::server::ControlCommand::SetSettings {
            settings: musicos_init::server::SettingsUpdate {
                user_name: Some("Henry".to_string()),
                device_name: Some("RG35XX Daily".to_string()),
                audio_backend: Some("alsa".to_string()),
                alsa_device: Some("plughw:0,0".to_string()),
                brightness: Some(250),
                volume: Some(1.4),
                shuffle: Some(true),
                loop_mode: Some("all".to_string()),
                library_sort: Some("play_count".to_string()),
                library_group: Some("artist".to_string()),
                ..Default::default()
            },
        },
    ));

    let preferences = musicos_init::preferences::load();
    assert_eq!(preferences.user_name, "Henry");
    assert_eq!(preferences.device_name, "RG35XX Daily");
    assert_eq!(preferences.audio_backend, "alsa");
    assert_eq!(preferences.alsa_device, "plughw:0,0");
    assert_eq!(preferences.brightness, 100);
    assert_eq!(preferences.volume, 1.0);
    assert!(preferences.shuffle);
    assert_eq!(preferences.loop_mode, musicos_init::audio::LoopMode::All);
    assert_eq!(
        preferences.library_sort,
        musicos_init::preferences::LibrarySort::PlayCount
    );
    assert_eq!(
        preferences.library_group,
        musicos_init::preferences::LibraryGroup::Artist
    );

    std::env::remove_var("MUSICOS_SETTINGS_PATH");
    let _ = std::fs::remove_dir_all(root);
}

fn elf_update_payload() -> Vec<u8> {
    let mut payload = b"\x7fELFMusicOS test update".to_vec();
    payload.resize(64, 0);
    payload
}

fn squashfs_update_payload() -> Vec<u8> {
    let mut payload = b"hsqsMusicOS test squashfs update".to_vec();
    payload.resize(128, 0);
    payload
}

#[test]
fn volume_brightness_hud_and_chorded_modifier() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("musicos-hud-test-{}", std::process::id()));
    let db_path = root.join("media.sqlite3");
    let mut app = MusicOs::with_db_path(Resolution::HD_4_3, db_path.to_str().unwrap());

    app.screen = Screen::Welcome; // start away from home screen

    let initial_vol = app.player.get_status().volume;
    let initial_bri = app.brightness;
    assert_eq!(app.hud_t, 0.0);

    // 1. Regular Volume Adjustment
    app.handle_input(InputEvent::Button(Button::VolumeUp));
    assert_eq!(app.hud_type, musicos_init::app::HudType::Volume);
    assert!(app.hud_timer > 1.9); // initialized to 2.0
    let vol_after_up = app.player.get_status().volume;
    assert!((vol_after_up - (initial_vol + 0.05).min(1.0)).abs() < 0.001);

    // 2. Modifier Chord (Home Pressed + Volume Down)
    app.handle_input(InputEvent::Button(Button::Home));
    assert!(app.home_held);
    assert!(!app.home_chorded);

    app.handle_input(InputEvent::Button(Button::VolumeDown));
    assert_eq!(app.hud_type, musicos_init::app::HudType::Brightness);
    assert_eq!(app.brightness, initial_bri.saturating_sub(5).max(5));
    assert!(app.home_chorded);

    app.handle_input(InputEvent::ButtonRelease(Button::Home));
    assert!(!app.home_held);
    // Should NOT have navigated to Home Screen because it was chorded!
    assert_ne!(app.screen, Screen::Home);

    // 3. Regular Home Button navigation (without chord)
    app.handle_input(InputEvent::Button(Button::Home));
    app.handle_input(InputEvent::ButtonRelease(Button::Home));
    assert_eq!(app.screen, Screen::Home);

    let _ = std::fs::remove_dir_all(root);
}
