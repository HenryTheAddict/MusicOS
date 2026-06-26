use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

const MIXER_CONTROLS: &[&str] = &[
    "Master",
    "Headphone",
    "Speaker",
    "PCM",
    "Line Out",
    "DAC",
    "Digital",
    "Playback",
];

#[derive(Debug, Clone, Serialize)]
pub struct BatteryInfo {
    pub capacity: Option<u8>,
    pub status: String,
    pub voltage_now: Option<i64>,
    pub current_now: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MixerProbe {
    pub control: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioDiagnostics {
    pub backend_env: Option<String>,
    pub alsa_device_env: Option<String>,
    pub alsa_config_path: Option<String>,
    pub pcm_candidates: Vec<String>,
    pub proc_asound_cards: Option<String>,
    pub proc_asound_pcm: Option<String>,
    pub amixer_path: Option<String>,
    pub aplay_path: Option<String>,
    pub mixer_controls: Option<String>,
    pub mixer_init_results: Vec<MixerProbe>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioTestResult {
    pub ok: bool,
    pub message: String,
    pub diagnostics: AudioDiagnostics,
}

pub fn apply_brightness(percent: u8) {
    let percent = if percent == 0 { 0 } else { percent.clamp(5, 100) };
    if let Err(err) = write_brightness(percent) {
        log::info!("brightness apply skipped: {err}");
    }
}

pub fn current_brightness_percent() -> Option<u8> {
    let path = brightness_path()?;
    let current = read_trimmed(path.clone())?.parse::<u32>().ok()?;
    let max = max_brightness_for(&path);
    Some(((current.saturating_mul(100) + max / 2) / max).clamp(1, 100) as u8)
}

pub fn brightness_value_for_percent(max: u32, percent: u8) -> u32 {
    let max = max.max(1);
    if percent == 0 {
        return 0;
    }
    let percent = percent.clamp(5, 100) as u32;
    ((max * percent + 50) / 100).clamp(1, max)
}

pub fn init_audio() {
    if let Ok(cards) = std::fs::read_to_string("/proc/asound/cards") {
        log::info!("alsa cards: {}", cards.replace('\n', " | "));
    } else {
        log::info!("alsa cards: /proc/asound/cards not readable");
    }

    let Some(amixer) = find_program("amixer") else {
        log::info!("audio mixer skipped: amixer not installed");
        return;
    };

    if let Ok(output) = Command::new(&amixer).arg("scontrols").output() {
        let text = String::from_utf8_lossy(&output.stdout).replace('\n', " | ");
        log::info!("audio mixer controls: {text}");
    }

    for control in MIXER_CONTROLS {
        run_amixer(&amixer, &["sset", control, "85%", "unmute"]);
    }
}

pub fn apply_system_volume(volume: f32) {
    let Some(amixer) = find_program("amixer") else {
        return;
    };
    let value = format!("{}%", (volume.clamp(0.0, 1.0) * 100.0).round() as u8);
    for control in MIXER_CONTROLS {
        run_amixer(&amixer, &["sset", control, &value, "unmute"]);
    }
}

pub fn audio_diagnostics() -> AudioDiagnostics {
    let amixer = find_program("amixer");
    let aplay = find_program("aplay");
    let mixer_controls = amixer.as_ref().and_then(|path| {
        Command::new(path).arg("scontrols").output().ok().map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stdout.is_empty() {
                stderr
            } else {
                stdout
            }
        })
    });
    let mixer_init_results = amixer
        .as_ref()
        .map(|path| {
            MIXER_CONTROLS
                .iter()
                .map(|control| probe_mixer_control(path, control))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    AudioDiagnostics {
        backend_env: std::env::var("MUSICOS_AUDIO_BACKEND").ok(),
        alsa_device_env: std::env::var("MUSICOS_ALSA_DEVICE").ok(),
        alsa_config_path: std::env::var("ALSA_CONFIG_PATH").ok(),
        pcm_candidates: crate::audio::alsa_pcm_candidates(
            std::env::var("MUSICOS_ALSA_DEVICE").ok().as_deref(),
        ),
        proc_asound_cards: read_file_for_diagnostics("/proc/asound/cards"),
        proc_asound_pcm: read_file_for_diagnostics("/proc/asound/pcm"),
        amixer_path: amixer.map(|path| path.display().to_string()),
        aplay_path: aplay.map(|path| path.display().to_string()),
        mixer_controls,
        mixer_init_results,
    }
}

pub fn run_audio_test() -> AudioTestResult {
    init_audio();
    let diagnostics = audio_diagnostics();
    let has_pcm = diagnostics
        .proc_asound_pcm
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_cards = diagnostics
        .proc_asound_cards
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let ok = has_pcm || has_cards;
    let message = if ok {
        format!(
            "Audio test found ALSA hardware; trying {}",
            diagnostics
                .pcm_candidates
                .first()
                .cloned()
                .unwrap_or_else(|| "default".to_string())
        )
    } else {
        "Audio test could not find ALSA cards or PCMs".to_string()
    };
    AudioTestResult {
        ok,
        message,
        diagnostics,
    }
}

pub fn battery_summary() -> String {
    let Some(info) = battery_info() else {
        return "battery status unavailable".to_string();
    };

    match info.capacity {
        Some(percent) if percent <= 15 && !info.status.eq_ignore_ascii_case("Charging") => {
            format!("low battery {percent}% - {}", info.status.to_ascii_lowercase())
        }
        Some(percent) if info.status.eq_ignore_ascii_case("Charging") => {
            format!("charging {percent}%")
        }
        Some(percent) => format!("battery {percent}% - {}", info.status.to_ascii_lowercase()),
        None => info.status,
    }
}

pub fn battery_info() -> Option<BatteryInfo> {
    let root = battery_root()?;
    let capacity = read_trimmed(root.join("capacity"))
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value.min(100));
    let status = read_trimmed(root.join("status")).unwrap_or_else(|| "Battery".to_string());
    let voltage_now = read_trimmed(root.join("voltage_now")).and_then(|value| value.parse().ok());
    let current_now = read_trimmed(root.join("current_now")).and_then(|value| value.parse().ok());
    Some(BatteryInfo {
        capacity,
        status,
        voltage_now,
        current_now,
    })
}

fn write_brightness(percent: u8) -> std::io::Result<()> {
    let Some(path) = brightness_path() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no backlight brightness path",
        ));
    };
    let value = brightness_value_for_percent(max_brightness_for(&path), percent);
    if let Some(parent) = path.parent() {
        let bl_power = parent.join("bl_power");
        if bl_power.exists() {
            let _ = std::fs::write(bl_power, "0");
        }
    }
    std::fs::write(path, value.to_string())
}

