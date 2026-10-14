use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::audio::{EqPreset, LoopMode};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;

pub const CURRENT_SCHEMA_VERSION: u16 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibrarySort {
    Title,
    Artist,
    Album,
    RecentlyAdded,
    PlayCount,
}

impl Default for LibrarySort {
    fn default() -> Self {
        Self::Title
    }
}

impl LibrarySort {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "artist" => Self::Artist,
            "album" => Self::Album,
            "recent" | "recently_added" | "recently-added" => Self::RecentlyAdded,
            "plays" | "play_count" | "play-count" => Self::PlayCount,
            _ => Self::Title,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::RecentlyAdded => "Recently Added",
            Self::PlayCount => "Play Count",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryGroup {
    None,
    Artist,
    Album,
    FirstLetter,
}

impl Default for LibraryGroup {
    fn default() -> Self {
        Self::None
    }
}

impl LibraryGroup {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "artist" => Self::Artist,
            "album" => Self::Album,
            "letter" | "first_letter" | "first-letter" => Self::FirstLetter,
            _ => Self::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::FirstLetter => "A-Z",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotionMode {
    Balanced,
    BatterySaver,
    Visuals,
    Reduced,
}

impl Default for MotionMode {
    fn default() -> Self {
        Self::Balanced
    }
}

impl MotionMode {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "battery" | "battery_saver" | "battery-saver" => Self::BatterySaver,
            "visuals" | "extra" | "full" => Self::Visuals,
            "reduced" | "reduced_motion" | "reduced-motion" => Self::Reduced,
            _ => Self::Balanced,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced",
            Self::BatterySaver => "Battery Saver",
            Self::Visuals => "Visuals",
            Self::Reduced => "Reduced Motion",
        }
    }

    pub fn profile(self) -> MotionProfile {
        match self {
            Self::Visuals => MotionProfile {
                screen_seconds: 0.28,
                artwork_seconds: 0.42,
                selection_response: 15.0,
                ambient_time_scale: 1.0,
                ambient_fps: 60,
                overshoot: 0.04,
            },
            Self::Balanced => MotionProfile {
                screen_seconds: 0.21,
                artwork_seconds: 0.32,
                selection_response: 17.0,
                ambient_time_scale: 0.8,
                ambient_fps: 30,
                overshoot: 0.02,
            },
            Self::BatterySaver => MotionProfile {
                screen_seconds: 0.12,
                artwork_seconds: 0.14,
                selection_response: 22.0,
                ambient_time_scale: 0.35,
                ambient_fps: 12,
                overshoot: 0.0,
            },
            Self::Reduced => MotionProfile {
                screen_seconds: 0.0,
                artwork_seconds: 0.0,
                selection_response: f32::INFINITY,
                ambient_time_scale: 0.0,
                ambient_fps: 0,
                overshoot: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionProfile {
    pub screen_seconds: f32,
    pub artwork_seconds: f32,
    pub selection_response: f32,
    pub ambient_time_scale: f32,
    pub ambient_fps: u16,
    pub overshoot: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub schema_version: u16,
    pub user_name: String,
    pub device_name: String,
    pub theme: ThemeName,
    pub language: Language,
    pub setup_complete: bool,
    pub time_configured: bool,
    pub clock_hour: u8,
    pub clock_minute: u8,
    pub clock_month: u8,
    pub clock_day: u8,
    pub clock_year: u16,
    pub brightness: u8,
    pub volume: f32,
    pub eq_preset: EqPreset,
    pub audio_backend: String,
    pub alsa_device: String,
    pub audio_output_mode: String,
    pub shuffle: bool,
    pub loop_mode: LoopMode,
    pub resume_queue_track_ids: Vec<i64>,
    pub resume_queue_index: usize,
    pub resume_track_id: Option<i64>,
    pub resume_position_seconds: f32,
    pub resume_paused: bool,
    pub library_sort: LibrarySort,
    pub library_group: LibraryGroup,
    pub sleep_timeout_minutes: u8,
    pub dim_timeout_seconds: u16,
    pub motion_mode: MotionMode,
    pub seek_step_seconds: u8,
    pub sleep_timer_default_minutes: u8,
    pub resume_queue_enabled: bool,
    pub shuffle_default_enabled: bool,
    pub first_run_audio_test_complete: bool,
    pub last_music_focus: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            user_name: "MusicOS User".to_string(),
            device_name: "MusicOS".to_string(),
            theme: ThemeName::Blue,
            language: Language::En,
            setup_complete: false,
            // The RG35XX boot chain initializes the system clock. Keep manual date/time
            // available in Settings without forcing a six-step wizard on first boot.
            time_configured: true,
            clock_hour: 7,
            clock_minute: 0,
            clock_month: 6,
            clock_day: 17,
            clock_year: 2026,
            brightness: 85,
            volume: 0.85,
            eq_preset: EqPreset::Flat,
            audio_backend: default_audio_backend(),
            alsa_device: String::new(),
            audio_output_mode: "speaker".to_string(),
            shuffle: false,
            loop_mode: LoopMode::None,
            resume_queue_track_ids: Vec::new(),
            resume_queue_index: 0,
            resume_track_id: None,
            resume_position_seconds: 0.0,
            resume_paused: true,
            library_sort: LibrarySort::Title,
            library_group: LibraryGroup::None,
            sleep_timeout_minutes: 0,
            dim_timeout_seconds: 45,
            motion_mode: MotionMode::Balanced,
            seek_step_seconds: 10,
            sleep_timer_default_minutes: 30,
            resume_queue_enabled: true,
            shuffle_default_enabled: false,
            first_run_audio_test_complete: false,
            last_music_focus: "albums".to_string(),
        }
    }
}

impl Preferences {
    pub fn sanitized(mut self) -> Self {
        self.schema_version = CURRENT_SCHEMA_VERSION;
        if self.user_name.trim().is_empty() {
            self.user_name = "MusicOS User".to_string();
        }
        if self.device_name.trim().is_empty() {
            self.device_name = "MusicOS".to_string();
        }
        self.clock_hour = self.clock_hour.min(23);
        self.clock_minute = self.clock_minute.min(59);
        self.clock_month = self.clock_month.clamp(1, 12);
        self.clock_day = self.clock_day.clamp(1, 31);
        self.clock_year = self.clock_year.clamp(2024, 2099);
        self.brightness = self.brightness.clamp(5, 100);
        self.volume = if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            Preferences::default().volume
        };
        self.audio_backend = match self.audio_backend.to_ascii_lowercase().as_str() {
            "alsa" => "alsa".to_string(),
            "rodio" => "rodio".to_string(),
            "none" | "silent" => "none".to_string(),
            _ => default_audio_backend(),
        };
        if self.audio_output_mode.trim().is_empty() {
            self.audio_output_mode = "speaker".to_string();
        }
        self.resume_position_seconds = if self.resume_position_seconds.is_finite() {
            self.resume_position_seconds.max(0.0)
        } else {
            0.0
        };
        if self.resume_queue_track_ids.is_empty() {
            self.resume_queue_index = 0;
        }
        self.dim_timeout_seconds = self.dim_timeout_seconds.clamp(10, 600);
        self.seek_step_seconds = self.seek_step_seconds.clamp(5, 60);
        self.sleep_timer_default_minutes = self.sleep_timer_default_minutes.min(180);
        if !self.resume_queue_enabled {
            self.resume_queue_track_ids.clear();
            self.resume_queue_index = 0;
            self.resume_track_id = None;
            self.resume_position_seconds = 0.0;
        }
        self.last_music_focus = match self.last_music_focus.to_ascii_lowercase().as_str() {
            "songs" | "artists" | "playlists" | "favorites" | "queue" | "history" => {
                self.last_music_focus.to_ascii_lowercase()
            }
            _ => "albums".to_string(),
        };
        self
    }
}

pub fn load() -> Preferences {
    let path = path();
    let Ok(bytes) = std::fs::read(&path) else {
        return Preferences::default();
    };
    match serde_json::from_slice::<Preferences>(&bytes) {
        Ok(preferences) => preferences.sanitized(),
        Err(err) => {
            log::error!("settings JSON at {} is corrupt: {err}", path.display());
            let backup = corrupt_backup_path(&path);
            if let Err(backup_err) = std::fs::write(&backup, &bytes) {
                log::error!(
                    "failed to back up corrupt settings to {}: {backup_err}",
                    backup.display()
                );
            } else {
                log::warn!("backed up corrupt settings to {}", backup.display());
            }
            Preferences::default()
        }
    }
}

pub fn save(preferences: &Preferences) -> Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    let preferences = preferences.clone().sanitized();
    let bytes = serde_json::to_vec_pretty(&preferences)?;
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(temp, &path)?;
    if let Some(parent) = path.parent() {
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

pub fn path() -> PathBuf {
    if let Ok(value) = std::env::var("MUSICOS_SETTINGS_PATH") {
        return PathBuf::from(value);
    }
    if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("MusicOS")
                .join("settings.json");
        }
        PathBuf::from("out/musicos-settings.json")
    } else {
        PathBuf::from("/var/lib/musicos/settings.json")
    }
}

