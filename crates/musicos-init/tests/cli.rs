use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

fn musicos_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_musicos"))
}

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("musicos-cli-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn command(root: &PathBuf) -> Command {
    let mut cmd = Command::new(musicos_bin());
    cmd.env("MUSICOS_MEDIA_DB", root.join("media.sqlite3"));
    cmd.env("MUSICOS_SETTINGS_PATH", root.join("settings.json"));
    cmd.env("MUSICOS_UPDATE_STAGING", root.join("staged-update"));
    cmd.env("MUSICOS_UPDATE_TARGET", root.join("installed-update"));
    cmd
}

#[test]
fn help_commands_print_usage() {
    let root = temp_root("help");
    let output = command(&root).arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("MusicOS library"));

    let output = command(&root).args(["scan", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Force a full metadata"));
}

#[test]
fn settings_get_json_returns_valid_json() {
    let root = temp_root("settings-json");
    let output = command(&root)
        .args(["settings", "get", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["device_name"], "MusicOS");
}

#[test]
fn doctor_succeeds_in_valid_test_environment() {
    let root = temp_root("doctor");
    let output = command(&root).args(["doctor", "--json"]).output().unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["db_ok"], true);
}

#[test]
fn sync_dry_run_does_not_write_to_device() {
    let root = temp_root("sync-dry-run");
    let music_dir = root.join("music");
    let device_dir = root.join("device");
    std::fs::create_dir_all(&music_dir).unwrap();
    std::fs::create_dir_all(&device_dir).unwrap();
    std::fs::write(music_dir.join("song.wav"), b"tiny fake wav for dry-run").unwrap();

    let output = command(&root)
        .env("MUSICOS_SYNC_DIR", &music_dir)
        .args([
            "sync",
            "start",
            device_dir.to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["writes"], false);
    assert_eq!(std::fs::read_dir(&device_dir).unwrap().count(), 0);
}

#[test]
fn sync_start_uploads_to_http_target_and_triggers_scan() {
    let root = temp_root("sync-http");
    let music_dir = root.join("music");
    std::fs::create_dir_all(music_dir.join("Artist").join("Album")).unwrap();
    std::fs::write(
        music_dir.join("Artist").join("Album").join("song.wav"),
        b"tiny fake wav upload body",
    )
    .unwrap();

    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let thread_seen = Arc::clone(&seen);
    let server = thread::spawn(move || {
        listener.set_nonblocking(false).unwrap();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request);
                    let content_length = header
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    let header_end = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .unwrap()
                        + 4;
                    while request.len() < header_end + content_length {
                        let read = stream.read(&mut buffer).unwrap();
                        if read == 0 {
                            break;
                        }
                        request.extend_from_slice(&buffer[..read]);
                    }
                    break;
                }
            }
            let text = String::from_utf8_lossy(&request).to_string();
            thread_seen.lock().unwrap().push(text.clone());
            let body = if text.starts_with("POST /api/upload ") {
                r#"{"status":"ok"}"#
            } else {
                r#"{"status":"ok"}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });

    let target = format!("http://{}", addr);
    let output = command(&root)
        .env("MUSICOS_SYNC_DIR", &music_dir)
        .args(["sync", "start", &target, "--json"])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["writes"], true);
    assert_eq!(value["uploaded"], 1);

    let requests = seen.lock().unwrap();
    assert!(requests
        .iter()
        .any(|request| request.starts_with("POST /api/upload ")));
    assert!(requests
        .iter()
        .any(|request| request.starts_with("POST /api/control ")));
    assert!(requests
        .iter()
        .any(|request| request.contains("X-Relative-Path: Artist/Album/song.wav")));
}

#[test]
fn invalid_device_id_returns_clean_error() {
    let root = temp_root("invalid-device");
    let output = command(&root)
        .args(["devices", "eject", "missing-device"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("device not found"));
    assert!(!stderr.to_ascii_lowercase().contains("panicked"));
}
