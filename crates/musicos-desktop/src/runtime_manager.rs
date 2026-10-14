use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};

use crate::models::{ControlCenterConfig, EmulatorStatus};

const MAX_LOG_LINES: usize = 700;

pub struct RuntimeManager {
    child: Option<Child>,
    logs: Arc<Mutex<VecDeque<String>>>,
    status: EmulatorStatus,
    media_dir: PathBuf,
}

impl RuntimeManager {
    pub fn new(config: &ControlCenterConfig) -> Self {
        let bin_path = emulator_binary();
        Self {
            child: None,
            logs: Arc::new(Mutex::new(VecDeque::new())),
            status: EmulatorStatus {
                target: config.library_target.clone(),
                db_path: config.db_path.clone(),
                sync_dir: config.sync_dir.clone(),
                bin_path,
                ..Default::default()
            },
            media_dir: config.media_dir.clone(),
        }
    }

    pub fn start(&mut self) -> Result<EmulatorStatus> {
        self.refresh();
        if self.status.running {
            return Ok(self.status.clone());
        }
        self.status.starting = true;
        if !self.status.bin_path.is_file() {
            build_emulator()?;
        }
        let mut child = Command::new(&self.status.bin_path)
            .env("MUSICOS_MEDIA_DB", &self.status.db_path)
            .env("MUSICOS_UPLOAD_DIR", &self.media_dir)
            .env("MUSICOS_TEST_AUDIO_BACKEND", "none")
            .env(
                "MUSICOS_COMPANION_PORT",
                target_port(&self.status.target)
                    .unwrap_or(18080)
                    .to_string(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {}", self.status.bin_path.display()))?;
        if let Some(stdout) = child.stdout.take() {
            stream_logs(stdout, self.logs.clone(), "preview");
        }
        if let Some(stderr) = child.stderr.take() {
            stream_logs(stderr, self.logs.clone(), "preview:error");
        }
        self.status.pid = Some(child.id());
        self.status.running = true;
        self.status.starting = false;
        self.status.last_error = None;
        self.child = Some(child);
        Ok(self.status.clone())
    }

    pub fn stop(&mut self) -> Result<EmulatorStatus> {
        if let Some(mut child) = self.child.take() {
            child.kill().context("stopping MusicOS runtime")?;
            let _ = child.wait();
        }
        self.status.running = false;
        self.status.starting = false;
        self.status.pid = None;
        Ok(self.status.clone())
    }

    pub fn restart(&mut self) -> Result<EmulatorStatus> {
        self.stop()?;
        self.start()
    }

    pub fn refresh(&mut self) -> EmulatorStatus {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(exit)) => {
                    self.status.running = false;
                    self.status.pid = None;
                    self.status.last_error = Some(format!("runtime exited with {exit}"));
                    self.child = None;
                }
                Ok(None) => {}
                Err(error) => {
                    self.status.last_error = Some(error.to_string());
                }
            }
        }
        self.status.clone()
    }

    pub fn logs(&self) -> Vec<String> {
        self.logs
            .lock()
            .map(|logs| logs.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn status(&self) -> EmulatorStatus {
        self.status.clone()
    }

    pub fn clear_logs(&self) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.clear();
        }
    }
}

impl Drop for RuntimeManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn stream_logs(
    stream: impl std::io::Read + Send + 'static,
    logs: Arc<Mutex<VecDeque<String>>>,
    source: &'static str,
) {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if let Ok(mut output) = logs.lock() {
                output.push_back(format!("[{source}] {line}"));
                while output.len() > MAX_LOG_LINES {
                    output.pop_front();
                }
            }
        }
    });
}

fn build_emulator() -> Result<()> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--bin",
            "musicos-mac",
            "--features",
            "desktop-app,desktop-audio",
        ])
        .status()
        .context("building musicos-mac")?;
    anyhow::ensure!(status.success(), "cargo build failed for musicos-mac");
    Ok(())
}

fn emulator_binary() -> PathBuf {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            let sibling = directory.join(if cfg!(windows) {
                "musicos-mac.exe"
            } else {
                "musicos-mac"
            });
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));
    let profile = std::env::var("MUSICOS_DESKTOP_PROFILE").unwrap_or_else(|_| "debug".to_string());
    target.join(profile).join(if cfg!(windows) {
        "musicos-mac.exe"
    } else {
        "musicos-mac"
    })
}

fn target_port(target: &str) -> Option<u16> {
    target
        .trim_end_matches('/')
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
}

#[allow(dead_code)]
fn _is_executable(path: &Path) -> bool {
    path.is_file()
}
