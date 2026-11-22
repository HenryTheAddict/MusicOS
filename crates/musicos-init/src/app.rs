use crate::audio::{AudioPlayer, EqPreset, LoopMode};
use crate::database::{Db, LibraryCounts, Pin, Playlist, Track};
use crate::input::{Button, InputEvent, SwipeDirection};
use crate::preferences::{LibraryGroup, LibrarySort, MotionMode, Preferences};
use crate::renderer::{Canvas, Color, Resolution};
use crate::server::{RuntimeSnapshot, ScanInfo, ServerEvent, SettingsUpdate};
use crate::ui;
use crate::ui::library::{Grid, LibraryView, COLUMNS};
use crate::ui::localization::Language;
use crate::ui::theme::ThemeName;
use crate::usb::UsbStatus;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum LibraryScanPolicy {
    EmbeddedDefaults,
    Explicit(crate::media::ScanRequest),
    ReadOnlyPreview,
}

#[derive(Debug, Clone)]
pub struct MusicOsOptions {
    pub db_path: PathBuf,
    pub scan_policy: LibraryScanPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    First,
    Welcome,
    Theme,
    DateTimeSetup,
    Home,
    MusicApp,
    Language,
    TestComplete,
    NowPlaying,
    SongsList,
    AlbumsList,
    ArtistsList,
    ArtistAlbumsList,
    AlbumTracksList,
    PlaylistsList,
    PlaylistTracksList,
    Queue,
    Search,
    RatingPicker,
    ContextMenu,
    Settings,
    SettingsDetail,
    Apps,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    GetStarted,
    Language,
    Yeah,
    Custom,
    ThemeSelect,
    HomeMusic,
    HomeApps,
    HomeSettings,
    HomeNowPlaying,
    Albums,
    Songs,
    Artists,
    Playlists,
    History,
    Favorites,
    Queue,
    ShuffleAll,
    Search,
    SearchKeyboard,
    SearchResults,
    ContextPlay,
    ContextPlayNext,
    ContextAddLater,
    ContextPin,
    ContextAddToPlaylist,
    ContextShuffle,
    ContextDelete,
    ContextRate,
    ContextMoveUp,
    ContextMoveDown,
    ContextClearUpcoming,
    ContextCancel,
    RatingStars,
    SettingsGeneral,
    SettingsAudio,
    SettingsLooks,
    SettingsTheme,
    SettingsLanguage,
    SettingsDateTime,
    SettingsBrightness,
    SettingsVolume,
    SettingsEq,
    SettingsSwitchObject,
    SettingsAbout,
    SettingsConnectivity,
    SettingsGetUpdate,
    SettingsUser,
    DateHour,
    DateMinute,
    DateMonth,
    DateDay,
    DateYear,
    DateDone,
    AppsMusic,
    AppsNowPlaying,
    AppsSettings,
    AppsUsbSync,
    HomePlaylist1,
    HomePlaylist2,
    HomePlaylist3,
    HomePlaylist4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomePane {
    Main,
    Detail,
}

const HOME_ROWS: [Focus; 8] = [
    Focus::HomeNowPlaying,
    Focus::HomeMusic,
    Focus::Artists,
    Focus::Songs,
    Focus::Playlists,
    Focus::Search,
    Focus::HomeSettings,
    Focus::HomeApps,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeNavigation {
    pub pane: HomePane,
    pub main_index: usize,
    pub detail_index: usize,
    pub remembered_detail_index: [usize; HOME_ROWS.len()],
}

impl Default for HomeNavigation {
    fn default() -> Self {
        Self {
            pane: HomePane::Main,
            main_index: 1,
            detail_index: 0,
            remembered_detail_index: [0; HOME_ROWS.len()],
        }
    }
}

#[derive(Clone)]
struct NavigationState {
    screen: Screen,
    focus: Focus,
    selected_index: usize,
    scroll_offset: f32,
    target_scroll_offset: f32,
    tracks_queue: Vec<Track>,
    albums_list: Vec<String>,
    artists_list: Vec<String>,
    artist_albums_list: Vec<String>,
    playlists_list: Vec<Playlist>,
    active_playlist: Option<Playlist>,
    active_artist: Option<String>,
    active_album: Option<String>,
    search_query: String,
    search_results: Vec<Track>,
    search_key_index: usize,
}

pub struct MusicOs {
    controls_seen: bool,
    pub screen: Screen,
    pub focus: Focus,
    pub previous_screen: Screen,
    pub previous_focus: Focus,
    pub active_theme: ThemeName,
    pub active_language: Language,
    pub user_name: String,
    pub device_name: String,
    pub audio_backend: String,
    pub alsa_device: String,
    pub audio_output_mode: String,
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
    pub scan_status: String,
    pub scan: ScanInfo,
    pub audio_test_status: String,
    pub db_path: String,
    pub scan_policy: LibraryScanPolicy,
    pub clock_hour: u8,
    pub clock_minute: u8,
    pub clock_month: u8,
    pub clock_day: u8,
    pub clock_year: u16,
    pub time_configured: bool,
    pub brightness: u8,
    pub clock_anchor: Instant,
    pub db: Db,
    pub player: AudioPlayer,
    pub selected_index: usize,
    pub home_navigation: HomeNavigation,
    pub tracks_queue: Vec<Track>,
    library_tracks: Vec<Track>,
    album_previews: Vec<ui::home::AlbumPreview>,
    recent_albums: usize,
    home_album_mosaic: Option<crate::renderer::CoverArt>,
    home_artist_portrait: Option<crate::renderer::CoverArt>,
    library_view: LibraryView,
    cached_library_counts: LibraryCounts,
    pub albums_list: Vec<String>,
    pub artists_list: Vec<String>,
    pub artist_albums_list: Vec<String>,
    pub playlists_list: Vec<Playlist>,
    pub pins_list: Vec<Pin>,
    pub active_playlist: Option<Playlist>,
    pub active_artist: Option<String>,
    pub active_album: Option<String>,
    pub search_query: String,
    pub search_results: Vec<Track>,
    pub search_key_index: usize,
    pub pending_rating: u8,
    rating_track_id: Option<i64>,
    navigation_stack: Vec<NavigationState>,
    navigation_restored: bool,
    pub scroll_offset: f32,
    pub target_scroll_offset: f32,
    pub current_cover_art: Option<crate::renderer::CoverArt>,
    previous_cover_art: Option<crate::renderer::CoverArt>,
    pub current_track_id: Option<i64>,
    pub cover_art_rx: Option<std::sync::mpsc::Receiver<Option<crate::renderer::CoverArt>>>,
    pub usb_status: UsbStatus,
    pub resolution: Resolution,
    pub last_draw_seconds: f32,
    pub last_frame_ms: f32,
    pub last_drawn_screen: Option<Screen>,
    pub outline_x: f32,
    pub outline_y: f32,
    pub outline_w: f32,
    pub outline_h: f32,
    pub outline_r: f32,
    pub outline_thick: f32,
    pub outline_active: bool,
    pub outline_animating: bool,
    pub screen_transition: f32,
    transition_direction: f32,
    button_feedback: Option<(&'static str, f32)>,
    edge_feedback: Option<(f32, f32)>,
    progress_sample: Option<(i64, f32)>,
    pub theme_colors_initialized: bool,
    pub cur_top_r: f32,
    pub cur_top_g: f32,
    pub cur_top_b: f32,
    pub cur_bottom_r: f32,
    pub cur_bottom_g: f32,
    pub cur_bottom_b: f32,
    pub hud_timer: f32,
    pub hud_t: f32,
    pub hud_type: HudType,
    pub home_held: bool,
    pub home_chorded: bool,
    pub cover_art_opacity: f32,
    pub is_asleep: bool,
    pub last_session_save: Instant,
    last_runtime_snapshot: Instant,
    pub last_interaction: Instant,
    pub display_dimmed: bool,
    pub wifi_sync_overlay: bool,
    pub wifi_sync_ip: String,
    pub wifi_sync_timeout: Option<Instant>,
    pub sleep_timer_deadline: Option<Instant>,
    scan_rx: Option<std::sync::mpsc::Receiver<ScanMessage>>,
    scan_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudType {
    Volume,
    Brightness,
}

#[derive(Debug)]
struct ScanResult {
    indexed: Result<usize, String>,
    elapsed_ms: u128,
}

#[derive(Debug)]
enum ScanMessage {
    Progress {
        indexed: usize,
        processed: usize,
        discovered: usize,
        current_root: Option<String>,
        elapsed_ms: u128,
    },
    Finished(ScanResult),
}

fn apply_audio_runtime_preferences(preferences: &Preferences) {
    match preferences.audio_backend.as_str() {
        "auto" => std::env::remove_var("MUSICOS_AUDIO_BACKEND"),
        backend => std::env::set_var("MUSICOS_AUDIO_BACKEND", backend),
    }
    if preferences.alsa_device.trim().is_empty() {
        std::env::remove_var("MUSICOS_ALSA_DEVICE");
    } else {
        std::env::set_var("MUSICOS_ALSA_DEVICE", preferences.alsa_device.trim());
    }
}

fn restore_queue_tracks(db: &Db, preferences: &Preferences) -> Vec<Track> {
    if !preferences.resume_queue_enabled {
        return Vec::new();
    }
    preferences
        .resume_queue_track_ids
        .iter()
        .filter_map(|id| db.get_track_by_id(*id).ok().flatten())
        .collect()
}

fn eq_preset_from_str(value: &str) -> EqPreset {
    match value.to_ascii_lowercase().as_str() {
        "bass" => EqPreset::Bass,
        "vocal" => EqPreset::Vocal,
        "treble" => EqPreset::Treble,
        "night" => EqPreset::Night,
        _ => EqPreset::Flat,
    }
}

fn loop_mode_from_str(value: &str) -> LoopMode {
    match value.to_ascii_lowercase().as_str() {
        "all" => LoopMode::All,
        "one" => LoopMode::One,
        _ => LoopMode::None,
    }
}

fn sort_tracks(tracks: &mut [Track], sort: LibrarySort) {
    match sort {
        LibrarySort::Title => tracks.sort_by_key(|track| track.title.to_ascii_lowercase()),
        LibrarySort::Artist => tracks.sort_by_key(|track| {
            (
                track.artist.to_ascii_lowercase(),
                track.album.to_ascii_lowercase(),
                track.track_number,
                track.title.to_ascii_lowercase(),
            )
        }),
        LibrarySort::Album => tracks.sort_by_key(|track| {
            (
                track.album.to_ascii_lowercase(),
                track.track_number,
                track.title.to_ascii_lowercase(),
            )
        }),
        LibrarySort::RecentlyAdded => tracks.sort_by_key(|track| {
            (
                std::cmp::Reverse(track.indexed_at),
                track.title.to_ascii_lowercase(),
            )
        }),
        LibrarySort::PlayCount => tracks.sort_by_key(|track| {
            (
                std::cmp::Reverse(track.play_count),
                track.title.to_ascii_lowercase(),
            )
        }),
    }
}

fn jump_index_by_letter(labels: &[&str], selected_index: usize, direction: i32) -> usize {
    if labels.is_empty() {
        return 0;
    }
    let selected_index = selected_index.min(labels.len() - 1);
    let current = first_label_letter(labels[selected_index]);
    if direction >= 0 {
        for (index, label) in labels.iter().enumerate().skip(selected_index + 1) {
            if first_label_letter(label) != current {
                return index;
            }
        }
        (selected_index + 8).min(labels.len() - 1)
    } else {
        for index in (0..selected_index).rev() {
            if first_label_letter(labels[index]) != current {
                let letter = first_label_letter(labels[index]);
                let mut start = index;
                while start > 0 && first_label_letter(labels[start - 1]) == letter {
                    start -= 1;
                }
                return start;
            }
        }
        selected_index.saturating_sub(8)
    }
}

fn first_label_letter(label: &str) -> char {
    label
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .unwrap_or('#')
}

const SEARCH_KEYS: [&str; 30] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "SPACE", "DELETE", "CLEAR", "RESULTS",
];

impl MusicOs {
    pub fn new(resolution: Resolution) -> Self {
        let db_path = std::env::var("MUSICOS_MEDIA_DB")
            .unwrap_or_else(|_| "/var/lib/musicos/media.sqlite3".to_string());
        Self::with_db_path(resolution, &db_path)
    }

    pub fn with_db_path(resolution: Resolution, db_path: &str) -> Self {
        Self::with_options(
            resolution,
            MusicOsOptions {
                db_path: PathBuf::from(db_path),
                scan_policy: LibraryScanPolicy::EmbeddedDefaults,
            },
        )
    }

    pub fn with_options(resolution: Resolution, options: MusicOsOptions) -> Self {
        let db_path = options.db_path.display().to_string();
        let db = Db::open(&db_path).unwrap_or_else(|err| {
            log::error!("failed to open sqlite DB at {db_path}: {err}");
            log::warn!("library persistence unavailable; using a temporary in-memory database");
            Db::open(":memory:").unwrap_or_else(|error| {
                log::error!("cannot initialize temporary library: {error:#}");
                std::process::exit(1)
            })
        });

        let preferences = crate::preferences::load();
        apply_audio_runtime_preferences(&preferences);
        let mut player = AudioPlayer::new();
        player.set_volume(preferences.volume);
        player.set_eq_preset(preferences.eq_preset);
        player.set_shuffle(preferences.shuffle);
        player.set_loop_mode(preferences.loop_mode);
        let restore_tracks = restore_queue_tracks(&db, &preferences);
        if !restore_tracks.is_empty() {
            let index = preferences
                .resume_queue_index
                .min(restore_tracks.len().saturating_sub(1));
            player.restore_queue_paused(restore_tracks, index, preferences.resume_position_seconds);
        }
        crate::system::apply_brightness(preferences.brightness);

        let mut app = Self {
            controls_seen: preferences.controls_seen,
            screen: if !preferences.setup_complete {
                Screen::First
            } else if !preferences.time_configured {
                Screen::DateTimeSetup
            } else {
                Screen::Home
            },
            focus: if !preferences.setup_complete {
                Focus::GetStarted
            } else if !preferences.time_configured {
                Focus::DateHour
            } else {
                Focus::HomeMusic
            },
            previous_screen: Screen::First,
            previous_focus: Focus::GetStarted,
            active_theme: preferences.theme,
            active_language: preferences.language,
            user_name: preferences.user_name,
            device_name: preferences.device_name,
            audio_backend: preferences.audio_backend,
            alsa_device: preferences.alsa_device,
            audio_output_mode: preferences.audio_output_mode,
            library_sort: preferences.library_sort,
            library_group: preferences.library_group,
            sleep_timeout_minutes: preferences.sleep_timeout_minutes,
            dim_timeout_seconds: preferences.dim_timeout_seconds,
            motion_mode: preferences.motion_mode,
            seek_step_seconds: preferences.seek_step_seconds,
            sleep_timer_default_minutes: preferences.sleep_timer_default_minutes,
            resume_queue_enabled: preferences.resume_queue_enabled,
            shuffle_default_enabled: preferences.shuffle_default_enabled,
            first_run_audio_test_complete: preferences.first_run_audio_test_complete,
            scan_status: "Library ready".to_string(),
            scan: ScanInfo::default(),
            audio_test_status: "Audio test not run".to_string(),
            db_path,
            scan_policy: options.scan_policy,
            clock_hour: preferences.clock_hour.min(23),
            clock_minute: preferences.clock_minute.min(59),
            clock_month: preferences.clock_month.clamp(1, 12),
            clock_day: preferences.clock_day.clamp(1, 31),
            clock_year: preferences.clock_year.clamp(2024, 2099),
            time_configured: preferences.time_configured,
            brightness: preferences.brightness.clamp(5, 100),
            clock_anchor: Instant::now(),
            db,
            player,
            selected_index: 0,
            home_navigation: HomeNavigation::default(),
            tracks_queue: Vec::new(),
            library_tracks: Vec::new(),
            album_previews: Vec::new(),
            recent_albums: 0,
            home_album_mosaic: None,
            home_artist_portrait: None,
            library_view: LibraryView::default(),
            cached_library_counts: LibraryCounts::default(),
            albums_list: Vec::new(),
            artists_list: Vec::new(),
            artist_albums_list: Vec::new(),
            playlists_list: Vec::new(),
            pins_list: Vec::new(),
            active_playlist: None,
            active_artist: None,
            active_album: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_key_index: 0,
            pending_rating: 0,
            rating_track_id: None,
            navigation_stack: Vec::new(),
            navigation_restored: false,
            scroll_offset: 0.0,
            target_scroll_offset: 0.0,
            current_cover_art: None,
            previous_cover_art: None,
            current_track_id: None,
            cover_art_rx: None,
            usb_status: UsbStatus::default(),
            resolution,
            last_draw_seconds: 0.0,
            last_frame_ms: 0.0,
            last_drawn_screen: None,
            outline_x: 0.0,
            outline_y: 0.0,
            outline_w: 0.0,
            outline_h: 0.0,
            outline_r: 0.0,
            outline_thick: 0.0,
            outline_active: false,
            outline_animating: false,
            screen_transition: 1.0,
            transition_direction: 1.0,
            button_feedback: None,
            edge_feedback: None,
            progress_sample: None,
            theme_colors_initialized: false,
            cur_top_r: 0.0,
            cur_top_g: 0.0,
            cur_top_b: 0.0,
            cur_bottom_r: 0.0,
            cur_bottom_g: 0.0,
            cur_bottom_b: 0.0,
            hud_timer: 0.0,
            hud_t: 0.0,
            hud_type: HudType::Volume,
            home_held: false,
            home_chorded: false,
            cover_art_opacity: 0.0,
            is_asleep: false,
            last_session_save: Instant::now(),
            last_runtime_snapshot: Instant::now() - Duration::from_secs(1),
            last_interaction: Instant::now(),
            display_dimmed: false,
            wifi_sync_overlay: false,
            wifi_sync_ip: String::new(),
            wifi_sync_timeout: None,
            sleep_timer_deadline: None,
            scan_rx: None,
            scan_pending: false,
        };
        app.refresh_library();
        app
    }

    pub fn next_tick_deadline(&self, now: Instant) -> Instant {
        let mut deadline = self.last_runtime_snapshot + Duration::from_millis(250);
        if self.animation_in_progress() || self.hud_timer > 0.0 {
            deadline = deadline.min(now + Duration::from_millis(2));
        }
        if self.scan_rx.is_some() || self.cover_art_rx.is_some() {
            deadline = deadline.min(now + Duration::from_millis(20));
        }
        if self.current_track_id.is_some() {
            deadline = deadline.min(self.last_session_save + Duration::from_secs(5));
        }
        if !self.is_asleep {
            if !self.display_dimmed && self.dim_timeout_seconds > 0 {
                deadline = deadline.min(
                    self.last_interaction + Duration::from_secs(self.dim_timeout_seconds as u64),
                );
            }
            if self.sleep_timeout_minutes > 0 && !self.player.is_playing() {
                deadline = deadline.min(
                    self.last_interaction
                        + Duration::from_secs(self.sleep_timeout_minutes as u64 * 60),
                );
            }
        }
        if let Some(sleep) = self.sleep_timer_deadline {
            deadline = deadline.min(sleep);
        }
        deadline
    }

    pub fn tick(&mut self) {
        self.player.tick();
        self.poll_library_scan();

        let status = self.player.get_status();
        let current_id = status.track.as_ref().map(|t| t.id);
        if current_id != self.current_track_id {
            self.current_track_id = current_id;
            if current_id.is_none() || self.motion_mode == MotionMode::Reduced {
                self.current_cover_art = None;
                self.previous_cover_art = None;
            } else if let Some(art) = self.current_cover_art.take() {
                self.previous_cover_art = Some(art);
            }
            self.cover_art_opacity = 0.0;

            if let Some(ref track) = status.track {
                let track = track.clone();
                let db_path = self.db_path.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.cover_art_rx = Some(rx);
                std::thread::spawn(move || {
                    let art = crate::database::Db::open(&db_path)
                        .ok()
                        .and_then(|db| crate::media::cached_cover_art_for_track(&db, &track));
                    let _ = tx.send(art);
                });
            } else {
                self.cover_art_rx = None;
            }
        }

        if let Some(ref rx) = self.cover_art_rx {
            if let Ok(art_opt) = rx.try_recv() {
                self.current_cover_art = art_opt;
                self.cover_art_rx = None;
                self.cover_art_opacity = 0.0;
            }
        }

        if self.current_cover_art.is_none() && self.cover_art_rx.is_some() {
            self.cover_art_opacity = 0.0;
        }

        if self.current_list_len() > 0 {
            let (item_h, row_h, content_top, content_bottom, list_origin) = match self.screen {
                Screen::Language => (144.0, 124.0, 92.0, 700.0, 112.0),
                Screen::PlaylistsList
                | Screen::PlaylistTracksList
                | Screen::Queue
                | Screen::Search => (
                    ui::music::COLLECTION_ROW_STEP,
                    ui::music::COLLECTION_ROW_HEIGHT,
                    204.0,
                    864.0,
                    220.0,
                ),
                _ => (144.0, 128.0, 252.0, 862.0, 260.0),
            };
            let sel = self.selected_index as f32;
            let selected_y = list_origin + sel * item_h - self.target_scroll_offset;

            if selected_y < content_top {
                self.target_scroll_offset = sel * item_h - (content_top - list_origin);
            } else if selected_y + row_h > content_bottom {
                self.target_scroll_offset = sel * item_h - (content_bottom - row_h - list_origin);
            }

            let max_scroll = ((self.current_list_len().saturating_sub(1)) as f32 * item_h).max(0.0);
            self.target_scroll_offset = self.target_scroll_offset.clamp(0.0, max_scroll);
        }

        if (self.target_scroll_offset - self.scroll_offset).abs() > 0.1 {
            let delta = self.target_scroll_offset - self.scroll_offset;
            let factor = if delta.abs() > 360.0 { 0.64 } else { 0.42 };
            self.scroll_offset += delta * factor;
        } else {
            self.scroll_offset = self.target_scroll_offset;
        }

        let dt = 1.0 / 60.0;
        if self.hud_timer > 0.0 {
            self.hud_timer = (self.hud_timer - dt).max(0.0);
            self.hud_t = (self.hud_t + dt * 6.0).min(1.0);
        } else {
            self.hud_t = (self.hud_t - dt * 4.0).max(0.0);
        }

        if current_id.is_some() && self.last_session_save.elapsed() >= Duration::from_secs(5) {
            self.save_preferences(false);
            self.last_session_save = Instant::now();
        }
        if !self.is_asleep {
            let idle = self.last_interaction.elapsed();
            if !self.display_dimmed
                && self.dim_timeout_seconds > 0
                && idle >= Duration::from_secs(self.dim_timeout_seconds as u64)
            {
                let dimmed = (self.brightness / 3).max(5);
                crate::system::apply_brightness(dimmed);
                self.display_dimmed = true;
            }
            if self.sleep_timeout_minutes > 0
                && !self.player.is_playing()
                && idle >= Duration::from_secs(self.sleep_timeout_minutes as u64 * 60)
            {
                self.enter_sleep();
            }
        }
        if let Some(deadline) = self.sleep_timer_deadline {
            if Instant::now() >= deadline {
                self.sleep_timer_deadline = None;
                self.scan_status = "Sleep timer expired".to_string();
                self.enter_sleep();
            }
        }
        // API snapshots perform database and system queries, so keep them well below the
        // render loop frequency on low-power devices.
        if self.last_runtime_snapshot.elapsed() >= Duration::from_millis(250) {
            self.publish_runtime_snapshot();
            self.last_runtime_snapshot = Instant::now();
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) -> bool {
        if self.is_asleep {
            if let InputEvent::Button(_) = event {
                self.is_asleep = false;
                self.display_dimmed = false;
                self.last_interaction = Instant::now();
                self.home_held = false;
                self.home_chorded = true;
                crate::system::apply_brightness(self.brightness);
                return true;
            }
            return false;
        }
        self.last_interaction = Instant::now();
        if self.display_dimmed {
            self.display_dimmed = false;
            crate::system::apply_brightness(self.brightness);
        }

        if self.wifi_sync_overlay {
            self.wifi_sync_overlay = false;
            return true;
        }

        let old_screen = self.screen;
        let old_focus = self.focus;
        let old_idx = self.selected_index;
        let mut feedback = false;
        if self.screen == Screen::Home {
            self.sync_home_navigation_from_focus();
        }

        match event {
            InputEvent::WifiSync => {
                log::info!("WifiSync event triggered!");
                self.wifi_sync_ip = get_local_ip();
                self.wifi_sync_overlay = true;
                self.wifi_sync_timeout = Some(Instant::now() + Duration::from_secs(8));
                return true;
            }
            InputEvent::Button(Button::Confirm) | InputEvent::Tap => self.activate(),
            InputEvent::Button(Button::Back) => self.back(),
            InputEvent::Button(Button::Home) => {
                self.home_held = true;
                self.home_chorded = false;
            }
            InputEvent::ButtonRelease(Button::Home) => {
                self.home_held = false;
                if !self.home_chorded {
                    self.navigation_stack.clear();
                    self.screen = Screen::Home;
                    self.home_navigation = HomeNavigation::default();
                    self.focus = Focus::HomeMusic;
                }
                self.home_chorded = false;
            }
            InputEvent::Button(Button::Menu) => {
                if self.screen == Screen::Apps {
                    self.screen = self.previous_screen;
                    self.focus = self.previous_focus;
                } else {
                    self.previous_screen = self.screen;
                    self.previous_focus = self.focus;
                    self.screen = Screen::Apps;
                    self.focus = match old_screen {
                        Screen::MusicApp
                        | Screen::SongsList
                        | Screen::AlbumsList
                        | Screen::ArtistsList
                        | Screen::ArtistAlbumsList
                        | Screen::AlbumTracksList
                        | Screen::PlaylistsList
                        | Screen::PlaylistTracksList
                        | Screen::Queue => Focus::AppsMusic,
                        Screen::NowPlaying => Focus::AppsNowPlaying,
                        Screen::Settings | Screen::SettingsDetail => Focus::AppsSettings,
                        _ => Focus::AppsMusic,
                    };
                }
            }
            InputEvent::Button(Button::Sleep) => {
                self.enter_sleep();
            }
            InputEvent::Button(Button::PlayPause) => {
                self.player.play_pause();
                self.save_preferences(false);
            }
            InputEvent::Button(Button::PreviousTrack) => {
                self.player.prev();
                self.save_preferences(false);
            }
            InputEvent::Button(Button::NextTrack) => {
                self.player.next();
                self.save_preferences(false);
            }
            InputEvent::Button(Button::VolumeUp) => {
                if self.home_held {
                    self.adjust_brightness(5);
                    self.home_chorded = true;
                } else {
                    self.adjust_volume(0.05);
                }
            }
            InputEvent::Button(Button::VolumeDown) => {
                if self.home_held {
                    self.adjust_brightness(-5);
                    self.home_chorded = true;
                } else {
                    self.adjust_volume(-0.05);
                }
            }
            InputEvent::Button(Button::Context) => {
                if self.can_open_home_preview_with_context() {
                    self.enter_home_detail();
                } else if self.can_open_search_results_with_context() {
                    self.focus = Focus::SearchResults;
                    self.selected_index = 0;
                    self.scroll_offset = 0.0;
                    self.target_scroll_offset = 0.0;
                } else if self.screen != Screen::ContextMenu && self.can_open_context_menu() {
                    self.previous_screen = self.screen;
                    self.previous_focus = self.focus;
                    self.screen = Screen::ContextMenu;
                    self.focus = Focus::ContextPlay;
                }
            }
            InputEvent::Button(Button::Down) | InputEvent::Swipe(SwipeDirection::Down) => {
                self.move_down();
            }
            InputEvent::Button(Button::Up) | InputEvent::Swipe(SwipeDirection::Up) => {
                self.move_up();
            }
            InputEvent::Button(Button::Right) | InputEvent::Swipe(SwipeDirection::Right) => {
                self.move_right();
            }
            InputEvent::Button(Button::Left) | InputEvent::Swipe(SwipeDirection::Left) => {
                self.move_left();
            }
            InputEvent::AnyButton => {
                if self.screen == Screen::TestComplete {
                    self.screen = Screen::Welcome;
                }
            }
            InputEvent::Power => self.enter_sleep(),
            InputEvent::ButtonRelease(_) => {}
        }
        let moved =
            self.screen != old_screen || self.focus != old_focus || self.selected_index != old_idx;
        if self.screen != old_screen
            && self.screen != Screen::ContextMenu
            && old_screen != Screen::ContextMenu
        {
            if !self.navigation_restored {
                self.target_scroll_offset = 0.0;
                self.scroll_offset = 0.0;
            }
            self.screen_transition = if self.motion_mode == MotionMode::Reduced {
                1.0
            } else {
                0.0
            };
            self.transition_direction = if event == InputEvent::Button(Button::Back) {
                -1.0
            } else {
                1.0
            };
        }
        if self.motion_mode != MotionMode::Reduced {
            let key = match event {
                InputEvent::Button(Button::Confirm) | InputEvent::Tap => Some("A"),
                InputEvent::Button(Button::Back) if moved => Some("B"),
                InputEvent::Button(Button::Context) if moved => Some("X"),
                _ => None,
            };
            if let Some(key) = key {
                self.button_feedback = Some((key, 0.0));
                feedback = true;
            }
            let list_len = self.current_list_len();
            if !moved && list_len > 0 {
                let columns = if matches!(
                    self.screen,
                    Screen::AlbumsList | Screen::ArtistsList | Screen::ArtistAlbumsList
                ) {
                    COLUMNS
                } else {
                    1
                };
                let direction = match event {
                    InputEvent::Button(Button::Up) | InputEvent::Swipe(SwipeDirection::Up)
                        if self.selected_index < columns =>
                    {
                        Some(-1.0)
                    }
                    InputEvent::Button(Button::Down) | InputEvent::Swipe(SwipeDirection::Down)
                        if self.selected_index / columns == (list_len - 1) / columns =>
                    {
                        Some(1.0)
                    }
                    _ => None,
                };
                if let Some(direction) = direction {
                    self.edge_feedback = Some((direction, 0.0));
                    feedback = true;
                }
            } else if moved {
                self.edge_feedback = None;
            }
        }
        self.navigation_restored = false;

        moved || feedback
    }

    pub fn set_last_frame_ms(&mut self, frame_ms: f32) {
        self.last_frame_ms = frame_ms.max(0.0);
    }

    pub fn start_library_scan(&mut self) {
        self.start_library_scan_with_status("Scanning library...", false, None);
    }

    pub fn start_library_rescan(&mut self, mode: &str, source_id: Option<i64>) {
        let reindex_all = matches!(mode, "full" | "all" | "rebuild");
        let status = if source_id.is_some() {
            "Refreshing library source..."
        } else if reindex_all {
            "Full library rescan..."
        } else {
            "Scanning changed library files..."
        };
        self.start_library_scan_with_status(status, reindex_all, source_id);
    }

    fn start_library_scan_with_status(
        &mut self,
        status: &str,
        reindex_all: bool,
        source_id: Option<i64>,
    ) {
        if matches!(self.scan_policy, LibraryScanPolicy::ReadOnlyPreview) {
            self.scan_status =
                "Device Preview is read-only; refresh the library from the desktop app".to_string();
            self.scan.state = "idle".to_string();
            self.scan.last_error = None;
            crate::server::push_runtime_log(
                "scan",
                "ignored scan request in read-only Device Preview".to_string(),
            );
            return;
        }
        if self.scan_rx.is_some() {
            self.scan_pending = true;
            self.scan_status = "Finishing current scan, then checking new files...".to_string();
            self.scan.state = "running".to_string();
            return;
        }

        self.scan_status = status.to_string();
        self.scan = ScanInfo {
            state: "running".to_string(),
            indexed: 0,
            processed: 0,
            discovered: 0,
            current_root: None,
            elapsed_ms: 0,
            last_error: None,
        };
        crate::server::push_runtime_log("scan", status.to_string());
        let db_path = self.db_path.clone();
        let scan_policy = self.scan_policy.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.scan_rx = Some(rx);

        std::thread::spawn(move || {
            let started = Instant::now();
            let indexed = crate::database::Db::open(&db_path)
                .and_then(|db| {
                    if let Some(source_id) = source_id {
                        crate::media::index_library_source_with_progress(
                            &db,
                            source_id,
                            reindex_all,
                            |progress| {
                                let _ = tx.send(ScanMessage::Progress {
                                    indexed: progress.indexed,
                                    processed: progress.processed,
                                    discovered: progress.discovered,
                                    current_root: progress.current_root,
                                    elapsed_ms: started.elapsed().as_millis(),
                                });
                            },
                        )
                    } else {
                        match scan_policy {
                            LibraryScanPolicy::EmbeddedDefaults if reindex_all => {
                                crate::media::reindex_default_locations_with_progress(
                                    &db,
                                    |progress| {
                                        let _ = tx.send(ScanMessage::Progress {
                                            indexed: progress.indexed,
                                            processed: progress.processed,
                                            discovered: progress.discovered,
                                            current_root: progress.current_root,
                                            elapsed_ms: started.elapsed().as_millis(),
                                        });
                                    },
                                )
                            }
                            LibraryScanPolicy::EmbeddedDefaults => {
                                crate::media::index_default_locations_with_progress(
                                    &db,
                                    |progress| {
                                        let _ = tx.send(ScanMessage::Progress {
                                            indexed: progress.indexed,
                                            processed: progress.processed,
                                            discovered: progress.discovered,
                                            current_root: progress.current_root,
                                            elapsed_ms: started.elapsed().as_millis(),
                                        });
                                    },
                                )
                            }
                            LibraryScanPolicy::Explicit(mut request) => {
                                request.mode = if reindex_all {
                                    crate::media::ScanMode::Full
                                } else {
                                    crate::media::ScanMode::Changed
                                };
                                let cancel = std::sync::atomic::AtomicBool::new(false);
                                crate::media::scan_library_with_progress(
                                    std::path::Path::new(&db_path),
                                    &request,
                                    &cancel,
                                    |progress| {
                                        let _ = tx.send(ScanMessage::Progress {
                                            indexed: progress.indexed,
                                            processed: progress.processed,
                                            discovered: progress.discovered,
                                            current_root: progress.current_root,
                                            elapsed_ms: started.elapsed().as_millis(),
                                        });
                                    },
                                )
                                .map(|result| result.indexed)
                            }
                            LibraryScanPolicy::ReadOnlyPreview => Ok(0),
                        }
                    }
                })
                .map_err(|err| format!("{err:#}"));
            let _ = tx.send(ScanMessage::Finished(ScanResult {
                indexed,
                elapsed_ms: started.elapsed().as_millis(),
            }));
        });
    }

    fn poll_library_scan(&mut self) {
        let Some(rx) = self.scan_rx.take() else {
            return;
        };

        let result = loop {
            match rx.try_recv() {
                Ok(ScanMessage::Progress {
                    indexed,
                    processed,
                    discovered,
                    current_root,
                    elapsed_ms,
                }) => {
                    self.scan.state = "running".to_string();
                    self.scan.indexed = indexed;
                    self.scan.processed = processed;
                    self.scan.discovered = discovered;
                    self.scan.current_root = current_root.clone();
                    self.scan.elapsed_ms = elapsed_ms;
                    self.scan.last_error = None;
                    self.scan_status = if let Some(root) = current_root {
                        format!("Checking {root}: {processed}/{discovered} ({indexed} updated)")
                    } else {
                        format!("Checking library: {processed}/{discovered} ({indexed} updated)")
                    };
                }
                Ok(ScanMessage::Finished(result)) => break result,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.scan_rx = Some(rx);
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.scan_status = "Scan failed: worker stopped".to_string();
                    self.scan.state = "error".to_string();
                    self.scan.last_error = Some("worker stopped".to_string());
                    crate::server::push_runtime_log("scan", "Scan failed: worker stopped");
                    return;
                }
            }
        };

        match result.indexed {
            Ok(indexed) => {
                self.refresh_library();
                let counts = self.library_counts();
                self.scan.state = "complete".to_string();
                self.scan.indexed = indexed;
                self.scan.elapsed_ms = result.elapsed_ms;
                self.scan.current_root = None;
                self.scan.last_error = None;
                self.scan_status = format!(
                    "Scan complete: {indexed} indexed, {} songs, {} albums, {} playlists in {:.1}s",
                    counts.tracks,
                    counts.albums,
                    counts.playlists,
                    result.elapsed_ms as f32 / 1000.0
                );
                crate::server::push_runtime_log("scan", self.scan_status.clone());
            }
            Err(err) => {
                self.scan_status = format!("Scan failed: {err}");
                self.scan.state = "error".to_string();
                self.scan.elapsed_ms = result.elapsed_ms;
                self.scan.current_root = None;
                self.scan.last_error = Some(err);
                crate::server::push_runtime_log("scan", self.scan_status.clone());
            }
        }
        if self.scan_pending {
            self.scan_pending = false;
            self.start_library_scan_with_status("Checking newly synced files...", false, None);
        }
    }

    pub fn handle_server_event(&mut self, event: ServerEvent) -> bool {
        match event {
            ServerEvent::Button(b) => self.handle_input(InputEvent::Button(b)),
            ServerEvent::Pointer(ev) => self.handle_input(ev),
            ServerEvent::UploadComplete => {
                self.start_library_scan_with_status(
                    "Upload complete; scanning library...",
                    false,
                    None,
                );
                true
            }
            ServerEvent::UsbSync => {
                self.run_usb_sync();
                true
            }
            ServerEvent::ShowControls => false,
            ServerEvent::Control(cmd) => {
                match cmd {
                    crate::server::ControlCommand::PlayPause => self.player.play_pause(),
                    crate::server::ControlCommand::Next => self.player.next(),
                    crate::server::ControlCommand::Prev => self.player.prev(),
                    crate::server::ControlCommand::Volume { value } => {
                        self.player.set_volume(value);
                        self.save_preferences(false);
                        self.hud_type = HudType::Volume;
                        self.hud_timer = 2.0;
                    }
                    crate::server::ControlCommand::Brightness { value } => {
                        self.set_brightness(value);
                    }
                    crate::server::ControlCommand::Seek { value } => {
                        self.player.seek(value);
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::SeekRelative { value } => {
                        let next = self.player.get_position() + value;
                        self.player.seek(next);
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::SeekAbsolute { value } => {
                        self.player.seek(value);
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::SleepTimer { minutes } => {
                        let minutes = minutes.min(180);
                        self.sleep_timer_deadline = if minutes == 0 {
                            None
                        } else {
                            Some(Instant::now() + Duration::from_secs(minutes as u64 * 60))
                        };
                        self.save_preferences(false);
                        self.scan_status = if minutes == 0 {
                            "Sleep timer cleared".to_string()
                        } else {
                            format!("Sleep timer set to {minutes} minutes")
                        };
                    }
                    crate::server::ControlCommand::Shuffle => {
                        self.player.toggle_shuffle();
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::Loop => {
                        self.player.toggle_loop_mode();
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::PlayTrack { id } => {
                        if let Ok(Some(track)) = self.db.get_track_by_id(id) {
                            let _ = self.db.increment_play_count(track.id);
                            self.player.set_queue(vec![track], 0);
                            self.apply_shuffle_default();
                            self.screen = Screen::NowPlaying;
                            self.focus = Focus::Albums;
                            self.save_preferences(false);
                        }
                    }
                    crate::server::ControlCommand::PlayTracks { ids, start_index } => {
                        let tracks: Vec<_> = ids
                            .into_iter()
                            .take(5000)
                            .filter_map(|id| self.db.get_track_by_id(id).ok().flatten())
                            .collect();
                        if !tracks.is_empty() {
                            let start_index = start_index.min(tracks.len().saturating_sub(1));
                            self.player.set_queue(tracks, start_index);
                            self.apply_shuffle_default();
                            self.screen = Screen::NowPlaying;
                            self.focus = Focus::Albums;
                            self.save_preferences(false);
                        }
                    }
                    crate::server::ControlCommand::PlayPlaylist { id, start_index } => {
                        if id == 0 {
                            if let Ok(tracks) = self.db.get_tracks() {
                                if !tracks.is_empty() {
                                    self.player.set_queue(tracks, start_index);
                                    self.apply_shuffle_default();
                                    self.screen = Screen::NowPlaying;
                                    self.save_preferences(false);
                                }
                            }
                        } else if let Ok(tracks) = self.db.get_playlist_tracks(id) {
                            if !tracks.is_empty() {
                                self.player.set_queue(tracks, start_index);
                                self.apply_shuffle_default();
                                self.screen = Screen::NowPlaying;
                                self.save_preferences(false);
                            }
                        }
                    }
                    crate::server::ControlCommand::PlayMix { id, start_index } => {
                        if let Ok(tracks) = self.db.get_mix_tracks(&id, 500) {
                            if !tracks.is_empty() {
                                let start_index =
                                    start_index.unwrap_or(0).min(tracks.len().saturating_sub(1));
                                self.player.set_queue(tracks, start_index);
                                self.apply_shuffle_default();
                                self.screen = Screen::NowPlaying;
                                self.focus = Focus::Albums;
                                self.save_preferences(false);
                            }
                        }
                    }
                    crate::server::ControlCommand::SetTheme { name } => {
                        self.active_theme = ThemeName::from_str(&name);
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::SetLanguage { name } => {
                        self.active_language = Language::from_str(&name);
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::ToggleFavorite { id } => {
                        if let Ok(favorited) = self.db.toggle_favorite(id) {
                            self.player.update_track_favorite(id, favorited);
                            for track in &mut self.library_tracks {
                                if track.id == id {
                                    track.favorited = favorited;
                                }
                            }
                            self.cached_library_counts = self
                                .db
                                .library_counts()
                                .unwrap_or(self.cached_library_counts);
                        }
                    }
                    crate::server::ControlCommand::SetRating { id, rating } => {
                        if rating <= 5 && self.db.set_track_rating(id, rating).is_ok() {
                            self.player.update_track_rating(id, rating);
                            for track in &mut self.tracks_queue {
                                if track.id == id {
                                    track.rating = rating;
                                }
                            }
                            for track in &mut self.search_results {
                                if track.id == id {
                                    track.rating = rating;
                                }
                            }
                        }
                    }
                    crate::server::ControlCommand::DeletePlaylist { id } => {
                        let _ = self.db.delete_playlist(id);
                        self.refresh_library();
                    }
                    crate::server::ControlCommand::RenamePlaylist { id, name } => {
                        let _ = self.db.rename_playlist(id, &name);
                        self.refresh_library();
                    }
                    crate::server::ControlCommand::CreatePlaylist { name } => {
                        let _ = self.db.create_playlist(&name);
                        self.refresh_library();
                    }
                    crate::server::ControlCommand::AddTrackToPlaylist {
                        playlist_id,
                        track_id,
                    } => {
                        let _ = self.db.add_track_to_playlist(playlist_id, track_id);
                    }
                    crate::server::ControlCommand::RemoveTrackFromPlaylist {
                        playlist_id,
                        track_id,
                    } => {
                        let _ = self.db.remove_track_from_playlist(playlist_id, track_id);
                    }
                    crate::server::ControlCommand::SetSettings { settings } => {
                        self.apply_settings_update(settings);
                    }
                    crate::server::ControlCommand::ResetSettings => {
                        self.reset_user_settings();
                    }
                    crate::server::ControlCommand::ScanLibrary => {
                        self.scan_library();
                    }
                    crate::server::ControlCommand::RescanLibrary { mode, source_id } => {
                        self.start_library_rescan(&mode, source_id);
                    }
                    crate::server::ControlCommand::AudioTest => {
                        self.run_audio_test();
                    }
                    crate::server::ControlCommand::QueueAdd { id, position } => {
                        if let Ok(Some(track)) = self.db.get_track_by_id(id) {
                            if position
                                .as_deref()
                                .map(|value| value.eq_ignore_ascii_case("next"))
                                .unwrap_or(false)
                            {
                                self.player.queue_add_next(track);
                            } else {
                                self.player.queue_add_later(track);
                            }
                            self.save_preferences(false);
                        }
                    }
                    crate::server::ControlCommand::QueueRemove { index } => {
                        self.player.queue_remove(index);
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::QueueMove { from, to } => {
                        if self.player.queue_move(from, to) {
                            self.save_preferences(false);
                        }
                    }
                    crate::server::ControlCommand::QueueSelect { index } => {
                        if self.player.queue_select(index) {
                            self.screen = Screen::NowPlaying;
                            self.save_preferences(false);
                        }
                    }
                    crate::server::ControlCommand::QueueClear => {
                        self.player.queue_clear();
                        self.save_preferences(false);
                    }
                    crate::server::ControlCommand::CleanupMissingFiles => {
                        match self.db.cleanup_missing_files() {
                            Ok(removed) => {
                                self.refresh_library();
                                self.scan_status =
                                    format!("Removed {removed} missing library entries");
                            }
                            Err(err) => {
                                self.scan_status = format!("Missing-file cleanup failed: {err:#}");
                            }
                        }
                    }
                }
                true
            }
        }
    }

    pub fn draw(&mut self, canvas: &mut Canvas<'_>, seconds: f32) {
        if self.is_asleep {
            canvas.clear(Color::BLACK);
            return;
        }
        canvas.language = self.active_language;
        let (tt_color, tb_color) = self.active_theme.gradient_colors();
        let dt = (seconds - self.last_draw_seconds).max(0.0).min(0.1);
        let feedback_dt = dt.min(1.0 / 30.0);
        self.last_draw_seconds = seconds;
        let motion = self.motion_mode.profile();
        let motion_seconds = seconds * motion.ambient_time_scale;

        self.progress_sample = if self.screen == Screen::NowPlaying {
            self.player.get_current_track().map(|track| {
                let position = self.player.get_position();
                let displayed = match self.progress_sample {
                    Some((id, last))
                        if id == track.id
                            && self.motion_mode != MotionMode::Reduced
                            && self.player.is_playing()
                            && (position - last).abs() < 1.0 =>
                    {
                        (last + (position - last) * (1.0 - (-20.0 * dt).exp()))
                            .clamp(0.0, track.duration.max(0) as f32)
                    }
                    _ => position,
                };
                (track.id, displayed)
            })
        } else {
            None
        };

        if self.current_cover_art.is_some() || self.cover_art_rx.is_none() {
            self.cover_art_opacity = if motion.artwork_seconds <= f32::EPSILON {
                1.0
            } else {
                (self.cover_art_opacity + dt / motion.artwork_seconds).min(1.0)
            };
            if self.cover_art_opacity >= 1.0 {
                self.previous_cover_art = None;
            }
        }
        if self.motion_mode == MotionMode::Reduced {
            self.button_feedback = None;
            self.edge_feedback = None;
            self.previous_cover_art = None;
        }
        let press = self
            .button_feedback
            .map(|(key, t)| (key, 1.0 - crate::ui::transitions::ease_out_cubic(t)));
        canvas.legend_press = press;
        canvas.focus_origin = (self.outline_active
            && self.last_drawn_screen == Some(self.screen)
            && self.motion_mode != MotionMode::Reduced)
            .then_some((self.outline_x, self.outline_y));
        canvas.selected_box = None;

        if !self.theme_colors_initialized {
            self.cur_top_r = tt_color.r as f32;
            self.cur_top_g = tt_color.g as f32;
            self.cur_top_b = tt_color.b as f32;
            self.cur_bottom_r = tb_color.r as f32;
            self.cur_bottom_g = tb_color.g as f32;
            self.cur_bottom_b = tb_color.b as f32;
            self.theme_colors_initialized = true;
        } else {
            let factor = (8.0 * dt).min(1.0);
            self.cur_top_r += (tt_color.r as f32 - self.cur_top_r) * factor;
            self.cur_top_g += (tt_color.g as f32 - self.cur_top_g) * factor;
            self.cur_top_b += (tt_color.b as f32 - self.cur_top_b) * factor;
            self.cur_bottom_r += (tb_color.r as f32 - self.cur_bottom_r) * factor;
            self.cur_bottom_g += (tb_color.g as f32 - self.cur_bottom_g) * factor;
            self.cur_bottom_b += (tb_color.b as f32 - self.cur_bottom_b) * factor;
        }

        let animated_top = Color::rgb(
            self.cur_top_r as u8,
            self.cur_top_g as u8,
            self.cur_top_b as u8,
        );
        let animated_bottom = Color::rgb(
            self.cur_bottom_r as u8,
            self.cur_bottom_g as u8,
            self.cur_bottom_b as u8,
        );
        canvas.theme_top_color = Some(animated_top);
        canvas.theme_bottom_color = Some(animated_bottom);
        canvas.dark_theme = self.active_theme == ThemeName::Dark;

        let transition_t = if self.motion_mode == MotionMode::Reduced {
            1.0
        } else {
            crate::ui::transitions::ease_out_cubic(self.screen_transition)
        };

        if self.screen == Screen::ContextMenu {
            self.draw_screen(
                self.previous_screen,
                self.previous_focus,
                canvas,
                motion_seconds,
                1.0,
            );
            let title = self.context_menu_title();
            let actions = self.context_actions();
            ui::draw_context_menu(
                canvas,
                self.focus,
                self.active_theme,
                &title,
                &actions,
                transition_t,
                motion_seconds,
            );
        } else {
            self.draw_screen(self.screen, self.focus, canvas, motion_seconds, 1.0);
        }

        ui::status_bar(
            canvas,
            if matches!(
                self.screen,
                Screen::Home
                    | Screen::AlbumsList
                    | Screen::ArtistsList
                    | Screen::SongsList
                    | Screen::ArtistAlbumsList
                    | Screen::AlbumTracksList
            ) {
                ""
            } else {
                self.screen_title()
            },
            &self.format_clock(),
            &crate::system::battery_summary(),
            self.active_theme,
            self.player.is_playing(),
        );

        let legend = self.button_legend();
        let home_legend = matches!(
            self.screen,
            Screen::Home
                | Screen::MusicApp
                | Screen::Settings
                | Screen::SettingsDetail
                | Screen::NowPlaying
                | Screen::First
                | Screen::Welcome
                | Screen::TestComplete
                | Screen::Theme
                | Screen::DateTimeSetup
                | Screen::Language
                | Screen::Apps
                | Screen::About
                | Screen::PlaylistsList
                | Screen::PlaylistTracksList
                | Screen::Queue
                | Screen::Search
                | Screen::RatingPicker
        );
        if home_legend {
            ui::home_legend_bar(canvas, legend);
        } else if !legend.is_empty() {
            ui::legend_bar(canvas, legend);
        }

        let screen_changed = Some(self.screen) != self.last_drawn_screen;
        self.last_drawn_screen = Some(self.screen);

        if let Some((tx, ty, tw, th, tr, t_thick)) = canvas.selected_box {
            if screen_changed || !self.outline_active {
                self.outline_x = tx;
                self.outline_y = ty;
                self.outline_w = tw;
                self.outline_h = th;
                self.outline_r = tr;
                self.outline_thick = t_thick;
                self.outline_active = true;
            } else {
                let factor = if motion.selection_response.is_finite() {
                    1.0 - (-motion.selection_response * feedback_dt).exp()
                } else {
                    1.0
                };
                self.outline_x += (tx - self.outline_x) * factor;
                self.outline_y += (ty - self.outline_y) * factor;
                self.outline_w += (tw - self.outline_w) * factor;
                self.outline_h += (th - self.outline_h) * factor;
                self.outline_r += (tr - self.outline_r) * factor;
                self.outline_thick += (t_thick - self.outline_thick) * factor;
            }

            let outline_pulse = if motion.ambient_fps == 0 {
                0.0
            } else {
                crate::ui::transitions::pulse01(motion_seconds, 4.0, 0.0)
            };
            let outline_color = ui::accent(canvas);
            let edge_y = self.edge_feedback.map_or(0.0, |(direction, t)| {
                direction * 8.0 * (t * std::f32::consts::PI).sin()
            });
            let press_scale = press
                .filter(|(key, _)| *key == "A")
                .map_or(0.0, |(_, value)| value);
            let inset = 3.0 * press_scale;
            let outline_y = self.outline_y + edge_y + inset;
            let dist = (tx - self.outline_x).abs()
                + (ty - self.outline_y).abs()
                + (tw - self.outline_w).abs()
                + (th - self.outline_h).abs();
            if dist > 0.5 || press_scale > 0.0 {
                crate::ui::rr(
                    canvas,
                    self.outline_x + inset,
                    outline_y,
                    self.outline_w - 2.0 * inset,
                    self.outline_h - 2.0 * inset,
                    self.outline_r,
                    outline_color.with_alpha((14.0 + 22.0 * press_scale) as u8),
                );
            }
            crate::ui::rr_stroke(
                canvas,
                self.outline_x + inset,
                outline_y,
                self.outline_w - 2.0 * inset,
                self.outline_h - 2.0 * inset,
                self.outline_r,
                self.outline_thick + outline_pulse * 0.6,
                outline_color,
            );

            self.outline_animating = dist > 0.5;
        } else {
            self.outline_active = false;
            self.outline_animating = false;
        }

        if self.screen_transition < 1.0 {
            if transition_t < 1.0 && self.screen != Screen::ContextMenu {
                let remaining = 1.0 - transition_t;
                let offset =
                    (44.0 * remaining * self.transition_direction * canvas.resolution().scale())
                        .round() as i32;
                let wash = ui::shell(canvas);
                let bottom = if home_legend { 878.0 } else { 918.0 };
                canvas.slide_content(offset, 64.0, bottom, wash, (70.0 * remaining) as u8);
            }
            self.screen_transition = if motion.screen_seconds <= f32::EPSILON {
                1.0
            } else {
                let next = self.screen_transition + dt / motion.screen_seconds;
                if self.screen_transition == 0.0 {
                    next.clamp(0.01, 0.16)
                } else {
                    next.min(1.0)
                }
            };
        }
        self.button_feedback = self.button_feedback.and_then(|(key, t)| {
            (t + feedback_dt / 0.14 < 1.0).then_some((key, t + feedback_dt / 0.14))
        });
        self.edge_feedback = self.edge_feedback.and_then(|(direction, t)| {
            (t + feedback_dt / 0.16 < 1.0).then_some((direction, t + feedback_dt / 0.16))
        });

        if self.hud_t > 0.0 {
            self.draw_hud(canvas, motion_seconds);
        }

        if self.wifi_sync_overlay {
            if let Some(timeout) = self.wifi_sync_timeout {
                if Instant::now() > timeout {
                    self.wifi_sync_overlay = false;
                }
            }
        }

        if self.wifi_sync_overlay {
            self.draw_wifi_sync_overlay(canvas, motion_seconds);
        }
    }

    fn draw_wifi_sync_overlay(&self, canvas: &mut Canvas<'_>, _seconds: f32) {
        let res = canvas.resolution();
        let scale = res.scale();

        let card_w = 640.0;
        let card_h = 320.0;
        let card_r = 32.0;
        let card_x = (res.width as f32 / scale - card_w) / 2.0;
        let card_y = (res.height as f32 / scale - card_h) / 2.0;

        crate::ui::rr(
            canvas,
            0.0,
            0.0,
            res.width as f32 / scale,
            res.height as f32 / scale,
            0.0,
            Color::rgba(0, 0, 0, 160),
        );

        crate::ui::shadow_rr(canvas, card_x, card_y, card_w, card_h, card_r, 12.0, 24.0);
        crate::ui::rr(
            canvas,
            card_x,
            card_y,
            card_w,
            card_h,
            card_r,
            ui::surface(canvas),
        );
        let accent = ui::accent(canvas);

        canvas.text_centered(
            "Wi-Fi Sync Diagnostics",
            card_x + card_w / 2.0,
            card_y + 40.0,
            38.0,
            ui::ink(canvas),
            crate::ui::FontWeight::Bold,
        );

        canvas.text_centered(
            "Connect companion app to:",
            card_x + card_w / 2.0,
            card_y + 110.0,
            28.0,
            ui::ink_secondary(canvas),
            crate::ui::FontWeight::Regular,
        );

        let ip_url = format!("http://{}:18080", self.wifi_sync_ip);
        canvas.text_centered(
            &ip_url,
            card_x + card_w / 2.0,
            card_y + 170.0,
            42.0,
            accent,
            crate::ui::FontWeight::Bold,
        );

        canvas.text_centered(
            "Press any key to dismiss",
            card_x + card_w / 2.0,
            card_y + 250.0,
            24.0,
            ui::ink_secondary(canvas),
            crate::ui::FontWeight::Regular,
        );
    }

    fn draw_hud(&self, canvas: &mut Canvas<'_>, _seconds: f32) {
        let res = canvas.resolution();
        let scale = res.scale();

        let hud_w = 800.0;
        let hud_h = 96.0;
        let hud_r = 48.0;
        let hud_x = 240.0;

        let target_y = 32.0;
        let start_y = -112.0;
        let hud_y = start_y + (target_y - start_y) * self.hud_t;

        crate::ui::shadow_rr(canvas, hud_x, hud_y, hud_w, hud_h, hud_r, 8.0, 16.0);

        crate::ui::rr(
            canvas,
            hud_x,
            hud_y,
            hud_w,
            hud_h,
            hud_r,
            ui::surface(canvas),
        );
        let accent = ui::accent(canvas);

        let (label, val_str, ratio) = match self.hud_type {
            HudType::Volume => {
                let vol = self.player.get_status().volume;
                ("Volume", format!("{}%", (vol * 100.0).round() as u8), vol)
            }
            HudType::Brightness => {
                let bri = self.brightness;
                ("Brightness", format!("{}%", bri), bri as f32 / 100.0)
            }
        };

        canvas.text(
            label,
            hud_x + 48.0,
            hud_y + 30.0,
            36.0,
            ui::ink(canvas),
            crate::ui::FontWeight::Bold,
        );

        let track_x = hud_x + 280.0;
        let track_y = hud_y + 40.0;
        let track_w = 360.0;
        let track_h = 16.0;
        let track_r = 8.0;

        crate::ui::rr(
            canvas,
            track_x,
            track_y,
            track_w,
            track_h,
            track_r,
            ui::control(canvas),
        );

        let fill_w = track_w * ratio;
        if fill_w > 0.0 {
            crate::ui::rr(canvas, track_x, track_y, fill_w, track_h, track_r, accent);
        }

        let knob_x = track_x + fill_w;
        let knob_y = track_y + track_h / 2.0;
        let knob_radius = 16.0;

        canvas.circle(
            res.scale_x(knob_x),
            res.scale_y(knob_y),
            knob_radius * scale,
            accent,
        );
        canvas.circle(
            res.scale_x(knob_x),
            res.scale_y(knob_y),
            6.0 * scale,
            Color::WHITE,
        );

        canvas.text_right(
            &val_str,
            hud_x + hud_w - 48.0,
            hud_y + 30.0,
            36.0,
            ui::ink(canvas),
            crate::ui::FontWeight::Bold,
        );
    }

    pub fn animation_in_progress(&self) -> bool {
        self.outline_animating
            || self.button_feedback.is_some()
            || self.edge_feedback.is_some()
            || (self.screen == Screen::NowPlaying
                && (self.current_cover_art.is_some() || self.previous_cover_art.is_some())
                && (self.current_cover_art.is_some() || self.cover_art_rx.is_none())
                && self.cover_art_opacity < 1.0)
            || self.screen_transition < 1.0
            || self.hud_t > 0.0
            || (self.target_scroll_offset - self.scroll_offset).abs() > 0.5
            || self.theme_colors_initialized && {
                let (tt_color, tb_color) = self.active_theme.gradient_colors();
                (self.cur_top_r - tt_color.r as f32).abs() > 0.5
                    || (self.cur_bottom_r - tb_color.r as f32).abs() > 0.5
            }
    }

    fn draw_screen(
        &self,
        screen: Screen,
        focus: Focus,
        canvas: &mut Canvas<'_>,
        seconds: f32,
        transition_t: f32,
    ) {
        log::debug!("draw_screen: screen={:?}, focus={:?}", screen, focus);

        match screen {
            Screen::First => ui::screens::draw_first(
                canvas,
                focus,
                seconds,
                self.active_language,
                self.active_theme,
                transition_t,
            ),
            Screen::Welcome => ui::screens::draw_welcome(
                canvas,
                seconds,
                self.active_language,
                self.active_theme,
                transition_t,
            ),
            Screen::Theme => ui::screens::draw_theme_selection(
                canvas,
                focus,
                self.active_theme,
                seconds,
                self.active_language,
                transition_t,
            ),
            Screen::DateTimeSetup => ui::settings::draw_date_time_setup(
                canvas,
                focus,
                self.clock_hour,
                self.clock_minute,
                self.clock_month,
                self.clock_day,
                self.clock_year,
                self.active_theme,
                seconds,
                transition_t,
            ),
            Screen::Language => ui::screens::draw_language_selection(
                canvas,
                self.active_language,
                self.selected_index,
                self.scroll_offset,
                seconds,
                self.active_language,
                self.active_theme,
                transition_t,
            ),
            Screen::TestComplete => {
                ui::screens::draw_test_complete(canvas, seconds, self.active_language, transition_t)
            }

            Screen::Home => ui::home::draw_home(
                canvas,
                focus,
                self.home_navigation.pane,
                self.home_navigation.main_index,
                self.cached_library_counts,
                &self.album_previews,
                self.recent_albums,
                self.home_album_mosaic.as_ref(),
                self.home_artist_portrait.as_ref(),
                &self.playlists_list,
                &self.player.get_status(),
                !self.tracks_queue.is_empty(),
                self.current_cover_art.as_ref(),
            ),
            Screen::MusicApp => ui::music::draw_music_menu(
                canvas,
                focus,
                seconds,
                self.active_language,
                self.active_theme,
                &self.pins_list,
                self.cached_library_counts,
                self.player.queue_tracks().len(),
                transition_t,
            ),
            Screen::SongsList => self.library_view.tracks(
                canvas,
                self.active_language.translate("songs"),
                &self.tracks_queue,
                self.selected_index,
                false,
            ),
            Screen::AlbumsList => self.library_view.grid(
                canvas,
                self.active_language.translate("albums"),
                &self.albums_list,
                self.selected_index,
                Grid::Albums,
            ),
            Screen::ArtistsList => self.library_view.grid(
                canvas,
                self.active_language.translate("artists"),
                &self.artists_list,
                self.selected_index,
                Grid::Artists,
            ),
            Screen::ArtistAlbumsList => self.library_view.grid(
                canvas,
                self.active_artist.as_deref().unwrap_or("Artist"),
                &self.artist_albums_list,
                self.selected_index,
                Grid::ArtistAlbums,
            ),
            Screen::AlbumTracksList => self.library_view.tracks(
                canvas,
                self.active_album.as_deref().unwrap_or("Album"),
                &self.tracks_queue,
                self.selected_index,
                true,
            ),
            Screen::PlaylistsList => {
                let names: Vec<String> =
                    self.playlists_list.iter().map(|p| p.name.clone()).collect();
                ui::music::draw_items_list(
                    canvas,
                    self.active_language.translate("playlists"),
                    &names,
                    self.selected_index,
                    self.active_theme,
                    self.scroll_offset,
                    transition_t,
                    seconds,
                )
            }
            Screen::PlaylistTracksList => {
                let title = self
                    .active_playlist
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or("Playlist Tracks");
                ui::music::draw_tracks_list(
                    canvas,
                    &self.library_view,
                    title,
                    &self.tracks_queue,
                    self.selected_index,
                    self.active_theme,
                    self.scroll_offset,
                    transition_t,
                    seconds,
                )
            }
            Screen::Queue => ui::music::draw_tracks_list(
                canvas,
                &self.library_view,
                self.active_language.translate("queue"),
                &self.tracks_queue,
                self.selected_index,
                self.active_theme,
                self.scroll_offset,
                transition_t,
                seconds,
            ),
            Screen::Search => ui::music::draw_search(
                canvas,
                &self.library_view,
                &self.search_query,
                &self.search_results,
                self.search_key_index,
                self.selected_index,
                focus,
                self.active_theme,
                self.scroll_offset,
                transition_t,
                seconds,
            ),
            Screen::RatingPicker => {
                let title = self
                    .rating_track_id
                    .and_then(|id| self.db.get_track_by_id(id).ok().flatten())
                    .map(|track| track.title)
                    .unwrap_or_else(|| "Track".to_string());
                ui::music::draw_rating_picker(
                    canvas,
                    &title,
                    self.pending_rating,
                    self.active_theme,
                    seconds,
                )
            }
            Screen::NowPlaying => {
                let status = self.player.get_status();
                ui::nowplaying::draw_now_playing(
                    canvas,
                    focus,
                    seconds,
                    self.active_language,
                    self.active_theme,
                    &status,
                    &self.player.queue_tracks(),
                    self.current_cover_art.as_ref(),
                    self.previous_cover_art.as_ref(),
                    self.cover_art_opacity,
                    self.progress_sample
                        .map_or(status.position, |(_, value)| value),
                )
            }
            Screen::Settings => ui::settings::draw_settings(
                canvas,
                focus,
                seconds,
                self.active_language,
                self.active_theme,
                transition_t,
            ),
            Screen::SettingsDetail => ui::settings::draw_settings_detail(
                canvas,
                focus,
                self.brightness,
                (self.player.get_status().volume * 100.0).round() as u8,
                self.player.get_eq_preset(),
                self.active_theme,
                &crate::system::battery_summary(),
                seconds,
                transition_t,
            ),
            Screen::Apps => ui::apps::draw_apps(
                canvas,
                focus,
                seconds,
                self.active_language,
                self.active_theme,
                self.library_counts(),
                &self.usb_status,
                transition_t,
            ),
            Screen::About => {
                let audio_label = if self.alsa_device.trim().is_empty() {
                    self.audio_backend.clone()
                } else {
                    format!("{} {}", self.audio_backend, self.alsa_device)
                };
                ui::apps::draw_about(
                    canvas,
                    focus,
                    seconds,
                    self.active_theme,
                    self.library_counts(),
                    &self.usb_status,
                    self.resolution,
                    &self.user_name,
                    &self.device_name,
                    self.player.queue_tracks().len(),
                    &audio_label,
                    &self.scan_status,
                    &self.audio_test_status,
                    &crate::system::battery_summary(),
                    &self.db_path,
                    &crate::preferences::path().display().to_string(),
                    transition_t,
                )
            }
            Screen::ContextMenu => {}
        }
    }

    pub fn force_screen_for_snapshot(&mut self, screen: Screen) {
        self.screen = screen;
        self.screen_transition = 1.0;
        self.focus = match screen {
            Screen::First => Focus::GetStarted,
            Screen::Welcome => Focus::GetStarted,
            Screen::Theme => Focus::ThemeSelect,
            Screen::DateTimeSetup => Focus::DateHour,
            Screen::Home => Focus::HomeMusic,
            Screen::MusicApp => Focus::Albums,
            Screen::Language => Focus::Language,
            Screen::TestComplete => Focus::GetStarted,
            Screen::NowPlaying => Focus::Albums,
            Screen::Queue => Focus::Queue,
            Screen::Search => Focus::SearchKeyboard,
            Screen::RatingPicker => Focus::RatingStars,
            Screen::Settings => Focus::SettingsGeneral,
            Screen::SettingsDetail => Focus::SettingsLanguage,
            Screen::Apps => Focus::AppsMusic,
            Screen::About => Focus::SettingsAbout,
            Screen::ContextMenu => Focus::ContextPlay,
            _ => Focus::Albums,
        };
    }

    fn reset_list_state(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0.0;
        self.target_scroll_offset = 0.0;
    }

    fn push_navigation(&mut self) {
        self.navigation_stack.push(NavigationState {
            screen: self.screen,
            focus: self.focus,
            selected_index: self.selected_index,
            scroll_offset: self.scroll_offset,
            target_scroll_offset: self.target_scroll_offset,
            tracks_queue: self.tracks_queue.clone(),
            albums_list: self.albums_list.clone(),
            artists_list: self.artists_list.clone(),
            artist_albums_list: self.artist_albums_list.clone(),
            playlists_list: self.playlists_list.clone(),
            active_playlist: self.active_playlist.clone(),
            active_artist: self.active_artist.clone(),
            active_album: self.active_album.clone(),
            search_query: self.search_query.clone(),
            search_results: self.search_results.clone(),
            search_key_index: self.search_key_index,
        });
    }

    fn pop_navigation(&mut self) -> bool {
        let Some(state) = self.navigation_stack.pop() else {
            return false;
        };
        self.screen = state.screen;
        self.focus = state.focus;
        self.selected_index = state.selected_index;
        self.scroll_offset = state.scroll_offset;
        self.target_scroll_offset = state.target_scroll_offset;
        self.tracks_queue = state.tracks_queue;
        self.albums_list = state.albums_list;
        self.artists_list = state.artists_list;
        self.artist_albums_list = state.artist_albums_list;
        self.playlists_list = state.playlists_list;
        self.active_playlist = state.active_playlist;
        self.active_artist = state.active_artist;
        self.active_album = state.active_album;
        self.search_query = state.search_query;
        self.search_results = state.search_results;
        self.search_key_index = state.search_key_index;
        self.navigation_restored = true;
        true
    }

    fn refresh_search_results(&mut self) {
        self.search_results = self
            .db
            .search_tracks(&self.search_query, 200)
            .unwrap_or_default();
        self.selected_index = self
            .selected_index
            .min(self.search_results.len().saturating_sub(1));
    }

    fn activate_search_key(&mut self) {
        match SEARCH_KEYS[self.search_key_index] {
            "SPACE" => self.search_query.push(' '),
            "DELETE" => {
                self.search_query.pop();
            }
            "CLEAR" => self.search_query.clear(),
            "RESULTS" => {
                if !self.search_query.trim().is_empty() {
                    self.focus = Focus::SearchResults;
                    self.selected_index = 0;
                    self.scroll_offset = 0.0;
                    self.target_scroll_offset = 0.0;
                }
                return;
            }
            key => self.search_query.push_str(key),
        }
        self.refresh_search_results();
    }

    fn init_language_screen(&mut self) {
        let idx = match self.active_language {
            Language::En => 0,
            Language::Es => 1,
            Language::De => 2,
            Language::Ja => 3,
            Language::Zh => 4,
        };
        self.selected_index = idx;
        self.scroll_offset = idx as f32 * 144.0;
        self.target_scroll_offset = idx as f32 * 144.0;
    }

    fn current_list_len(&self) -> usize {
        match self.screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => self.tracks_queue.len(),
            Screen::AlbumsList => self.albums_list.len(),
            Screen::ArtistsList => self.artists_list.len(),
            Screen::ArtistAlbumsList => self.artist_albums_list.len(),
            Screen::PlaylistsList => self.playlists_list.len(),
            Screen::Search if self.focus == Focus::SearchResults => self.search_results.len(),
            Screen::Language => 5,
            _ => 0,
        }
    }

    fn jump_active_list(&mut self, direction: i32) {
        match self.screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => {
                let labels = self
                    .tracks_queue
                    .iter()
                    .map(|track| track.title.as_str())
                    .collect::<Vec<_>>();
                self.selected_index = jump_index_by_letter(&labels, self.selected_index, direction);
            }
            Screen::Search if self.focus == Focus::SearchResults => {
                let labels = self
                    .search_results
                    .iter()
                    .map(|track| track.title.as_str())
                    .collect::<Vec<_>>();
                self.selected_index = jump_index_by_letter(&labels, self.selected_index, direction);
            }
            Screen::PlaylistsList => {
                let labels = self
                    .playlists_list
                    .iter()
                    .map(|item| item.name.as_str())
                    .collect::<Vec<_>>();
                self.selected_index = jump_index_by_letter(&labels, self.selected_index, direction);
            }
            _ => {}
        }
    }

    fn activate(&mut self) {
        match (self.screen, self.focus) {
            (Screen::First, Focus::Language) => {
                self.previous_screen = self.screen;
                self.previous_focus = self.focus;
                self.screen = Screen::Language;
                self.init_language_screen();
            }
            (Screen::First, _) if self.controls_seen => {
                self.screen = Screen::Theme;
                self.focus = Focus::ThemeSelect;
            }
            (Screen::First, _) => self.screen = Screen::Welcome,
            (Screen::Welcome, _) => {
                self.controls_seen = true;
                self.save_preferences(false);
                self.screen = Screen::Theme;
                self.focus = Focus::ThemeSelect;
            }
            (Screen::Theme, _) => {
                if self.previous_screen == Screen::Settings {
                    self.save_preferences(true);
                    self.screen = Screen::Settings;
                    self.focus = Focus::SettingsLooks;
                } else if !self.time_configured {
                    self.save_preferences(false);
                    self.screen = Screen::DateTimeSetup;
                    self.focus = Focus::DateHour;
                } else {
                    self.save_preferences(true);
                    self.screen = Screen::Home;
                    self.focus = Focus::HomeMusic;
                }
            }
            (Screen::DateTimeSetup, Focus::DateHour) => self.focus = Focus::DateMinute,
            (Screen::DateTimeSetup, Focus::DateMinute) => self.focus = Focus::DateMonth,
            (Screen::DateTimeSetup, Focus::DateMonth) => self.focus = Focus::DateDay,
            (Screen::DateTimeSetup, Focus::DateDay) => self.focus = Focus::DateYear,
            (Screen::DateTimeSetup, Focus::DateYear) => self.focus = Focus::DateDone,
            (Screen::DateTimeSetup, Focus::DateDone) | (Screen::DateTimeSetup, _) => {
                self.time_configured = true;
                self.clock_anchor = Instant::now();
                self.save_preferences(true);
                if self.previous_screen == Screen::SettingsDetail {
                    self.screen = Screen::SettingsDetail;
                    self.focus = Focus::SettingsDateTime;
                } else {
                    self.screen = Screen::Home;
                    self.focus = Focus::HomeMusic;
                }
            }
            (Screen::Home, focus) if Self::home_detail_focus_index(focus).is_some() => {
                self.activate_home_detail()
            }
            (Screen::Home, Focus::HomeNowPlaying) => {
                self.push_navigation();
                self.previous_screen = Screen::Home;
                self.previous_focus = self.focus;
                self.screen = Screen::NowPlaying;
                self.focus = Focus::Albums;
            }
            (Screen::Home, Focus::HomeSettings) => {
                self.previous_screen = Screen::Home;
                self.previous_focus = self.focus;
                self.screen = Screen::Settings;
                self.focus = Focus::SettingsGeneral;
            }
            (Screen::Home, Focus::HomeApps) => {
                self.previous_screen = Screen::Home;
                self.previous_focus = self.focus;
                self.screen = Screen::Apps;
                self.focus = Focus::AppsMusic;
            }
            (Screen::Apps, Focus::AppsMusic) => {
                self.push_navigation();
                self.previous_screen = Screen::Apps;
                self.previous_focus = self.focus;
                self.screen = Screen::MusicApp;
                self.focus = Focus::Albums;
            }
            (Screen::Apps, Focus::AppsNowPlaying) => {
                self.push_navigation();
                self.previous_screen = Screen::Apps;
                self.previous_focus = self.focus;
                self.screen = Screen::NowPlaying;
                self.focus = Focus::Albums;
            }
            (Screen::Apps, Focus::AppsSettings) => {
                self.previous_screen = Screen::Apps;
                self.previous_focus = self.focus;
                self.screen = Screen::Settings;
                self.focus = Focus::SettingsGeneral;
            }
            (Screen::Apps, Focus::AppsUsbSync) => {
                self.previous_screen = Screen::Apps;
                self.previous_focus = self.focus;
                self.run_usb_sync();
                self.screen = Screen::About;
                self.focus = Focus::SettingsAbout;
            }
            (Screen::MusicApp | Screen::Home, Focus::Songs) => {
                if let Ok(mut tracks) = self.db.get_tracks() {
                    sort_tracks(&mut tracks, self.library_sort);
                    self.push_navigation();
                    self.tracks_queue = tracks;
                    self.screen = Screen::SongsList;
                    self.reset_list_state();
                }
            }
            (Screen::MusicApp, Focus::Albums) | (Screen::Home, Focus::HomeMusic) => {
                if let Ok(albums) = self.db.get_albums() {
                    self.push_navigation();
                    self.albums_list = albums;
                    self.screen = Screen::AlbumsList;
                    self.reset_list_state();
                }
            }
            (Screen::MusicApp | Screen::Home, Focus::Search) => {
                self.push_navigation();
                self.search_query.clear();
                self.search_results.clear();
                self.search_key_index = 0;
                self.screen = Screen::Search;
                self.focus = Focus::SearchKeyboard;
                self.reset_list_state();
            }
            (Screen::ContextMenu, Focus::ContextPlay) => {
                self.activate_context_play(false);
            }
            (Screen::ContextMenu, Focus::ContextPlayNext) => {
                self.apply_context_queue_action(Focus::ContextPlayNext);
            }
            (Screen::ContextMenu, Focus::ContextAddLater) => {
                self.apply_context_queue_action(Focus::ContextAddLater);
            }
            (Screen::ContextMenu, Focus::ContextPin) => {
                if let Some((pin_type, target_id)) = self.context_pin_target() {
                    if self.db.is_pinned(pin_type, target_id).unwrap_or(false) {
                        let _ = self.db.unpin_item(pin_type, target_id);
                    } else {
                        let _ = self.db.pin_item(pin_type, target_id);
                    }
                }
                self.screen = self.previous_screen;
                self.focus = self.previous_focus;
            }
            (Screen::ContextMenu, Focus::ContextAddToPlaylist) => {
                self.add_context_track_to_default_playlist();
                self.screen = self.previous_screen;
                self.focus = self.previous_focus;
            }
            (Screen::ContextMenu, Focus::ContextShuffle) => {
                self.activate_context_play(true);
            }
            (Screen::ContextMenu, Focus::ContextDelete) => {
                self.delete_context_target();
                self.screen = self.previous_screen;
                self.focus = self.previous_focus;
            }
            (Screen::ContextMenu, Focus::ContextRate) => self.open_rating_picker(),
            (Screen::ContextMenu, Focus::ContextMoveUp) => {
                if self.previous_screen == Screen::Queue && self.selected_index > 0 {
                    let to = self.selected_index - 1;
                    let _ = self.player.queue_move(self.selected_index, to);
                    self.selected_index = to;
                    self.tracks_queue = self.player.queue_tracks();
                    self.save_preferences(false);
                }
                self.close_context();
            }
            (Screen::ContextMenu, Focus::ContextMoveDown) => {
                if self.previous_screen == Screen::Queue
                    && self.selected_index + 1 < self.tracks_queue.len()
                {
                    let to = self.selected_index + 1;
                    let _ = self.player.queue_move(self.selected_index, to);
                    self.selected_index = to;
                    self.tracks_queue = self.player.queue_tracks();
                    self.save_preferences(false);
                }
                self.close_context();
            }
            (Screen::ContextMenu, Focus::ContextClearUpcoming) => {
                self.player.queue_clear_upcoming();
                self.tracks_queue = self.player.queue_tracks();
                self.selected_index = self
                    .selected_index
                    .min(self.tracks_queue.len().saturating_sub(1));
                self.save_preferences(false);
                self.close_context();
            }
            (Screen::ContextMenu, Focus::ContextCancel) => {
                self.screen = self.previous_screen;
                self.focus = self.previous_focus;
            }
            (Screen::MusicApp | Screen::Home, Focus::Artists) => {
                if let Ok(artists) = self.db.get_artists() {
                    self.push_navigation();
                    self.artists_list = artists;
                    self.screen = Screen::ArtistsList;
                    self.reset_list_state();
                }
            }
            (Screen::MusicApp | Screen::Home, Focus::Playlists) => {
                if let Ok(playlists) = self.db.get_playlists() {
                    self.push_navigation();
                    self.playlists_list = playlists;
                    self.screen = Screen::PlaylistsList;
                    self.reset_list_state();
                }
            }
            (Screen::MusicApp, Focus::Favorites) => {
                if let Ok(favorites) = self.db.get_favorites() {
                    self.push_navigation();
                    self.tracks_queue = favorites;
                    self.screen = Screen::SongsList;
                    self.reset_list_state();
                }
            }
            (Screen::MusicApp, Focus::Queue) => {
                self.push_navigation();
                self.tracks_queue = self.player.queue_tracks();
                self.selected_index = self.player.current_index().unwrap_or(0);
                if self.selected_index >= self.tracks_queue.len() {
                    self.selected_index = 0;
                }
                self.previous_screen = Screen::MusicApp;
                self.previous_focus = Focus::Queue;
                self.screen = Screen::Queue;
                self.scroll_offset = 0.0;
                self.target_scroll_offset = 0.0;
            }
            (Screen::MusicApp, Focus::History) => {
                if let Ok(history) = self.db.get_history(25) {
                    self.push_navigation();
                    self.tracks_queue = history;
                    self.screen = Screen::SongsList;
                    self.reset_list_state();
                }
            }
            (Screen::MusicApp, Focus::ShuffleAll) => {
                if let Ok(tracks) = self.db.get_tracks() {
                    if !tracks.is_empty() {
                        self.player.set_queue(tracks, 0);
                        self.player.set_shuffle(true);
                        self.screen = Screen::NowPlaying;
                        self.focus = Focus::Albums;
                    }
                }
            }
            (Screen::Queue, _) => {
                if self.player.queue_select(self.selected_index) {
                    self.push_navigation();
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                    self.save_preferences(false);
                }
            }
            (Screen::SongsList, _) => {
                if self.selected_index < self.tracks_queue.len() {
                    let track = self.tracks_queue[self.selected_index].clone();
                    let _ = self.db.increment_play_count(track.id);
                    self.push_navigation();
                    self.player
                        .set_queue(self.tracks_queue.clone(), self.selected_index);
                    self.apply_shuffle_default();
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                }
            }
            (Screen::AlbumsList, _) => {
                if self.selected_index < self.albums_list.len() {
                    let album = self.albums_list[self.selected_index].clone();
                    let tracks = self.db.get_album_tracks(&album).unwrap_or_default();
                    self.push_navigation();
                    self.active_artist = None;
                    self.active_album = Some(album);
                    self.tracks_queue = tracks;
                    self.screen = Screen::AlbumTracksList;
                    self.reset_list_state();
                }
            }
            (Screen::ArtistsList, _) => {
                if self.selected_index < self.artists_list.len() {
                    let artist = self.artists_list[self.selected_index].clone();
                    let albums = self.db.get_artist_albums(&artist).unwrap_or_default();
                    self.push_navigation();
                    self.active_artist = Some(artist);
                    self.artist_albums_list = albums;
                    self.screen = Screen::ArtistAlbumsList;
                    self.reset_list_state();
                }
            }
            (Screen::ArtistAlbumsList, _) => {
                if let (Some(artist), Some(album)) = (
                    self.active_artist.clone(),
                    self.artist_albums_list.get(self.selected_index).cloned(),
                ) {
                    let tracks = self
                        .db
                        .get_artist_album_tracks(&artist, &album)
                        .unwrap_or_default();
                    self.push_navigation();
                    self.active_album = Some(album);
                    self.tracks_queue = tracks;
                    self.screen = Screen::AlbumTracksList;
                    self.reset_list_state();
                }
            }
            (Screen::AlbumTracksList, _) => {
                if self.selected_index < self.tracks_queue.len() {
                    let track = self.tracks_queue[self.selected_index].clone();
                    let _ = self.db.increment_play_count(track.id);
                    self.push_navigation();
                    self.player
                        .set_queue(self.tracks_queue.clone(), self.selected_index);
                    self.apply_shuffle_default();
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                }
            }
            (Screen::PlaylistsList, _) => {
                if self.selected_index < self.playlists_list.len() {
                    let playlist = self.playlists_list[self.selected_index].clone();
                    if let Ok(tracks) = self.db.get_playlist_tracks(playlist.id) {
                        self.push_navigation();
                        self.tracks_queue = tracks;
                        self.active_playlist = Some(playlist.clone());
                        self.screen = Screen::PlaylistTracksList;
                        self.reset_list_state();
                    }
                }
            }
            (Screen::PlaylistTracksList, _) => {
                if self.selected_index < self.tracks_queue.len() {
                    let track = self.tracks_queue[self.selected_index].clone();
                    let _ = self.db.increment_play_count(track.id);
                    self.push_navigation();
                    self.player
                        .set_queue(self.tracks_queue.clone(), self.selected_index);
                    self.apply_shuffle_default();
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                }
            }
            (Screen::Search, Focus::SearchKeyboard) => self.activate_search_key(),
            (Screen::Search, Focus::SearchResults) => {
                if self.selected_index < self.search_results.len() {
                    let track = self.search_results[self.selected_index].clone();
                    let _ = self.db.increment_play_count(track.id);
                    self.push_navigation();
                    self.player
                        .set_queue(self.search_results.clone(), self.selected_index);
                    self.apply_shuffle_default();
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                }
            }
            (Screen::RatingPicker, Focus::RatingStars) => {
                if let Some(id) = self.rating_track_id {
                    if self.db.set_track_rating(id, self.pending_rating).is_ok() {
                        self.player.update_track_rating(id, self.pending_rating);
                        for track in &mut self.tracks_queue {
                            if track.id == id {
                                track.rating = self.pending_rating;
                            }
                        }
                        for track in &mut self.search_results {
                            if track.id == id {
                                track.rating = self.pending_rating;
                            }
                        }
                    }
                }
                self.pop_navigation();
            }
            (Screen::NowPlaying, _) => match self.focus {
                Focus::Albums => self.player.play_pause(),
                Focus::Artists => self.player.prev(),
                Focus::Playlists => self.player.next(),
                Focus::Songs => self.player.toggle_loop_mode(),
                Focus::History => self.player.toggle_shuffle(),
                Focus::Favorites => self.toggle_current_favorite(),
                _ => {}
            },
            (Screen::Settings, Focus::SettingsGeneral)
            | (Screen::Settings, Focus::SettingsLanguage) => {
                self.previous_screen = Screen::Settings;
                self.previous_focus = Focus::SettingsGeneral;
                self.screen = Screen::SettingsDetail;
                self.focus = Focus::SettingsLanguage;
            }
            (Screen::Settings, Focus::SettingsAudio) => {
                self.previous_screen = Screen::Settings;
                self.previous_focus = Focus::SettingsAudio;
                self.screen = Screen::SettingsDetail;
                self.focus = Focus::SettingsVolume;
            }
            (Screen::SettingsDetail, Focus::SettingsLanguage) => {
                self.previous_screen = self.screen;
                self.previous_focus = self.focus;
                self.screen = Screen::Language;
                self.focus = Focus::Language;
                self.init_language_screen();
            }
            (Screen::SettingsDetail, Focus::SettingsDateTime) => {
                self.previous_screen = self.screen;
                self.previous_focus = self.focus;
                self.screen = Screen::DateTimeSetup;
                self.focus = Focus::DateHour;
            }
            (Screen::SettingsDetail, Focus::SettingsBrightness)
            | (Screen::SettingsDetail, Focus::SettingsVolume)
            | (Screen::SettingsDetail, Focus::SettingsEq)
            | (Screen::SettingsDetail, Focus::SettingsSwitchObject) => match self.focus {
                Focus::SettingsBrightness => self.adjust_brightness(10),
                Focus::SettingsVolume => self.adjust_volume(0.10),
                Focus::SettingsEq => self.cycle_eq(1),
                Focus::SettingsSwitchObject => {
                    self.active_theme = self.active_theme.cycle(1);
                    self.save_preferences(false);
                }
                _ => {}
            },
            (Screen::Settings, Focus::SettingsLooks) | (Screen::Settings, Focus::SettingsTheme) => {
                self.previous_screen = self.screen;
                self.previous_focus = self.focus;
                self.screen = Screen::SettingsDetail;
                self.focus = Focus::SettingsSwitchObject;
            }
            (Screen::Settings, Focus::SettingsAbout) => {
                self.previous_screen = Screen::Settings;
                self.previous_focus = Focus::SettingsAbout;
                self.screen = Screen::About;
                self.focus = Focus::SettingsAbout;
            }
            (Screen::Settings, Focus::SettingsConnectivity) => {
                self.previous_screen = Screen::Settings;
                self.previous_focus = Focus::SettingsConnectivity;
                self.start_library_scan();
                self.screen = Screen::About;
                self.focus = Focus::SettingsAbout;
            }
            (Screen::Settings, Focus::SettingsGetUpdate) => {
                self.previous_screen = Screen::Settings;
                self.previous_focus = Focus::SettingsGetUpdate;
                self.run_usb_update();
                self.screen = Screen::About;
                self.focus = Focus::SettingsAbout;
            }
            (Screen::Settings, Focus::SettingsUser) => {
                self.previous_screen = Screen::Settings;
                self.previous_focus = Focus::SettingsUser;
                self.run_usb_sync();
                self.screen = Screen::About;
                self.focus = Focus::SettingsAbout;
            }
            (Screen::About, _) => {}
            (Screen::Language, _) => {
                self.save_preferences(false);
                if self.previous_screen == Screen::SettingsDetail {
                    self.screen = Screen::SettingsDetail;
                    self.focus = Focus::SettingsLanguage;
                } else if self.previous_screen == Screen::Settings {
                    self.screen = Screen::Settings;
                    self.focus = Focus::SettingsGeneral;
                } else {
                    self.screen = Screen::First;
                    self.focus = Focus::GetStarted;
                }
            }
            (Screen::TestComplete, _) => self.screen = Screen::Welcome,
            _ => {}
        }
    }

    fn play_home_playlist(&mut self, index: usize) {
        if let Ok(playlists) = self.db.get_playlists() {
            if index < playlists.len() {
                if let Ok(tracks) = self.db.get_playlist_tracks(playlists[index].id) {
                    if !tracks.is_empty() {
                        self.player.set_queue(tracks, 0);
                        self.apply_shuffle_default();
                        self.screen = Screen::NowPlaying;
                        self.focus = Focus::Albums;
                    }
                }
                return;
            }
        }

        if let Ok(tracks) = self.db.get_tracks() {
            if index < tracks.len() {
                self.player.set_queue(tracks, index);
                self.apply_shuffle_default();
                self.screen = Screen::NowPlaying;
                self.focus = Focus::Albums;
            }
        }
    }

    fn library_counts(&self) -> LibraryCounts {
        self.db.library_counts().unwrap_or_default()
    }

    fn apply_shuffle_default(&mut self) {
        if self.shuffle_default_enabled {
            self.player.set_shuffle(true);
        }
    }

    fn current_preferences(&self, setup_complete: bool) -> Preferences {
        let mut preferences = crate::preferences::load();
        let status = self.player.get_status();
        preferences.user_name = self.user_name.clone();
        preferences.device_name = self.device_name.clone();
        preferences.theme = self.active_theme;
        preferences.language = self.active_language;
        preferences.setup_complete |= setup_complete;
        preferences.controls_seen |= self.controls_seen;
        preferences.time_configured = self.time_configured;
        preferences.clock_hour = self.clock_hour;
        preferences.clock_minute = self.clock_minute;
        preferences.clock_month = self.clock_month;
        preferences.clock_day = self.clock_day;
        preferences.clock_year = self.clock_year;
        preferences.brightness = self.brightness;
        preferences.volume = status.volume;
        preferences.eq_preset = self.player.get_eq_preset();
        preferences.audio_backend = self.audio_backend.clone();
        preferences.alsa_device = self.alsa_device.clone();
        preferences.audio_output_mode = self.audio_output_mode.clone();
        preferences.shuffle = self.player.shuffle_enabled();
        preferences.loop_mode = self.player.loop_mode();
        preferences.resume_queue_track_ids = self.player.queue_track_ids();
        preferences.resume_queue_index = self.player.current_index().unwrap_or(0);
        preferences.resume_track_id = status.track.as_ref().map(|track| track.id);
        preferences.resume_position_seconds = status.position;
        preferences.resume_paused = !status.is_playing;
        preferences.library_sort = self.library_sort;
        preferences.library_group = self.library_group;
        preferences.sleep_timeout_minutes = self.sleep_timeout_minutes;
        preferences.dim_timeout_seconds = self.dim_timeout_seconds;
        preferences.motion_mode = self.motion_mode;
        preferences.seek_step_seconds = self.seek_step_seconds;
        preferences.sleep_timer_default_minutes = self.sleep_timer_default_minutes;
        preferences.resume_queue_enabled = self.resume_queue_enabled;
        preferences.shuffle_default_enabled = self.shuffle_default_enabled;
        preferences.first_run_audio_test_complete = self.first_run_audio_test_complete;
        preferences.last_music_focus = self.music_focus_name().to_string();
        preferences.sanitized()
    }

    pub fn save_preferences(&self, setup_complete: bool) {
        let preferences = self.current_preferences(setup_complete);
        if let Err(err) = crate::preferences::save(&preferences) {
            log::warn!("failed to save MusicOS preferences: {err:#}");
        }
    }

    fn music_focus_name(&self) -> &'static str {
        match self.focus {
            Focus::Search | Focus::SearchKeyboard | Focus::SearchResults => "search",
            Focus::Songs | Focus::ShuffleAll => "songs",
            Focus::Artists => "artists",
            Focus::Playlists => "playlists",
            Focus::Favorites => "favorites",
            Focus::Queue => "queue",
            Focus::History => "history",
            _ => "albums",
        }
    }

    fn publish_runtime_snapshot(&self) {
        crate::server::publish_runtime_snapshot(RuntimeSnapshot {
            settings: self.current_preferences(false),
            player: self.player.get_status(),
            queue: self.player.queue_tracks(),
            queue_index: self.player.current_index(),
            counts: self.library_counts(),
            db_path: self.db_path.clone(),
            settings_path: crate::preferences::path().display().to_string(),
            scan_status: self.scan_status.clone(),
            scan: self.scan.clone(),
            usb_status: self.usb_status.message.clone(),
            battery_status: crate::system::battery_summary(),
            indexed_roots: crate::media::last_scan_roots(),
            cover_art_cache_count: self.db.cover_art_cache_count().unwrap_or_default(),
            cover_art_memory_cache_count: crate::media::cover_art_memory_cache_len(),
            frame_ms: self.last_frame_ms,
        });
    }

    fn apply_settings_update(&mut self, update: SettingsUpdate) {
        if let Some(value) = update.user_name {
            let value = value.trim();
            if !value.is_empty() {
                self.user_name = value.to_string();
            }
        }
        if let Some(value) = update.device_name {
            let value = value.trim();
            if !value.is_empty() {
                self.device_name = value.to_string();
            }
        }
        if let Some(value) = update.theme {
            self.active_theme = ThemeName::from_str(&value);
        }
        if let Some(value) = update.language {
            self.active_language = Language::from_str(&value);
        }
        if let Some(value) = update.audio_backend {
            self.audio_backend = match value.to_ascii_lowercase().as_str() {
                "alsa" => "alsa".to_string(),
                "rodio" => "rodio".to_string(),
                "none" | "silent" => "none".to_string(),
                _ => "auto".to_string(),
            };
        }
        if let Some(value) = update.alsa_device {
            self.alsa_device = value.trim().to_string();
        }
        if let Some(value) = update.audio_output_mode {
            let value = value.trim();
            if !value.is_empty() {
                self.audio_output_mode = value.to_string();
            }
        }
        if let Some(value) = update.brightness {
            self.brightness = value.clamp(5, 100);
            crate::system::apply_brightness(self.brightness);
        }
        if let Some(value) = update.volume {
            self.player.set_volume(value);
        }
        if let Some(value) = update.eq_preset {
            self.player.set_eq_preset(eq_preset_from_str(&value));
        }
        if let Some(value) = update.shuffle {
            self.player.set_shuffle(value);
        }
        if let Some(value) = update.loop_mode {
            self.player.set_loop_mode(loop_mode_from_str(&value));
        }
        if let Some(value) = update.library_sort {
            self.library_sort = LibrarySort::from_str(&value);
            self.sort_library();
        }
        if let Some(value) = update.library_group {
            self.library_group = LibraryGroup::from_str(&value);
        }
        if let Some(value) = update.sleep_timeout_minutes {
            self.sleep_timeout_minutes = value.min(180);
        }
        if let Some(value) = update.dim_timeout_seconds {
            self.dim_timeout_seconds = value.clamp(10, 600);
        }
        if let Some(value) = update.motion_mode {
            self.motion_mode = MotionMode::from_str(&value);
        }
        if let Some(value) = update.seek_step_seconds {
            self.seek_step_seconds = value.clamp(5, 60);
        }
        if let Some(value) = update.sleep_timer_default_minutes {
            self.sleep_timer_default_minutes = value.min(180);
        }
        if let Some(value) = update.resume_queue_enabled {
            self.resume_queue_enabled = value;
        }
        if let Some(value) = update.shuffle_default_enabled {
            self.shuffle_default_enabled = value;
        }
        if let Some(value) = update.first_run_audio_test_complete {
            self.first_run_audio_test_complete = value;
        }
        let preferences = self.current_preferences(false);
        apply_audio_runtime_preferences(&preferences);
        self.save_preferences(false);
    }

    fn reset_user_settings(&mut self) {
        let defaults = Preferences::default();
        self.user_name = defaults.user_name;
        self.device_name = defaults.device_name;
        self.active_theme = defaults.theme;
        self.active_language = defaults.language;
        self.audio_backend = defaults.audio_backend;
        self.alsa_device = defaults.alsa_device;
        self.audio_output_mode = defaults.audio_output_mode;
        self.library_sort = defaults.library_sort;
        self.library_group = defaults.library_group;
        self.sleep_timeout_minutes = defaults.sleep_timeout_minutes;
        self.dim_timeout_seconds = defaults.dim_timeout_seconds;
        self.motion_mode = defaults.motion_mode;
        self.seek_step_seconds = defaults.seek_step_seconds;
        self.sleep_timer_default_minutes = defaults.sleep_timer_default_minutes;
        self.resume_queue_enabled = defaults.resume_queue_enabled;
        self.shuffle_default_enabled = defaults.shuffle_default_enabled;
        self.first_run_audio_test_complete = false;
        self.brightness = defaults.brightness;
        self.player.set_volume(defaults.volume);
        self.player.set_eq_preset(defaults.eq_preset);
        self.player.set_shuffle(defaults.shuffle);
        self.player.set_loop_mode(defaults.loop_mode);
        crate::system::apply_brightness(self.brightness);
        apply_audio_runtime_preferences(&self.current_preferences(false));
        self.save_preferences(false);
        self.scan_status = "Settings reset; media library preserved".to_string();
    }

    fn scan_library(&mut self) {
        self.start_library_scan();
    }

    fn run_audio_test(&mut self) {
        let result = crate::system::run_audio_test();
        self.audio_test_status = result.message;
        self.first_run_audio_test_complete = result.ok;
        self.save_preferences(false);
    }

    pub fn refresh_library(&mut self) {
        if let Ok(tracks) = self.db.get_tracks() {
            (self.album_previews, self.recent_albums) = ui::home::album_previews(&tracks);
            self.home_album_mosaic = ui::home::album_mosaic(&self.album_previews);
            self.home_artist_portrait = ui::home::artist_preview(&tracks);
            self.library_view.update(&tracks);
            self.library_tracks = tracks;
        }
        if let Ok(albums) = self.db.get_albums() {
            self.albums_list = albums;
        }
        if let Ok(artists) = self.db.get_artists() {
            self.artists_list = artists;
        }
        if let Ok(playlists) = self.db.get_playlists() {
            self.playlists_list = playlists;
        }
        if let Ok(pins) = self.db.get_pins() {
            self.pins_list = pins;
        }
        self.cached_library_counts = self.db.library_counts().unwrap_or_default();
        sort_tracks(&mut self.library_tracks, self.library_sort);
        if !self.search_query.trim().is_empty() {
            self.refresh_search_results();
        }
    }

    fn sort_library(&mut self) {
        sort_tracks(&mut self.library_tracks, self.library_sort);
    }

    pub fn run_usb_sync(&mut self) {
        self.usb_status = match crate::usb::sync_from_usb(&self.db) {
            Ok(status) => status,
            Err(err) => UsbStatus {
                message: format!("USB sync failed: {err:#}"),
                ..UsbStatus::default()
            },
        };

        self.refresh_library();
    }

    pub fn run_usb_update(&mut self) {
        self.usb_status = match crate::usb::get_update_from_usb() {
            Ok(status) => status,
            Err(err) => UsbStatus {
                message: format!("USB update failed: {err:#}"),
                ..UsbStatus::default()
            },
        };
    }

    fn button_legend(&self) -> &'static str {
        match (self.screen, self.focus) {
            (Screen::First, _) => "D-pad  Move     A  Select",
            (Screen::Theme, _) => "D-pad  Change theme     A  Select     B  Back",
            (Screen::DateTimeSetup, _) => "D-pad  Adjust     A  Next     B  Back",
            (Screen::RatingPicker, _) => "Left / Right  Rate     A  Save     B  Back",
            (Screen::Welcome | Screen::TestComplete, _) => "A  Continue     B  Back",
            (Screen::About, _) => "B  Back",
            (Screen::ContextMenu | Screen::SettingsDetail, _) => "A  Select     B  Back",
            (Screen::NowPlaying, _) if self.can_open_context_menu() => {
                "A  Select     B  Back     X  More"
            }
            (Screen::NowPlaying, _) => "B  Back",
            (Screen::Search, Focus::SearchKeyboard)
                if self.can_open_search_results_with_context() =>
            {
                "D-pad  Move     A  Type     B  Back     X  Results"
            }
            (Screen::Search, Focus::SearchKeyboard) => "D-pad  Move     A  Type     B  Back",
            (Screen::Search, Focus::SearchResults) if self.can_open_context_menu() => {
                "A  Play     B  Keyboard     X  More"
            }
            (Screen::Search, Focus::SearchResults) => "B  Keyboard",
            (Screen::Home, _) if self.can_open_home_preview_with_context() => {
                "A  Select     B  Back     X  More"
            }
            (Screen::Home, _) => "A  Select     B  Back",
            (Screen::Language | Screen::Apps | Screen::Settings, _) => {
                "D-pad  Move     A  Select     B  Back"
            }
            (Screen::MusicApp, _) => "A  Select     B  Back",
            (Screen::ArtistAlbumsList, _) => "D-pad  Move     A  Open     B  Back",
            (Screen::AlbumsList | Screen::ArtistsList, _) => {
                if self.can_open_context_menu() {
                    "D-pad  Move     A  Open     B  Back     X  More"
                } else {
                    "D-pad  Move     A  Open     B  Back"
                }
            }
            (
                Screen::AlbumTracksList
                | Screen::SongsList
                | Screen::PlaylistsList
                | Screen::PlaylistTracksList
                | Screen::Queue,
                _,
            ) => {
                if self.can_open_context_menu() {
                    "A  Select     B  Back     X  More"
                } else {
                    "A  Select     B  Back"
                }
            }
            _ => "",
        }
    }

    fn can_open_home_preview_with_context(&self) -> bool {
        self.screen == Screen::Home
            && self.home_navigation.pane == HomePane::Main
            && matches!(
                HOME_ROWS[self.home_navigation.main_index],
                Focus::HomeMusic | Focus::HomeNowPlaying | Focus::Artists | Focus::Playlists
            )
            && self.home_detail_item_count() > 0
    }

    fn can_open_search_results_with_context(&self) -> bool {
        self.screen == Screen::Search
            && self.focus == Focus::SearchKeyboard
            && !self.search_query.trim().is_empty()
    }

    fn can_open_context_menu(&self) -> bool {
        match self.screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => self.tracks_queue.get(self.selected_index).is_some(),
            Screen::Search if self.focus == Focus::SearchResults => {
                self.search_results.get(self.selected_index).is_some()
            }
            Screen::AlbumsList => self
                .albums_list
                .get(self.selected_index)
                .is_some_and(|album| {
                    self.db
                        .has_tracks_in_collection(Some(album), None)
                        .unwrap_or(false)
                }),
            Screen::ArtistsList => {
                self.artists_list
                    .get(self.selected_index)
                    .is_some_and(|artist| {
                        self.db
                            .has_tracks_in_collection(None, Some(artist))
                            .unwrap_or(false)
                    })
            }
            Screen::PlaylistsList => self.playlists_list.get(self.selected_index).is_some(),
            Screen::Home => false,
            Screen::NowPlaying => self.player.get_current_track().is_some(),
            _ => false,
        }
    }

    fn context_actions(&self) -> Vec<(Focus, String, bool)> {
        if self.previous_screen == Screen::Queue {
            return vec![
                (Focus::ContextPlay, "Play".to_string(), false),
                (
                    Focus::ContextPlayNext,
                    "Move to Play Next".to_string(),
                    false,
                ),
                (Focus::ContextAddLater, "Move to End".to_string(), false),
                (Focus::ContextMoveUp, "Move Up".to_string(), false),
                (Focus::ContextMoveDown, "Move Down".to_string(), false),
                (Focus::ContextRate, "Rate Track".to_string(), false),
                (Focus::ContextDelete, "Remove from Queue".to_string(), true),
                (
                    Focus::ContextClearUpcoming,
                    "Clear Upcoming".to_string(),
                    true,
                ),
                (Focus::ContextCancel, "Cancel".to_string(), false),
            ];
        }

        if self.context_selected_track().is_some() {
            let mut actions = vec![
                (Focus::ContextPlay, "Play".to_string(), false),
                (Focus::ContextPlayNext, "Play Next".to_string(), false),
                (Focus::ContextAddLater, "Add to Up Next".to_string(), false),
                (
                    Focus::ContextPin,
                    if self.context_item_is_pinned() {
                        "Unpin".to_string()
                    } else {
                        "Pin".to_string()
                    },
                    false,
                ),
                (
                    Focus::ContextAddToPlaylist,
                    "Add to Playlist".to_string(),
                    false,
                ),
                (Focus::ContextRate, "Rate Track".to_string(), false),
            ];
            if self.previous_screen == Screen::PlaylistTracksList {
                actions.push((
                    Focus::ContextDelete,
                    "Remove from Playlist".to_string(),
                    true,
                ));
            }
            actions.push((Focus::ContextCancel, "Cancel".to_string(), false));
            return actions;
        }

        if self.previous_screen == Screen::PlaylistsList {
            return vec![
                (Focus::ContextPlay, "Play".to_string(), false),
                (
                    Focus::ContextPin,
                    if self.context_item_is_pinned() {
                        "Unpin".to_string()
                    } else {
                        "Pin".to_string()
                    },
                    false,
                ),
                (Focus::ContextDelete, "Delete Playlist".to_string(), true),
                (Focus::ContextCancel, "Cancel".to_string(), false),
            ];
        }

        vec![
            (Focus::ContextPlay, "Play".to_string(), false),
            (
                Focus::ContextAddToPlaylist,
                "Add to Playlist".to_string(),
                false,
            ),
            (Focus::ContextShuffle, "Shuffle".to_string(), false),
            (Focus::ContextCancel, "Cancel".to_string(), false),
        ]
    }

    fn context_selected_track(&self) -> Option<Track> {
        match self.previous_screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => self.tracks_queue.get(self.selected_index).cloned(),
            Screen::Search => self.search_results.get(self.selected_index).cloned(),
            Screen::NowPlaying => self.player.get_current_track().cloned(),
            _ => None,
        }
    }

    fn move_context_focus(&mut self, delta: isize) {
        let actions = self.context_actions();
        if actions.is_empty() {
            return;
        }
        let current = actions
            .iter()
            .position(|(focus, _, _)| *focus == self.focus)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(actions.len() as isize) as usize;
        self.focus = actions[next].0;
    }

    fn close_context(&mut self) {
        self.screen = self.previous_screen;
        self.focus = self.previous_focus;
    }

    fn open_rating_picker(&mut self) {
        let Some(track) = self.context_selected_track() else {
            self.close_context();
            return;
        };
        self.close_context();
        self.push_navigation();
        self.rating_track_id = Some(track.id);
        self.pending_rating = track.rating;
        self.screen = Screen::RatingPicker;
        self.focus = Focus::RatingStars;
    }

    fn apply_context_queue_action(&mut self, action: Focus) {
        let Some(track) = self.context_selected_track() else {
            self.close_context();
            return;
        };
        if self.previous_screen == Screen::Queue {
            let selected = self.selected_index;
            match action {
                Focus::ContextPlayNext => {
                    let to = self
                        .player
                        .current_index()
                        .map(|index| (index + 1).min(self.tracks_queue.len().saturating_sub(1)))
                        .unwrap_or(0);
                    let _ = self.player.queue_move(selected, to);
                    self.selected_index = to;
                }
                Focus::ContextAddLater => {
                    let to = self.tracks_queue.len().saturating_sub(1);
                    let _ = self.player.queue_move(selected, to);
                    self.selected_index = to;
                }
                _ => {}
            }
        } else if action == Focus::ContextPlayNext {
            self.player.queue_add_next(track);
        } else {
            self.player.queue_add_later(track);
        }
        if self.previous_screen == Screen::Queue {
            self.tracks_queue = self.player.queue_tracks();
        }
        self.save_preferences(false);
        self.close_context();
    }

    fn home_detail_item_count(&self) -> usize {
        match HOME_ROWS[self.home_navigation.main_index] {
            Focus::HomeMusic | Focus::Artists => usize::from(!self.library_tracks.is_empty()),
            Focus::Playlists => self.playlists_list.len().min(4),
            Focus::HomeNowPlaying => {
                usize::from(
                    self.player.get_current_track().is_some()
                        || !self.library_tracks.is_empty()
                        || !self.tracks_queue.is_empty(),
                ) * 4
            }
            Focus::HomeApps | Focus::HomeSettings => 4,
            _ => 0,
        }
    }

    fn home_detail_focus_index(focus: Focus) -> Option<usize> {
        match focus {
            Focus::HomePlaylist1 => Some(0),
            Focus::HomePlaylist2 => Some(1),
            Focus::HomePlaylist3 => Some(2),
            Focus::HomePlaylist4 => Some(3),
            _ => None,
        }
    }

    fn home_detail_focus_for_index(index: usize) -> Focus {
        match index.min(3) {
            0 => Focus::HomePlaylist1,
            1 => Focus::HomePlaylist2,
            2 => Focus::HomePlaylist3,
            _ => Focus::HomePlaylist4,
        }
    }

    fn sync_home_navigation_from_focus(&mut self) {
        let main_index = HOME_ROWS.iter().position(|&focus| focus == self.focus);
        if let Some(main_index) = main_index {
            self.home_navigation.pane = HomePane::Main;
            self.home_navigation.main_index = main_index;
        } else if let Some(detail_index) = Self::home_detail_focus_index(self.focus) {
            self.home_navigation.pane = HomePane::Detail;
            self.home_navigation.detail_index = detail_index;
        }
    }

    fn move_home_main(&mut self, delta: isize) {
        self.home_navigation.pane = HomePane::Main;
        self.home_navigation.main_index = (self.home_navigation.main_index as isize + delta)
            .rem_euclid(HOME_ROWS.len() as isize)
            as usize;
        self.focus = HOME_ROWS[self.home_navigation.main_index];
    }

    fn move_home_detail(&mut self, delta: isize) {
        let count = self.home_detail_item_count();
        if count == 0 {
            self.leave_home_detail();
            return;
        }
        let current = self.home_navigation.detail_index.min(count - 1);
        let next = (current as isize + delta).rem_euclid(count as isize) as usize;
        self.home_navigation.detail_index = next;
        self.home_navigation.remembered_detail_index[self.home_navigation.main_index] = next;
        self.focus = Self::home_detail_focus_for_index(next);
    }

    fn enter_home_detail(&mut self) {
        if self.home_navigation.pane != HomePane::Main {
            return;
        }
        let count = self.home_detail_item_count();
        if count == 0 {
            return;
        }
        let index = self.home_navigation.remembered_detail_index[self.home_navigation.main_index]
            .min(count - 1);
        self.home_navigation.pane = HomePane::Detail;
        self.home_navigation.detail_index = index;
        self.focus = Self::home_detail_focus_for_index(index);
    }

    fn leave_home_detail(&mut self) {
        if self.home_navigation.pane != HomePane::Detail {
            return;
        }
        self.home_navigation.remembered_detail_index[self.home_navigation.main_index] =
            self.home_navigation.detail_index;
        self.home_navigation.pane = HomePane::Main;
        self.focus = HOME_ROWS[self.home_navigation.main_index];
    }

    fn activate_home_detail(&mut self) {
        let index = self.home_navigation.detail_index;
        match HOME_ROWS[self.home_navigation.main_index] {
            Focus::HomeMusic | Focus::Artists => {
                if let Some((tracks, start)) = self.all_tracks_queue() {
                    self.push_navigation();
                    self.player.set_queue(tracks, start);
                    self.player.set_shuffle(true);
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                }
            }
            Focus::HomeNowPlaying => match index {
                0 => self.player.play_pause(),
                1 => self.player.prev(),
                2 => self.player.next(),
                _ => {
                    self.push_navigation();
                    self.previous_screen = Screen::Home;
                    self.screen = Screen::Queue;
                    self.focus = Focus::Queue;
                    self.tracks_queue = self.player.queue_tracks();
                    self.selected_index = self.player.current_index().unwrap_or(0);
                }
            },
            Focus::Playlists => self.play_home_playlist(index),
            Focus::HomeApps => match index {
                0 => {
                    self.push_navigation();
                    self.screen = Screen::MusicApp;
                    self.focus = Focus::Albums;
                }
                1 => {
                    self.push_navigation();
                    self.screen = Screen::NowPlaying;
                    self.focus = Focus::Albums;
                }
                2 => {
                    self.run_usb_sync();
                }
                _ => {
                    self.screen = Screen::Apps;
                    self.focus = Focus::AppsUsbSync;
                }
            },
            Focus::HomeSettings => {
                self.previous_screen = Screen::Home;
                self.screen = if index == 3 {
                    Screen::Settings
                } else {
                    Screen::SettingsDetail
                };
                self.focus = match index {
                    0 => Focus::SettingsLanguage,
                    1 => Focus::SettingsVolume,
                    2 => Focus::SettingsLooks,
                    _ => Focus::SettingsGetUpdate,
                };
            }
            _ => {}
        }
        self.save_preferences(false);
    }

    fn context_pin_target(&self) -> Option<(&'static str, i64)> {
        match self.previous_screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => self
                .tracks_queue
                .get(self.selected_index)
                .map(|track| ("track", track.id)),
            Screen::Search => self
                .search_results
                .get(self.selected_index)
                .map(|track| ("track", track.id)),
            Screen::PlaylistsList => self
                .playlists_list
                .get(self.selected_index)
                .map(|playlist| ("playlist", playlist.id)),
            Screen::Home => match self.previous_focus {
                Focus::HomePlaylist1
                | Focus::HomePlaylist2
                | Focus::HomePlaylist3
                | Focus::HomePlaylist4 => self
                    .context_home_playlist()
                    .map(|playlist| ("playlist", playlist.id)),
                Focus::HomeNowPlaying => self
                    .player
                    .get_current_track()
                    .map(|track| ("track", track.id)),
                _ => None,
            },
            Screen::NowPlaying => self
                .player
                .get_current_track()
                .map(|track| ("track", track.id)),
            _ => None,
        }
    }

    pub fn context_item_is_pinned(&self) -> bool {
        if let Some((pin_type, target_id)) = self.context_pin_target() {
            self.db.is_pinned(pin_type, target_id).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn context_menu_title(&self) -> String {
        match self.previous_screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => self
                .tracks_queue
                .get(self.selected_index)
                .map(|track| track.title.clone())
                .unwrap_or_else(|| "Track".to_string()),
            Screen::Search => self
                .search_results
                .get(self.selected_index)
                .map(|track| track.title.clone())
                .unwrap_or_else(|| "Search Result".to_string()),
            Screen::ArtistAlbumsList => self
                .artist_albums_list
                .get(self.selected_index)
                .cloned()
                .unwrap_or_else(|| "Album".to_string()),
            Screen::PlaylistsList => self
                .playlists_list
                .get(self.selected_index)
                .map(|playlist| playlist.name.clone())
                .unwrap_or_else(|| "Playlist".to_string()),
            Screen::AlbumsList => self
                .albums_list
                .get(self.selected_index)
                .cloned()
                .unwrap_or_else(|| "Album".to_string()),
            Screen::ArtistsList => self
                .artists_list
                .get(self.selected_index)
                .cloned()
                .unwrap_or_else(|| "Artist".to_string()),
            Screen::NowPlaying => self
                .player
                .get_current_track()
                .map(|track| track.title.clone())
                .unwrap_or_else(|| "Track".to_string()),
            Screen::Home => match self.previous_focus {
                Focus::HomePlaylist1
                | Focus::HomePlaylist2
                | Focus::HomePlaylist3
                | Focus::HomePlaylist4 => self
                    .context_home_playlist()
                    .map(|playlist| playlist.name)
                    .unwrap_or_else(|| "Playlist".to_string()),
                Focus::HomeNowPlaying => self
                    .player
                    .get_current_track()
                    .map(|track| track.title.clone())
                    .unwrap_or_else(|| "Now Playing".to_string()),
                Focus::HomeMusic => "Music".to_string(),
                _ => "Home".to_string(),
            },
            _ => self
                .player
                .get_current_track()
                .map(|track| track.title.clone())
                .unwrap_or_else(|| "Item".to_string()),
        }
    }

    fn activate_context_play(&mut self, shuffle: bool) {
        if self.context_targets_current_player() {
            if shuffle {
                self.player.set_shuffle(true);
            }
            self.player.play_pause();
            self.screen = self.previous_screen;
            self.focus = self.previous_focus;
            return;
        }

        let queue_and_index = self.context_queue();

        if let Some((tracks, index)) = queue_and_index {
            if let Some(track) = tracks.get(index) {
                let _ = self.db.increment_play_count(track.id);
            }
            self.close_context();
            self.push_navigation();
            if shuffle {
                self.player.set_shuffle(true);
            }
            self.player.set_queue(tracks, index);
            if !shuffle {
                self.apply_shuffle_default();
            }
            self.screen = Screen::NowPlaying;
            self.focus = Focus::Albums;
        } else {
            self.screen = self.previous_screen;
            self.focus = self.previous_focus;
        }
    }

    fn context_queue(&self) -> Option<(Vec<Track>, usize)> {
        match self.previous_screen {
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistTracksList
            | Screen::Queue => {
                if self.selected_index < self.tracks_queue.len() {
                    Some((self.tracks_queue.clone(), self.selected_index))
                } else {
                    None
                }
            }
            Screen::Search => {
                if self.selected_index < self.search_results.len() {
                    Some((self.search_results.clone(), self.selected_index))
                } else {
                    None
                }
            }
            Screen::ArtistAlbumsList => {
                let artist = self.active_artist.as_deref()?;
                let album = self.artist_albums_list.get(self.selected_index)?;
                self.db
                    .get_artist_album_tracks(artist, album)
                    .ok()
                    .filter(|tracks| !tracks.is_empty())
                    .map(|tracks| (tracks, 0))
            }
            Screen::AlbumsList => self
                .albums_list
                .get(self.selected_index)
                .and_then(|album| self.filtered_tracks(|track| track.album == *album)),
            Screen::ArtistsList => self
                .artists_list
                .get(self.selected_index)
                .and_then(|artist| self.filtered_tracks(|track| track.artist == *artist)),
            Screen::PlaylistsList => self.playlists_list.get(self.selected_index).and_then(|p| {
                self.db
                    .get_playlist_tracks(p.id)
                    .ok()
                    .filter(|tracks| !tracks.is_empty())
                    .map(|tracks| (tracks, 0))
            }),
            Screen::Home => self.context_home_queue(),
            Screen::MusicApp => self.all_tracks_queue(),
            Screen::NowPlaying => self
                .player
                .get_current_track()
                .map(|track| (vec![track.clone()], 0)),
            _ => None,
        }
    }

    fn all_tracks_queue(&self) -> Option<(Vec<Track>, usize)> {
        self.db
            .get_tracks()
            .ok()
            .filter(|tracks| !tracks.is_empty())
            .map(|tracks| (tracks, 0))
    }

    fn context_home_queue(&self) -> Option<(Vec<Track>, usize)> {
        match self.previous_focus {
            Focus::HomeMusic => self.all_tracks_queue(),
            Focus::HomePlaylist1
            | Focus::HomePlaylist2
            | Focus::HomePlaylist3
            | Focus::HomePlaylist4 => self.context_home_playlist().and_then(|playlist| {
                self.db
                    .get_playlist_tracks(playlist.id)
                    .ok()
                    .filter(|tracks| !tracks.is_empty())
                    .map(|tracks| (tracks, 0))
            }),
            Focus::HomeNowPlaying => self
                .player
                .get_current_track()
                .map(|track| (vec![track.clone()], 0)),
            _ => None,
        }
    }

    fn context_targets_current_player(&self) -> bool {
        self.previous_screen == Screen::NowPlaying
            || (self.previous_screen == Screen::Home
                && self.previous_focus == Focus::HomeNowPlaying)
    }

    fn context_home_playlist(&self) -> Option<Playlist> {
        let index = match self.previous_focus {
            Focus::HomePlaylist1 => 0,
            Focus::HomePlaylist2 => 1,
            Focus::HomePlaylist3 => 2,
            Focus::HomePlaylist4 => 3,
            _ => return None,
        };
        self.db.get_playlists().ok()?.into_iter().nth(index)
    }

    fn filtered_tracks<F>(&self, predicate: F) -> Option<(Vec<Track>, usize)>
    where
        F: Fn(&Track) -> bool,
    {
        let mut tracks = self.db.get_tracks().ok()?;
        tracks.retain(predicate);
        if tracks.is_empty() {
            None
        } else {
            Some((tracks, 0))
        }
    }

    fn add_context_track_to_default_playlist(&mut self) {
        let track_ids = self.context_track_ids();
        if track_ids.is_empty() {
            return;
        }

        let playlist_id = self.context_adds_playlist_id();

        if let Some(playlist_id) = playlist_id {
            for track_id in track_ids {
                let _ = self.db.add_track_to_playlist(playlist_id, track_id);
            }
            self.refresh_library();
        }
    }

    fn context_track_ids(&self) -> Vec<i64> {
        if let Some((pin_type, id)) = self.context_pin_target() {
            if pin_type == "track" {
                return vec![id];
            }
        }
        self.context_queue()
            .map(|(tracks, _)| tracks.into_iter().map(|track| track.id).collect())
            .unwrap_or_default()
    }

    fn context_adds_playlist_id(&self) -> Option<i64> {
        self.db
            .get_playlists()
            .ok()
            .and_then(|playlists| {
                playlists
                    .into_iter()
                    .find(|playlist| playlist.name == "Context Adds")
                    .map(|playlist| playlist.id)
            })
            .or_else(|| self.db.create_playlist("Context Adds").ok())
    }

    fn delete_context_target(&mut self) {
        match self.previous_screen {
            Screen::PlaylistsList => {
                if let Some(playlist) = self.playlists_list.get(self.selected_index) {
                    let _ = self.db.delete_playlist(playlist.id);
                }
                if let Ok(playlists) = self.db.get_playlists() {
                    self.playlists_list = playlists;
                    self.selected_index = self
                        .selected_index
                        .min(self.playlists_list.len().saturating_sub(1));
                }
            }
            Screen::PlaylistTracksList => {
                if let (Some(playlist_id), Some(track_id)) = (
                    self.active_playlist.as_ref().map(|playlist| playlist.id),
                    self.tracks_queue
                        .get(self.selected_index)
                        .map(|track| track.id),
                ) {
                    let _ = self.db.remove_track_from_playlist(playlist_id, track_id);
                    if let Ok(tracks) = self.db.get_playlist_tracks(playlist_id) {
                        self.tracks_queue = tracks;
                        self.selected_index = self
                            .selected_index
                            .min(self.tracks_queue.len().saturating_sub(1));
                    }
                }
            }
            Screen::Queue => {
                self.player.queue_remove(self.selected_index);
                self.tracks_queue = self.player.queue_tracks();
                self.selected_index = self
                    .selected_index
                    .min(self.tracks_queue.len().saturating_sub(1));
                self.save_preferences(false);
            }
            Screen::Home => {
                if let Some(playlist) = self.context_home_playlist() {
                    let _ = self.db.delete_playlist(playlist.id);
                    self.refresh_library();
                }
            }
            _ => {}
        }
    }

    fn back(&mut self) {
        match self.screen {
            Screen::First => {}
            Screen::Welcome => self.screen = Screen::First,
            Screen::Theme => {
                if self.previous_screen == Screen::Settings {
                    self.screen = Screen::Settings;
                    self.focus = Focus::SettingsLooks;
                } else {
                    self.screen = Screen::First;
                    self.focus = Focus::GetStarted;
                }
            }
            Screen::DateTimeSetup => {
                if self.previous_screen == Screen::SettingsDetail {
                    self.screen = Screen::SettingsDetail;
                    self.focus = Focus::SettingsDateTime;
                } else {
                    self.screen = Screen::Theme;
                    self.focus = Focus::ThemeSelect;
                }
            }
            Screen::Home => {
                if self.home_navigation.pane == HomePane::Detail {
                    self.leave_home_detail();
                }
            }
            Screen::MusicApp => {
                if self.pop_navigation() {
                    return;
                } else if self.previous_screen == Screen::Apps {
                    self.screen = Screen::Apps;
                    self.focus = Focus::AppsMusic;
                } else {
                    self.screen = Screen::Home;
                    self.focus = Focus::HomeMusic;
                }
                self.reset_list_state();
            }
            Screen::SongsList => {
                if !self.pop_navigation() {
                    self.screen = Screen::MusicApp;
                    self.focus = Focus::Songs;
                }
            }
            Screen::AlbumsList => {
                if !self.pop_navigation() {
                    self.screen = Screen::MusicApp;
                    self.focus = Focus::Albums;
                }
            }
            Screen::ArtistsList => {
                if !self.pop_navigation() {
                    self.screen = Screen::MusicApp;
                    self.focus = Focus::Artists;
                }
            }
            Screen::ArtistAlbumsList | Screen::AlbumTracksList => {
                self.pop_navigation();
            }
            Screen::PlaylistsList => {
                if !self.pop_navigation() {
                    self.screen = Screen::MusicApp;
                    self.focus = Focus::Playlists;
                }
            }
            Screen::PlaylistTracksList => {
                if !self.pop_navigation() {
                    self.screen = Screen::PlaylistsList;
                    self.selected_index = 0;
                }
            }
            Screen::Queue => {
                if !self.pop_navigation() {
                    self.screen = Screen::MusicApp;
                    self.focus = Focus::Queue;
                }
            }
            Screen::Search => {
                if self.focus == Focus::SearchResults {
                    self.focus = Focus::SearchKeyboard;
                } else if !self.search_query.is_empty() {
                    self.search_query.pop();
                    self.refresh_search_results();
                } else {
                    self.pop_navigation();
                }
            }
            Screen::RatingPicker => {
                self.pop_navigation();
            }
            Screen::Language => {
                if self.previous_screen == Screen::SettingsDetail {
                    self.screen = Screen::SettingsDetail;
                    self.focus = Focus::SettingsLanguage;
                } else if self.previous_screen == Screen::Settings {
                    self.screen = Screen::Settings;
                    self.focus = Focus::SettingsGeneral;
                } else {
                    self.screen = Screen::First;
                    self.focus = Focus::Language;
                }
            }
            Screen::NowPlaying => {
                if self.pop_navigation() {
                    return;
                } else if self.previous_screen == Screen::Apps {
                    self.screen = Screen::Apps;
                    self.focus = Focus::AppsNowPlaying;
                } else {
                    self.screen = Screen::Home;
                    self.focus = Focus::HomeNowPlaying;
                }
            }
            Screen::Settings => {
                if self.previous_screen == Screen::Apps {
                    self.screen = Screen::Apps;
                    self.focus = Focus::AppsSettings;
                } else {
                    self.screen = Screen::Home;
                    self.focus = Focus::HomeSettings;
                }
            }
            Screen::SettingsDetail => {
                self.screen = Screen::Settings;
                self.focus = if self.previous_screen == Screen::Settings {
                    self.previous_focus
                } else {
                    Focus::SettingsGeneral
                };
            }
            Screen::Apps => {
                self.screen = Screen::Home;
                self.focus = Focus::HomeApps;
            }
            Screen::About => {
                if self.previous_screen == Screen::Apps {
                    self.screen = Screen::Apps;
                    self.focus = Focus::AppsUsbSync;
                } else {
                    self.screen = Screen::Settings;
                    self.focus = self.previous_focus;
                }
            }
            Screen::TestComplete => self.screen = Screen::Welcome,
            Screen::ContextMenu => {
                self.close_context();
            }
        }
    }

    fn move_library_grid(&mut self, row: isize, col: isize) {
        let len = self.current_list_len();
        if len == 0 {
            return;
        }
        let selected = self.selected_index.min(len - 1);
        let current_row = selected / COLUMNS;
        let current_col = selected % COLUMNS;
        let next_row = current_row
            .saturating_add_signed(row)
            .min((len - 1) / COLUMNS);
        let first = next_row * COLUMNS;
        let last_col = (len - 1 - first).min(COLUMNS - 1);
        self.selected_index = first + current_col.saturating_add_signed(col).min(last_col);
    }

    fn move_down(&mut self) {
        match self.screen {
            Screen::Theme => {
                if self.focus == Focus::ThemeSelect {
                    self.focus = Focus::Yeah;
                }
            }
            Screen::First => {
                if self.focus == Focus::GetStarted {
                    self.focus = Focus::Language;
                }
            }
            Screen::Home => {
                if self.home_navigation.pane == HomePane::Detail {
                    self.move_home_detail(1);
                } else {
                    self.move_home_main(1);
                }
            }
            Screen::Apps => {
                self.focus = match self.focus {
                    Focus::AppsMusic => Focus::AppsNowPlaying,
                    Focus::AppsNowPlaying => Focus::AppsSettings,
                    Focus::AppsSettings => Focus::AppsUsbSync,
                    Focus::AppsUsbSync => Focus::AppsMusic,
                    _ => self.focus,
                };
            }
            Screen::MusicApp => {
                self.focus = match self.focus {
                    Focus::Search => Focus::Albums,
                    Focus::Albums => Focus::Songs,
                    Focus::Songs => Focus::Artists,
                    Focus::Artists => Focus::Playlists,
                    Focus::Playlists => Focus::Favorites,
                    Focus::Favorites => Focus::Queue,
                    Focus::Queue => Focus::History,
                    Focus::History => Focus::ShuffleAll,
                    Focus::ShuffleAll => Focus::Search,
                    _ => self.focus,
                };
            }
            Screen::NowPlaying => {
                self.focus = match self.focus {
                    Focus::Artists => Focus::Songs,
                    Focus::Albums => Focus::Favorites,
                    Focus::Playlists => Focus::History,
                    Focus::Songs => Focus::Artists,
                    Focus::Favorites => Focus::Albums,
                    Focus::History => Focus::Playlists,
                    _ => Focus::Albums,
                };
            }
            Screen::SongsList | Screen::AlbumTracksList | Screen::Queue => {
                if !self.tracks_queue.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.tracks_queue.len();
                }
            }
            Screen::AlbumsList | Screen::ArtistsList | Screen::ArtistAlbumsList => {
                self.move_library_grid(1, 0);
            }
            Screen::PlaylistsList => {
                if !self.playlists_list.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.playlists_list.len();
                }
            }
            Screen::PlaylistTracksList => {
                if !self.tracks_queue.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.tracks_queue.len();
                }
            }
            Screen::Language => {
                self.selected_index = (self.selected_index + 1) % 5;
                self.active_language = match self.selected_index {
                    0 => Language::En,
                    1 => Language::Es,
                    2 => Language::De,
                    3 => Language::Ja,
                    4 => Language::Zh,
                    _ => Language::En,
                };
                self.save_preferences(false);
            }
            Screen::Settings => {
                self.focus = match self.focus {
                    Focus::SettingsGeneral => Focus::SettingsAudio,
                    Focus::SettingsAudio => Focus::SettingsLooks,
                    Focus::SettingsLooks => Focus::SettingsConnectivity,
                    Focus::SettingsConnectivity => Focus::SettingsUser,
                    Focus::SettingsUser => Focus::SettingsGetUpdate,
                    Focus::SettingsGetUpdate => Focus::SettingsAbout,
                    Focus::SettingsAbout => Focus::SettingsGeneral,
                    Focus::SettingsTheme => Focus::SettingsGeneral,
                    Focus::SettingsLanguage => Focus::SettingsGeneral,
                    _ => self.focus,
                };
            }
            Screen::SettingsDetail => {
                self.focus = match self.focus {
                    Focus::SettingsLanguage => Focus::SettingsDateTime,
                    Focus::SettingsDateTime => Focus::SettingsBrightness,
                    Focus::SettingsBrightness => Focus::SettingsVolume,
                    Focus::SettingsVolume => Focus::SettingsEq,
                    Focus::SettingsEq => Focus::SettingsSwitchObject,
                    Focus::SettingsSwitchObject => Focus::SettingsLanguage,
                    _ => Focus::SettingsLanguage,
                };
            }
            Screen::DateTimeSetup => {
                self.adjust_date_time(1);
            }
            Screen::Search if self.focus == Focus::SearchKeyboard => {
                self.search_key_index = (self.search_key_index + 6) % SEARCH_KEYS.len();
            }
            Screen::Search => {
                if !self.search_results.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.search_results.len();
                }
            }
            Screen::RatingPicker => self.pending_rating = self.pending_rating.saturating_sub(1),
            Screen::ContextMenu => self.move_context_focus(1),
            _ => {}
        }
    }

    fn move_up(&mut self) {
        match self.screen {
            Screen::Theme => {
                if self.focus == Focus::Yeah || self.focus == Focus::Custom {
                    self.focus = Focus::ThemeSelect;
                }
            }
            Screen::First => {
                if self.focus == Focus::Language {
                    self.focus = Focus::GetStarted;
                }
            }
            Screen::Home => {
                if self.home_navigation.pane == HomePane::Detail {
                    self.move_home_detail(-1);
                } else {
                    self.move_home_main(-1);
                }
            }
            Screen::Apps => {
                self.focus = match self.focus {
                    Focus::AppsMusic => Focus::AppsUsbSync,
                    Focus::AppsUsbSync => Focus::AppsSettings,
                    Focus::AppsSettings => Focus::AppsNowPlaying,
                    Focus::AppsNowPlaying => Focus::AppsMusic,
                    _ => self.focus,
                };
            }
            Screen::MusicApp => {
                self.focus = match self.focus {
                    Focus::Search => Focus::ShuffleAll,
                    Focus::Albums => Focus::Search,
                    Focus::Songs => Focus::Albums,
                    Focus::Artists => Focus::Songs,
                    Focus::Playlists => Focus::Artists,
                    Focus::Favorites => Focus::Playlists,
                    Focus::Queue => Focus::Favorites,
                    Focus::History => Focus::Queue,
                    Focus::ShuffleAll => Focus::History,
                    _ => self.focus,
                };
            }
            Screen::NowPlaying => {
                self.focus = match self.focus {
                    Focus::Artists => Focus::Songs,
                    Focus::Albums => Focus::Favorites,
                    Focus::Playlists => Focus::History,
                    Focus::Songs => Focus::Artists,
                    Focus::Favorites => Focus::Albums,
                    Focus::History => Focus::Playlists,
                    _ => Focus::Albums,
                };
            }
            Screen::SongsList | Screen::AlbumTracksList | Screen::Queue => {
                if !self.tracks_queue.is_empty() {
                    self.selected_index = if self.selected_index == 0 {
                        self.tracks_queue.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                }
            }
            Screen::AlbumsList | Screen::ArtistsList | Screen::ArtistAlbumsList => {
                self.move_library_grid(-1, 0);
            }
            Screen::PlaylistsList => {
                if !self.playlists_list.is_empty() {
                    self.selected_index = if self.selected_index == 0 {
                        self.playlists_list.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                }
            }
            Screen::PlaylistTracksList => {
                if !self.tracks_queue.is_empty() {
                    self.selected_index = if self.selected_index == 0 {
                        self.tracks_queue.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                }
            }
            Screen::Language => {
                self.selected_index = if self.selected_index == 0 {
                    4
                } else {
                    self.selected_index - 1
                };
                self.active_language = match self.selected_index {
                    0 => Language::En,
                    1 => Language::Es,
                    2 => Language::De,
                    3 => Language::Ja,
                    4 => Language::Zh,
                    _ => Language::En,
                };
                self.save_preferences(false);
            }
            Screen::Settings => {
                self.focus = match self.focus {
                    Focus::SettingsGeneral => Focus::SettingsAbout,
                    Focus::SettingsAbout => Focus::SettingsGetUpdate,
                    Focus::SettingsGetUpdate => Focus::SettingsUser,
                    Focus::SettingsUser => Focus::SettingsConnectivity,
                    Focus::SettingsConnectivity => Focus::SettingsLooks,
                    Focus::SettingsLooks => Focus::SettingsAudio,
                    Focus::SettingsAudio => Focus::SettingsGeneral,
                    Focus::SettingsTheme => Focus::SettingsLooks,
                    Focus::SettingsLanguage => Focus::SettingsGeneral,
                    _ => self.focus,
                };
            }
            Screen::SettingsDetail => {
                self.focus = match self.focus {
                    Focus::SettingsLanguage => Focus::SettingsSwitchObject,
                    Focus::SettingsSwitchObject => Focus::SettingsEq,
                    Focus::SettingsEq => Focus::SettingsVolume,
                    Focus::SettingsVolume => Focus::SettingsBrightness,
                    Focus::SettingsBrightness => Focus::SettingsDateTime,
                    Focus::SettingsDateTime => Focus::SettingsLanguage,
                    _ => Focus::SettingsLanguage,
                };
            }
            Screen::DateTimeSetup => {
                self.adjust_date_time(-1);
            }
            Screen::Search if self.focus == Focus::SearchKeyboard => {
                self.search_key_index =
                    (self.search_key_index + SEARCH_KEYS.len() - 6) % SEARCH_KEYS.len();
            }
            Screen::Search => {
                if !self.search_results.is_empty() {
                    self.selected_index = if self.selected_index == 0 {
                        self.search_results.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                }
            }
            Screen::RatingPicker => self.pending_rating = (self.pending_rating + 1).min(5),
            Screen::ContextMenu => self.move_context_focus(-1),
            _ => {}
        }
    }

    fn move_right(&mut self) {
        match self.screen {
            Screen::Theme => match self.focus {
                Focus::ThemeSelect => {
                    self.active_theme = self.active_theme.cycle(1);
                    self.save_preferences(false);
                }
                Focus::Custom => {
                    self.focus = Focus::Yeah;
                }
                _ => {}
            },
            Screen::Home => self.enter_home_detail(),
            Screen::NowPlaying => {
                self.focus = match self.focus {
                    Focus::Songs => Focus::Artists,
                    Focus::Artists => Focus::Albums,
                    Focus::Albums => Focus::Playlists,
                    Focus::Playlists => Focus::History,
                    Focus::History => Focus::Favorites,
                    Focus::Favorites => Focus::Songs,
                    _ => self.focus,
                };
            }
            Screen::AlbumsList | Screen::ArtistsList | Screen::ArtistAlbumsList => {
                self.move_library_grid(0, 1);
            }
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistsList
            | Screen::PlaylistTracksList
            | Screen::Queue => self.jump_active_list(1),
            Screen::Search if self.focus == Focus::SearchKeyboard => {
                let row = self.search_key_index / 6;
                let col = self.search_key_index % 6;
                self.search_key_index = row * 6 + (col + 1) % 6;
            }
            Screen::Search => self.focus = Focus::SearchResults,
            Screen::RatingPicker => self.pending_rating = (self.pending_rating + 1).min(5),
            Screen::SettingsDetail => match self.focus {
                Focus::SettingsBrightness => self.adjust_brightness(5),
                Focus::SettingsVolume => self.adjust_volume(0.05),
                Focus::SettingsEq => self.cycle_eq(1),
                _ => {}
            },
            Screen::DateTimeSetup => {
                self.focus = match self.focus {
                    Focus::DateHour => Focus::DateMinute,
                    Focus::DateMinute => Focus::DateMonth,
                    Focus::DateMonth => Focus::DateDay,
                    Focus::DateDay => Focus::DateYear,
                    Focus::DateYear => Focus::DateDone,
                    Focus::DateDone => Focus::DateHour,
                    _ => Focus::DateHour,
                };
            }
            _ => {}
        }
    }

    fn move_left(&mut self) {
        match self.screen {
            Screen::Theme => match self.focus {
                Focus::ThemeSelect => {
                    self.active_theme = self.active_theme.cycle(-1);
                    self.save_preferences(false);
                }
                Focus::Yeah => {
                    self.focus = Focus::Custom;
                }
                _ => {}
            },
            Screen::Home => self.leave_home_detail(),
            Screen::NowPlaying => {
                self.focus = match self.focus {
                    Focus::Songs => Focus::Favorites,
                    Focus::Favorites => Focus::History,
                    Focus::History => Focus::Playlists,
                    Focus::Playlists => Focus::Albums,
                    Focus::Albums => Focus::Artists,
                    Focus::Artists => Focus::Songs,
                    _ => self.focus,
                };
            }
            Screen::AlbumsList | Screen::ArtistsList | Screen::ArtistAlbumsList => {
                self.move_library_grid(0, -1);
            }
            Screen::SongsList
            | Screen::AlbumTracksList
            | Screen::PlaylistsList
            | Screen::PlaylistTracksList
            | Screen::Queue => self.jump_active_list(-1),
            Screen::Search if self.focus == Focus::SearchKeyboard => {
                let row = self.search_key_index / 6;
                let col = self.search_key_index % 6;
                self.search_key_index = row * 6 + (col + 5) % 6;
            }
            Screen::Search => self.focus = Focus::SearchKeyboard,
            Screen::RatingPicker => self.pending_rating = self.pending_rating.saturating_sub(1),
            Screen::SettingsDetail => match self.focus {
                Focus::SettingsBrightness => self.adjust_brightness(-5),
                Focus::SettingsVolume => self.adjust_volume(-0.05),
                Focus::SettingsEq => self.cycle_eq(-1),
                _ => {}
            },
            Screen::DateTimeSetup => {
                self.focus = match self.focus {
                    Focus::DateHour => Focus::DateDone,
                    Focus::DateMinute => Focus::DateHour,
                    Focus::DateMonth => Focus::DateMinute,
                    Focus::DateDay => Focus::DateMonth,
                    Focus::DateYear => Focus::DateDay,
                    Focus::DateDone => Focus::DateYear,
                    _ => Focus::DateHour,
                };
            }
            _ => {}
        }
    }
}

impl MusicOs {
    fn adjust_date_time(&mut self, delta: i32) {
        match self.focus {
            Focus::DateHour => {
                self.clock_hour = wrap_i32(self.clock_hour as i32 + delta, 0, 23) as u8;
            }
            Focus::DateMinute => {
                self.clock_minute = wrap_i32(self.clock_minute as i32 + delta, 0, 59) as u8;
            }
            Focus::DateMonth => {
                self.clock_month = wrap_i32(self.clock_month as i32 + delta, 1, 12) as u8;
                self.clock_day = self
                    .clock_day
                    .min(days_in_month(self.clock_month, self.clock_year) as u8);
            }
            Focus::DateDay => {
                self.clock_day = wrap_i32(
                    self.clock_day as i32 + delta,
                    1,
                    days_in_month(self.clock_month, self.clock_year),
                ) as u8;
            }
            Focus::DateYear => {
                self.clock_year = wrap_i32(self.clock_year as i32 + delta, 2024, 2099) as u16;
                self.clock_day = self
                    .clock_day
                    .min(days_in_month(self.clock_month, self.clock_year) as u8);
            }
            _ => {}
        }
        self.clock_anchor = Instant::now();
        self.save_preferences(false);
    }

    fn adjust_brightness(&mut self, delta: i32) {
        let next = (self.brightness as i32 + delta).clamp(5, 100) as u8;
        if next != self.brightness {
            self.set_brightness(next);
        }
        self.hud_type = HudType::Brightness;
        self.hud_timer = 2.0;
    }

    fn set_brightness(&mut self, value: u8) {
        let next = value.clamp(5, 100);
        self.brightness = next;
        crate::system::apply_brightness(next);
        self.save_preferences(false);
        self.hud_type = HudType::Brightness;
        self.hud_timer = 2.0;
    }

    fn adjust_volume(&mut self, delta: f32) {
        let volume = (self.player.get_status().volume + delta).clamp(0.0, 1.0);
        self.player.set_volume(volume);
        self.save_preferences(false);
        self.hud_type = HudType::Volume;
        self.hud_timer = 2.0;
    }

    fn cycle_eq(&mut self, direction: i32) {
        let preset = if direction >= 0 {
            self.player.get_eq_preset().next()
        } else {
            self.player.get_eq_preset().previous()
        };
        self.player.set_eq_preset(preset);
        self.save_preferences(false);
    }

    fn toggle_current_favorite(&mut self) {
        let Some(track) = self.player.get_current_track().cloned() else {
            return;
        };
        match self.db.toggle_favorite(track.id) {
            Ok(favorited) => {
                self.player.update_track_favorite(track.id, favorited);
                for queued in &mut self.tracks_queue {
                    if queued.id == track.id {
                        queued.favorited = favorited;
                    }
                }
                for library_track in &mut self.library_tracks {
                    if library_track.id == track.id {
                        library_track.favorited = favorited;
                    }
                }
                self.cached_library_counts = self
                    .db
                    .library_counts()
                    .unwrap_or(self.cached_library_counts);
            }
            Err(err) => log::warn!("failed to toggle favorite for {}: {err:#}", track.id),
        }
    }

    fn format_clock(&self) -> String {
        if let Ok(value) = std::env::var("MUSICOS_TIME_TEXT") {
            if !value.trim().is_empty() {
                return value;
            }
        }

        let elapsed_minutes = self.clock_anchor.elapsed().as_secs() / 60;
        let total_minutes =
            self.clock_hour as u64 * 60 + self.clock_minute as u64 + elapsed_minutes;
        let hours = (total_minutes / 60) % 24;
        let minutes = total_minutes % 60;
        let am_pm = if hours >= 12 { "PM" } else { "AM" };
        let disp_hours = if hours % 12 == 0 { 12 } else { hours % 12 };

        format!("{disp_hours}:{minutes:02} {am_pm}")
    }

    fn screen_title(&self) -> &'static str {
        match self.screen {
            Screen::First => "musicOS",
            Screen::Welcome => "First boot",
            Screen::Theme => "Appearance",
            Screen::DateTimeSetup => "Date & Time",
            Screen::Home => "musicOS",
            Screen::MusicApp => "Music",
            Screen::Language => "Language",
            Screen::TestComplete => "Ready",
            Screen::NowPlaying => "Now Playing",
            Screen::SongsList => "Songs",
            Screen::AlbumsList => "Albums",
            Screen::ArtistsList => "Artists",
            Screen::ArtistAlbumsList => "Artist",
            Screen::AlbumTracksList => "Album",
            Screen::PlaylistsList => "Playlists",
            Screen::PlaylistTracksList => "Playlist",
            Screen::Queue => "Queue",
            Screen::Search => "Search",
            Screen::RatingPicker => "Rating",
            Screen::ContextMenu => "Options",
            Screen::Settings => "Settings",
            Screen::SettingsDetail => "Settings",
            Screen::Apps => "Apps",
            Screen::About => "About",
        }
    }

    pub fn enter_sleep(&mut self) {
        self.is_asleep = true;
        self.player.pause();
        crate::system::apply_brightness(0);

        #[cfg(target_os = "linux")]
        {
            log::info!("Suspending system to RAM...");
            if let Err(err) = std::fs::write("/sys/power/state", "mem") {
                log::error!("Failed to write to /sys/power/state: {}", err);
            }
            log::info!("System woke up from suspend!");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_grids_keep_dpad_focus_on_real_cells() {
        let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, ":memory:");
        let items: Vec<String> = (0..5).map(|i| format!("Item {i}")).collect();
        app.albums_list = items.clone();
        app.artists_list = items.clone();
        app.artist_albums_list = items;
        for screen in [
            Screen::AlbumsList,
            Screen::ArtistsList,
            Screen::ArtistAlbumsList,
        ] {
            app.force_screen_for_snapshot(screen);
            app.selected_index = 0;
            for (button, expected) in [
                (Button::Right, 1),
                (Button::Right, 2),
                (Button::Down, 4),
                (Button::Right, 4),
                (Button::Left, 3),
                (Button::Up, 0),
            ] {
                app.handle_input(InputEvent::Button(button));
                assert_eq!(app.selected_index, expected);
            }
        }
    }

    #[test]
    fn context_legend_matches_the_available_x_action() {
        let root = tempfile::tempdir().unwrap();
        let db_path = root.path().join("media.sqlite3");
        let mut app = MusicOs::with_db_path(Resolution::VGA_4_3, db_path.to_str().unwrap());

        for screen in [
            Screen::MusicApp,
            Screen::ArtistAlbumsList,
            Screen::SongsList,
        ] {
            app.force_screen_for_snapshot(screen);
            assert!(!app.button_legend().contains("X  More"));
            app.handle_input(InputEvent::Button(Button::Context));
            assert_eq!(app.screen, screen);
        }

        app.db
            .insert_track("/music/demo.ogg", "Demo", "Artist", "Album", 120, 1, 1)
            .unwrap();
        app.refresh_library();

        for screen in [Screen::AlbumsList, Screen::ArtistsList] {
            app.force_screen_for_snapshot(screen);
            assert!(app.button_legend().contains("X  More"));
            app.handle_input(InputEvent::Button(Button::Context));
            assert_eq!(app.screen, Screen::ContextMenu);
        }

        app.force_screen_for_snapshot(Screen::Home);
        assert!(app.button_legend().contains("X  More"));
        app.handle_input(InputEvent::Button(Button::Context));
        assert_eq!(app.home_navigation.pane, HomePane::Detail);
        assert!(!app.button_legend().contains("X  "));

        app.force_screen_for_snapshot(Screen::Search);
        assert!(!app.button_legend().contains("X  Results"));
        app.search_query = "Demo".into();
        app.refresh_search_results();
        assert!(app.button_legend().contains("X  Results"));
        app.handle_input(InputEvent::Button(Button::Context));
        assert_eq!(app.focus, Focus::SearchResults);
        assert!(app.button_legend().contains("X  More"));
    }

    #[test]
    fn deadlines_preserve_snapshots_animation_and_timers() {
        let mut app = MusicOs::new(Resolution {
            width: 640,
            height: 480,
        });
        let now = Instant::now();
        app.last_runtime_snapshot = now;
        app.outline_animating = false;
        app.screen_transition = 1.0;
        app.hud_t = 0.0;
        app.hud_timer = 0.0;
        app.theme_colors_initialized = false;
        app.scroll_offset = app.target_scroll_offset;
        app.dim_timeout_seconds = 0;
        app.sleep_timeout_minutes = 0;
        assert_eq!(
            app.next_tick_deadline(now),
            now + Duration::from_millis(250)
        );
        app.sleep_timer_deadline = Some(now + Duration::from_millis(7));
        assert_eq!(app.next_tick_deadline(now), now + Duration::from_millis(7));
        app.outline_animating = true;
        assert_eq!(app.next_tick_deadline(now), now + Duration::from_millis(2));
        app.outline_animating = false;
        app.sleep_timer_deadline = None;
        app.last_interaction = now - Duration::from_secs(1);
        app.dim_timeout_seconds = 1;
        assert_eq!(app.next_tick_deadline(now), now);
        app.display_dimmed = true;
        app.current_track_id = Some(1);
        app.last_session_save = now - Duration::from_secs(5);
        assert_eq!(app.next_tick_deadline(now), now);
    }
}

fn wrap_i32(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        max
    } else if value > max {
        min
    } else {
        value
    }
}

fn days_in_month(month: u8, year: u16) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 31,
    }
}

fn is_leap_year(year: u16) -> bool {
    let year = year as i32;
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn get_local_ip() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        for target in &[
            "8.8.8.8:80",
            "1.1.1.1:80",
            "192.168.1.1:80",
            "192.168.0.1:80",
        ] {
            if socket.connect(target).is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    let ip = addr.ip().to_string();
                    if ip != "0.0.0.0" && ip != "127.0.0.1" {
                        return ip;
                    }
                }
            }
        }
    }
    "127.0.0.1".to_string()
}