fn corrupt_backup_path(path: &std::path::Path) -> PathBuf {
    let mut backup = path.as_os_str().to_os_string();
    backup.push(".bad");
    PathBuf::from(backup)
}

fn default_audio_backend() -> String {
    if cfg!(target_os = "linux") {
        "alsa".to_string()
    } else {
        "auto".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn old_settings_json_migrates_to_v3_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root =
            std::env::temp_dir().join(format!("musicos-prefs-migrate-{}", std::process::id()));
        let path = root.join("settings.json");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            &path,
            r#"{"theme":"Green","language":"En","brightness":2,"volume":1.7,"shuffle":true}"#,
        )
        .unwrap();
        std::env::set_var("MUSICOS_SETTINGS_PATH", &path);

        let preferences = load();

        assert_eq!(preferences.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(preferences.theme, ThemeName::Green);
        assert_eq!(preferences.brightness, 5);
        assert_eq!(preferences.volume, 1.0);
        assert!(preferences.shuffle);
        assert!(preferences.resume_paused);

        std::env::remove_var("MUSICOS_SETTINGS_PATH");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_settings_are_backed_up_and_defaults_load() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root =
            std::env::temp_dir().join(format!("musicos-prefs-corrupt-{}", std::process::id()));
        let path = root.join("settings.json");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&path, b"{not json").unwrap();
        std::env::set_var("MUSICOS_SETTINGS_PATH", &path);

        let preferences = load();

        assert_eq!(preferences.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(preferences.theme, Preferences::default().theme);
        assert!(root.join("settings.json.bad").exists());

        std::env::remove_var("MUSICOS_SETTINGS_PATH");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn motion_profiles_scale_down_without_losing_state_feedback() {
        let visuals = MotionMode::Visuals.profile();
        let balanced = MotionMode::Balanced.profile();
        let saver = MotionMode::BatterySaver.profile();
        let reduced = MotionMode::Reduced.profile();

        assert!(visuals.artwork_seconds > balanced.artwork_seconds);
        assert!(balanced.artwork_seconds > saver.artwork_seconds);
        assert_eq!(saver.ambient_fps, 12);
        assert_eq!(reduced.ambient_fps, 0);
        assert_eq!(reduced.screen_seconds, 0.0);
        assert_eq!(reduced.overshoot, 0.0);
    }
}