fn brightness_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("MUSICOS_BRIGHTNESS_PATH") {
        let path = PathBuf::from(path);
        if path.exists() || path.file_name().and_then(|n| n.to_str()) == Some("brightness") {
            return Some(path);
        }
    }

    for root in ["/sys/class/backlight", "/sys/class/leds"] {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut candidates = entries.flatten().collect::<Vec<_>>();
        candidates.sort_by_key(|entry| panel_score(root, entry.path().as_path()));
        for entry in candidates {
            let path = entry.path().join("brightness");
            if path.exists() && is_panel_like(root, entry.path().as_path()) {
                return Some(path);
            }
        }
    }
    None
}

fn battery_root() -> Option<PathBuf> {
    let entries = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = read_trimmed(path.join("type")).unwrap_or_default();
        if kind.eq_ignore_ascii_case("battery") {
            return Some(path);
        }
    }
    None
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn max_brightness_for(path: &Path) -> u32 {
    path.parent()
        .map(|parent| parent.join("max_brightness"))
        .and_then(read_trimmed)
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(255)
}

fn find_program(name: &str) -> Option<PathBuf> {
    ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
        .iter()
        .map(|root| Path::new(root).join(name))
        .find(|path| path.exists())
}

fn probe_mixer_control(amixer: &Path, control: &str) -> MixerProbe {
    match Command::new(amixer).args(["sget", control]).output() {
        Ok(output) if output.status.success() => MixerProbe {
            control: control.to_string(),
            ok: true,
            message: String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("ok")
                .to_string(),
        },
        Ok(output) => MixerProbe {
            control: control.to_string(),
            ok: false,
            message: String::from_utf8_lossy(&output.stderr)
                .trim()
                .to_string(),
        },
        Err(err) => MixerProbe {
            control: control.to_string(),
            ok: false,
            message: err.to_string(),
        },
    }
}

fn read_file_for_diagnostics(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn run_amixer(amixer: &Path, args: &[&str]) {
    match Command::new(amixer).args(args).output() {
        Ok(output) if output.status.success() => {
            log::info!("audio mixer ok: {} {}", amixer.display(), args.join(" "));
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::info!(
                "audio mixer skipped: {} {} rc={:?} {}",
                amixer.display(),
                args.join(" "),
                output.status.code(),
                stderr.trim()
            );
        }
        Err(err) => log::info!("audio mixer command failed: {err}"),
    }
}

fn is_panel_like(root: &str, path: &Path) -> bool {
    if root.ends_with("backlight") {
        return true;
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| {
            let name = name.to_ascii_lowercase();
            name.contains("lcd") || name.contains("backlight") || name.contains("panel")
        })
        .unwrap_or(false)
}

fn panel_score(root: &str, path: &Path) -> u8 {
    if root.ends_with("backlight") {
        return 0;
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("lcd-backlight") || name == "backlight" {
        0
    } else if name.contains("backlight") || name.contains("panel") {
        1
    } else if name.contains("lcd") {
        2
    } else {
        9
    }
}

#[cfg(test)]
mod tests {
    use super::brightness_value_for_percent;

    #[test]
    fn brightness_percent_maps_to_backlight_values() {
        assert_eq!(brightness_value_for_percent(255, 100), 255);
        assert_eq!(brightness_value_for_percent(255, 50), 128);
        assert_eq!(brightness_value_for_percent(255, 1), 13);
        assert_eq!(brightness_value_for_percent(8, 100), 8);
    }
}
