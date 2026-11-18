use musicos_init::{app::MusicOs, database::Db, media, renderer::Resolution, usb};
use std::{path::Path, sync::Mutex};

static ENV: Mutex<()> = Mutex::new(());

fn elf() -> Vec<u8> {
    let mut bytes = vec![0; 120];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
    let machine: u16 = if std::env::consts::ARCH == "aarch64" {
        183
    } else {
        62
    };
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
    bytes
}

fn squashfs() -> Vec<u8> {
    let mut bytes = vec![0; 96];
    bytes[..4].copy_from_slice(b"hsqs");
    bytes[12..16].copy_from_slice(&4096u32.to_le_bytes());
    bytes[28..30].copy_from_slice(&4u16.to_le_bytes());
    bytes[40..48].copy_from_slice(&96u64.to_le_bytes());
    bytes
}

fn configure(root: &Path) {
    for name in ["usb", "stage", "target"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    for (key, path) in [
        ("MUSICOS_USB_ROOTS", "usb"),
        ("MUSICOS_UPDATE_STAGING", "stage"),
        ("MUSICOS_UPDATE_TARGET", "target/init"),
        ("MUSICOS_UPDATE_IMAGE_TARGET", "target/knulli"),
        ("MUSICOS_SETTINGS_PATH", "settings.json"),
    ] {
        std::env::set_var(key, root.join(path));
    }
}

fn diagnostic(root: &Path, outcome: &str) {
    let text = std::fs::read_to_string(root.join("stage/last-update-result")).unwrap();
    println!("{text}");
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["outcome"], outcome);
    assert!(value["timestamp"].is_number());
}

#[test]
fn malformed_packages_preserve_target_and_pending_update() {
    let _guard = ENV.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    configure(root.path());
    let root = root.path();
    std::fs::write(root.join("target/init"), b"original").unwrap();
    std::fs::write(root.join("stage/musicos-init.next"), elf()).unwrap();
    let mut wrong_arch = elf();
    wrong_arch[18..20].copy_from_slice(&0u16.to_le_bytes());
    let mut wrong_class = elf();
    wrong_class[4] = 1;
    let mut wrong_version = squashfs();
    wrong_version[28] = 3;
    let mut wrong_block = squashfs();
    wrong_block[12..16].copy_from_slice(&4097u32.to_le_bytes());
    let mut truncated_image = squashfs();
    truncated_image[40..48].copy_from_slice(&1000u64.to_le_bytes());
    for payload in [
        wrong_arch,
        wrong_class,
        elf()[..100].to_vec(),
        wrong_version,
        wrong_block,
        truncated_image,
        b"garbage".to_vec(),
    ] {
        std::fs::write(root.join("usb/musicos-init"), payload).unwrap();
        assert!(usb::get_update_from_usb().is_err());
        assert_eq!(
            std::fs::read(root.join("target/init")).unwrap(),
            b"original"
        );
        assert_eq!(
            std::fs::read(root.join("stage/musicos-init.next")).unwrap(),
            elf()
        );
        diagnostic(root, "failure");
    }
}

#[test]
fn boot_cleans_interrupted_copies_and_rejects_partial_next() {
    let _guard = ENV.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    configure(root.path());
    let root = root.path();
    std::fs::write(root.join("target/init"), b"original").unwrap();
    std::fs::write(root.join("stage/musicos-init.tmp"), b"partial").unwrap();
    std::fs::write(root.join("target/.init.installing"), b"partial").unwrap();
    std::fs::write(root.join("stage/musicos-init.next"), &elf()[..64]).unwrap();
    assert!(usb::install_pending_update().is_err());
    assert!(!root.join("stage/musicos-init.tmp").exists());
    assert!(!root.join("target/.init.installing").exists());
    assert_eq!(
        std::fs::read(root.join("target/init")).unwrap(),
        b"original"
    );
    diagnostic(root, "failure");
    std::fs::write(root.join("stage/musicos-init.next"), elf()).unwrap();
    std::fs::create_dir(root.join("stage/musicos-init.previous")).unwrap();
    assert!(usb::install_pending_update().is_err());
    assert_eq!(
        std::fs::read(root.join("target/init")).unwrap(),
        b"original"
    );
    diagnostic(root, "failure");
    std::fs::remove_dir(root.join("stage/musicos-init.previous")).unwrap();
    assert!(usb::install_pending_update().unwrap().is_some());
    assert_eq!(
        std::fs::read(root.join("stage/musicos-init.previous")).unwrap(),
        b"original"
    );
    diagnostic(root, "success");
    assert!(usb::install_pending_update().unwrap().is_none());
    std::fs::write(root.join("usb/musicos-init"), squashfs()).unwrap();
    usb::get_update_from_usb().unwrap();
    assert_eq!(
        std::fs::read(root.join("target/knulli")).unwrap(),
        squashfs()
    );
}

