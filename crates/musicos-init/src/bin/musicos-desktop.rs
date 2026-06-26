use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let profile = std::env::var("MUSICOS_DESKTOP_PROFILE").unwrap_or_else(|_| "debug".to_string());
    let bin_dir = PathBuf::from(target_dir).join(&profile);
    let mac_bin = bin_dir.join(bin_name("musicos-mac"));
    let sync_bin = bin_dir.join(bin_name("musicos-sync"));

    ensure_binary(&mac_bin, "musicos-mac")?;
    ensure_binary(&sync_bin, "musicos-sync")?;

    let port = std::env::var("MUSICOS_COMPANION_PORT").unwrap_or_else(|_| "18080".to_string());
    let db_path = std::env::var("MUSICOS_MEDIA_DB").unwrap_or_else(|_| {
        let _ = std::fs::create_dir_all("out");
        "out/desktop-media.sqlite3".to_string()
    });
    let sync_target =
        std::env::var("MUSICOS_SYNC_TARGET").unwrap_or_else(|_| format!("http://127.0.0.1:{port}"));
    let sync_dir = std::env::var("MUSICOS_SYNC_DIR").unwrap_or_else(|_| "./music".to_string());

    println!("MusicOS desktop suite");
    println!("  emulator: {}", mac_bin.display());
    println!("  sync app: {}", sync_bin.display());
    println!("  API:      {sync_target}");
    println!("  DB:       {db_path}");
    println!("  media:    {sync_dir}");

    let mut emulator = Command::new(&mac_bin)
        .env("MUSICOS_COMPANION_PORT", &port)
        .env("MUSICOS_MEDIA_DB", &db_path)
        .spawn()
        .with_context(|| format!("starting {}", mac_bin.display()))?;

    wait_for_api(&sync_target, Duration::from_secs(6));

    let mut sync = Command::new(&sync_bin)
        .env("MUSICOS_SYNC_TARGET", &sync_target)
        .env("MUSICOS_SYNC_DIR", &sync_dir)
        .spawn()
        .with_context(|| format!("starting {}", sync_bin.display()))?;

    wait_pair(&mut emulator, &mut sync)
}

fn ensure_binary(path: &PathBuf, package_bin: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    println!("Building missing {package_bin}...");
    let status = Command::new("cargo")
        .args([
            "build",
            "--bin",
            package_bin,
            "--features",
            "desktop-app,desktop-audio",
        ])
        .status()
        .with_context(|| format!("building {package_bin}"))?;
    anyhow::ensure!(status.success(), "cargo build failed for {package_bin}");
    anyhow::ensure!(
        path.exists(),
        "built binary not found at {}",
        path.display()
    );
    Ok(())
}

fn wait_for_api(target: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if http_get_ok(target) {
            return;
        }
        thread::sleep(Duration::from_millis(120));
    }
}

fn http_get_ok(target: &str) -> bool {
    let Some((host, port)) = parse_target(target) else {
        return false;
    };
    let Ok(mut stream) = std::net::TcpStream::connect((host.as_str(), port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let request = format!("GET /api/status HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    use std::io::{Read, Write};
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok() && response.contains("200")
}

fn parse_target(raw: &str) -> Option<(String, u16)> {
    let without_scheme = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))
        .unwrap_or(raw);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let (host, port) = authority.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

fn wait_pair(emulator: &mut Child, sync: &mut Child) -> Result<()> {
    loop {
        if let Some(status) = emulator.try_wait()? {
            let _ = sync.kill();
            anyhow::bail!("emulator exited with {status}");
        }
        if let Some(status) = sync.try_wait()? {
            let _ = emulator.kill();
            anyhow::bail!("sync app exited with {status}");
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn bin_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
