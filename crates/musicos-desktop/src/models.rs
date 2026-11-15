use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const DESKTOP_CONFIG_VERSION: u32 = 5;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct TaskId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    LibraryImport,
    LibraryScan,
    ReferenceScan,
    SyncDryRun,
    Hashing,
    Transcoding,
    Upload,
    Verification,
    PlaylistComparison,
    PlaylistApply,
    PlaylistRollback,
    Backup,
    Restore,
    DiagnosticsExport,
    PerformanceSmoke,
    Update,
    EmulatorStart,
    EmulatorStop,
    EmulatorRestart,
    MetadataLookup,
    ArtworkDownload,
    Fingerprinting,
    LibraryRelocation,
    MetadataWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPhase {
    Queued,
    Discovering,
    Processing,
    Hashing,
    Transcoding,
    Uploading,
    Verifying,
    Applying,
    RollingBack,
    Starting,
    Stopping,
    Finishing,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskProgress {
    Indeterminate,
    Items { current: u64, total: u64 },
    Bytes { current: u64, total: u64 },
}

impl TaskProgress {
    pub fn fraction(&self) -> Option<f32> {
        let (current, total) = match *self {
            Self::Indeterminate => return None,
            Self::Items { current, total } | Self::Bytes { current, total } => (current, total),
        };
        (total > 0).then(|| (current as f32 / total as f32).clamp(0.0, 1.0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAction {
    Cancel,
    Retry,
    Reveal,
    ViewResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationTask {
    pub id: TaskId,
    pub kind: TaskKind,
    pub title: String,
    pub phase: TaskPhase,
    pub status: TaskStatus,
    pub progress: TaskProgress,
    pub current_item: Option<String>,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub cancellable: bool,
    pub retryable: bool,
    pub summary: Option<String>,
    pub error: Option<String>,
    pub origin: Option<DesktopSection>,
    #[serde(default)]
    pub log: Vec<String>,
}

impl OperationTask {
    pub const MAX_LOG_LINES: usize = 100;

    pub fn push_log(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > Self::MAX_LOG_LINES {
            self.log.drain(..self.log.len() - Self::MAX_LOG_LINES);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopSection {
    Music,
    #[default]
    Albums,
    Artists,
    Genres,
    Songs,
    Favorites,
    Queue,
    Playlists,
    Devices,
    Sync,
    Updates,
    Logs,
    Settings,
}

impl DesktopSection {
    pub const ALL: [Self; 12] = [
        Self::Albums,
        Self::Artists,
        Self::Genres,
        Self::Songs,
        Self::Favorites,
        Self::Queue,
        Self::Playlists,
        Self::Devices,
        Self::Sync,
        Self::Updates,
        Self::Logs,
        Self::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Music => "Music",
            Self::Albums => "Albums",
            Self::Artists => "Artists",
            Self::Genres => "Genres",
            Self::Songs => "Songs",
            Self::Favorites => "Favorites",
            Self::Queue => "Queue",
            Self::Playlists => "Playlists",
            Self::Devices => "Device Preview",
            Self::Sync => "Sync",
            Self::Updates => "Updates",
            Self::Logs => "Diagnostics",
            Self::Settings => "Preferences",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Self::Music
            | Self::Albums
            | Self::Artists
            | Self::Genres
            | Self::Songs
            | Self::Favorites
            | Self::Queue => "LIBRARY",
            Self::Playlists => "PLAYLISTS",
            Self::Devices => "DEVICES",
            Self::Sync | Self::Updates => "OPERATIONS",
            Self::Logs | Self::Settings => "UTILITIES",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryViewMode {
    #[default]
    Songs,
    Albums,
    Artists,
    CoverFlow,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LibraryScopeKey {
    Music,
    Songs,
    #[default]
    Albums,
    Artists,
    Genres,
    Favorites,
    History,
    Queue,
    Playlist,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryPresentation {
    #[default]
    Table,
    Grid,
    Flow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryUiPreferences {
    pub last_scope: LibraryScopeKey,
    pub presentation_by_scope: BTreeMap<LibraryScopeKey, LibraryPresentation>,
    pub sort_by_scope: BTreeMap<LibraryScopeKey, SortDescriptor>,
    pub album_grid_card_width: u16,
}

impl Default for LibraryUiPreferences {
    fn default() -> Self {
        let presentation_by_scope = [
            (LibraryScopeKey::Music, LibraryPresentation::Table),
            (LibraryScopeKey::Songs, LibraryPresentation::Table),
            (LibraryScopeKey::Albums, LibraryPresentation::Grid),
            (LibraryScopeKey::Artists, LibraryPresentation::Grid),
            (LibraryScopeKey::Genres, LibraryPresentation::Table),
            (LibraryScopeKey::Favorites, LibraryPresentation::Table),
            (LibraryScopeKey::History, LibraryPresentation::Table),
            (LibraryScopeKey::Queue, LibraryPresentation::Table),
            (LibraryScopeKey::Playlist, LibraryPresentation::Table),
        ]
        .into_iter()
        .collect();
        Self {
            last_scope: LibraryScopeKey::Albums,
            presentation_by_scope,
            sort_by_scope: BTreeMap::new(),
            album_grid_card_width: 190,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackOutput {
    #[default]
    Computer,
    Device,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "route", rename_all = "snake_case")]
pub enum LibraryRoute {
    Songs,
    Albums,
    Album { album: String, artist: String },
    Artists,
    Artist { artist: String },
    Genres,
    Genre { genre: String },
    Flow { focused_album: usize },
    Playlist { id: i64 },
    Queue,
    Favorites,
    History,
}

impl Default for LibraryRoute {
    fn default() -> Self {
        Self::Songs
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    #[default]
    Title,
    Artist,
    Album,
    Genre,
    Year,
    TrackNumber,
    Duration,
    Rating,
    PlayCount,
    DateAdded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SortDescriptor {
    pub field: SortField,
    pub descending: bool,
}

impl Default for SortDescriptor {
    fn default() -> Self {
        Self {
            field: SortField::Title,
            descending: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnPreference {
    pub field: String,
    pub width: f32,
    pub visible: bool,
}

impl Default for ColumnPreference {
    fn default() -> Self {
        Self {
            field: "title".to_string(),
            width: 300.0,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackPreferences {
    pub queue_track_ids: Vec<i64>,
    pub queue_index: usize,
    pub position: f32,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceDensity {
    Compact,
    #[default]
    Standard,
    Comfortable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearancePreferences {
    pub mode: AppearanceMode,
    pub accent: String,
    pub density: InterfaceDensity,
    pub sidebar_width: u16,
    pub row_height: u16,
}

impl Default for AppearancePreferences {
    fn default() -> Self {
        Self {
            mode: AppearanceMode::System,
            accent: "cobalt".to_string(),
            density: InterfaceDensity::Standard,
            sidebar_width: 220,
            row_height: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EnrichmentPreferences {
    pub enabled: bool,
    pub enrich_existing: bool,
    pub provider: String,
    pub auto_apply_threshold: f32,
    pub review_threshold: f32,
    pub download_artwork: bool,
    pub acoustid_enabled: bool,
}

impl Default for EnrichmentPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            enrich_existing: true,
            provider: "musicbrainz".to_string(),
            auto_apply_threshold: 0.90,
            review_threshold: 0.72,
            download_artwork: true,
            acoustid_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ArtworkPreferences {
    pub upgrade_below_pixels: u32,
    pub disk_cache_megabytes: u32,
    pub preferred_download_pixels: u32,
}

impl Default for ArtworkPreferences {
    fn default() -> Self {
        Self {
            upgrade_below_pixels: 600,
            disk_cache_megabytes: 2048,
            preferred_download_pixels: 1200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LibrarySourcePreferences {
    pub watch_reference_folders: bool,
    pub scan_on_startup: bool,
    pub follow_symlinks: bool,
    pub ignore_hidden: bool,
}

impl Default for LibrarySourcePreferences {
    fn default() -> Self {
        Self {
            watch_reference_folders: true,
            scan_on_startup: true,
            follow_symlinks: false,
            ignore_hidden: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodec {
    #[default]
    Copy,
    Vorbis,
    Aac,
    Mp3,
    Flac,
}

impl AudioCodec {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Vorbis => "vorbis",
            Self::Aac => "aac",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleRatePolicy {
    #[default]
    Auto,
    Hz44100,
    Hz48000,
}

impl SampleRatePolicy {
    pub fn as_transcode_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Hz44100 => "44100",
            Self::Hz48000 => "48000",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelPolicy {
    #[default]
    Auto,
    Stereo,
    Mono,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatProfile {
    pub id: String,
    pub name: String,
    pub codec: AudioCodec,
    pub vorbis_quality: f32,
    pub sample_rate: SampleRatePolicy,
    pub channels: ChannelPolicy,
    pub preserve_metadata: bool,
    pub include_artwork: bool,
}

impl Default for FormatProfile {
    fn default() -> Self {
        Self {
            id: "original".to_string(),
            name: "Original".to_string(),
            codec: AudioCodec::Copy,
            vorbis_quality: 5.0,
            sample_rate: SampleRatePolicy::Auto,
            channels: ChannelPolicy::Auto,
            preserve_metadata: true,
            include_artwork: true,
        }
    }
}

pub fn built_in_format_profiles() -> Vec<FormatProfile> {
    [
        ("original", "Original", AudioCodec::Copy, 5.0),
        ("ogg_compact", "OGG Compact", AudioCodec::Vorbis, 3.0),
        ("ogg_standard", "OGG Standard", AudioCodec::Vorbis, 5.0),
        ("ogg_high", "OGG High", AudioCodec::Vorbis, 7.0),
    ]
    .into_iter()
    .map(|(id, name, codec, vorbis_quality)| FormatProfile {
        id: id.to_string(),
        name: name.to_string(),
        codec,
        vorbis_quality,
        ..FormatProfile::default()
    })
    .collect()
}

impl Default for PlaybackPreferences {
    fn default() -> Self {
        Self {
            queue_track_ids: Vec::new(),
            queue_index: 0,
            position: 0.0,
            volume: 1.0,
            shuffle: false,
            repeat: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlCenterConfig {
    pub version: u32,
    pub target: String,
    pub library_target: String,
    pub device_target: String,
    pub selected_output: PlaybackOutput,
    pub sync_dir: PathBuf,
    pub media_dir: PathBuf,
    pub db_path: PathBuf,
    pub last_section: DesktopSection,
    pub library_view: LibraryViewMode,
    pub sort: SortDescriptor,
    pub columns: Vec<ColumnPreference>,
    pub last_route: LibraryRoute,
    pub playback: PlaybackPreferences,
    pub library_ui: LibraryUiPreferences,
    pub enrichment: EnrichmentPreferences,
    pub artwork: ArtworkPreferences,
    pub source_preferences: LibrarySourcePreferences,
    pub appearance: AppearancePreferences,
    pub format_profiles: Vec<FormatProfile>,
    pub selected_sync_profile: String,
}

impl Default for ControlCenterConfig {
    fn default() -> Self {
        let port = std::env::var("MUSICOS_COMPANION_PORT").unwrap_or_else(|_| "18080".to_string());
        let library_target = format!("http://127.0.0.1:{port}");
        let data_dir = crate::storage::desktop_data_dir();
        Self {
            version: DESKTOP_CONFIG_VERSION,
            target: std::env::var("MUSICOS_SYNC_TARGET").unwrap_or_else(|_| library_target.clone()),
            library_target: library_target.clone(),
            device_target: std::env::var("MUSICOS_SYNC_TARGET")
                .unwrap_or_else(|_| library_target.clone()),
            selected_output: PlaybackOutput::Computer,
            sync_dir: std::env::var_os("MUSICOS_SYNC_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir.join("Sync")),
            media_dir: std::env::var_os("MUSICOS_MEDIA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir.join("MusicOS Media")),
            db_path: std::env::var_os("MUSICOS_MEDIA_DB")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir.join("desktop-media.sqlite3")),
            last_section: DesktopSection::Albums,
            library_view: LibraryViewMode::Albums,
            sort: SortDescriptor::default(),
            columns: default_columns(),
            last_route: LibraryRoute::Albums,
            playback: PlaybackPreferences::default(),
            library_ui: LibraryUiPreferences::default(),
            enrichment: EnrichmentPreferences::default(),
            artwork: ArtworkPreferences::default(),
            source_preferences: LibrarySourcePreferences::default(),
            appearance: AppearancePreferences::default(),
            format_profiles: built_in_format_profiles(),
            selected_sync_profile: "ogg_standard".to_string(),
        }
    }
}

impl FormatProfile {
    pub fn to_transcode_options(&self) -> TranscodeOptions {
        let bitrate = if self.codec == AudioCodec::Vorbis {
            // Retain the old adapter contract while the native transcode path
            // uses the Vorbis quality value directly.
            match self.vorbis_quality.round() as i32 {
                i32::MIN..=3 => "112k",
                4..=5 => "160k",
                6..=7 => "224k",
                _ => "320k",
            }
        } else {
            ""
        };
        TranscodeOptions {
            codec: self.codec.as_str().to_string(),
            bitrate: bitrate.to_string(),
            sample_rate: self.sample_rate.as_transcode_value().to_string(),
            vorbis_quality: (self.codec == AudioCodec::Vorbis).then_some(self.vorbis_quality),
            channels: match self.channels {
                ChannelPolicy::Auto => None,
                ChannelPolicy::Stereo => Some(2),
                ChannelPolicy::Mono => Some(1),
            },
        }
    }
}

fn default_columns() -> Vec<ColumnPreference> {
    [
        ("number", 38.0),
        ("title", 300.0),
        ("artist", 220.0),
        ("album", 260.0),
        ("genre", 150.0),
        ("duration", 70.0),
    ]
    .into_iter()
    .map(|(field, width)| ColumnPreference {
        field: field.to_string(),
        width,
        visible: true,
    })
    .collect()
}

#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
pub struct Track {
    #[serde(default)] pub album_artist: Option<String>,
    #[serde(default)] pub release_id: Option<String>,
    #[serde(default)] pub release_group_id: Option<String>,
    #[serde(default)] pub audio_quality: Option<AudioQuality>,
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: i32,
    #[serde(default)]
    pub disc_number: i32,
    pub duration: i32,
    #[serde(default)]
    pub indexed_at: i64,
    #[serde(default)]
    pub favorited: bool,
    #[serde(default)]
    pub play_count: i32,
    #[serde(default)]
    pub rating: u8,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub year: i32,
    #[serde(default)]
    pub stable_id: String,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub cover_art_path: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub available: bool,
}

#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
pub struct AudioQuality {pub format:String,pub lossless:bool,pub sample_rate:Option<u32>,pub bit_depth:Option<u8>,pub bitrate:Option<u32>,pub channels:Option<u8>}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub track: Option<Track>,
    pub is_playing: bool,
    pub position: f32,
    pub volume: f32,
    pub shuffle: bool,
    pub loop_mode: serde_json::Value,
    pub queue_len: usize,
    pub queue_index: Option<usize>,
    pub backend: String,
    pub selected_pcm: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackSnapshot {
    pub status: String,
    pub player: Option<PlayerState>,
    #[serde(default)]
    pub queue: Vec<Track>,
    pub queue_index: Option<usize>,
    #[serde(default)]
    pub revision: u64,
    pub active_output: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub track_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mix {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub track_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibrarySource {
    pub id: i64,
    pub kind: String,
    pub path: PathBuf,
    pub display_name: String,
    pub enabled: bool,
    pub available: bool,
    pub last_scan_at: Option<i64>,
    pub last_error: Option<String>,
    #[serde(default)] pub song_count:usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportProgress {
    pub running: bool,
    pub found: usize,
    pub processed: usize,
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub cancel_requested: bool,
    pub current: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscodeOptions {
    pub codec: String,
    pub bitrate: String,
    pub sample_rate: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vorbis_quality: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncProgress {
    pub running: bool,
    pub found: usize,
    pub uploaded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancel_requested: bool,
    pub current: String,
    pub errors: Vec<String>,
    pub failed_files: Vec<PathBuf>,
    pub transcoded: usize,
    pub source_bytes: u64,
    pub output_bytes: u64,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmulatorStatus {
    pub running: bool,
    pub starting: bool,
    pub pid: Option<u32>,
    pub target: String,
    pub db_path: PathBuf,
    pub sync_dir: PathBuf,
    pub bin_path: PathBuf,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaylistManifest {
    pub stable_id: String,
    pub name: String,
    pub revision: i64,
    pub track_ids: Vec<String>,
    pub deleted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaylistConflict {
    pub key: String,
    pub name: String,
    pub field: String,
    pub desktop: serde_json::Value,
    pub device: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaylistSyncItem {
    pub key: String,
    pub base: PlaylistManifest,
    pub desktop: PlaylistManifest,
    pub device: PlaylistManifest,
    pub merged: PlaylistManifest,
    pub conflicts: Vec<PlaylistConflict>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaylistSyncPlan {
    pub generated_at: String,
    pub items: Vec<PlaylistSyncItem>,
    pub conflicts: Vec<PlaylistConflict>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaylistSyncResult {
    pub applied: Vec<String>,
    pub unresolved: Vec<PlaylistConflict>,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum DesktopCommand {
    Navigate(DesktopSection),
    SetView(LibraryViewMode),
    Refresh,
    Search(String),
    PlayPause,
    Previous,
    Next,
    StartEmulator,
    StopEmulator,
    StartSync(Vec<PathBuf>),
    CancelSync,
    CancelTask(TaskId),
    RetryTask(TaskId),
    ClearCompletedTasks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum DesktopEvent {
    LibraryLoaded(Vec<Track>),
    ImportProgress(ImportProgress),
    SyncProgress(SyncProgress),
    EmulatorChanged(EmulatorStatus),
    TaskCreated(OperationTask),
    TaskUpdated(OperationTask),
    TaskCompleted(OperationTask),
    Error(String),
    Notice(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibrarySnapshot {
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub mixes: Vec<Mix>,
    pub sources: Vec<LibrarySource>,
    pub counts: BTreeMap<String, i64>,
}

#[cfg(test)]
mod task_tests {
    use super::*;

    #[test]
    fn progress_fraction_is_bounded_and_handles_unknown_totals() {
        assert_eq!(TaskProgress::Indeterminate.fraction(), None);
        assert_eq!(
            TaskProgress::Items {
                current: 5,
                total: 10
            }
            .fraction(),
            Some(0.5)
        );
        assert_eq!(
            TaskProgress::Items {
                current: 15,
                total: 10
            }
            .fraction(),
            Some(1.0)
        );
        assert_eq!(
            TaskProgress::Bytes {
                current: 1,
                total: 0
            }
            .fraction(),
            None
        );
    }

    #[test]
    fn operation_log_is_bounded() {
        let mut task = OperationTask {
            id: TaskId(1),
            kind: TaskKind::LibraryImport,
            title: "Import".into(),
            phase: TaskPhase::Processing,
            status: TaskStatus::Running,
            progress: TaskProgress::Indeterminate,
            current_item: None,
            started_at_ms: 0,
            completed_at_ms: None,
            cancellable: true,
            retryable: false,
            summary: None,
            error: None,
            origin: Some(DesktopSection::Music),
            log: Vec::new(),
        };
        for index in 0..150 {
            task.push_log(index.to_string());
        }
        assert_eq!(task.log.len(), OperationTask::MAX_LOG_LINES);
        assert_eq!(task.log.first().map(String::as_str), Some("50"));
    }

    #[test]
    fn ogg_profiles_preserve_originals_and_map_quality_for_native_sync() {
        let profiles = built_in_format_profiles();
        assert_eq!(profiles[0].codec, AudioCodec::Copy);
        let standard = profiles
            .iter()
            .find(|profile| profile.id == "ogg_standard")
            .unwrap();
        let options = standard.to_transcode_options();
        assert_eq!(options.codec, "vorbis");
        assert_eq!(options.vorbis_quality, Some(5.0));
        assert_eq!(options.sample_rate, "auto");
    }
}