#[test]
fn removed_root_during_scan_preserves_rows_and_marks_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let media_root = root.path().join("removable");
    std::fs::create_dir(&media_root).unwrap();
    std::fs::write(media_root.join("song.mp3"), b"fixture").unwrap();
    let db = Db::open(root.path().join("db.sqlite").to_str().unwrap()).unwrap();
    let source = db
        .add_library_source("reference", media_root.to_str().unwrap(), "USB")
        .unwrap();
    media::index_library_source_with_progress(&db, source.id, false, |_| {}).unwrap();
    let before = db.library_source_track_count(source.id).unwrap();
    assert!(before > 0);
    let mut removed = false;
    media::index_library_source_with_progress(&db, source.id, false, |_| {
        if !removed {
            std::fs::remove_dir_all(&media_root).unwrap();
            removed = true;
        }
    })
    .unwrap();
    assert_eq!(db.library_source_track_count(source.id).unwrap(), before);
    assert!(!db.library_source(source.id).unwrap().unwrap().available);
}

#[test]
fn unopenable_persistent_database_uses_independent_memory_databases() {
    let _guard = ENV.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    std::env::set_var("MUSICOS_SETTINGS_PATH", root.path().join("settings"));
    let app = MusicOs::with_db_path(Resolution::VGA_4_3, root.path().to_str().unwrap());
    assert!(app.db.create_playlist("temporary").is_ok());
    let other = Db::open(":memory:").unwrap();
    assert!(other.create_playlist("independent").is_ok());
}

#[cfg(unix)]
#[test]
fn traversal_errors_preserve_unseen_rows() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let album = root.path().join("usb/album");
    std::fs::create_dir_all(&album).unwrap();
    std::fs::write(album.join("song.mp3"), b"fixture").unwrap();
    let db = Db::open(root.path().join("db.sqlite").to_str().unwrap()).unwrap();
    let source = db
        .add_library_source(
            "reference",
            root.path().join("usb").to_str().unwrap(),
            "USB",
        )
        .unwrap();
    media::index_library_source_with_progress(&db, source.id, false, |_| {}).unwrap();
    let before = db.library_source_track_count(source.id).unwrap();
    assert!(before > 0);
    std::fs::set_permissions(&album, std::fs::Permissions::from_mode(0o000)).unwrap();
    let result = media::index_library_source_with_progress(&db, source.id, false, |_| {});
    std::fs::set_permissions(&album, std::fs::Permissions::from_mode(0o755)).unwrap();
    result.unwrap();
    assert_eq!(db.library_source_track_count(source.id).unwrap(), before);
    assert!(!db.library_source(source.id).unwrap().unwrap().available);
}

#[test]
fn readonly_database_write_returns_error_and_app_falls_back() {
    let _guard = ENV.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("db.sqlite");
    let db = Db::open(path.to_str().unwrap()).unwrap();
    db.create_playlist("persisted").unwrap();
    drop(db);
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let error = conn
        .execute("INSERT INTO playlists(name) VALUES ('unwritable')", [])
        .unwrap_err();
    println!("read-only write surfaced: {error}");
    assert!(error.to_string().contains("readonly"));
    std::env::set_var("MUSICOS_SETTINGS_PATH", root.path().join("settings"));
    let app = MusicOs::with_db_path(Resolution::VGA_4_3, "/proc/musicos-resilience/db.sqlite");
    assert!(app.db.create_playlist("fallback").is_ok());
}

#[test]
fn external_image_scenario() {
    let Ok(root) = std::env::var("MUSICOS_RESILIENCE_ROOT") else {
        return;
    };
    let _guard = ENV.lock().unwrap();
    let root = Path::new(&root);
    configure(root);
    match std::env::var("MUSICOS_RESILIENCE_SCENARIO")
        .unwrap()
        .as_str()
    {
        "copy" | "full" => {
            let full = std::env::var("MUSICOS_RESILIENCE_SCENARIO").unwrap() == "full";
            let mut bytes = elf();
            bytes.resize(if full { 32 } else { 128 } * 1024 * 1024, 0);
            std::fs::write(root.join("usb/musicos-init"), bytes).unwrap();
            std::fs::write(root.join("target/init"), b"original").unwrap();
            let result = usb::get_update_from_usb();
            if full {
                assert!(result.is_err());
                diagnostic(root, "failure");
                assert_eq!(
                    std::fs::read(root.join("target/init")).unwrap(),
                    b"original"
                );
            } else {
                result.unwrap();
                diagnostic(root, "success");
            }
        }
        "readonly" => {
            let app = MusicOs::with_db_path(
                Resolution::VGA_4_3,
                root.join("storage/db.sqlite").to_str().unwrap(),
            );
            let error = app.db.create_playlist("read-only write").unwrap_err();
            println!("read-only filesystem write surfaced after app startup: {error:#}");
            assert!(format!("{error:#}").contains("readonly"));
        }
        "db-full" => {
            let db = Db::open(root.join("storage/db.sqlite").to_str().unwrap()).unwrap();
            let name = "x".repeat(1024 * 1024);
            let mut error = None;
            for index in 0..32 {
                if let Err(failure) = db.create_playlist(&format!("{index}{name}")) {
                    error = Some(failure);
                    break;
                }
            }
            let error = error.expect("tiny filesystem should fill");
            println!("full DB write surfaced: {error:#}");
            assert!(format!("{error:#}").contains("full"));
        }
        value => panic!("unknown scenario {value}"),
    }
}
