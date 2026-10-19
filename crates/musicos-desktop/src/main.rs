use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use gpui::{
    Stateful,rgba, 
    actions, div, img, prelude::*, px, relative, rgb, size, svg, uniform_list, App, Application,
    Bounds, ClickEvent, Context, Div, Entity, FocusHandle, Focusable, FontWeight, KeyBinding,
    KeyDownEvent, Menu, MenuItem, MouseButton, ObjectFit, PathPromptOptions, ScrollWheelEvent,
    SharedString, StyledImage, SystemMenuType, TitlebarOptions, Window, WindowAppearance,
    WindowBounds, WindowOptions,
};
use musicos_desktop::api_client::ApiClient;
use musicos_desktop::assets::DesktopAssets;
use musicos_desktop::backup;
use musicos_desktop::diagnostics;
use musicos_desktop::library_import::import_managed_library;
use musicos_desktop::local::{
    enrich_library, EnrichmentProgress, LibraryQuery, LibraryRepository, LocalLibraryRepository,
    LocalPlaybackCommand, LocalPlaybackService, LocalPlaybackSnapshot, MetadataReviewItem,
};
use musicos_desktop::models::{
    AppearanceMode, DesktopSection, ImportProgress, InterfaceDensity, LibraryPresentation,
    LibrarySnapshot, LibraryViewMode, OperationTask, Playlist, SortField, SyncProgress, TaskId,
    TaskKind, TaskPhase, TaskStatus, Track,
};
use musicos_desktop::operations::TaskCenter;
use musicos_desktop::reference_sources::ReferenceSourceWatcher;
use musicos_desktop::runtime_manager::RuntimeManager;
use musicos_desktop::state::library::{
    AlbumId, AppRoute, ArtistId, LibraryDrilldown, LibraryIndex,
    LibraryRoute as DesktopLibraryRoute, LibraryScope,
};
use musicos_desktop::state::playback_status::PlaybackIntent;
use musicos_desktop::storage::{append_sync_history, load_config, save_config};
use musicos_desktop::sync::sync_files;
use musicos_desktop::ui::text_input::{
    Backspace as SearchBackspace, Clear as ClearSearch, Copy as SearchCopy, Cut as SearchCut,
    Delete as SearchDelete, End as SearchEnd, Home as SearchHome, Left as SearchLeft,
    Paste as SearchPaste, Right as SearchRight, SearchChanged, SearchInput,
    SelectAll as SearchSelectAll, SelectLeft as SearchSelectLeft, SelectRight as SearchSelectRight,
};
use musicos_desktop::ui::theme::ThemePalette;
use musicos_desktop::ControlCenterConfig;

const CHROME_TOP: u32 = 0xf5f5f7;
const CHROME_BOTTOM: u32 = 0xe8e8ed;
const BORDER: u32 = 0xd2d2d7;
const BORDER_DARK: u32 = 0x86868b;
const CONTENT: u32 = 0xffffff;
const TEXT: u32 = 0x1d1d1f;
const MUTED: u32 = 0x6e6e73;
const BLUE: u32 = 0x007aff;
const ROW_ALT: u32 = 0xf5f5f7;
const LCD: u32 = 0xececf0;

const DARK_CHROME_TOP: u32 = 0x1c1c1e;
const DARK_CHROME_BOTTOM: u32 = 0x2c2c2e;
const DARK_BORDER: u32 = 0x38383a;
const DARK_BORDER_DARK: u32 = 0x48484a;
const DARK_CONTENT: u32 = 0x161617;
const DARK_TEXT: u32 = 0xf5f5f7;
const DARK_MUTED: u32 = 0x98989d;
const DARK_BLUE: u32 = 0x0a84ff;
const DARK_ROW_ALT: u32 = 0x1c1c1e;
const DARK_LCD: u32 = 0x1c1c1e;

static DARK_APPEARANCE: AtomicBool = AtomicBool::new(false);

actions!(
    musicos,
    [
        ImportMusic,
        RefreshLibrary,
        TogglePlayback,
        PreviousTrack,
        NextTrack,
        ShowSongs,
        ShowAlbums,
        ShowArtists,
        ShowFlow,
        ShowPreferences,
        ShowDiagnostics,
        ToggleActivity,
        FocusSearch,
        NewPlaylist,
        DeleteFromPlaylist,
        SaveQueueAsPlaylist,
        ClearQueue,
        ExportPlaylist,
        ImportPlaylist,
        ToggleFullscreen,
        ToggleMiniPlayer,
        Quit
    ]
);

#[derive(Clone)]
enum PendingConfirmation {
    ClearQueue,
    DeletePlaylist {
        id: i64,
        name: String,
    },
    RemoveSource {
        id: i64,
        name: String,
    },
    CleanupIndexedLeftovers {
        revision: u64,
        count: usize,
        playlist_references: usize,
    },
    ResetDatabase,
    RestoreBackup {
        path: std::path::PathBuf,
    },
    InstallUpdate,
}

#[derive(Clone)]
struct ContextMenuState {
    track_id: i64,
    position: gpui::Point<gpui::Pixels>,
    playlists_open: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SettingsCategory {
    General,
    #[default]
    Library,
    Metadata,
    Playback,
    Formats,
    Appearance,
    Preview,
    Advanced,
}

struct ControlCenter {
    config: ControlCenterConfig,
    route: AppRoute,
    snapshot: LibrarySnapshot,
    library_index: LibraryIndex,
    library_revision: u64,
    artwork_generation: u64,
    artwork_loading: bool,
    artist_portraits: HashMap<ArtistId, PathBuf>,
    artist_portraits_loading: bool,
    artist_portrait_cancel: Arc<AtomicBool>,
    enrichment_started: bool,
    enrichment_running: bool,
    enrichment_cancel: Arc<AtomicBool>,
    enrichment_task: Option<TaskId>,
    enrichment_counts: BTreeMap<String, usize>,
    metadata_reviews: Vec<MetadataReviewItem>,
    selected_album: Option<AlbumId>,
    flow_focus: usize,
    selected_track: Option<usize>,
    selected_track_id: Option<i64>,
    active_playlist: Option<Playlist>,
    playlist_tracks: Vec<Track>,
    search: String,
    search_input: Entity<SearchInput>,
    playlist_name_input: Entity<SearchInput>,
    playlist_renaming: bool,
    loading: bool,
    error: Option<String>,
    notice: Option<String>,
    repository: LocalLibraryRepository,
    playback_service: Option<LocalPlaybackService>,
    preview: RuntimeManager,
    focus_handle: FocusHandle,
    operation_cancel: Arc<AtomicBool>,
    import_progress: ImportProgress,
    sync_progress: SyncProgress,
    task_center: TaskCenter,
    import_task: Option<TaskId>,
    sync_task: Option<TaskId>,
    activity_open: bool,
    compact_player: bool,
    expanded_window_size: Option<gpui::Size<gpui::Pixels>>,
    playback: LocalPlaybackSnapshot,
    playback_intent: Option<PlaybackIntent>,
    next_playback_command_id: u64,
    reference_watcher: Option<ReferenceSourceWatcher>,
    source_audit: musicos_desktop::local::SourceAuditReport,
    pending_confirmation: Option<PendingConfirmation>,
    context_menu: Option<ContextMenuState>,
    settings_category: SettingsCategory,
    theme: ThemePalette,
    last_playback_persist: Instant,
    last_persisted_queue_revision: u64,
    devices_mode: bool,
    sparklist_modal_open: bool,
}

impl ControlCenter {
    fn new(cx: &mut Context<Self>) -> Self {
        let config = load_config().unwrap_or_default();
        let route = route_from_config(&config);
        let repository =
            LocalLibraryRepository::new(config.db_path.clone(), config.media_dir.clone());
        let repository_error = repository.ensure_ready().err();
        let playback_service = LocalPlaybackService::start_with_preferences(
            config.db_path.clone(),
            config.playback.clone(),
        );
        let playback_error = playback_service
            .as_ref()
            .err()
            .map(|error| format!("{error:#}"));
        let search_input = cx.new(|cx| SearchInput::new(cx, "Search library"));
        let playlist_name_input = cx.new(|cx| SearchInput::new(cx, "Playlist name"));
        cx.subscribe(&search_input, |this, _, event: &SearchChanged, cx| {
            this.search = event.0.clone();
            this.selected_track = None;
            this.selected_track_id = None;
            this.rebuild_library_index();
            cx.notify();
        })
        .detach();
        let mut this = Self {
            repository,
            playback_service: playback_service.ok(),
            preview: RuntimeManager::new(&config),
            focus_handle: cx.focus_handle(),
            operation_cancel: Arc::new(AtomicBool::new(false)),
            import_progress: ImportProgress::default(),
            sync_progress: SyncProgress::default(),
            task_center: TaskCenter::default(),
            import_task: None,
            sync_task: None,
            activity_open: false,
            compact_player: false,
            expanded_window_size: None,
            playback: LocalPlaybackSnapshot::default(),
            playback_intent: None,
            next_playback_command_id: 1,
            reference_watcher: ReferenceSourceWatcher::new().ok(),
            source_audit: musicos_desktop::local::SourceAuditReport::default(),
            pending_confirmation: None,
            context_menu: None,
            settings_category: SettingsCategory::Library,
            theme: ThemePalette::light(),
            last_playback_persist: Instant::now(),
            last_persisted_queue_revision: 0,
            devices_mode: false,
            sparklist_modal_open: false,
            config,
            route,
            snapshot: LibrarySnapshot::default(),
            library_index: LibraryIndex::default(),
            library_revision: 0,
            artwork_generation: 0,
            artwork_loading: false,
            artist_portraits: HashMap::new(),
            artist_portraits_loading: false,
            artist_portrait_cancel: Arc::new(AtomicBool::new(false)),
            enrichment_started: false,
            enrichment_running: false,
            enrichment_cancel: Arc::new(AtomicBool::new(false)),
            enrichment_task: None,
            enrichment_counts: BTreeMap::new(),
            metadata_reviews: Vec::new(),
            selected_album: None,
            flow_focus: 0,
            selected_track: None,
            selected_track_id: None,
            active_playlist: None,
            playlist_tracks: Vec::new(),
            search: String::new(),
            search_input,
            playlist_name_input,
            playlist_renaming: false,
            loading: false,
            error: repository_error
                .map(|error| format!("Could not open the local library: {error:#}"))
                .or(playback_error.map(|error| {
                    format!("The library is ready, but local audio could not start: {error}")
                })),
            notice: Some("Loading the local library…".to_string()),
        };
        let scan_on_startup = this.config.source_preferences.scan_on_startup;
        this.load_library(scan_on_startup, cx);
        this.start_reference_monitor(cx);
        this.start_playback_monitor(cx);
        this
    }

    #[cfg(test)]
    fn test_fixture(_cx: &mut Context<Self>, track_count: usize) -> Self {
        let config = ControlCenterConfig::default();
        let tracks: Vec<Track> = (0..track_count)
            .map(|index| Track {
                id: index as i64,
                title: format!("Song {index:05}"),
                artist: format!("Artist {}", index % 100),
                album: format!("Album {}", index % 500),
                duration: 180,
                ..Default::default()
            })
            .collect();
        let library_index = LibraryIndex::build(
            Arc::from(tracks.clone()),
            &LibraryScope::Music,
            "",
            config.sort,
            1,
        );
        let search_input = cx.new(|cx| SearchInput::new(cx, "Search library"));
        let playlist_name_input = cx.new(|cx| SearchInput::new(cx, "Playlist name"));
        Self {
            repository: LocalLibraryRepository::new(
                config.db_path.clone(),
                config.media_dir.clone(),
            ),
            playback_service: None,
            preview: RuntimeManager::new(&config),
            focus_handle: cx.focus_handle(),
            config,
            route: AppRoute::Library(DesktopLibraryRoute::new(
                LibraryScope::Music,
                LibraryPresentation::Table,
            )),
            snapshot: LibrarySnapshot {
                tracks,
                ..Default::default()
            },
            library_index,
            library_revision: 1,
            artwork_generation: 0,
            artwork_loading: false,
            artist_portraits: HashMap::new(),
            artist_portraits_loading: false,
            artist_portrait_cancel: Arc::new(AtomicBool::new(false)),
            enrichment_started: false,
            enrichment_running: false,
            enrichment_cancel: Arc::new(AtomicBool::new(false)),
            enrichment_task: None,
            enrichment_counts: BTreeMap::new(),
            metadata_reviews: Vec::new(),
            selected_album: None,
            flow_focus: 0,
            selected_track: None,
            selected_track_id: None,
            active_playlist: None,
            playlist_tracks: Vec::new(),
            search: String::new(),
            search_input,
            playlist_name_input,
            playlist_renaming: false,
            loading: false,
            error: None,
            notice: None,
            operation_cancel: Arc::new(AtomicBool::new(false)),
            import_progress: ImportProgress::default(),
            sync_progress: SyncProgress::default(),
            task_center: TaskCenter::default(),
            import_task: None,
            sync_task: None,
            activity_open: false,
            compact_player: false,
            expanded_window_size: None,
            playback: LocalPlaybackSnapshot::default(),
            playback_intent: None,
            next_playback_command_id: 1,
            reference_watcher: None,
            source_audit: musicos_desktop::local::SourceAuditReport::default(),
            pending_confirmation: None,
            context_menu: None,
            settings_category: SettingsCategory::Library,
            theme: ThemePalette::light(),
            last_playback_persist: Instant::now(),
            last_persisted_queue_revision: 0,
            devices_mode: false,
            sparklist_modal_open: false,
        }
    }

    fn navigate(&mut self, section: DesktopSection, cx: &mut Context<Self>) {
        if section != DesktopSection::Artists {
            self.artist_portrait_cancel.store(true, Ordering::Relaxed);
        }
        if section == DesktopSection::Playlists {
            self.active_playlist = None;
            self.playlist_tracks.clear();
        }
        self.config.last_section = section;
        self.route = if let Some(scope) = LibraryScope::from_section(section) {
            let presentation = self
                .config
                .library_ui
                .presentation_by_scope
                .get(&scope.key())
                .copied()
                .unwrap_or_else(|| scope.default_presentation());
            self.config.library_ui.last_scope = scope.key();
            AppRoute::Library(DesktopLibraryRoute::new(scope, presentation))
        } else {
            AppRoute::Section(section)
        };
        self.selected_album = None;
        self.flow_focus = 0;
        self.rebuild_library_index();
        let _ = save_config(&self.config);
        if section == DesktopSection::Artists {
            self.load_artist_portraits(cx);
        }
        cx.notify();
    }

    fn set_view(&mut self, presentation: LibraryPresentation, cx: &mut Context<Self>) {
        let Some(route) = self.route.library_mut() else {
            return;
        };
        if !route.scope.supports(presentation) {
            return;
        }
        route.presentation = presentation;
        route.drilldown = None;
        self.config
            .library_ui
            .presentation_by_scope
            .insert(route.scope.key(), presentation);
        self.config.library_view = match presentation {
            LibraryPresentation::Table => LibraryViewMode::Songs,
            LibraryPresentation::Grid => LibraryViewMode::Albums,
            LibraryPresentation::Flow => LibraryViewMode::CoverFlow,
        };
        self.selected_album = None;
        self.flow_focus = self
            .flow_focus
            .min(self.library_index.albums.len().saturating_sub(1));
        let _ = save_config(&self.config);
        cx.notify();
    }

    fn section(&self) -> DesktopSection {
        self.route.section()
    }

    fn set_drilldown(&mut self, drilldown: Option<LibraryDrilldown>, cx: &mut Context<Self>) {
        if let Some(route) = self.route.library_mut() {
            route.drilldown = drilldown;
        }
        self.selected_track = None;
        self.selected_track_id = None;
        cx.notify();
    }

    fn open_album(&mut self, id: AlbumId, cx: &mut Context<Self>) {
        self.set_drilldown(Some(LibraryDrilldown::Album(id)), cx);
    }

    fn open_artist(&mut self, id: ArtistId, cx: &mut Context<Self>) {
        self.set_drilldown(Some(LibraryDrilldown::Artist(id)), cx);
    }

    fn open_genre(&mut self, name: String, cx: &mut Context<Self>) {
        self.set_drilldown(Some(LibraryDrilldown::Genre(name)), cx);
    }

    fn close_drilldown(&mut self, cx: &mut Context<Self>) {
        self.set_drilldown(None, cx);
    }

    fn rebuild_library_index(&mut self) {
        self.library_revision = self.library_revision.wrapping_add(1);
        let scope = self
            .route
            .library()
            .map(|route| route.scope.clone())
            .unwrap_or(LibraryScope::Music);
        self.library_index = LibraryIndex::build(
            Arc::from(self.snapshot.tracks.clone()),
            &scope,
            &self.search,
            self.config.sort,
            self.library_revision,
        );
        self.flow_focus = self
            .flow_focus
            .min(self.library_index.albums.len().saturating_sub(1));
    }

    fn create_playlist(&mut self, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let existing = self
            .snapshot
            .playlists
            .iter()
            .map(|playlist| playlist.name.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut name = "New Playlist".to_string();
        let mut suffix = 2;
        while existing.contains(name.as_str()) {
            name = format!("New Playlist {suffix}");
            suffix += 1;
        }
        let task = cx
            .background_executor()
            .spawn(async move { repository.create_playlist(&name) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(playlist) => {
                        this.notice = Some(format!("Created {}", playlist.name));
                        this.playlist_name_input
                            .update(cx, |input, cx| input.set_value(playlist.name.clone(), cx));
                        this.active_playlist = Some(playlist);
                        this.playlist_renaming = true;
                        this.playlist_tracks.clear();
                        this.route = AppRoute::Section(DesktopSection::Playlists);
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not create playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_playlist(&mut self, playlist: Playlist, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let id = playlist.id;
        self.route = AppRoute::Section(DesktopSection::Playlists);
        self.active_playlist = Some(playlist);
        self.playlist_renaming = false;
        self.selected_track = None;
        self.selected_track_id = None;
        let task = cx
            .background_executor()
            .spawn(async move { repository.playlist_tracks(id) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(tracks) => this.playlist_tracks = tracks,
                    Err(error) => this.error = Some(format!("Could not open playlist: {error:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn close_playlist(&mut self, cx: &mut Context<Self>) {
        self.active_playlist = None;
        self.playlist_renaming = false;
        self.playlist_tracks.clear();
        self.selected_track = None;
        self.selected_track_id = None;
        cx.notify();
    }

    fn rename_active_playlist(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.clone() else {
            return;
        };
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.rename_playlist(playlist.id, &new_name) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(renamed) => {
                        this.notice = Some(format!("Renamed to {}", renamed.name));
                        this.active_playlist = Some(renamed);
                        this.playlist_renaming = false;
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not rename playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn begin_playlist_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.as_ref() else {
            return;
        };
        self.playlist_name_input
            .update(cx, |input, cx| input.set_value(playlist.name.clone(), cx));
        self.playlist_renaming = true;
        window.focus(&self.playlist_name_input.focus_handle(cx));
        cx.notify();
    }

    fn commit_playlist_rename(&mut self, cx: &mut Context<Self>) {
        let name = self.playlist_name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some("Playlist name cannot be empty".to_string());
            cx.notify();
            return;
        }
        if self
            .active_playlist
            .as_ref()
            .is_some_and(|playlist| playlist.name == name)
        {
            self.playlist_renaming = false;
            cx.notify();
            return;
        }
        self.rename_active_playlist(name, cx);
    }

    fn cancel_playlist_rename(&mut self, cx: &mut Context<Self>) {
        self.playlist_renaming = false;
        cx.notify();
    }

    fn delete_active_playlist(&mut self, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.clone() else {
            return;
        };
        let repository = self.repository.clone();
        let id = playlist.id;
        self.pending_confirmation = None;
        let task = cx
            .background_executor()
            .spawn(async move { repository.delete_playlist(id) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.notice = Some(format!("Deleted {}", playlist.name));
                        this.active_playlist = None;
                        this.playlist_tracks.clear();
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not delete playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn request_delete_active_playlist(&mut self, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.as_ref() else {
            return;
        };
        self.pending_confirmation = Some(PendingConfirmation::DeletePlaylist {
            id: playlist.id,
            name: playlist.name.clone(),
        });
        cx.notify();
    }

    fn add_selected_to_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        let Some(track_id) = self.selected_track_id else {
            self.notice = Some("Select a song before adding it to a playlist".to_string());
            cx.notify();
            return;
        };
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.add_tracks(playlist_id, &[track_id]) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.notice = Some("Added to playlist".to_string());
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not add to playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn remove_selected_from_playlist(&mut self, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.clone() else {
            return;
        };
        let Some(track_id) = self.selected_track_id else {
            return;
        };
        let repository = self.repository.clone();
        let id = playlist.id;
        let task = cx
            .background_executor()
            .spawn(async move { repository.remove_tracks(id, &[track_id]) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.playlist_tracks.retain(|track| track.id != track_id);
                        this.selected_track = None;
                        this.selected_track_id = None;
                        this.notice = Some("Removed from playlist".to_string());
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not remove from playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn move_selected_playlist_item(&mut self, direction: isize, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.clone() else {
            return;
        };
        if !self.search.trim().is_empty() {
            self.notice = Some("Clear search before rearranging a playlist".to_string());
            cx.notify();
            return;
        }
        let Some(from) = self.selected_track else {
            self.notice = Some("Select a playlist song first".to_string());
            cx.notify();
            return;
        };
        let to = from
            .saturating_add_signed(direction)
            .min(self.playlist_tracks.len().saturating_sub(1));
        if from == to || from >= self.playlist_tracks.len() {
            return;
        }
        let mut reordered = self.playlist_tracks.clone();
        reordered.swap(from, to);
        let ids = reordered.iter().map(|track| track.id).collect::<Vec<_>>();
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.reorder_tracks(playlist.id, &ids) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        this.playlist_tracks = reordered;
                        this.selected_track = Some(to);
                        this.selected_track_id = this.playlist_tracks.get(to).map(|track| track.id);
                        this.notice = Some("Playlist order updated".to_string());
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not reorder playlist: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn save_queue_as_playlist(&mut self, cx: &mut Context<Self>) {
        let queue_ids: Vec<i64> = self.playback.queue.iter().map(|track| track.id).collect();
        if queue_ids.is_empty() {
            self.notice = Some("The queue is empty".to_string());
            cx.notify();
            return;
        }
        let repository = self.repository.clone();
        let existing = self
            .snapshot
            .playlists
            .iter()
            .map(|playlist| playlist.name.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut name = "Queue Playlist".to_string();
        let mut suffix = 2;
        while existing.contains(name.as_str()) {
            name = format!("Queue Playlist {suffix}");
            suffix += 1;
        }
        let queue_count = queue_ids.len();
        let task = cx.background_executor().spawn(async move {
            let playlist = repository.create_playlist(&name)?;
            repository.add_tracks(playlist.id, &queue_ids)?;
            Ok::<_, anyhow::Error>(playlist)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(playlist) => {
                        this.notice =
                            Some(format!("Saved {} tracks as {}", queue_count, playlist.name));
                        this.refresh_library(cx);
                    }
                    Err(error) => this.error = Some(format!("Could not save queue: {error:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn request_clear_queue(&mut self, cx: &mut Context<Self>) {
        if self.playback.queue.is_empty() {
            self.notice = Some("The queue is already empty".to_string());
        } else {
            self.pending_confirmation = Some(PendingConfirmation::ClearQueue);
        }
        cx.notify();
    }

    fn clear_queue(&mut self, cx: &mut Context<Self>) {
        self.pending_confirmation = None;
        match self
            .playback_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local audio is unavailable"))
            .and_then(|service| service.send(LocalPlaybackCommand::ClearQueue))
        {
            Ok(()) => {
                self.selected_track = None;
                self.selected_track_id = None;
                self.notice = Some("Queue cleared".to_string());
            }
            Err(error) => self.error = Some(format!("Could not clear queue: {error:#}")),
        }
        cx.notify();
    }

    fn clear_upcoming(&mut self, cx: &mut Context<Self>) {
        let upcoming = self
            .playback
            .queue_index
            .map(|index| self.playback.queue.len().saturating_sub(index + 1))
            .unwrap_or(self.playback.queue.len());
        if upcoming == 0 {
            self.notice = Some("There are no upcoming songs".to_string());
            cx.notify();
            return;
        }
        self.send_playback(LocalPlaybackCommand::ClearUpcoming, cx);
        self.selected_track = None;
        self.selected_track_id = None;
        self.notice = Some(format!("Removed {upcoming} upcoming songs"));
    }

    fn selected_queue_index(&self) -> Option<usize> {
        if self.section() != DesktopSection::Queue {
            return None;
        }
        if let Some(visible_index) = self.selected_track {
            if let Some(queue_index) = self.visible_track_indices().get(visible_index).copied() {
                return Some(queue_index);
            }
        }
        self.selected_track_id
            .and_then(|id| self.playback.queue.iter().position(|track| track.id == id))
    }

    fn play_selected_queue_item(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected_queue_index() else {
            self.notice = Some("Select a queued song first".to_string());
            cx.notify();
            return;
        };
        self.send_playback(LocalPlaybackCommand::SelectQueueIndex(index), cx);
    }

    fn move_selected_queue_item(&mut self, direction: isize, cx: &mut Context<Self>) {
        let Some(from) = self.selected_queue_index() else {
            self.notice = Some("Select a queued song first".to_string());
            cx.notify();
            return;
        };
        let to = from
            .saturating_add_signed(direction)
            .min(self.playback.queue.len().saturating_sub(1));
        if from == to {
            return;
        }
        self.send_playback(LocalPlaybackCommand::MoveQueueItem { from, to }, cx);
        self.selected_track = Some(to);
        self.notice = Some("Queue order updated".to_string());
    }

    fn remove_selected_queue_item(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected_queue_index() else {
            self.notice = Some("Select a queued song first".to_string());
            cx.notify();
            return;
        };
        self.send_playback(LocalPlaybackCommand::RemoveQueueIndex(index), cx);
        self.selected_track = None;
        self.selected_track_id = None;
        self.notice = Some("Removed from queue".to_string());
    }

    fn export_active_playlist(&mut self, cx: &mut Context<Self>) {
        let Some(playlist) = self.active_playlist.clone() else {
            self.notice = Some("Open a playlist first".to_string());
            cx.notify();
            return;
        };
        let directory = std::env::current_dir().unwrap_or_default().join("out");
        let receiver = cx.prompt_for_new_path(&directory, Some(&format!("{}.m3u", playlist.name)));
        let repository = self.repository.clone();
        let id = playlist.id;
        cx.spawn(async move |this, cx| {
            let destination = receiver.await.ok().and_then(Result::ok).flatten();
            if let Some(destination) = destination {
                let display_path = destination.clone();
                let task = cx.background_executor().spawn(async move {
                    musicos_desktop::local::playlists::export_m3u(&repository, id, &destination)
                });
                let result = task.await;
                this.update(cx, |this, cx| {
                    match result {
                        Ok(_) => {
                            this.notice = Some(format!("Exported to {}", display_path.display()));
                        }
                        Err(error) => {
                            this.error = Some(format!("Playlist export failed: {error:#}"))
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn import_playlist_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import Playlist".into()),
        });
        let repository = self.repository.clone();
        cx.spawn(async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .and_then(Result::ok)
                .flatten()
                .and_then(|mut paths| paths.pop());
            if let Some(path) = path {
                let task = cx.background_executor().spawn(async move {
                    musicos_desktop::local::playlists::import_m3u(&repository, &path)
                });
                let result = task.await;
                this.update(cx, |this, cx| {
                    match result {
                        Ok(import) => {
                            this.notice = Some(format!(
                                "Imported {} tracks; {} missing",
                                import.imported,
                                import.missing.len()
                            ));
                            this.refresh_library(cx);
                        }
                        Err(error) => {
                            this.error = Some(format!("Playlist import failed: {error:#}"))
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn start_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview.status().running {
            let _ = self.preview.stop();
        }
        let task_id = self.task_center.start(
            TaskKind::EmulatorStart,
            "Launch Device Preview",
            Some(DesktopSection::Devices),
            false,
        );
        self.task_center.set_phase(
            task_id,
            musicos_desktop::models::TaskPhase::Starting,
            Some("Launching the optional MusicOS preview".into()),
        );
        let bin_path = self.preview.refresh().bin_path;
        let config = self.config.clone();
        let task = cx.background_executor().spawn(async move {
            let mut manager = RuntimeManager::new(&config);
            let result = manager.start().and_then(|status| {
                ApiClient::new(&config.library_target)?
                    .wait_until_ready(Duration::from_secs(15))?;
                Ok(status)
            });
            (manager, result)
        });
        cx.spawn(async move |this, cx| {
            let (manager, result) = task.await;
            this.update(cx, |this, cx| {
                this.preview = manager;
                match result {
                    Ok(status) => {
                        this.task_center.complete(
                            task_id,
                            format!(
                                "Device Preview started{}",
                                status
                                    .pid
                                    .map(|pid| format!(" · PID {pid}"))
                                    .unwrap_or_default()
                            ),
                        );
                        this.notice = Some(format!(
                            "Device Preview {} on {}",
                            status
                                .pid
                                .map(|pid| format!("PID {pid}"))
                                .unwrap_or_default(),
                            status.target
                        ));
                    }
                    Err(error) => {
                        this.task_center.fail(task_id, format!("{error:#}"));
                        this.error = Some(format!(
                            "Could not launch {}: {error:#}",
                            bin_path.display()
                        ));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh_library(&mut self, cx: &mut Context<Self>) {
        self.load_library(true, cx);
    }

    fn load_library(&mut self, scan_for_changes: bool, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        let task_id = self.task_center.start(
            TaskKind::LibraryScan,
            if scan_for_changes {
                "Refresh library"
            } else {
                "Open library"
            },
            Some(DesktopSection::Music),
            false,
        );
        self.task_center.set_phase(
            task_id,
            musicos_desktop::models::TaskPhase::Discovering,
            Some("Reading tracks, playlists and sources".into()),
        );
        cx.notify();
        let repository = self.repository.clone();
        let cancel = self.operation_cancel.clone();
        enum ScanUpdate {
            Progress(musicos_init::media::ScanProgress),
            Stage(
                bool,
                Result<
                    (
                        LibrarySnapshot,
                        musicos_desktop::local::SourceAuditReport,
                        BTreeMap<String, usize>,
                        Vec<MetadataReviewItem>,
                    ),
                    String,
                >,
            ),
        }
        let (sender, receiver) = async_channel::bounded::<ScanUpdate>(4);
        cx.background_executor()
            .spawn(async move {
                let progress_sender = sender.clone();
                let load_snapshot = |repository: &LocalLibraryRepository| {
                    let (snapshot, audit) =
                        repository.snapshot_with_audit(&LibraryQuery::default())?;
                    let counts = repository.metadata_enrichment_counts()?;
                    let reviews = repository.metadata_review_items(50)?;
                    Ok::<_, anyhow::Error>((snapshot, audit, counts, reviews))
                };
                let cached = load_snapshot(&repository).map_err(|error| format!("{error:#}"));
                if !scan_for_changes {
                    let _ = sender.send(ScanUpdate::Stage(true, cached)).await;
                    return;
                }
                let cached_ok = cached.is_ok();
                if sender.send(ScanUpdate::Stage(false, cached)).await.is_err() || !cached_ok {
                    return;
                }
                let refreshed = repository
                    .scan(
                        musicos_init::media::ScanMode::Changed,
                        &cancel,
                        |progress| {
                            let _ = progress_sender.try_send(ScanUpdate::Progress(progress));
                        },
                    )
                    .and_then(|_| load_snapshot(&repository))
                    .map_err(|error| format!("{error:#}"));
                let _ = sender.send(ScanUpdate::Stage(true, refreshed)).await;
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(update) = receiver.recv().await {
                let is_final = matches!(&update, ScanUpdate::Stage(true, _));
                if this
                    .update(cx, |this, cx| {
                        match update {
                            ScanUpdate::Progress(progress) => {
                                this.task_center.update_scan(
                                    task_id,
                                    progress.processed,
                                    progress.discovered,
                                    progress.current_root,
                                );
                            }
                            ScanUpdate::Stage(_, result) => match result {
                                Ok((snapshot, source_audit, enrichment_counts, metadata_reviews)) => {
                                    let track_count = snapshot.tracks.len();
                                    this.snapshot = snapshot;
                                    this.source_audit = source_audit;
                                    this.enrichment_counts = enrichment_counts;
                                    this.metadata_reviews = metadata_reviews;
                                    this.rebuild_library_index();
                                    if let Some(watcher) = this.reference_watcher.as_mut() {
                                        let roots = this
                                            .snapshot
                                            .sources
                                            .iter()
                                            .filter(|source| {
                                                source.kind == "reference" && source.enabled
                                            })
                                            .map(|source| source.path.clone());
                                        if let Err(error) = watcher.replace_roots(roots) {
                                            this.error = Some(format!(
                                                "Could not watch reference folders: {error:#}"
                                            ));
                                        }
                                    }
                                    if is_final {
                                        this.loading = false;
                                        this.task_center.complete(
                                            task_id,
                                            format!("{track_count} tracks loaded"),
                                        );
                                        this.notice = Some("Library is up to date".to_string());
                                        this.load_missing_artwork(cx);
                                        if this.section() == DesktopSection::Artists {
                                            this.load_artist_portraits(cx);
                                        }
                                        this.start_metadata_enrichment(false, cx);
                                    } else {
                                        this.notice = Some(format!(
                                            "{track_count} local tracks loaded · checking for changes"
                                        ));
                                    }
                                }
                                Err(error) => {
                                    this.loading = false;
                                    this.task_center.fail(task_id, error.clone());
                                    this.error = Some(format!(
                                        "Could not refresh the local library: {error}"
                                    ));
                                }
                            },
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
                if is_final {
                    break;
                }
            }
        })
        .detach();
    }

    fn load_missing_artwork(&mut self, cx: &mut Context<Self>) {
        if self.artwork_loading {
            return;
        }
        let now_playing_id = self.playback.track.as_ref().map(|track| track.id);
        let mut seen = HashSet::new();
        let mut track_ids = self
            .library_index
            .albums
            .iter()
            .filter_map(|album| {
                let index = *self
                    .library_index
                    .track_by_id
                    .get(&album.representative_track)?;
                let track = &self.library_index.tracks[index];
                let needs_artwork = track
                    .cover_art_path
                    .as_ref()
                    .is_none_or(|path| !path.is_absolute() || !path.is_file());
                (needs_artwork && seen.insert(track.id)).then_some(track.id)
            })
            .collect::<Vec<_>>();
        track_ids.sort_by_key(|track_id| usize::from(Some(*track_id) != now_playing_id));
        if track_ids.is_empty() {
            return;
        }

        self.artwork_generation = self.artwork_generation.wrapping_add(1);
        let generation = self.artwork_generation;
        self.artwork_loading = true;
        let db_path = self.config.db_path.clone();
        let (sender, receiver) = async_channel::bounded(3);
        cx.background_executor()
            .spawn(async move {
                let mut batch = Vec::with_capacity(8);
                for track_id in track_ids {
                    let result =
                        musicos_desktop::local::artwork::ensure_artwork(db_path.clone(), track_id)
                            .map_err(|error| format!("{error:#}"));
                    batch.push((track_id, result));
                    if batch.len() == 8 {
                        if sender.send(std::mem::take(&mut batch)).await.is_err() {
                            return;
                        }
                    }
                }
                if !batch.is_empty() {
                    let _ = sender.send(batch).await;
                }
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(batch) = receiver.recv().await {
                if this
                    .update(cx, |this, cx| {
                        if this.artwork_generation != generation {
                            return;
                        }
                        for (track_id, result) in batch {
                            match result {
                                Ok(path) => {
                                    if let Some(track) = this
                                        .snapshot
                                        .tracks
                                        .iter_mut()
                                        .find(|track| track.id == track_id)
                                    {
                                        track.cover_art_path = path;
                                    }
                                }
                                Err(error) => {
                                    log::warn!("desktop-artwork: track {track_id} failed: {error}");
                                }
                            }
                        }
                        this.rebuild_library_index();
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            this.update(cx, |this, cx| {
                if this.artwork_generation == generation {
                    this.artwork_loading = false;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn load_artist_portraits(&mut self, cx: &mut Context<Self>) {
        if self.artist_portraits_loading || self.library_index.artists.is_empty() {
            return;
        }
        let artists = self
            .library_index
            .artists
            .iter()
            .filter(|artist| !artist.name.eq_ignore_ascii_case("unknown artist"))
            .map(|artist| (artist.id.clone(), artist.name.clone()))
            .collect::<Vec<_>>();
        if artists.is_empty() {
            return;
        }
        for (id, name) in &artists {
            if let Some(path) = musicos_desktop::local::artist_artwork::cached_artist_portrait(
                &self.config.db_path,
                name,
            ) {
                self.artist_portraits.insert(id.clone(), path);
            }
        }

        self.artist_portrait_cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.artist_portrait_cancel.clone();
        let db_path = self.config.db_path.clone();
        let names = artists
            .iter()
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>();
        let ids = artists.into_iter().collect::<HashMap<_, _>>();
        let ids = ids
            .into_iter()
            .map(|(id, name)| (name, id))
            .collect::<HashMap<_, _>>();
        let (sender, receiver) = async_channel::bounded(4);
        self.artist_portraits_loading = true;
        cx.background_executor()
            .spawn(async move {
                let progress_sender = sender.clone();
                let _ = musicos_desktop::local::artist_artwork::ensure_artist_portraits(
                    &db_path,
                    &names,
                    &cancel,
                    |processed, total, name, path| {
                        if let Some(id) = ids.get(name) {
                            let _ = progress_sender.send_blocking((
                                processed,
                                total,
                                id.clone(),
                                path.map(Path::to_path_buf),
                            ));
                        }
                    },
                );
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok((processed, total, id, path)) = receiver.recv().await {
                if this
                    .update(cx, |this, cx| {
                        if let Some(path) = path {
                            this.artist_portraits.insert(id, path);
                        }
                        this.artist_portraits_loading = processed < total;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            this.update(cx, |this, cx| {
                this.artist_portraits_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn start_metadata_enrichment(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.enrichment_running
            || self.snapshot.tracks.is_empty()
            || (!force && (!self.config.enrichment.enabled || self.enrichment_started))
        {
            return;
        }
        self.enrichment_started = true;
        self.enrichment_running = true;
        self.enrichment_cancel.store(false, Ordering::Relaxed);
        let task_id = self.task_center.start(
            TaskKind::MetadataLookup,
            "Enrich metadata and artwork",
            Some(DesktopSection::Music),
            true,
        );
        self.enrichment_task = Some(task_id);
        self.task_center.set_phase(
            task_id,
            TaskPhase::Custom("Matching metadata".to_string()),
            Some("Preparing MusicBrainz lookups".into()),
        );
        self.notice = Some("Matching library metadata in the background…".to_string());
        let db_path = self.config.db_path.clone();
        let track_ids = self
            .snapshot
            .tracks
            .iter()
            .map(|track| track.id)
            .collect::<Vec<_>>();
        let preferences = self.config.enrichment.clone();
        let artwork_preferences = self.config.artwork.clone();
        let cancel = self.enrichment_cancel.clone();
        enum EnrichmentUpdate {
            Progress(EnrichmentProgress),
            Complete(Result<musicos_desktop::local::EnrichmentRunReport, String>),
        }
        let (sender, receiver) = async_channel::bounded::<EnrichmentUpdate>(4);
        cx.background_executor()
            .spawn(async move {
                let progress_sender = sender.clone();
                let result = enrich_library(
                    &db_path,
                    &track_ids,
                    &preferences,
                    &artwork_preferences,
                    &cancel,
                    |progress| {
                        let _ = progress_sender.try_send(EnrichmentUpdate::Progress(progress));
                    },
                )
                .map_err(|error| format!("{error:#}"));
                let _ = sender.send(EnrichmentUpdate::Complete(result)).await;
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(update) = receiver.recv().await {
                let finished = matches!(&update, EnrichmentUpdate::Complete(_));
                if this
                    .update(cx, |this, cx| {
                        match update {
                            EnrichmentUpdate::Progress(progress) => {
                                this.task_center.update_items(
                                    task_id,
                                    if progress.artwork > 0 {
                                        TaskPhase::Custom("Downloading artwork".to_string())
                                    } else {
                                        TaskPhase::Custom("Matching metadata".to_string())
                                    },
                                    progress.processed,
                                    progress.total,
                                    Some(progress.current),
                                );
                            }
                            EnrichmentUpdate::Complete(Ok(report)) => {
                                this.enrichment_running = false;
                                this.enrichment_task = None;
                                if report.cancelled {
                                    this.task_center.cancel(
                                        task_id,
                                        format!("Cancelled after {} tracks", report.processed),
                                    );
                                    this.notice = Some(format!(
                                        "Metadata matching cancelled after {} tracks",
                                        report.processed
                                    ));
                                } else {
                                    this.task_center.complete(
                                        task_id,
                                        format!(
                                            "{} updated · {} review · {} unmatched · {} artwork",
                                            report.applied,
                                            report.review,
                                            report.unmatched,
                                            report.artwork
                                        ),
                                    );
                                    this.notice = Some(format!(
                                        "Metadata updated for {} tracks · {} need review",
                                        report.applied, report.review
                                    ));
                                }
                                this.refresh_library(cx);
                            }
                            EnrichmentUpdate::Complete(Err(error)) => {
                                this.enrichment_running = false;
                                this.enrichment_task = None;
                                this.task_center.fail(task_id, error.clone());
                                this.error = Some(format!("Metadata enrichment failed: {error}"));
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
                if finished {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    fn cancel_metadata_enrichment(&mut self, cx: &mut Context<Self>) {
        if self.enrichment_running {
            self.enrichment_cancel.store(true, Ordering::Relaxed);
            if let Some(task_id) = self.enrichment_task {
                self.task_center.request_cancel(task_id);
            }
            self.notice = Some("Stopping metadata matching after the current request…".to_string());
            cx.notify();
        }
    }

    fn start_reference_monitor(&mut self, cx: &mut Context<Self>) {
        if self.reference_watcher.is_none() {
            return;
        }
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| loop {
            executor.timer(Duration::from_millis(500)).await;
            let update = this.update(cx, |this, cx| {
                if !this.config.source_preferences.watch_reference_folders {
                    return;
                }
                let changed_roots = this
                    .reference_watcher
                    .as_mut()
                    .map(|watcher| watcher.changed_roots_after_debounce(Duration::from_millis(900)))
                    .unwrap_or_default();
                if !changed_roots.is_empty() {
                    let source_ids = this
                        .snapshot
                        .sources
                        .iter()
                        .filter(|source| {
                            source.kind == "reference"
                                && source.enabled
                                && changed_roots.iter().any(|root| root == &source.path)
                        })
                        .map(|source| source.id)
                        .collect::<Vec<_>>();
                    this.rescan_reference_sources(source_ids, cx);
                }
            });
            if update.is_err() {
                break;
            }
        })
        .detach();
    }

    fn choose_reference_source(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Add Reference Folder".into()),
        });
        cx.spawn(async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .and_then(Result::ok)
                .flatten()
                .and_then(|mut paths| paths.pop());
            if let Some(path) = path {
                this.update(cx, |this, cx| this.add_reference_source(path, cx))
                    .ok();
            }
        })
        .detach();
    }

    fn add_reference_source(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.notice = Some(format!("Adding reference folder {}…", path.display()));
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.add_reference_source(&path) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(source) => {
                        this.notice =
                            Some(format!("Added reference folder {}", source.display_name));
                        this.rescan_source(source.id, cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not add reference folder: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn rescan_source(&mut self, id: i64, cx: &mut Context<Self>) {
        self.scan_source(id, musicos_init::media::ScanMode::Changed, cx);
    }

    fn full_rescan_source(&mut self, id: i64, cx: &mut Context<Self>) {
        self.scan_source(id, musicos_init::media::ScanMode::Full, cx);
    }

    fn scan_source(
        &mut self,
        id: i64,
        mode: musicos_init::media::ScanMode,
        _cx: &mut Context<Self>,
    ) {
        let repository = self.repository.clone();
        let cancel = self.operation_cancel.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.scan_sources(mode, Some(vec![id]), &cancel, |_| {}) });
        self.notice = Some(if mode == musicos_init::media::ScanMode::Full {
            "Rebuilding reference-folder metadata…".to_string()
        } else {
            "Checking reference folder for changes…".to_string()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.notice = Some(format!(
                            "Reference scan complete · {} files updated",
                            result.indexed
                        ));
                        this.enrichment_started = false;
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Reference rescan failed: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn rescan_all_reference_sources(&mut self, cx: &mut Context<Self>) {
        let sources = self
            .snapshot
            .sources
            .iter()
            .filter(|source| source.kind == "reference" && source.enabled)
            .map(|source| source.id)
            .collect::<Vec<_>>();
        self.rescan_reference_sources(sources, cx);
    }

    fn rescan_reference_sources(&mut self, sources: Vec<i64>, cx: &mut Context<Self>) {
        if sources.is_empty() {
            return;
        }
        let repository = self.repository.clone();
        let cancel = self.operation_cancel.clone();
        let task = cx.background_executor().spawn(async move {
            repository.scan_sources(
                musicos_init::media::ScanMode::Changed,
                Some(sources.clone()),
                &cancel,
                |_| {},
            )?;
            Ok::<usize, anyhow::Error>(sources.len())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(count) => {
                        this.notice = Some(format!(
                            "Updated {count} reference folder{}",
                            if count == 1 { "" } else { "s" }
                        ));
                        this.enrichment_started = false;
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Automatic reference rescan failed: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_source(&mut self, id: i64, enabled: bool, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.update_source(id, enabled) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(source) => {
                        this.notice = Some(format!(
                            "{} {}",
                            if source.enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            },
                            source.display_name
                        ));
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not update reference folder: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn request_remove_source(&mut self, id: i64, name: String, cx: &mut Context<Self>) {
        self.pending_confirmation = Some(PendingConfirmation::RemoveSource { id, name });
        cx.notify();
    }

    fn confirm_remove_source(&mut self, id: i64, cx: &mut Context<Self>) {
        self.pending_confirmation = None;
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.remove_source(id) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.notice = Some("Reference folder removed".to_string());
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not remove reference folder: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn player_command(&mut self, command: &'static str, cx: &mut Context<Self>) {
        if command == "play_pause" && self.playback.track.is_none() {
            let tracks = self.visible_tracks();
            if let Some(track) = self
                .selected_track_id
                .and_then(|id| tracks.iter().find(|track| track.id == id))
                .or_else(|| tracks.first())
            {
                self.play_visible_track(track.id, cx);
                return;
            }
        }
        let command = match command {
            "play_pause" => LocalPlaybackCommand::PlayPause,
            "pause" => LocalPlaybackCommand::Pause,
            "prev" | "previous" => LocalPlaybackCommand::Previous,
            "next" => LocalPlaybackCommand::Next,
            "shuffle" => LocalPlaybackCommand::ToggleShuffle,
            "repeat" => LocalPlaybackCommand::CycleRepeat,
            _ => {
                self.error = Some(format!("Unsupported local playback command: {command}"));
                cx.notify();
                return;
            }
        };
        match self
            .playback_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local audio is unavailable"))
            .and_then(|service| service.send(command))
        {
            Ok(()) => self.begin_playback_intent(None),
            Err(error) => self.error = Some(format!("Playback error: {error:#}")),
        }
        cx.notify();
    }

    fn play_track(&mut self, id: i64, cx: &mut Context<Self>) {
        match self
            .playback_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local audio is unavailable"))
            .and_then(|service| service.send(LocalPlaybackCommand::PlayTrack(id)))
        {
            Ok(()) => self.begin_playback_intent(Some(id)),
            Err(error) => self.error = Some(format!("Could not play track: {error:#}")),
        }
        cx.notify();
    }

    fn play_tracks(&mut self, ids: Vec<i64>, cx: &mut Context<Self>) {
        if ids.is_empty() {
            return;
        }
        let first_track = ids.first().copied();
        match self
            .playback_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local audio is unavailable"))
            .and_then(|service| {
                service.send(LocalPlaybackCommand::PlayTracks {
                    ids,
                    start_index: 0,
                })
            }) {
            Ok(()) => self.begin_playback_intent(first_track),
            Err(error) => self.error = Some(format!("Could not play collection: {error:#}")),
        }
        cx.notify();
    }

    fn play_visible_track(&mut self, track_id: i64, cx: &mut Context<Self>) {
        let collection = self.track_collection();
        let ids = self
            .visible_track_indices()
            .into_iter()
            .filter_map(|index| collection.get(index).map(|track| track.id))
            .collect::<Vec<_>>();
        let Some(start_index) = ids.iter().position(|id| *id == track_id) else {
            self.play_track(track_id, cx);
            return;
        };

        match self
            .playback_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local audio is unavailable"))
            .and_then(|service| service.send(LocalPlaybackCommand::PlayTracks { ids, start_index }))
        {
            Ok(()) => self.begin_playback_intent(Some(track_id)),
            Err(error) => self.error = Some(format!("Could not play selection: {error:#}")),
        }
        cx.notify();
    }

    fn seek_to(&mut self, fraction: f32, cx: &mut Context<Self>) {
        let Some(track) = self.playback.track.as_ref() else {
            return;
        };
        if track.duration <= 0 {
            return;
        }
        let seconds = fraction.clamp(0.0, 1.0) * track.duration as f32;
        self.send_playback(LocalPlaybackCommand::SeekAbsolute(seconds), cx);
    }

    fn set_volume(&mut self, fraction: f32, cx: &mut Context<Self>) {
        self.send_playback(
            LocalPlaybackCommand::SetVolume(fraction.clamp(0.0, 1.0)),
            cx,
        );
    }

    fn play_next(&mut self, id: i64, cx: &mut Context<Self>) {
        self.send_playback(LocalPlaybackCommand::PlayNext(id), cx);
        self.notice = Some("Playing next".to_string());
    }

    fn add_to_queue(&mut self, id: i64, cx: &mut Context<Self>) {
        self.send_playback(LocalPlaybackCommand::AddToQueue(id), cx);
        self.notice = Some("Added to queue".to_string());
    }

    fn send_playback(&mut self, command: LocalPlaybackCommand, cx: &mut Context<Self>) {
        match self
            .playback_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local audio is unavailable"))
            .and_then(|service| service.send(command))
        {
            Ok(()) => {}
            Err(error) => self.error = Some(format!("Playback error: {error:#}")),
        }
        cx.notify();
    }

    fn begin_playback_intent(&mut self, track_id: Option<i64>) {
        let command_id = self.next_playback_command_id;
        self.next_playback_command_id = self.next_playback_command_id.wrapping_add(1);
        self.playback_intent = Some(PlaybackIntent {
            command_id,
            expected_revision: self.playback.revision.saturating_add(1),
            track_id,
            issued_at: Instant::now(),
        });
        if self
            .notice
            .as_deref()
            .is_some_and(|notice| notice.starts_with("Starting local playback"))
        {
            self.notice = None;
        }
    }

    fn start_playback_monitor(&mut self, cx: &mut Context<Self>) {
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| loop {
            let delay = match this.update(cx, |this, cx| {
                let snapshot = this
                    .playback_service
                    .as_ref()
                    .map(LocalPlaybackService::snapshot);
                if let Some(snapshot) = snapshot {
                    if snapshot.revision > this.playback.revision {
                        this.playback = snapshot;
                        if this.playback_intent.as_ref().is_some_and(|intent| {
                            intent
                                .is_satisfied(this.playback.revision, this.playback.track.as_ref())
                        }) {
                            this.playback_intent = None;
                        } else if this.playback_intent.as_ref().is_some_and(|intent| {
                            intent.issued_at.elapsed() >= Duration::from_secs(2)
                        }) {
                            this.error = Some(this.playback.error.clone().unwrap_or_else(|| {
                                "Local audio did not start within two seconds".to_string()
                            }));
                            this.playback_intent = None;
                        }
                        let queue_changed =
                            this.playback.queue_revision != this.last_persisted_queue_revision;
                        if queue_changed
                            || this.last_playback_persist.elapsed() >= Duration::from_secs(5)
                        {
                            if queue_changed {
                                this.config.playback.queue_track_ids =
                                    this.playback.queue.iter().map(|track| track.id).collect();
                                this.last_persisted_queue_revision = this.playback.queue_revision;
                            }
                            this.config.playback.queue_index =
                                this.playback.queue_index.unwrap_or_default();
                            this.config.playback.position = this.playback.position;
                            this.config.playback.volume = this.playback.volume;
                            this.config.playback.shuffle = this.playback.shuffle;
                            this.config.playback.repeat = this.playback.repeat.clone();
                            this.last_playback_persist = Instant::now();
                            let config = this.config.clone();
                            cx.background_executor()
                                .spawn(async move {
                                    let _ = save_config(&config);
                                })
                                .detach();
                        }
                        cx.notify();
                    }
                }
                if this.playback.is_playing {
                    Duration::from_millis(200)
                } else {
                    Duration::from_secs(1)
                }
            }) {
                Ok(delay) => delay,
                Err(_) => break,
            };
            executor.timer(delay).await;
        })
        .detach();
    }

    fn choose_import(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some("Import Music".into()),
        });
        cx.spawn(async move |this, cx| {
            let paths = receiver.await.ok().and_then(Result::ok).flatten();
            if let Some(paths) = paths {
                this.update(cx, |this, cx| this.start_import(paths, cx))
                    .ok();
            }
        })
        .detach();
    }

    fn start_import(&mut self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        if self.import_progress.running || self.sync_progress.running {
            return;
        }
        self.operation_cancel.store(false, Ordering::Relaxed);
        self.import_progress = ImportProgress {
            running: true,
            found: paths.len(),
            ..Default::default()
        };
        let task_id = self.task_center.start(
            TaskKind::LibraryImport,
            "Import music",
            Some(DesktopSection::Music),
            true,
        );
        self.import_task = Some(task_id);
        self.notice = Some("Importing music…".to_string());
        let media_dir = self.config.media_dir.clone();
        let cancel = self.operation_cancel.clone();
        let (sender, receiver) = async_channel::unbounded();
        cx.background_executor()
            .spawn(async move {
                let progress_sender = sender.clone();
                let result = import_managed_library(&paths, &media_dir, &cancel, |progress| {
                    let _ = progress_sender.try_send(Ok(progress.clone()));
                });
                if let Err(error) = result {
                    let _ = sender.send(Err(format!("{error:#}"))).await;
                }
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(result) = receiver.recv().await {
                this.update(cx, |this, cx| {
                    match result {
                        Ok(progress) => {
                            if let Some(task_id) = this.import_task {
                                this.task_center.update_import(task_id, &progress);
                            }
                            let complete = !progress.running;
                            if complete {
                                this.notice = Some(format!(
                                    "Imported {} tracks; {} duplicates",
                                    progress.imported, progress.duplicates
                                ));
                            }
                            this.import_progress = progress;
                            if complete {
                                this.enrichment_started = false;
                                this.refresh_library(cx);
                            }
                        }
                        Err(error) => {
                            this.import_progress.running = false;
                            if let Some(task_id) = this.import_task {
                                this.task_center.fail(task_id, error.clone());
                            }
                            this.error = Some(format!("Import failed: {error}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn start_sync(&mut self, cx: &mut Context<Self>) {
        if self.import_progress.running || self.sync_progress.running {
            return;
        }
        self.operation_cancel.store(false, Ordering::Relaxed);
        self.sync_progress = SyncProgress {
            running: true,
            ..Default::default()
        };
        let task_id = self.task_center.start(
            TaskKind::Upload,
            "Sync MusicOS device",
            Some(DesktopSection::Sync),
            true,
        );
        self.sync_task = Some(task_id);
        self.notice = Some("Synchronizing music…".to_string());
        let root = self.config.sync_dir.clone();
        let target = self.config.device_target.clone();
        let transcode = self
            .config
            .format_profiles
            .iter()
            .find(|profile| profile.id == self.config.selected_sync_profile)
            .cloned()
            .unwrap_or_default()
            .to_transcode_options();
        let cancel = self.operation_cancel.clone();
        let (sender, receiver) = async_channel::unbounded();
        cx.background_executor()
            .spawn(async move {
                let result = ApiClient::new(target).and_then(|client| {
                    let progress_sender = sender.clone();
                    sync_files(&root, None, &client, &transcode, &cancel, |progress| {
                        let _ = progress_sender.try_send(Ok(progress.clone()));
                    })
                });
                if let Err(error) = result {
                    let _ = sender.send(Err(format!("{error:#}"))).await;
                }
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(result) = receiver.recv().await {
                this.update(cx, |this, cx| {
                    match result {
                        Ok(progress) => {
                            if let Some(task_id) = this.sync_task {
                                this.task_center.update_sync(task_id, &progress);
                            }
                            if !progress.running {
                                this.notice = Some(format!(
                                    "Sync complete: {} uploaded, {} skipped, {} failed",
                                    progress.uploaded, progress.skipped, progress.failed
                                ));
                                let history = progress.clone();
                                cx.background_executor()
                                    .spawn(async move {
                                        let _ = append_sync_history(&history);
                                    })
                                    .detach();
                            }
                            this.sync_progress = progress;
                        }
                        Err(error) => {
                            this.sync_progress.running = false;
                            if let Some(task_id) = this.sync_task {
                                this.task_center.fail(task_id, error.clone());
                            }
                            this.error = Some(format!("Sync failed: {error}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn cancel_operation(&mut self, cx: &mut Context<Self>) {
        self.operation_cancel.store(true, Ordering::Relaxed);
        if let Some(task_id) = self.import_task.or(self.sync_task) {
            self.task_center.request_cancel(task_id);
        }
        self.notice = Some("Cancellation requested…".to_string());
        cx.notify();
    }

    fn toggle_activity(&mut self, cx: &mut Context<Self>) {
        self.activity_open = !self.activity_open;
        cx.notify();
    }

    fn toggle_mini_player(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.compact_player {
            self.compact_player = false;
            window.resize(
                self.expanded_window_size
                    .take()
                    .unwrap_or_else(|| size(px(1280.0), px(800.0))),
            );
        } else {
            self.expanded_window_size = Some(window.bounds().size);
            self.compact_player = true;
            self.activity_open = false;
            self.context_menu = None;
            window.resize(size(px(456.0), px(164.0)));
        }
        window.activate_window();
        cx.notify();
    }

    fn compact_player_button(
        &self,
        id: &'static str,
        path: &'static str,
        active: bool,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .size(px(27.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(self.theme.hover))
            .active(|style| style.bg(self.theme.separator))
            .child(svg().path(path).size(px(15.0)).text_color(if active {
                self.theme.selected
            } else {
                self.theme.text
            }))
    }

    fn render_compact_player(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let now_playing = self.playback.track.as_ref();
        let title = now_playing
            .map(|track| track.title.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Nothing playing")
            .to_string();
        let artist = now_playing
            .map(|track| track.artist.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Choose a song in MusicOS")
            .to_string();
        let artwork_path = now_playing.and_then(|track| track.cover_art_path.clone());
        let duration = now_playing.map(|track| track.duration).unwrap_or_default();
        let timeline_fraction = if duration > 0 {
            (self.playback.position / duration as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let elapsed = format_duration(self.playback.position.max(0.0) as i32);
        let remaining = format_duration((duration as f32 - self.playback.position).max(0.0) as i32);
        let play_icon = if self.playback.is_playing {
            "icons/desktop/pause.svg"
        } else {
            "icons/desktop/play.svg"
        };
        let repeat_icon = if self.playback.repeat == "one" {
            "icons/desktop/repeat-one.svg"
        } else {
            "icons/desktop/repeat.svg"
        };
        let seek_bounds = Rc::new(RefCell::new(Bounds::default()));

        div()
            .id("compact-player")
            .size_full()
            .track_focus(&self.focus_handle)
            .font_family("Figtree")
            .bg(self.theme.window)
            .text_color(self.theme.text)
            .on_action(
                cx.listener(|this, _: &TogglePlayback, _, cx| {
                    this.player_command("play_pause", cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &PreviousTrack, _, cx| this.player_command("previous", cx)),
            )
            .on_action(cx.listener(|this, _: &NextTrack, _, cx| this.player_command("next", cx)))
            .on_action(cx.listener(|this, _: &ToggleMiniPlayer, window, cx| {
                this.toggle_mini_player(window, cx)
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => this.toggle_mini_player(window, cx),
                    "space" => this.player_command("play_pause", cx),
                    "left" => this.player_command("previous", cx),
                    "right" => this.player_command("next", cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .h_full()
                    .pt(px(27.0))
                    .px(px(10.0))
                    .pb(px(9.0))
                    .flex()
                    .gap(px(10.0))
                    .child(
                        div()
                            .size(px(96.0))
                            .flex_none()
                            .overflow_hidden()
                            .rounded(px(8.0))
                            .bg(self.theme.row_alternate)
                            .when_some(artwork_path.clone(), |cell, path| {
                                cell.child(
                                    img(path)
                                        .size_full()
                                        .rounded(px(8.0))
                                        .object_fit(ObjectFit::Cover)
                                        .with_loading(|| {
                                            artwork_placeholder_sized(96.0).into_any_element()
                                        })
                                        .with_fallback(|| {
                                            artwork_placeholder_sized(96.0).into_any_element()
                                        }),
                                )
                            })
                            .when(artwork_path.is_none(), |cell| {
                                cell.child(artwork_placeholder_sized(96.0))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .child(
                                                div()
                                                    .overflow_hidden()
                                                    .text_ellipsis()
                                                    .whitespace_nowrap()
                                                    .text_size(px(13.5))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(title),
                                            )
                                            .child(
                                                div()
                                                    .overflow_hidden()
                                                    .text_ellipsis()
                                                    .whitespace_nowrap()
                                                    .text_size(px(10.5))
                                                    .text_color(self.theme.secondary_text)
                                                    .child(artist),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("compact-expand")
                                            .h(px(25.0))
                                            .px(px(7.0))
                                            .flex_none()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .rounded(px(6.0))
                                            .cursor_pointer()
                                            .text_size(px(10.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(self.theme.secondary_text)
                                            .hover(|style| {
                                                style
                                                    .bg(self.theme.hover)
                                                    .text_color(self.theme.text)
                                            })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_mini_player(window, cx)
                                            }))
                                            .child(
                                                svg()
                                                    .path("icons/desktop/expand.svg")
                                                    .size(px(13.0)),
                                            )
                                            .child("Expand"),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .w(px(28.0))
                                            .text_size(px(9.0))
                                            .text_color(self.theme.secondary_text)
                                            .child(elapsed),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(14.0))
                                            .flex()
                                            .items_center()
                                            .on_children_prepainted({
                                                let seek_bounds = seek_bounds.clone();
                                                move |bounds, _, _| {
                                                    if let Some(bounds) = bounds.first() {
                                                        *seek_bounds.borrow_mut() = *bounds;
                                                    }
                                                }
                                            })
                                            .child(
                                                div()
                                                    .id("compact-seek")
                                                    .w_full()
                                                    .h(px(4.0))
                                                    .rounded(px(2.0))
                                                    .bg(self.theme.separator)
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, {
                                                        let seek_bounds = seek_bounds.clone();
                                                        cx.listener(
                                                            move |this,
                                                                  event: &gpui::MouseDownEvent,
                                                                  _,
                                                                  cx| {
                                                                let bounds = *seek_bounds.borrow();
                                                                let width =
                                                                    f32::from(bounds.size.width);
                                                                if width > 0.0 {
                                                                    let fraction = f32::from(
                                                                        event.position.x
                                                                            - bounds.origin.x,
                                                                    ) / width;
                                                                    this.seek_to(fraction, cx);
                                                                }
                                                            },
                                                        )
                                                    })
                                                    .child(
                                                        div()
                                                            .h_full()
                                                            .w(relative(timeline_fraction))
                                                            .rounded(px(2.0))
                                                            .bg(self.theme.selected),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(34.0))
                                            .text_size(px(9.0))
                                            .text_color(self.theme.secondary_text)
                                            .child(format!("−{remaining}")),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(1.0))
                                            .child(
                                                self.compact_player_button(
                                                    "compact-shuffle",
                                                    "icons/desktop/shuffle.svg",
                                                    self.playback.shuffle,
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.player_command("shuffle", cx)
                                                })),
                                            )
                                            .child(
                                                self.compact_player_button(
                                                    "compact-previous",
                                                    "icons/desktop/previous.svg",
                                                    false,
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.player_command("previous", cx)
                                                })),
                                            )
                                            .child(
                                                div()
                                                    .id("compact-play")
                                                    .size(px(31.0))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(px(8.0))
                                                    .bg(self.theme.selected)
                                                    .cursor_pointer()
                                                    .hover(|style| style.opacity(0.88))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.player_command("play_pause", cx)
                                                    }))
                                                    .child(
                                                        svg()
                                                            .path(play_icon)
                                                            .size(px(17.0))
                                                            .text_color(gpui::white()),
                                                    ),
                                            )
                                            .child(
                                                self.compact_player_button(
                                                    "compact-next",
                                                    "icons/desktop/next.svg",
                                                    false,
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.player_command("next", cx)
                                                })),
                                            )
                                            .child(
                                                self.compact_player_button(
                                                    "compact-repeat",
                                                    repeat_icon,
                                                    self.playback.repeat != "none",
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.player_command("repeat", cx)
                                                })),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(9.5))
                                            .text_color(self.theme.secondary_text)
                                            .child(format!(
                                                "{} in queue",
                                                self.playback.queue.len()
                                            )),
                                    ),
                            ),
                    ),
            )
    }

    fn clear_completed_tasks(&mut self, cx: &mut Context<Self>) {
        self.task_center.clear_completed();
        cx.notify();
    }

    fn stop_preview(&mut self, cx: &mut Context<Self>) {
        let task_id = self.task_center.start(
            TaskKind::EmulatorStop,
            "Stop Device Preview",
            Some(DesktopSection::Devices),
            false,
        );
        self.task_center.set_phase(
            task_id,
            musicos_desktop::models::TaskPhase::Stopping,
            Some("Stopping the optional preview".into()),
        );
        match self.preview.stop() {
            Ok(_) => {
                self.task_center.complete(task_id, "Device Preview stopped");
                self.notice =
                    Some("Device Preview stopped; local playback is unchanged".to_string());
            }
            Err(error) => {
                self.task_center.fail(task_id, format!("{error:#}"));
                self.error = Some(format!("Could not stop Device Preview: {error:#}"));
            }
        }
        cx.notify();
    }

    fn show_preview_controls(&mut self, cx: &mut Context<Self>) {
        if !self.preview.status().running {
            self.notice = Some(
                "Device Preview is stopped. Launch it to open the controls reference.".to_string(),
            );
            cx.notify();
            return;
        }
        let target = self.config.library_target.clone();
        let task = cx.background_executor().spawn(async move {
            ApiClient::new(target)?
                .post::<_, serde_json::Value>("/api/preview/show-controls", &serde_json::json!({}))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => this.notice = Some("Preview controls opened".to_string()),
                    Err(error) => {
                        this.error = Some(format!("Could not show preview controls: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn reveal_managed_library(&mut self, cx: &mut Context<Self>) {
        self.reveal_library_path(self.config.media_dir.clone(), cx);
    }

    fn reveal_library_path(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        match musicos_desktop::platform::reveal_in_file_manager(&path) {
            Ok(()) => self.notice = Some("Library folder revealed".to_string()),
            Err(error) => {
                self.error = Some(format!("Could not reveal the library folder: {error:#}"))
            }
        }
        cx.notify();
    }

    fn export_diagnostics(&mut self, cx: &mut Context<Self>) {
        let directory = std::env::current_dir().unwrap_or_default().join("out");
        let receiver = cx.prompt_for_new_path(
            &directory,
            Some(&format!(
                "musicos-diagnostics-{}.json",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            )),
        );
        let config = self.config.clone();
        let emulator = self.preview.status();
        let logs = self.preview.logs();
        cx.spawn(async move |this, cx| {
            let destination = receiver.await.ok().and_then(Result::ok).flatten();
            if let Some(destination) = destination {
                let task = cx.background_executor().spawn(async move {
                    diagnostics::export(&destination, &config, &emulator, &logs)?;
                    Ok::<_, anyhow::Error>(destination)
                });
                let result = task.await;
                this.update(cx, |this, cx| {
                    match result {
                        Ok(path) => {
                            this.notice =
                                Some(format!("Diagnostics exported to {}", path.display()));
                        }
                        Err(error) => {
                            this.error = Some(format!("Diagnostics export failed: {error:#}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn clear_cover_cache(&mut self, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.clear_cover_cache() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(cleared) => {
                        this.notice = Some(format!("Cleared {cleared} cached artwork entries"));
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not clear artwork cache: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn repair_library(&mut self, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.repair() });
        self.notice = Some("Checking and repairing the library database…".to_string());
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.notice = Some("Library database repair completed".to_string());
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Library repair failed: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn request_indexed_leftovers_cleanup(&mut self, cx: &mut Context<Self>) {
        if self.source_audit.hidden_tracks.is_empty() {
            self.notice = Some("No indexed leftovers were found".to_string());
            cx.notify();
            return;
        }
        self.pending_confirmation = Some(PendingConfirmation::CleanupIndexedLeftovers {
            revision: self.source_audit.revision,
            count: self.source_audit.hidden_tracks.len(),
            playlist_references: self.source_audit.playlist_references,
        });
        cx.notify();
    }

    fn confirm_indexed_leftovers_cleanup(
        &mut self,
        expected_revision: u64,
        _cx: &mut Context<Self>,
    ) {
        self.pending_confirmation = None;
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.cleanup_indexed_leftovers(expected_revision) });
        self.notice = Some("Backing up the database before index cleanup…".to_string());
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(report) => {
                        this.notice = Some(format!(
                            "Removed {} index records · backup saved to {}",
                            report.removed_records,
                            report.backup_path.display()
                        ));
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Indexed leftovers were not removed: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn reveal_cover_cache(&mut self, cx: &mut Context<Self>) {
        let path = self
            .config
            .db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("cover-art");
        cx.reveal_path(if path.exists() {
            &path
        } else {
            path.parent().unwrap_or(&path)
        });
        self.notice = Some(format!("Revealed {}", path.display()));
        cx.notify();
    }

    fn request_database_reset(&mut self, cx: &mut Context<Self>) {
        self.pending_confirmation = Some(PendingConfirmation::ResetDatabase);
        cx.notify();
    }

    fn confirm_database_reset(&mut self, cx: &mut Context<Self>) {
        self.pending_confirmation = None;
        self.playback_service = None;
        let _ = self.preview.stop();
        let config = self.config.clone();
        let task = cx
            .background_executor()
            .spawn(async move { diagnostics::reset_database(&config, true) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(backup) => {
                        this.notice = Some(format!(
                            "Database reset; previous database preserved at {}",
                            backup.display()
                        ));
                        this.repository = LocalLibraryRepository::new(
                            this.config.db_path.clone(),
                            this.config.media_dir.clone(),
                        );
                        this.playback_service = LocalPlaybackService::start_with_preferences(
                            this.config.db_path.clone(),
                            this.config.playback.clone(),
                        )
                        .ok();
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Database reset failed: {error:#}"));
                        this.playback_service = LocalPlaybackService::start_with_preferences(
                            this.config.db_path.clone(),
                            this.config.playback.clone(),
                        )
                        .ok();
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn cancel_confirmation(&mut self, cx: &mut Context<Self>) {
        self.pending_confirmation = None;
        cx.notify();
    }

    fn open_context_menu(
        &mut self,
        track_id: i64,
        position: gpui::Point<gpui::Pixels>,
        _cx: &mut Context<Self>,
    ) {
        self.selected_track_id = Some(track_id);
        self.context_menu = Some(ContextMenuState {
            track_id,
            position,
            playlists_open: false,
        });
        cx.notify();
    }

    fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.context_menu.is_some() {
            self.context_menu = None;
            cx.notify();
        }
    }

    fn toggle_context_menu_playlists(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.context_menu.as_mut() {
            menu.playlists_open = !menu.playlists_open;
            cx.notify();
        }
    }

    fn context_play(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.close_context_menu(cx);
        self.play_visible_track(track_id, cx);
    }

    fn context_play_next(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.close_context_menu(cx);
        self.play_next(track_id, cx);
    }

    fn context_add_to_queue(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.close_context_menu(cx);
        self.add_to_queue(track_id, cx);
    }

    fn context_add_to_playlist(&mut self, playlist_id: i64, track_id: i64, cx: &mut Context<Self>) {
        self.close_context_menu(cx);
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.add_tracks(playlist_id, &[track_id]) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.notice = Some("Added to playlist".to_string());
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not add to playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn context_remove_from_playlist(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.close_context_menu(cx);
        let Some(playlist) = self.active_playlist.clone() else {
            return;
        };
        let repository = self.repository.clone();
        let id = playlist.id;
        let task = cx
            .background_executor()
            .spawn(async move { repository.remove_tracks(id, &[track_id]) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.playlist_tracks.retain(|track| track.id != track_id);
                        this.notice = Some("Removed from playlist".to_string());
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not remove from playlist: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_favorite_track(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.close_context_menu(cx);
        let repository = self.repository.clone();
        let task = cx
            .background_executor()
            .spawn(async move { repository.toggle_favorite(track_id) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(favorited) => {
                        if let Some(track) = this
                            .snapshot
                            .tracks
                            .iter_mut()
                            .find(|track| track.id == track_id)
                        {
                            track.favorited = favorited;
                        }
                        this.notice = Some(if favorited {
                            "Added to favorites".to_string()
                        } else {
                            "Removed from favorites".to_string()
                        });
                        this.rebuild_library_index();
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not update favorite: {error:#}"))
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn backup_library(&mut self, cx: &mut Context<Self>) {
        let directory = std::env::current_dir().unwrap_or_default().join("out");
        let receiver = cx.prompt_for_new_path(
            &directory,
            Some(&format!(
                "musicos-library-backup-{}.json",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            )),
        );
        let repository = self.repository.clone();
        let settings = serde_json::to_value(&self.config).unwrap_or_default();
        cx.spawn(async move |this, cx| {
            let destination = receiver.await.ok().and_then(Result::ok).flatten();
            if let Some(destination) = destination {
                let task = cx.background_executor().spawn(async move {
                    let document = backup::collect_repository(&repository, settings)?;
                    backup::save(&destination, &document)?;
                    Ok::<_, anyhow::Error>(destination)
                });
                let result = task.await;
                this.update(cx, |this, cx| {
                    match result {
                        Ok(path) => {
                            this.notice =
                                Some(format!("Library backup saved to {}", path.display()));
                        }
                        Err(error) => {
                            this.error = Some(format!("Library backup failed: {error:#}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn choose_restore_backup(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose MusicOS Backup".into()),
        });
        cx.spawn(async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .and_then(Result::ok)
                .flatten()
                .and_then(|mut paths| paths.pop());
            if let Some(path) = path {
                let validation_path = path.clone();
                let task = cx
                    .background_executor()
                    .spawn(async move { backup::load(&validation_path) });
                let result = task.await;
                this.update(cx, |this, cx| {
                    match result {
                        Ok(_) => {
                            this.pending_confirmation =
                                Some(PendingConfirmation::RestoreBackup { path });
                        }
                        Err(error) => {
                            this.error = Some(format!("Could not read backup: {error:#}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn confirm_restore_backup(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.pending_confirmation = None;
        let repository = self.repository.clone();
        let task = cx.background_executor().spawn(async move {
            let document = backup::load(&path)?;
            backup::restore_repository(&repository, &document)
        });
        self.notice = Some("Restoring settings and playlists…".to_string());
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.notice = Some(format!(
                            "Restored settings, {} playlists, and {} playlist tracks",
                            result.restored_playlists, result.restored_playlist_tracks
                        ));
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Library restore failed: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn request_update_install(&mut self, cx: &mut Context<Self>) {
        self.pending_confirmation = Some(PendingConfirmation::InstallUpdate);
        cx.notify();
    }

    fn confirm_update_install(&mut self, cx: &mut Context<Self>) {
        self.pending_confirmation = None;
        let target = self.config.device_target.clone();
        let task = cx.background_executor().spawn(async move {
            ApiClient::new(target)?
                .post::<_, serde_json::Value>("/api/updates/install", &serde_json::json!({}))
        });
        self.notice = Some("Checking removable storage for a staged update…".to_string());
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(value) => {
                        this.notice = value
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                            .or_else(|| Some("Update request completed".to_string()));
                    }
                    Err(error) => {
                        this.error = Some(format!("Update installation failed: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_input.focus_handle(cx).is_focused(window)
            || self.playlist_name_input.focus_handle(cx).is_focused(window)
        {
            return;
        }
        if self
            .route
            .library()
            .is_some_and(|route| route.presentation == LibraryPresentation::Flow)
        {
            match event.keystroke.key.as_str() {
                "left" => {
                    self.move_flow(-1, cx);
                    return;
                }
                "right" => {
                    self.move_flow(1, cx);
                    return;
                }
                "enter" => {
                    if let Some(album) = self.library_index.albums.get(self.flow_focus) {
                        let ids = album
                            .track_indices
                            .iter()
                            .map(|index| self.library_index.tracks[*index].id)
                            .collect();
                        self.play_tracks(ids, cx);
                    }
                    return;
                }
                _ => {}
            }
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                if self.context_menu.is_some() {
                    self.close_context_menu(cx);
                    return;
                }
                self.selected_track = None;
                self.selected_track_id = None;
            }
            "up" => {
                self.selected_track =
                    Some(self.selected_visible_index().unwrap_or(1).saturating_sub(1));
                self.selected_track_id = self
                    .visible_tracks()
                    .get(self.selected_track.unwrap_or_default())
                    .map(|track| track.id);
            }
            "down" => {
                let last = self.visible_tracks().len().saturating_sub(1);
                self.selected_track = Some(
                    self.selected_visible_index()
                        .map_or(0, |index| index + 1)
                        .min(last),
                );
                self.selected_track_id = self
                    .visible_tracks()
                    .get(self.selected_track.unwrap_or_default())
                    .map(|track| track.id);
            }
            "enter" => {
                if self.section() == DesktopSection::Queue {
                    self.play_selected_queue_item(cx);
                    return;
                }
                let tracks = self.visible_tracks();
                if let Some(track) = self
                    .selected_track_id
                    .and_then(|id| tracks.iter().find(|track| track.id == id))
                    .or_else(|| tracks.first())
                {
                    self.play_visible_track(track.id, cx);
                    return;
                }
            }
            "space" => {
                self.player_command("play_pause", cx);
                return;
            }
            "backspace" | "delete" => {
                if self.section() == DesktopSection::Queue {
                    self.remove_selected_queue_item(cx);
                    return;
                }
                if self.section() == DesktopSection::Playlists && self.active_playlist.is_some() {
                    self.remove_selected_from_playlist(cx);
                    return;
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn visible_tracks(&self) -> Vec<Track> {
        self.visible_track_indices()
            .into_iter()
            .filter_map(|index| self.track_collection().get(index).cloned())
            .collect()
    }

    fn track_collection(&self) -> &[Track] {
        if self.section() == DesktopSection::Queue {
            &self.playback.queue
        } else if self.section() == DesktopSection::Playlists && self.active_playlist.is_some() {
            &self.playlist_tracks
        } else {
            &self.snapshot.tracks
        }
    }

    fn visible_track_indices(&self) -> Vec<usize> {
        if self.section() == DesktopSection::Queue {
            let query = self.search.trim().to_lowercase();
            return self
                .playback
                .queue
                .iter()
                .enumerate()
                .filter(|(_, track)| {
                    query.is_empty()
                        || track.title.to_lowercase().contains(&query)
                        || track.artist.to_lowercase().contains(&query)
                        || track.album.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect();
        }
        if self.section() == DesktopSection::Playlists && self.active_playlist.is_some() {
            let query = self.search.trim().to_lowercase();
            return self
                .playlist_tracks
                .iter()
                .enumerate()
                .filter(|(_, track)| {
                    query.is_empty()
                        || track.title.to_lowercase().contains(&query)
                        || track.artist.to_lowercase().contains(&query)
                        || track.album.to_lowercase().contains(&query)
                        || track.genre.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect();
        }
        if let Some(drilldown) = self
            .route
            .library()
            .and_then(|route| route.drilldown.as_ref())
        {
            let tracks = &self.library_index.tracks;
            let query = self.search.trim().to_lowercase();
            let base = match drilldown {
                LibraryDrilldown::Album(id) => self
                    .library_index
                    .albums
                    .iter()
                    .find(|album| &album.id == id)
                    .map(|album| album.track_indices.clone()),
                LibraryDrilldown::Artist(id) => self
                    .library_index
                    .artists
                    .iter()
                    .find(|artist| &artist.id == id)
                    .map(|artist| artist.track_indices.clone()),
                LibraryDrilldown::Genre(name) => self
                    .library_index
                    .genres
                    .iter()
                    .find(|genre| &genre.name == name)
                    .map(|genre| genre.track_indices.clone()),
            }
            .unwrap_or_default();
            let mut indices = base
                .into_iter()
                .filter(|index| {
                    let track = &tracks[*index];
                    query.is_empty()
                        || track.title.to_lowercase().contains(&query)
                        || track.artist.to_lowercase().contains(&query)
                        || track.album.to_lowercase().contains(&query)
                        || track.genre.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            let field = self.config.sort.field;
            indices.sort_by(|left, right| compare_tracks(&tracks[*left], &tracks[*right], field));
            if self.config.sort.descending {
                indices.reverse();
            }
            return indices;
        }
        if matches!(
            &self.route,
            AppRoute::Library(DesktopLibraryRoute {
                scope: LibraryScope::Music
                    | LibraryScope::Songs
                    | LibraryScope::Albums
                    | LibraryScope::Artists
                    | LibraryScope::Genres
                    | LibraryScope::Favorites
                    | LibraryScope::History,
                ..
            })
        ) {
            return self.library_index.filtered_tracks.clone();
        }
        let query = self.search.trim().to_lowercase();
        let tracks = self.track_collection();
        let mut indices = tracks
            .iter()
            .enumerate()
            .filter(|track| {
                let track = track.1;
                let section_match = match self.section() {
                    DesktopSection::Favorites => track.favorited,
                    _ => true,
                };
                section_match
                    && (query.is_empty()
                        || track.title.to_lowercase().contains(&query)
                        || track.artist.to_lowercase().contains(&query)
                        || track.album.to_lowercase().contains(&query)
                        || track.genre.to_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let field = self.config.sort.field;
        indices.sort_by(|left, right| compare_tracks(&tracks[*left], &tracks[*right], field));
        if self.config.sort.descending {
            indices.reverse();
        }
        indices
    }

    fn toggle_sort(&mut self, field: SortField, cx: &mut Context<Self>) {
        if self.config.sort.field == field {
            self.config.sort.descending = !self.config.sort.descending;
        } else {
            self.config.sort.field = field;
            self.config.sort.descending = false;
        }
        self.rebuild_library_index();
        let _ = save_config(&self.config);
        self.selected_track = self.selected_track_id.and_then(|selected_id| {
            self.visible_track_indices()
                .into_iter()
                .position(|index| self.track_collection()[index].id == selected_id)
        });
        cx.notify();
    }

    fn selected_visible_index(&self) -> Option<usize> {
        let selected_id = self.selected_track_id?;
        self.visible_track_indices()
            .into_iter()
            .position(|index| self.track_collection()[index].id == selected_id)
    }

    fn sortable_column(
        &self,
        _cx: &mut Context<Self>,
        label: &'static str,
        width: f32,
        field: SortField,
    ) -> gpui::Stateful<Div> {
        let active = self.config.sort.field == field;
        let direction = if self.config.sort.descending {
            "↓"
        } else {
            "↑"
        };
        div()
            .id(("sort-column", field as usize))
            .w(px(width))
            .min_w(px(width))
            .h_full()
            .px(px(7.0))
            .flex()
            .items_center()
            .justify_between()
            .overflow_hidden()
            .border_r_1()
            .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(ui_color(0xe9e9ee, 0x2c2c2e))))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_sort(field, cx)))
            .child(label)
            .when(active, |header| {
                header.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(ui_color(BLUE, DARK_BLUE)))
                        .child(direction),
                )
            })
    }

    fn chrome_button(
        id: impl Into<gpui::ElementId>,
        label: impl Into<SharedString>,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .h(px(29.0))
            .min_w(px(32.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .bg(rgb(ui_color(0xe4e8ed, 0x303844)))
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(ui_color(0xd8dde5, 0x3a4553))))
            .active(|style| style.bg(rgb(ui_color(0xcbd2dc, 0x465364))))
            .child(label.into())
    }

    fn chrome_icon_button(
        id: impl Into<gpui::ElementId>,
        path: &'static str,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .size(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .bg(rgb(ui_color(0xe4e8ed, 0x303844)))
            .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(ui_color(0xd8dde5, 0x3a4553))))
            .active(|style| style.bg(rgb(ui_color(0xcbd2dc, 0x465364))))
            .child(
                svg()
                    .path(path)
                    .size(px(15.0))
                    .text_color(rgb(ui_color(TEXT, DARK_TEXT))),
            )
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .w(px(720.0))
            .h(px(62.0))
            .flex_none()
            .relative()
            .bg(rgb(0x00baff))
            .border_2()
            .border_color(rgb(0xffffff))
            .child(
                // 1. Traffic lights capsule: x: 18, y: 19, w: 96, h: 24, bg #D6DEFD
                div()
                    .absolute()
                    .left(px(18.0))
                    .top(px(19.0))
                    .w(px(96.0))
                    .h(px(24.0))
                    .rounded_full()
                    .bg(rgb(0xd6defd))
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .justify_between()
                    // Red
                    .child(
                        div()
                            .id("traffic-red")
                            .size(px(24.0))
                            .rounded_full()
                            .bg(rgb(0xff0000))
                            .border_1()
                            .border_color(rgb(0xc90000))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|_, _, _, cx| cx.quit()))
                            .child(
                                svg()
                                    .path("icons/ui/tinyX.svg")
                                    .size(px(8.0))
                            )
                    )
                    // Yellow
                    .child(
                        div()
                            .id("traffic-yellow")
                            .size(px(24.0))
                            .rounded_full()
                            .bg(rgb(0xffe100))
                            .border_1()
                            .border_color(rgb(0xffa900))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.compact_player = !this.compact_player;
                                cx.notify();
                            }))
                            .child(
                                svg()
                                    .path("icons/ui/tinyMinimize.svg")
                                    .size(px(8.0))
                            )
                    )
                    // Green
                    .child(
                        div()
                            .id("traffic-green")
                            .size(px(24.0))
                            .rounded_full()
                            .bg(rgb(0x1fc800))
                            .border_1()
                            .border_color(rgb(0x1bab00))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(
                                svg()
                                    .path("icons/ui/tinyMaximize.svg")
                                    .size(px(8.0))
                            )
                    )
            )
            .child(
                // 2. Navigation Scope Pill: x: 130, y: 7, w: 222, h: 48, bg #0064DF
                div()
                    .id("scope-toggle-pill")
                    .absolute()
                    .left(px(130.0))
                    .top(px(7.0))
                    .w(px(222.0))
                    .h(px(48.0))
                    .rounded_full()
                    .bg(rgb(0x0064df))
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.devices_mode = !this.devices_mode;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .pl(px(8.0))
                            .child(
                                svg()
                                    .path("icons/ui/3dots_(nocircle).svg")
                                    .size(px(32.0))
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_center()
                            .text_size(px(20.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child(if self.devices_mode { "Devices" } else { "Library" })
                    )
                    .child(
                        div()
                            .pr(px(12.0))
                            .child(
                                svg()
                                    .path("icons/ui/tinyMinimize.svg")
                                    .size(px(16.0))
                            )
                    )
            )
            .child(
                // 3. Now Playing Pill: x: 368, y: 7, w: 334, h: 48, bg #FFFFFF
                div()
                    .absolute()
                    .left(px(368.0))
                    .top(px(7.0))
                    .w(px(334.0))
                    .h(px(48.0))
                    .rounded_full()
                    .bg(rgb(0xffffff))
                    .shadow_sm()
                    .relative()
                    // Artwork: x: 8, y: 8, 32x32
                    .child(
                        div()
                            .absolute()
                            .left(px(8.0))
                            .top(px(8.0))
                            .size(px(32.0))
                            .rounded(px(4.0))
                            .overflow_hidden()
                            .bg(rgb(0xd6defd))
                            .child(
                                img("covers/sky.png")
                                    .size_full()
                                    .object_fit(ObjectFit::Cover)
                            )
                    )
                    // Title: x: 44, y: 7, w: 145, h: 12
                    .child(
                        div()
                            .absolute()
                            .left(px(44.0))
                            .top(px(7.0))
                            .w(px(145.0))
                            .h(px(12.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x000000))
                            .child("Imminent Reccolection")
                    )
                    // Artist: x: 44, y: 22, w: 114, h: 8
                    .child(
                        div()
                            .absolute()
                            .left(px(44.0))
                            .top(px(22.0))
                            .w(px(114.0))
                            .h(px(10.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(8.0))
                            .text_color(rgb(0x707070))
                            .child("Solitude maybe? - HelloWorld")
                    )
                    // Progress bar: x: 44, y: 34, w: 145, h: 4
                    .child(
                        div()
                            .absolute()
                            .left(px(44.0))
                            .top(px(34.0))
                            .w(px(145.0))
                            .h(px(4.0))
                            .rounded(px(2.0))
                            .bg(rgb(0xd6effd))
                            .child(
                                div()
                                    .h_full()
                                    .w(px(42.0))
                                    .rounded(px(2.0))
                                    .bg(rgb(0x00a6ff))
                            )
                    )
                    // Queue icon: x: 201, y: 8, 16x16
                    .child(
                        div()
                            .id("nowplaying-queue")
                            .absolute()
                            .left(px(201.0))
                            .top(px(8.0))
                            .size(px(16.0))
                            .child(svg().path("icons/ui/queue.svg").size_full())
                    )
                    // Lossless badge: x: 217, y: 8, 16x16
                    .child(
                        div()
                            .absolute()
                            .left(px(217.0))
                            .top(px(8.0))
                            .size(px(16.0))
                            .child(svg().path("icons/ui/Lossless.svg").size_full())
                    )
                    // Repeat badge: x: 201, y: 24, 16x16
                    .child(
                        div()
                            .id("nowplaying-repeat")
                            .absolute()
                            .left(px(201.0))
                            .top(px(24.0))
                            .size(px(16.0))
                            .child(svg().path("icons/ui/RepeatOFF.svg").size_full())
                    )
                    // Shuffle badge: x: 217, y: 24, 16x16
                    .child(
                        div()
                            .id("nowplaying-shuffle")
                            .absolute()
                            .left(px(217.0))
                            .top(px(24.0))
                            .size(px(16.0))
                            .child(svg().path("icons/ui/ShuffleON.svg").size_full())
                    )
                    // Transport Prev: x: 233, y: 8, 32x32
                    .child(
                        div()
                            .id("nowplaying-prev")
                            .absolute()
                            .left(px(233.0))
                            .top(px(8.0))
                            .size(px(32.0))
                            .cursor_pointer()
                            .child(svg().path("icons/ui/prev.svg").size_full())
                    )
                    // Transport Play/Pause: x: 265, y: 8, 32x32
                    .child(
                        div()
                            .id("nowplaying-play")
                            .absolute()
                            .left(px(265.0))
                            .top(px(8.0))
                            .size(px(32.0))
                            .cursor_pointer()
                            .child(svg().path("icons/ui/pause.svg").size_full())
                    )
                    // Transport Next: x: 297, y: 8, 32x32
                    .child(
                        div()
                            .id("nowplaying-next")
                            .absolute()
                            .left(px(297.0))
                            .top(px(8.0))
                            .size(px(32.0))
                            .cursor_pointer()
                            .child(svg().path("icons/ui/next.svg").size_full())
                    )
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Div {
        if self.devices_mode {
            // Devices Mode Sidebar: width 255, height 378, bg #CBF1FF
            div()
                .size_full()
                .relative()
                .bg(rgb(0xcbf1ff))
                // Header: Henry Music 01 (w: 196, h: 20, bg #D6DEFD)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(px(196.0))
                        .h(px(20.0))
                        .bg(rgb(0xd6defd))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x000000))
                        .child("Henry Music 01")
                )
                // Device summary row (y: 24)
                .child(
                    div()
                        .absolute()
                        .top(px(24.0))
                        .left(px(11.0))
                        .w(px(232.0))
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            svg()
                                .path("icons/ui/device.svg")
                                .size(px(32.0))
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Henry Music 01")
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x606060))
                                        .child("Music 01 - 256GB - 200GB Free")
                                )
                        )
                        .child(
                            svg()
                                .path("icons/ui/3dots(incircle).svg")
                                .size(px(16.0))
                        )
                )
                // Storage Bar Card: y: 64, w: 232, h: 42, bg #00C3FF
                .child(
                    div()
                        .absolute()
                        .top(px(64.0))
                        .left(px(11.0))
                        .w(px(232.0))
                        .h(px(42.0))
                        .rounded(px(6.0))
                        .bg(rgb(0x00c3ff))
                        .p(px(6.0))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .w_full()
                                .h(px(16.0))
                                .rounded(px(3.0))
                                .bg(rgb(0xd6effd))
                                .overflow_hidden()
                                .child(
                                    div()
                                        .h_full()
                                        .w(px(160.0))
                                        .bg(rgb(0x00e5ff))
                                        .flex()
                                        .items_center()
                                        .pl(px(4.0))
                                        .text_size(px(9.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Music")
                                )
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .justify_between()
                                .text_size(px(8.5))
                                .text_color(rgb(0x000000))
                                .child("200GB Free")
                                .child("256GB Total")
                        )
                )
                // Quick(er) Settings Card: y: 114, w: 232, h: 84, bg #81C7FF
                .child(
                    div()
                        .absolute()
                        .top(px(114.0))
                        .left(px(11.0))
                        .w(px(232.0))
                        .h(px(84.0))
                        .rounded(px(6.0))
                        .bg(rgb(0x81c7ff))
                        .p(px(6.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("Quick(er) Settings")
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(4.0))
                                        .bg(rgb(0x476cff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(9.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("Lossless")
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(4.0))
                                        .bg(rgb(0x476cff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(9.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("Internet")
                                )
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(4.0))
                                        .bg(rgb(0xd2dbff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(8.5))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Auto-Sync")
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(4.0))
                                        .bg(rgb(0xd2dbff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(8.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Sparklist Auto")
                                )
                        )
                )
                // Audio Settings Card: y: 206, w: 232, h: 56, bg #81C7FF
                .child(
                    div()
                        .absolute()
                        .top(px(206.0))
                        .left(px(11.0))
                        .w(px(232.0))
                        .h(px(56.0))
                        .rounded(px(6.0))
                        .bg(rgb(0x81c7ff))
                        .p(px(6.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(9.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("Audio Settings")
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(4.0))
                                        .bg(rgb(0x00acff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(9.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("EQ")
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(24.0))
                                        .rounded(px(4.0))
                                        .bg(rgb(0x5f80ff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(9.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("Quality")
                                )
                        )
                )
                // 99% Card: y: 270, w: 232, h: 96, bg #0034FF
                .child(
                    div()
                        .absolute()
                        .top(px(270.0))
                        .left(px(11.0))
                        .w(px(232.0))
                        .h(px(96.0))
                        .rounded(px(8.0))
                        .bg(rgb(0x0034ff))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(32.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child("99%")
                )
        } else {
            // Library Mode Sidebar: width 196, height 414, bg #CBF1FF
            div()
                .size_full()
                .relative()
                .bg(rgb(0xcbf1ff))
                // Header: Library (w: 196, h: 20, bg #D6DEFD)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(px(196.0))
                        .h(px(20.0))
                        .bg(rgb(0xd6defd))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x000000))
                        .child("Library")
                )
                // Row 0 (Albums): y: 20, h: 32, bg #009AFF
                .child(
                    div()
                        .id("nav-albums")
                        .absolute()
                        .top(px(20.0))
                        .left_0()
                        .w(px(196.0))
                        .h(px(32.0))
                        .bg(rgb(0x009aff))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.navigate(DesktopSection::Albums, cx);
                        }))
                        .child(
                            div()
                                .size_full()
                                .relative()
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top_0()
                                        .size(px(32.0))
                                        .child(svg().path("icons/ui/Album.svg").size_full())
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(40.0))
                                        .top(px(8.0))
                                        .w(px(116.0))
                                        .text_center()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Albums")
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .right_0()
                                        .top_0()
                                        .size(px(32.0))
                                        .child(svg().path("icons/ui/3dots_(nocircle).svg").size_full())
                                )
                        )
                )
                // Row 1 (Artists): y: 52, h: 32, bg #AAE4FF
                .child(
                    div()
                        .id("nav-artists")
                        .absolute()
                        .top(px(52.0))
                        .left_0()
                        .w(px(196.0))
                        .h(px(32.0))
                        .bg(rgb(0xaae4ff))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.navigate(DesktopSection::Artists, cx);
                        }))
                        .child(
                            div()
                                .size_full()
                                .relative()
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top_0()
                                        .size(px(32.0))
                                        .child(svg().path("icons/ui/Artist.svg").size_full())
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(40.0))
                                        .top(px(8.0))
                                        .w(px(116.0))
                                        .text_center()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Artists")
                                )
                        )
                )
                // Row 2 (Genres): y: 84, h: 32, bg #AAE4FF
                .child(
                    div()
                        .id("nav-genres")
                        .absolute()
                        .top(px(84.0))
                        .left_0()
                        .w(px(196.0))
                        .h(px(32.0))
                        .bg(rgb(0xaae4ff))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.navigate(DesktopSection::Genres, cx);
                        }))
                        .child(
                            div()
                                .size_full()
                                .relative()
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top_0()
                                        .size(px(32.0))
                                        .child(svg().path("icons/ui/Genre.svg").size_full())
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(40.0))
                                        .top(px(8.0))
                                        .w(px(116.0))
                                        .text_center()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Genres")
                                )
                        )
                )
                // Row 3 (Songs): y: 116, h: 32, bg #AAE4FF
                .child(
                    div()
                        .id("nav-songs")
                        .absolute()
                        .top(px(116.0))
                        .left_0()
                        .w(px(196.0))
                        .h(px(32.0))
                        .bg(rgb(0xaae4ff))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.navigate(DesktopSection::Songs, cx);
                        }))
                        .child(
                            div()
                                .size_full()
                                .relative()
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top_0()
                                        .size(px(32.0))
                                        .child(svg().path("icons/ui/Song.svg").size_full())
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(40.0))
                                        .top(px(8.0))
                                        .w(px(116.0))
                                        .text_center()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .child("Songs")
                                )
                        )
                )
                // "New:" container: y: 150, h: 74, bg rgba(0, 186, 255, 0.5)
                .child(
                    div()
                        .absolute()
                        .top(px(150.0))
                        .left_0()
                        .w(px(196.0))
                        .h(px(74.0))
                        .bg(rgba(0x00baff80))
                        .child(
                            div()
                                .w_full()
                                .pt(px(4.0))
                                .text_center()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("New:")
                        )
                        .child(
                            div()
                                .pt(px(4.0))
                                .px(px(4.0))
                                .flex()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .id("btn-new-playlist")
                                        .w(px(92.0))
                                        .h(px(32.0))
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(rgba(0x00000080))
                                        .bg(rgb(0x00e6ff))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(4.0))
                                        .cursor_pointer()
                                        .child(svg().path("icons/ui/Playlist.svg").size(px(20.0)))
                                        .child(
                                            div()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0x000000))
                                                .child("Playlist")
                                        )
                                )
                                .child(
                                    div()
                                        .id("btn-new-sparklist")
                                        .w(px(92.0))
                                        .h(px(32.0))
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(rgba(0x00000080))
                                        .bg(rgb(0xa7dee4))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(4.0))
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.sparklist_modal_open = true;
                                            cx.notify();
                                        }))
                                        .child(svg().path("icons/ui/Sparklist.svg").size(px(20.0)))
                                        .child(
                                            div()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0x000000))
                                                .child("Sparklist")
                                        )
                                )
                        )
                )
                // Divider: y: 226, h: 2, bg #0075FF
                .child(
                    div()
                        .absolute()
                        .top(px(226.0))
                        .left_0()
                        .w(px(196.0))
                        .h(px(2.0))
                        .bg(rgb(0x0075ff))
                )
                // Playlists Box ("{user}s stuff:"): y: 228, w: 192, h: 184, bg #54B9FF
                .child(
                    div()
                        .absolute()
                        .top(px(228.0))
                        .left(px(2.0))
                        .w(px(192.0))
                        .h(px(184.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(rgb(0x0041ff))
                        .bg(rgb(0x54b9ff))
                        .relative()
                        // Header
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w(px(192.0))
                                .h(px(18.0))
                                .bg(rgb(0xd6defd))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("{user}s stuff:")
                        )
                        // Items list: 4 items starting at y: 20
                        .child(
                            div()
                                .absolute()
                                .top(px(18.0))
                                .left_0()
                                .w(px(181.0))
                                .h(px(164.0))
                                .flex()
                                .flex_col()
                                .child(self.render_playlist_row(0, "Treelist", "3h21m - 400 songs", "covers/sky.png", false, cx))
                                .child(self.render_playlist_row(1, "My Playlist40", "40m - 10 songs", "covers/tritower.png", false, cx))
                                .child(self.render_playlist_row(2, "Favorites", "Sparklist …", "covers/sparklist_1.png", true, cx))
                                .child(self.render_playlist_row(3, "My Playlist8", "40m - 10 songs", "covers/Overcast.png", false, cx))
                        )
                        // Scrollbar track: x: 181, y: 17, w: 8, h: 156
                        .child(
                            div()
                                .absolute()
                                .top(px(17.0))
                                .left(px(181.0))
                                .w(px(8.0))
                                .h(px(156.0))
                                .border_1()
                                .border_color(rgb(0x00195e))
                                .bg(rgb(0x739fff))
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(33.0))
                                        .bg(rgb(0x0048ff))
                                )
                        )
                )
        }
    }

    fn render_playlist_row(
        &self,
        index: usize,
        title: &'static str,
        subtitle: &'static str,
        cover_path: &'static str,
        has_sparklist_icon: bool,
        _cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(("playlist-item", index))
            .w(px(176.0))
            .h(px(41.0))
            .m(px(1.0))
            .rounded(px(4.0))
            .bg(rgba(0xe3e3e380))
            .relative()
            .cursor_pointer()
            // 32x32 Artwork
            .child(
                div()
                    .absolute()
                    .left(px(4.0))
                    .top(px(5.0))
                    .size(px(32.0))
                    .rounded(px(2.0))
                    .overflow_hidden()
                    .bg(rgb(0xd6defd))
                    .child(
                        img(cover_path)
                            .size_full()
                            .object_fit(ObjectFit::Cover)
                    )
            )
            // Title
            .child(
                div()
                    .absolute()
                    .left(px(38.0))
                    .top(px(5.0))
                    .w(px(99.0))
                    .h(px(12.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x000000))
                    .child(title)
            )
            // Blue underline under title
            .child(
                div()
                    .absolute()
                    .left(px(39.0))
                    .top(px(21.0))
                    .w(px(99.0))
                    .h(px(2.0))
                    .bg(rgb(0x0041ff))
            )
            // Subtitle
            .child(
                div()
                    .absolute()
                    .left(px(39.0))
                    .top(px(26.0))
                    .w(px(99.0))
                    .h(px(10.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_size(px(8.0))
                    .text_color(rgb(0x404040))
                    .child(subtitle)
            )
            // Sparklist icon if any
            .when(has_sparklist_icon, |row| {
                row.child(
                    div()
                        .absolute()
                        .left(px(97.0))
                        .top(px(4.0))
                        .size(px(16.0))
                        .child(svg().path("icons/ui/Sparklist.svg").size_full())
                )
            })
            // ShufflePlay button on right
            .child(
                div()
                    .absolute()
                    .left(px(140.0))
                    .top(px(5.0))
                    .size(px(32.0))
                    .child(svg().path("icons/ui/ShufflePlay.svg").size_full())
            )
    }

    fn render_browser_header(&self) -> Div {
        if self.devices_mode {
            // Devices Mode Header: w: 716, h: 32, bg #D6EDFD
            div()
                .size_full()
                .relative()
                .bg(rgb(0xd6edfd))
                .child(
                    div()
                        .absolute()
                        .left(px(8.0))
                        .top(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("Henry Music 01")
                        )
                        .child(
                            svg()
                                .path("icons/ui/tinyMinimize.svg")
                                .size(px(16.0))
                        )
                )
                .child(
                    div()
                        .absolute()
                        .right(px(8.0))
                        .top(px(4.0))
                        .flex()
                        .gap(px(8.0))
                        .child(
                            div()
                                .px(px(12.0))
                                .h(px(24.0))
                                .rounded(px(4.0))
                                .bg(rgb(0x0034ff))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
                                .child("Option")
                        )
                        .child(
                            div()
                                .px(px(12.0))
                                .h(px(24.0))
                                .rounded(px(4.0))
                                .bg(rgb(0x0034ff))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
                                .child("Option")
                        )
                        .child(
                            div()
                                .px(px(12.0))
                                .h(px(24.0))
                                .rounded(px(4.0))
                                .bg(rgb(0x0034ff))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
                                .child("Option")
                        )
                )
        } else {
            // Library Mode Header: w: 518, h: 32, bg #D6EDFD
            div()
                .size_full()
                .relative()
                .bg(rgb(0xd6edfd))
                .child(
                    div()
                        .absolute()
                        .left(px(8.0))
                        .top(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("Recently Added")
                        )
                        .child(
                            svg()
                                .path("icons/ui/tinyMinimize.svg")
                                .size(px(16.0))
                        )
                )
                .child(
                    // Search box: x: 262, y: 0, w: 256, h: 32, bg #869FFF
                    div()
                        .absolute()
                        .left(px(262.0))
                        .top_0()
                        .w(px(256.0))
                        .h(px(32.0))
                        .bg(rgb(0x869fff))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0x000000))
                                .child("Search!")
                        )
                )
        }
    }

    fn render_sparklist_modal(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("sparklist-modal-backdrop")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000066))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(256.0))
                    .h(px(420.0))
                    .rounded(px(16.0))
                    .border_2()
                    .border_color(rgb(0x0041ff))
                    .bg(rgb(0xd6edfd))
                    .shadow_lg()
                    .p(px(16.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .size(px(48.0))
                            .child(svg().path("icons/ui/Sparklist.svg").size_full())
                    )
                    .child(
                        div()
                            .text_center()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x000000))
                            .child("Hello!\nChoose a song\nyou like the\nvibe of!")
                    )
                    .child(
                        div()
                            .w(px(225.0))
                            .h(px(139.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(rgb(0x0041ff))
                            .bg(rgb(0xffffff))
                            .p(px(8.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(0x606060))
                                    .child("Select a seed track…")
                            )
                    )
                    .child(
                        div()
                            .id("btn-cancel-sparklist")
                            .px(px(24.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(rgb(0x0041ff))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sparklist_modal_open = false;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("Cancel")
                            )
                    )
            )
    }
    fn render_scoped_table(&self, cx: &mut Context<Self>) -> Div {
        if self.section() == DesktopSection::Queue {
            let confirmation = self.render_confirmation(cx);
            return div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(38.0))
                        .flex_none()
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .bg(rgb(ui_color(CHROME_TOP, DARK_CHROME_TOP)))
                        .border_b_1()
                        .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                        .child(
                            div()
                                .mr(px(5.0))
                                .text_size(px(10.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(self.theme.secondary_text)
                                .child(format!(
                                    "{} UP NEXT",
                                    self.playback
                                        .queue_index
                                        .map(|index| self
                                            .playback
                                            .queue
                                            .len()
                                            .saturating_sub(index + 1))
                                        .unwrap_or(self.playback.queue.len())
                                )),
                        )
                        .child(Self::chrome_button("queue-play", "Play selected").on_click(
                            cx.listener(|this, _, _, cx| this.play_selected_queue_item(cx)),
                        ))
                        .child(Self::chrome_button("queue-up", "Move up").on_click(
                            cx.listener(|this, _, _, cx| this.move_selected_queue_item(-1, cx)),
                        ))
                        .child(Self::chrome_button("queue-down", "Move down").on_click(
                            cx.listener(|this, _, _, cx| this.move_selected_queue_item(1, cx)),
                        ))
                        .child(Self::chrome_button("queue-remove", "Remove").on_click(
                            cx.listener(|this, _, _, cx| this.remove_selected_queue_item(cx)),
                        ))
                        .child(
                            Self::chrome_button("queue-save", "Save as Playlist").on_click(
                                cx.listener(|this, _, _, cx| this.save_queue_as_playlist(cx)),
                            ),
                        )
                        .child(
                            Self::chrome_button("queue-clear-upcoming", "Clear upcoming")
                                .on_click(cx.listener(|this, _, _, cx| this.clear_upcoming(cx))),
                        )
                        .child(
                            Self::chrome_button("queue-clear", "Stop & clear").on_click(
                                cx.listener(|this, _, _, cx| this.request_clear_queue(cx)),
                            ),
                        ),
                )
                .when_some(confirmation, |queue, confirmation| {
                    queue.child(confirmation)
                })
                .child(self.render_track_table(cx));
        }
        match self.route.library().map(|route| &route.scope) {
            Some(LibraryScope::Albums) => self.render_album_list(cx),
            Some(LibraryScope::Artists) => self.render_artist_list(cx),
            Some(LibraryScope::Genres) => self.render_genre_list(cx),
            _ => self.render_track_table(cx),
        }
    }

    fn render_detail_view(&self, cx: &mut Context<Self>) -> Div {
        let Some(drilldown) = self
            .route
            .library()
            .and_then(|route| route.drilldown.clone())
        else {
            return self.render_track_table(cx);
        };
        let index = &self.library_index;
        let (title, subtitle, artwork, track_indices) =
            match &drilldown {
                LibraryDrilldown::Album(id) => index
                    .albums
                    .iter()
                    .find(|album| &album.id == id)
                    .map(|album| {
                        let artwork = index.track_by_id.get(&album.representative_track).and_then(
                            |track_index| index.tracks[*track_index].cover_art_path.clone(),
                        );
                        (
                            album.title.clone(),
                            album.artist.clone(),
                            artwork,
                            album.track_indices.clone(),
                        )
                    })
                    .unwrap_or_default(),
                LibraryDrilldown::Artist(id) => index
                    .artists
                    .iter()
                    .find(|artist| &artist.id == id)
                    .map(|artist| {
                        let artwork = self.artist_portraits.get(id).cloned().or_else(|| {
                            artist.track_indices.first().and_then(|track_index| {
                                index.tracks[*track_index].cover_art_path.clone()
                            })
                        });
                        (
                            artist.name.clone(),
                            format!("{} albums", artist.album_ids.len()),
                            artwork,
                            artist.track_indices.clone(),
                        )
                    })
                    .unwrap_or_default(),
                LibraryDrilldown::Genre(name) => index
                    .genres
                    .iter()
                    .find(|genre| &genre.name == name)
                    .map(|genre| {
                        let artwork = genre.track_indices.first().and_then(|track_index| {
                            index.tracks[*track_index].cover_art_path.clone()
                        });
                        (
                            genre.name.clone(),
                            format!("{} albums", genre.album_ids.len()),
                            artwork,
                            genre.track_indices.clone(),
                        )
                    })
                    .unwrap_or_default(),
            };
        let kind_label = match &drilldown {
            LibraryDrilldown::Album(_) => "ALBUM",
            LibraryDrilldown::Artist(_) => "ARTIST",
            LibraryDrilldown::Genre(_) => "GENRE",
        };
        let artwork_is_portrait = matches!(&drilldown, LibraryDrilldown::Artist(_));
        let artwork_radius = if artwork_is_portrait { 60.0 } else { 8.0 };
        let loading_title = title.clone();
        let loading_subtitle = subtitle.clone();
        let failed_title = title.clone();
        let failed_subtitle = subtitle.clone();
        let missing_title = title.clone();
        let missing_subtitle = subtitle.clone();
        let track_count = track_indices.len();
        let duration_seconds: u64 = track_indices
            .iter()
            .map(|track_index| index.tracks[*track_index].duration.max(0) as u64)
            .sum();
        let track_ids = track_indices
            .iter()
            .map(|track_index| index.tracks[*track_index].id)
            .collect::<Vec<_>>();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(rgb(ui_color(CONTENT, DARK_CONTENT)))
            .child(
                div()
                    .h(px(46.0))
                    .flex_none()
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                    .bg(rgb(ui_color(CHROME_TOP, DARK_CHROME_TOP)))
                    .child(
                        Self::chrome_button("detail-back", "‹ Back")
                            .on_click(cx.listener(|this, _, _, cx| this.close_drilldown(cx))),
                    )
                    .child(Self::chrome_button("detail-play", "Play").on_click(
                        cx.listener(move |this, _, _, cx| this.play_tracks(track_ids.clone(), cx)),
                    )),
            )
            .child(
                div()
                    .flex_none()
                    .px(px(20.0))
                    .py(px(18.0))
                    .flex()
                    .items_center()
                    .gap(px(18.0))
                    .border_b_1()
                    .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                    .child(
                        div()
                            .size(px(120.0))
                            .flex_none()
                            .rounded(px(artwork_radius))
                            .overflow_hidden()
                            .when_some(artwork.clone(), |cell, path| {
                                cell.child(
                                    img(path)
                                        .size(px(120.0))
                                        .rounded(px(artwork_radius))
                                        .object_fit(ObjectFit::Cover)
                                        .with_loading(move || {
                                            if artwork_is_portrait {
                                                artist_portrait_placeholder(
                                                    120.0,
                                                    &loading_title,
                                                    &loading_subtitle,
                                                )
                                                .into_any_element()
                                            } else {
                                                artwork_placeholder_sized(120.0).into_any_element()
                                            }
                                        })
                                        .with_fallback(move || {
                                            if artwork_is_portrait {
                                                artist_portrait_placeholder(
                                                    120.0,
                                                    &failed_title,
                                                    &failed_subtitle,
                                                )
                                                .into_any_element()
                                            } else {
                                                artwork_placeholder_sized(120.0).into_any_element()
                                            }
                                        }),
                                )
                            })
                            .when(artwork.is_none(), |cell| {
                                if artwork_is_portrait {
                                    cell.child(artist_portrait_placeholder(
                                        120.0,
                                        &missing_title,
                                        &missing_subtitle,
                                    ))
                                } else {
                                    cell.child(artwork_placeholder_sized(120.0))
                                }
                            }),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(kind_label),
                            )
                            .child(
                                div()
                                    .text_size(px(22.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(subtitle),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(format!(
                                        "{track_count} tracks · {}",
                                        format_duration(duration_seconds as i32)
                                    )),
                            ),
                    ),
            )
            .child(self.render_track_table(cx))
    }

    fn render_album_list(&self, cx: &mut Context<Self>) -> Div {
        let albums = Arc::new(self.library_index.albums.clone());
        let selected_album = self.selected_album.clone();
        let row_count = albums.len();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(self.theme.surface)
            .child(
                div()
                    .h(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .bg(self.theme.chrome)
                    .border_b_1()
                    .border_color(self.theme.separator)
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.theme.secondary_text)
                    .child(column("", 52.0))
                    .child(column("Album", 300.0))
                    .child(column("Artist", 250.0))
                    .child(column("Year", 70.0))
                    .child(column("Tracks", 76.0))
                    .child(column("Time", 90.0)),
            )
            .child(
                uniform_list(
                    "album-list-rows",
                    row_count,
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        range
                            .filter_map(|index| {
                                let album = albums.get(index)?.clone();
                                let album_id = album.id.clone();
                                let is_selected = selected_album.as_ref() == Some(&album_id);
                                let artwork = this
                                    .library_index
                                    .track_by_id
                                    .get(&album.representative_track)
                                    .and_then(|track_index| {
                                        this.library_index.tracks[*track_index]
                                            .cover_art_path
                                            .clone()
                                    });
                                Some(
                                    div()
                                        .id(("album-list-row", index))
                                        .h(px(48.0))
                                        .flex()
                                        .items_center()
                                        .bg(if is_selected {
                                            this.theme.selected
                                        } else if index % 2 == 0 {
                                            this.theme.surface
                                        } else {
                                            this.theme.row_alternate
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff).into()
                                        } else {
                                            this.theme.text
                                        })
                                        .cursor_pointer()
                                        .when(!is_selected, |row| {
                                            row.hover(|style| style.bg(this.theme.hover))
                                        })
                                        .on_click(cx.listener(
                                            move |this, event: &ClickEvent, _, cx| {
                                                this.selected_album = Some(album_id.clone());
                                                if event.click_count() >= 2 {
                                                    this.open_album(album_id.clone(), cx);
                                                }
                                                cx.notify();
                                            },
                                        ))
                                        .child(
                                            div()
                                                .w(px(52.0))
                                                .h_full()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .when_some(artwork.clone(), |cell, path| {
                                                    cell.child(
                                                        img(path)
                                                            .size(px(36.0))
                                                            .rounded(px(3.0))
                                                            .object_fit(ObjectFit::Cover)
                                                            .with_loading(|| {
                                                                artwork_placeholder_sized(36.0)
                                                                    .into_any_element()
                                                            })
                                                            .with_fallback(|| {
                                                                artwork_placeholder_sized(36.0)
                                                                    .into_any_element()
                                                            }),
                                                    )
                                                })
                                                .when(artwork.is_none(), |cell| {
                                                    cell.child(artwork_placeholder_sized(36.0))
                                                }),
                                        )
                                        .child(collection_column(
                                            &album.title,
                                            300.0,
                                            FontWeight::SEMIBOLD,
                                        ))
                                        .child(collection_column(
                                            &album.artist,
                                            250.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &album
                                                .year
                                                .map(|year| year.to_string())
                                                .unwrap_or_else(|| "—".to_string()),
                                            70.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &album.track_indices.len().to_string(),
                                            76.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &format_duration(album.duration_seconds as i32),
                                            90.0,
                                            FontWeight::NORMAL,
                                        )),
                                )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
    }

    fn render_artist_list(&self, cx: &mut Context<Self>) -> Div {
        let artists = Arc::new(self.library_index.artists.clone());
        let row_count = artists.len();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(self.theme.surface)
            .child(collection_list_header("Artist", "Albums", "Tracks", "Time"))
            .child(
                uniform_list(
                    "artist-list-rows",
                    row_count,
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        range
                            .filter_map(|index| {
                                let artist = artists.get(index)?.clone();
                                let artist_id = artist.id.clone();
                                let portrait = this.artist_portraits.get(&artist.id).cloned();
                                let portrait_name = artist.name.clone();
                                let portrait_detail = format!(
                                    "{} albums · {} tracks",
                                    artist.album_ids.len(),
                                    artist.track_indices.len()
                                );
                                Some(
                                    div()
                                        .id(("artist-list-row", index))
                                        .h(px(42.0))
                                        .flex()
                                        .items_center()
                                        .bg(if index % 2 == 0 {
                                            this.theme.surface
                                        } else {
                                            this.theme.row_alternate
                                        })
                                        .text_color(this.theme.text)
                                        .cursor_pointer()
                                        .hover(|style| style.bg(this.theme.hover))
                                        .on_click(cx.listener(
                                            move |this, event: &ClickEvent, _, cx| {
                                                if event.click_count() >= 2 {
                                                    this.open_artist(artist_id.clone(), cx);
                                                }
                                            },
                                        ))
                                        .child(
                                            div()
                                                .w(px(420.0))
                                                .flex_none()
                                                .h_full()
                                                .px(px(10.0))
                                                .flex()
                                                .items_center()
                                                .gap(px(9.0))
                                                .child(
                                                    div()
                                                        .size(px(30.0))
                                                        .flex_none()
                                                        .rounded(px(15.0))
                                                        .overflow_hidden()
                                                        .bg(this.theme.row_alternate)
                                                        .when_some(
                                                            portrait.clone(),
                                                            |cell, path| {
                                                                let loading_name =
                                                                    portrait_name.clone();
                                                                let loading_detail =
                                                                    portrait_detail.clone();
                                                                let failed_name =
                                                                    portrait_name.clone();
                                                                let failed_detail =
                                                                    portrait_detail.clone();
                                                                cell.child(
                                                                img(path)
                                                                    .size_full()
                                                                    .rounded(px(15.0))
                                                                    .object_fit(ObjectFit::Cover)
                                                                    .with_loading(move || {
                                                                        artist_portrait_placeholder(
                                                                            30.0,
                                                                            &loading_name,
                                                                            &loading_detail,
                                                                        )
                                                                        .into_any_element()
                                                                    })
                                                                    .with_fallback(move || {
                                                                        artist_portrait_placeholder(
                                                                            30.0,
                                                                            &failed_name,
                                                                            &failed_detail,
                                                                        )
                                                                        .into_any_element()
                                                                    }),
                                                            )
                                                            },
                                                        )
                                                        .when(portrait.is_none(), |cell| {
                                                            cell.child(artist_portrait_placeholder(
                                                                30.0,
                                                                &portrait_name,
                                                                &portrait_detail,
                                                            ))
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .min_w(px(0.0))
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .whitespace_nowrap()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .child(artist.name.clone()),
                                                ),
                                        )
                                        .child(collection_column(
                                            &artist.album_ids.len().to_string(),
                                            100.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &artist.track_indices.len().to_string(),
                                            100.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &format_duration(artist.duration_seconds as i32),
                                            110.0,
                                            FontWeight::NORMAL,
                                        )),
                                )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
    }

    fn render_genre_list(&self, cx: &mut Context<Self>) -> Div {
        let genres = Arc::new(self.library_index.genres.clone());
        let row_count = genres.len();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(self.theme.surface)
            .child(collection_list_header("Genre", "Albums", "Tracks", "Time"))
            .child(
                uniform_list(
                    "genre-list-rows",
                    row_count,
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        range
                            .filter_map(|index| {
                                let genre = genres.get(index)?.clone();
                                let genre_name = genre.name.clone();
                                Some(
                                    div()
                                        .id(("genre-list-row", index))
                                        .h(px(42.0))
                                        .flex()
                                        .items_center()
                                        .bg(if index % 2 == 0 {
                                            this.theme.surface
                                        } else {
                                            this.theme.row_alternate
                                        })
                                        .text_color(this.theme.text)
                                        .cursor_pointer()
                                        .hover(|style| style.bg(this.theme.hover))
                                        .on_click(cx.listener(
                                            move |this, event: &ClickEvent, _, cx| {
                                                if event.click_count() >= 2 {
                                                    this.open_genre(genre_name.clone(), cx);
                                                }
                                            },
                                        ))
                                        .child(collection_column(
                                            &genre.name,
                                            420.0,
                                            FontWeight::SEMIBOLD,
                                        ))
                                        .child(collection_column(
                                            &genre.album_ids.len().to_string(),
                                            100.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &genre.track_indices.len().to_string(),
                                            100.0,
                                            FontWeight::NORMAL,
                                        ))
                                        .child(collection_column(
                                            &format_duration(genre.duration_seconds as i32),
                                            110.0,
                                            FontWeight::NORMAL,
                                        )),
                                )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
    }

    fn render_track_table(&self, cx: &mut Context<Self>) -> Div {
        let indices = Arc::new(self.visible_track_indices());
        let row_count = indices.len();
        let is_queue = self.section() == DesktopSection::Queue;
        let is_playlist =
            self.section() == DesktopSection::Playlists && self.active_playlist.is_some();
        if row_count == 0 {
            let (title, message) = if is_queue {
                (
                    "Your queue is empty",
                    "Play an album, playlist, or song to build Up Next.",
                )
            } else if is_playlist {
                (
                    "This playlist is empty",
                    "Add songs from their options menu, then arrange them here.",
                )
            } else if self.snapshot.tracks.is_empty() && self.search.trim().is_empty() {
                (
                    "Your library is empty",
                    "Import music to start building your collection.",
                )
            } else if !self.search.trim().is_empty() {
                ("No matches", "Nothing in your library matches this search.")
            } else {
                (
                    "No songs found",
                    "Try a different search or library source.",
                )
            };
            return div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(7.0))
                .bg(self.theme.surface)
                .text_color(self.theme.secondary_text)
                .child(
                    svg()
                        .path(if is_queue {
                            "icons/desktop/queue.svg"
                        } else {
                            "icons/desktop/playlist.svg"
                        })
                        .size(px(28.0))
                        .text_color(self.theme.secondary_text),
                )
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(self.theme.text)
                        .child(title),
                )
                .child(div().text_size(px(11.5)).child(message))
                .child({
                    let library_empty = !is_queue
                        && !is_playlist
                        && self.snapshot.tracks.is_empty()
                        && self.search.trim().is_empty();
                    let search_active = !self.search.trim().is_empty();
                    if library_empty {
                        Self::chrome_button("empty-tracks-import", "Import music…")
                            .on_click(cx.listener(|this, _, _, cx| this.choose_import(cx)))
                    } else if search_active && !is_queue && !is_playlist {
                        Self::chrome_button("empty-tracks-clear", "Clear search").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.search_input.update(cx, |input, cx| input.clear(cx));
                            }),
                        )
                    } else {
                        Self::chrome_button("empty-tracks-browse", "Browse songs").on_click(
                            cx.listener(|this, _, _, cx| this.navigate(DesktopSection::Songs, cx)),
                        )
                    }
                });
        }

        let selected = self.selected_track_id;
        let selected_row = self.selected_track;
        let queue_index = self.playback.queue_index;
        let queue_playing = self.playback.is_playing;
        let now_playing_id = self.playback.track.as_ref().map(|track| track.id);
        let row_height = self.config.appearance.row_height.clamp(26, 38) as f32;
        let header = div()
            .h(px(30.0))
            .flex_none()
            .flex()
            .items_center()
            .bg(rgb(ui_color(CHROME_BOTTOM, DARK_CHROME_BOTTOM)))
            .border_b_1()
            .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
            .when(is_queue, |header| {
                header
                    .child(column("Status", 76.0))
                    .child(column("Up Next", 300.0))
                    .child(column("Artist", 220.0))
                    .child(column("Album", 260.0))
                    .child(column("Genre", 150.0))
                    .child(column("Time", 70.0))
            })
            .when(is_playlist, |header| {
                header
                    .child(column("#", 38.0))
                    .child(column("Playlist order", 300.0))
                    .child(column("Artist", 220.0))
                    .child(column("Album", 260.0))
                    .child(column("Genre", 150.0))
                    .child(column("Time", 70.0))
            })
            .when(!is_queue && !is_playlist, |header| {
                header
                    .child(self.sortable_column(cx, "#", 38.0, SortField::TrackNumber))
                    .child(self.sortable_column(cx, "Name", 300.0, SortField::Title))
                    .child(self.sortable_column(cx, "Artist", 220.0, SortField::Artist))
                    .child(self.sortable_column(cx, "Album", 260.0, SortField::Album))
                    .child(self.sortable_column(cx, "Genre", 150.0, SortField::Genre))
                    .child(self.sortable_column(cx, "Time", 70.0, SortField::Duration))
            });

        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(self.theme.surface)
            .child(header)
            .child(
                uniform_list(
                    if is_queue { "queue-rows" } else { "track-rows" },
                    row_count,
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        range
                            .filter_map(|index| {
                                let queue_or_track_index = indices[index];
                                let track = this.track_collection().get(queue_or_track_index)?;
                                let track_id = track.id;
                                let is_now_playing = now_playing_id == Some(track_id);
                                let is_selected = if is_queue {
                                    selected_row == Some(index)
                                } else {
                                    selected == Some(track_id)
                                };
                                let queue_status = if is_queue {
                                    match queue_index {
                                        Some(current) if current == queue_or_track_index => {
                                            if queue_playing {
                                                "Playing"
                                            } else {
                                                "Paused"
                                            }
                                        }
                                        Some(current) if queue_or_track_index > current => {
                                            "Up next"
                                        }
                                        Some(_) => "Played",
                                        None => "Queued",
                                    }
                                } else {
                                    ""
                                };
                                Some(
                                    div()
                                        .id((
                                            if is_queue { "queue-row" } else { "track-row" },
                                            index,
                                        ))
                                        .h(px(row_height))
                                        .flex()
                                        .items_center()
                                        .bg(if is_selected {
                                            rgb(ui_color(BLUE, DARK_BLUE))
                                        } else if index % 2 == 0 {
                                            rgb(ui_color(CONTENT, DARK_CONTENT))
                                        } else {
                                            rgb(ui_color(ROW_ALT, DARK_ROW_ALT))
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(ui_color(TEXT, DARK_TEXT))
                                        })
                                        .text_size(px(12.5))
                                        .cursor_pointer()
                                        .when(!is_selected, |row| {
                                            row.hover(|style| {
                                                style.bg(rgb(ui_color(0xe9e9ee, 0x2c2c2e)))
                                            })
                                        })
                                        .on_click(cx.listener(
                                            move |this, event: &ClickEvent, _, cx| {
                                                this.selected_track = Some(index);
                                                this.selected_track_id = Some(track_id);
                                                if event.click_count() >= 2 {
                                                    if this.section() == DesktopSection::Queue {
                                                        this.send_playback(
                                                            LocalPlaybackCommand::SelectQueueIndex(
                                                                queue_or_track_index,
                                                            ),
                                                            cx,
                                                        );
                                                    } else {
                                                        this.play_visible_track(track_id, cx);
                                                    }
                                                }
                                                cx.notify();
                                            },
                                        ))
                                        .on_mouse_down(
                                            MouseButton::Right,
                                            cx.listener(
                                                move |this, event: &gpui::MouseDownEvent, _, cx| {
                                                    this.selected_track = Some(index);
                                                    this.selected_track_id = Some(track_id);
                                                    this.open_context_menu(
                                                        track_id,
                                                        event.position,
                                                        cx,
                                                    );
                                                },
                                            ),
                                        )
                                        .when(is_queue, |row| row.child(column(queue_status, 76.0)))
                                        .when(!is_queue, |row| {
                                            if is_now_playing {
                                                row.child(
                                                    div()
                                                        .w(px(38.0))
                                                        .min_w(px(38.0))
                                                        .h_full()
                                                        .px(px(7.0))
                                                        .flex()
                                                        .items_center()
                                                        .overflow_hidden()
                                                        .border_r_1()
                                                        .border_color(rgb(0xc6cad0))
                                                        .child(
                                                            svg()
                                                                .path(if queue_playing {
                                                                    "icons/desktop/pause.svg"
                                                                } else {
                                                                    "icons/desktop/play.svg"
                                                                })
                                                                .size(px(11.0))
                                                                .text_color(if is_selected {
                                                                    rgb(0xffffff)
                                                                } else {
                                                                    rgb(ui_color(BLUE, DARK_BLUE))
                                                                }),
                                                        ),
                                                )
                                            } else {
                                                row.child(column(&(index + 1).to_string(), 38.0))
                                            }
                                        })
                                        .child(
                                            div()
                                                .w(px(300.0))
                                                .min_w(px(300.0))
                                                .h_full()
                                                .px(px(7.0))
                                                .flex()
                                                .items_center()
                                                .overflow_hidden()
                                                .border_r_1()
                                                .border_color(rgb(0xc6cad0))
                                                .when(is_now_playing && !is_selected, |cell| {
                                                    cell.text_color(rgb(ui_color(BLUE, DARK_BLUE)))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                })
                                                .child(track.title.clone()),
                                        )
                                        .child(column(&track.artist, 220.0))
                                        .child(column(&track.album, 260.0))
                                        .child(column(
                                            if track.genre.is_empty() {
                                                "—"
                                            } else {
                                                &track.genre
                                            },
                                            150.0,
                                        ))
                                        .child(column(&format_duration(track.duration), 70.0)),
                                )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
    }
    fn render_playlists_page(&self, cx: &mut Context<Self>) -> Div {
        if let Some(playlist) = &self.active_playlist {
            let renaming = self.playlist_renaming;
            let confirmation = self.render_confirmation(cx);
            let ids = self
                .playlist_tracks
                .iter()
                .map(|track| track.id)
                .collect::<Vec<_>>();
            return div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .bg(rgb(ui_color(CONTENT, DARK_CONTENT)))
                .child(
                    div()
                        .h(px(54.0))
                        .flex_none()
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                        .bg(rgb(ui_color(CHROME_TOP, DARK_CHROME_TOP)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(9.0))
                                .child(
                                    Self::chrome_button("playlist-back", "‹ Playlists").on_click(
                                        cx.listener(|this, _, _, cx| this.close_playlist(cx)),
                                    ),
                                )
                                .when(!renaming, |header| {
                                    header.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(15.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
                                                    .child(playlist.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .text_color(self.theme.secondary_text)
                                                    .child(format!(
                                                        "{} tracks · {}",
                                                        self.playlist_tracks.len(),
                                                        total_duration(&self.playlist_tracks)
                                                    )),
                                            ),
                                    )
                                })
                                .when(renaming, |header| {
                                    header.child(
                                        div()
                                            .w(px(220.0))
                                            .h(px(29.0))
                                            .px(px(8.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(6.0))
                                            .border_1()
                                            .border_color(self.theme.focus)
                                            .bg(self.theme.surface)
                                            .child(self.playlist_name_input.clone()),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(Self::chrome_button("playlist-play", "Play").on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.play_tracks(ids.clone(), cx)
                                    }),
                                ))
                                .child(Self::chrome_button("playlist-move-up", "Up").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.move_selected_playlist_item(-1, cx)
                                    }),
                                ))
                                .child(Self::chrome_button("playlist-move-down", "Down").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.move_selected_playlist_item(1, cx)
                                    }),
                                ))
                                .child(
                                    Self::chrome_button("playlist-remove-track", "Remove")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.remove_selected_from_playlist(cx)
                                        })),
                                )
                                .when(!renaming, |actions| {
                                    actions.child(
                                        Self::chrome_button("playlist-rename", "Rename").on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.begin_playlist_rename(window, cx)
                                            }),
                                        ),
                                    )
                                })
                                .when(renaming, |actions| {
                                    actions
                                        .child(
                                            Self::chrome_button("playlist-rename-save", "Save")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.commit_playlist_rename(cx)
                                                })),
                                        )
                                        .child(
                                            Self::chrome_button("playlist-rename-cancel", "Cancel")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.cancel_playlist_rename(cx)
                                                })),
                                        )
                                })
                                .child(
                                    Self::chrome_button("playlist-export-active", "Export")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.export_active_playlist(cx)
                                        })),
                                )
                                .child(Self::chrome_button("playlist-delete", "Delete").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.request_delete_active_playlist(cx)
                                    }),
                                )),
                        ),
                )
                .when_some(confirmation, |page, confirmation| page.child(confirmation))
                .child(self.render_track_table(cx));
        }

        let mut page = div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(rgb(ui_color(CONTENT, DARK_CONTENT)))
            .child(
                div()
                    .h(px(52.0))
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Playlists"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("playlist-create", "New Playlist").on_click(
                                    cx.listener(|this, _, _, cx| this.create_playlist(cx)),
                                ),
                            )
                            .child(
                                Self::chrome_button("playlist-import", "Import M3U").on_click(
                                    cx.listener(|this, _, _, cx| this.import_playlist_file(cx)),
                                ),
                            ),
                    ),
            );
        if self.snapshot.playlists.is_empty() {
            return page.child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("No playlists yet"),
                    )
                    .child(div().text_size(px(12.0)).child(
                        "Create one here. Your local library does not need Device Preview.",
                    )),
            );
        }
        for playlist in &self.snapshot.playlists {
            let playlist = playlist.clone();
            let title = playlist.name.clone();
            let track_count = playlist.track_count;
            let add_id = playlist.id;
            page = page.child(
                div()
                    .id(("playlist-row", playlist.id as usize))
                    .h(px(42.0))
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(ui_color(0xe9e9ee, 0x2c2c2e))))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.open_playlist(playlist.clone(), cx)),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_size(px(12.0))
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(format!("{track_count} tracks")),
                            )
                            .child(
                                Self::chrome_button(
                                    ("playlist-add-track", add_id as usize),
                                    "Add selected",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.add_selected_to_playlist(add_id, cx)
                                    },
                                )),
                            ),
                    ),
            );
        }
        let smart_defs = musicos_desktop::local::smart_playlists::default_smart_playlists();
        page = page.child(
            div()
                .h(px(34.0))
                .px(px(18.0))
                .flex()
                .items_center()
                .border_b_1()
                .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                .bg(rgb(ui_color(CHROME_BOTTOM, DARK_CHROME_BOTTOM)))
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                .child("SMART PLAYLISTS"),
        );
        for def in &smart_defs {
            let count = def.evaluate(&self.snapshot.tracks).len();
            let name = def.name.clone();
            page = page.child(
                div()
                    .h(px(38.0))
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                            .child(format!("{count} tracks")),
                    ),
            );
        }
        page
    }

    fn render_cover_flow(&self, cx: &mut Context<Self>) -> Div {
        let albums = &self.library_index.albums;
        if albums.is_empty() {
            return div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .bg(self.theme.window)
                .text_color(self.theme.secondary_text)
                .child(artwork_placeholder_sized(120.0))
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("No albums match this view"),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .child("Clear the search or import music to use Flow."),
                );
        }
        let focus = self.flow_focus.min(albums.len().saturating_sub(1));
        let start = focus.saturating_sub(4);
        let end = (focus + 5).min(albums.len());
        let focused = albums[focus].clone();
        let focused_ids = focused
            .track_indices
            .iter()
            .map(|index| self.library_index.tracks[*index].id)
            .collect::<Vec<_>>();
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .bg(self.theme.window)
            .text_color(self.theme.text)
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = event.delta.pixel_delta(px(16.0));
                let delta_x = f32::from(delta.x);
                let delta_y = f32::from(delta.y);
                if delta_x.abs() >= delta_y.abs() && delta_x.abs() > 2.0 {
                    this.move_flow(if delta_x > 0.0 { 1 } else { -1 }, cx);
                }
            }))
            .child(
                div()
                    .w_full()
                    .h(px(390.0))
                    .flex()
                    .items_end()
                    .justify_center()
                    .overflow_hidden()
                    .children((start..end).map(|index| {
                        let album = albums[index].clone();
                        let distance = index.abs_diff(focus);
                        let focused_cover = distance == 0;
                        let (width, height, opacity, overlap) = match distance {
                            0 => (280.0, 280.0, 1.0, 0.0),
                            1 => (122.0, 204.0, 0.92, -12.0),
                            2 => (92.0, 170.0, 0.74, -10.0),
                            3 => (70.0, 142.0, 0.56, -8.0),
                            _ => (54.0, 116.0, 0.38, -6.0),
                        };
                        let artwork = self
                            .library_index
                            .track_by_id
                            .get(&album.representative_track)
                            .and_then(|track_index| {
                                self.library_index.tracks[*track_index]
                                    .cover_art_path
                                    .clone()
                            });
                        div()
                            .id(("flow-cover", index))
                            .ml(px(overlap))
                            .w(px(width))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .items_center()
                            .opacity(opacity)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                if event.click_count() >= 2 {
                                    this.open_album(album.id.clone(), cx);
                                } else {
                                    this.flow_focus = index;
                                    this.selected_album = Some(album.id.clone());
                                    cx.notify();
                                }
                            }))
                            .child(
                                div()
                                    .w(px(width))
                                    .h(px(height))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .overflow_hidden()
                                    .rounded(px(if focused_cover { 7.0 } else { 3.0 }))
                                    .border_1()
                                    .border_color(if focused_cover {
                                        self.theme.focus
                                    } else {
                                        self.theme.separator
                                    })
                                    .bg(self.theme.row_alternate)
                                    .shadow_lg()
                                    .when_some(artwork.clone(), |cover, path| {
                                        cover.child(
                                            img(path)
                                                .size_full()
                                                .object_fit(ObjectFit::Cover)
                                                .with_loading(move || {
                                                    artwork_placeholder_sized(width.min(height))
                                                        .into_any_element()
                                                })
                                                .with_fallback(move || {
                                                    artwork_placeholder_sized(width.min(height))
                                                        .into_any_element()
                                                }),
                                        )
                                    })
                                    .when(artwork.is_none(), |cover| {
                                        cover.child(artwork_placeholder_sized(width.min(height)))
                                    }),
                            )
                            .when_some(artwork, |cover, path| {
                                cover.child(
                                    div()
                                        .mt(px(3.0))
                                        .w(px(width * 0.94))
                                        .h(px(if focused_cover { 38.0 } else { 22.0 }))
                                        .overflow_hidden()
                                        .opacity(if focused_cover { 0.18 } else { 0.09 })
                                        .child(img(path).size_full().object_fit(ObjectFit::Cover)),
                                )
                            })
                    })),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(620.0))
                    .min_h(px(96.0))
                    .px(px(18.0))
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(18.0))
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_size(px(18.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(focused.title.clone()),
                            )
                            .child(
                                div()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_size(px(12.0))
                                    .text_color(self.theme.secondary_text)
                                    .child(focused.artist.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(self.theme.secondary_text)
                                    .child(format!(
                                        "{} · {} tracks · {}",
                                        focused
                                            .year
                                            .map(|year| year.to_string())
                                            .unwrap_or_else(|| "Unknown year".to_string()),
                                        focused.track_indices.len(),
                                        format_duration(focused.duration_seconds as i32)
                                    )),
                            ),
                    )
                    .child(
                        Self::chrome_button("flow-play", "Play album").on_click(cx.listener(
                            move |this, _, _, cx| this.play_tracks(focused_ids.clone(), cx),
                        )),
                    ),
            )
    }

    fn move_flow(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.library_index.albums.is_empty() {
            return;
        }
        self.flow_focus = if delta < 0 {
            self.flow_focus.saturating_sub(delta.unsigned_abs())
        } else {
            (self.flow_focus + delta as usize)
                .min(self.library_index.albums.len().saturating_sub(1))
        };
        self.selected_album = self
            .library_index
            .albums
            .get(self.flow_focus)
            .map(|album| album.id.clone());
        cx.notify();
    }

    fn render_collection_browser(&self, _viewport_width: f32, cx: &mut Context<Self>) -> Div {
        let albums: Vec<(String, String, Option<std::path::PathBuf>, Option<musicos_desktop::AlbumId>)> = if self.library_index.albums.is_empty() {
            (0..6)
                .map(|_| {
                    (
                        "Album Name".to_string(),
                        "artist".to_string(),
                        None,
                        None,
                    )
                })
                .collect()
        } else {
            self.library_index
                .albums
                .iter()
                .take(6)
                .map(|album| {
                    let artwork = self.artwork_for_track(album.representative_track);
                    (
                        album.title.clone(),
                        album.artist.clone(),
                        artwork,
                        Some(album.id.clone()),
                    )
                })
                .collect()
        };

        // 3-column x 2-row grid matching BaseUi (x: 21, y: 21, w: 448, h: 336)
        div()
            .size_full()
            .relative()
            .bg(rgb(0x00aeff))
            .child(
                div()
                    .absolute()
                    .top(px(21.0))
                    .left(px(21.0))
                    .w(px(448.0))
                    .h(px(336.0))
                    .flex()
                    .flex_wrap()
                    .gap_x(px(42.0))
                    .gap_y(px(16.0))
                    .children(
                        albums.into_iter().enumerate().map(|(index, (title, subtitle, artwork, album_id))| {
                            div()
                                .id(("album-card", index))
                                .w(px(128.0))
                                .h(px(160.0))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                    if let Some(id) = album_id.clone() {
                                        this.selected_album = Some(id.clone());
                                        if event.click_count() >= 2 {
                                            this.open_album(id, cx);
                                        }
                                    }
                                    cx.notify();
                                }))
                                .child(
                                    // 128x128 artwork
                                    div()
                                        .w(px(128.0))
                                        .h(px(128.0))
                                        .bg(rgb(0x000000))
                                        .when_some(artwork, |card, path| {
                                            card.child(
                                                img(path)
                                                    .size_full()
                                                    .object_fit(ObjectFit::Cover)
                                            )
                                        })
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .pt(px(6.0))
                                        .text_center()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x000000))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(title)
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .text_center()
                                        .text_size(px(8.5))
                                        .text_color(rgb(0x000000))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(subtitle)
                                )
                        })
                    )
            )
            .child(
                // Right scrollbar matching Sketch Frame 2 at x: 504, y: 61, w: 8, h: 256
                div()
                    .absolute()
                    .top(px(61.0))
                    .left(px(504.0))
                    .w(px(8.0))
                    .h(px(256.0))
                    .bg(rgb(0x0034ff))
                    .border_1()
                    .border_color(rgb(0x000000))
                    .child(
                        div()
                            .m(px(1.0))
                            .w(px(6.0))
                            .h(px(32.0))
                            .bg(rgb(0xffffff))
                    )
            )
    }

    fn render_artist_browser(&self, viewport_width: f32, cx: &mut Context<Self>) -> Div {
        let artists = Arc::new(self.library_index.artists.clone());
        let metrics = album_grid_metrics(
            viewport_width,
            self.config.appearance.sidebar_width as f32,
            176.0,
        );
        let columns = metrics.columns;
        let card_width = metrics.card_width;
        let portrait_size = (card_width - 28.0).clamp(116.0, 190.0);
        let card_height = portrait_size + 62.0;
        let row_height = card_height + 12.0;
        let row_count = artists.len().div_ceil(columns);
        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .bg(self.theme.window)
            .when(self.artist_portraits_loading, |view| {
                view.child(
                    div()
                        .h(px(30.0))
                        .flex_none()
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .border_b_1()
                        .border_color(self.theme.separator)
                        .bg(self.theme.elevated_surface)
                        .text_size(px(10.5))
                        .text_color(self.theme.secondary_text)
                        .child(
                            svg()
                                .path("icons/desktop/activity.svg")
                                .size(px(13.0))
                                .text_color(self.theme.focus),
                        )
                        .child("Finding artist portraits through MusicBrainz and Wikimedia…"),
                )
            })
            .child(
                uniform_list(
                    "artist-portrait-rows",
                    row_count,
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        range
                            .map(|row| {
                                div()
                                    .h(px(row_height))
                                    .px(px(12.0))
                                    .pt(px(12.0))
                                    .flex()
                                    .items_start()
                                    .gap(px(metrics.gap))
                                    .children((0..columns).filter_map(|column| {
                                        let index = row * columns + column;
                                        let artist = artists.get(index)?.clone();
                                        let artist_id = artist.id.clone();
                                        let portrait =
                                            this.artist_portraits.get(&artist.id).cloned();
                                        let fallback_name = artist.name.clone();
                                        let fallback_detail = format!(
                                            "{} albums · {} tracks",
                                            artist.album_ids.len(),
                                            artist.track_indices.len()
                                        );
                                        Some(
                                            div()
                                                .id(("artist-card", index))
                                                .w(px(card_width))
                                                .h(px(card_height))
                                                .flex_none()
                                                .px(px(6.0))
                                                .pt(px(6.0))
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .rounded(px(10.0))
                                                .cursor_pointer()
                                                .hover(|style| style.bg(this.theme.hover))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.open_artist(artist_id.clone(), cx)
                                                }))
                                                .child(
                                                    div()
                                                        .size(px(portrait_size))
                                                        .flex_none()
                                                        .overflow_hidden()
                                                        .rounded(px(portrait_size / 2.0))
                                                        .border_1()
                                                        .border_color(this.theme.separator)
                                                        .bg(this.theme.row_alternate)
                                                        .when_some(
                                                            portrait.clone(),
                                                            |cell, path| {
                                                                let loading_name =
                                                                    fallback_name.clone();
                                                                let loading_detail =
                                                                    fallback_detail.clone();
                                                                let failed_name =
                                                                    fallback_name.clone();
                                                                let failed_detail =
                                                                    fallback_detail.clone();
                                                                cell.child(
                                                                img(path)
                                                                    .size_full()
                                                                    .rounded(px(
                                                                        portrait_size / 2.0,
                                                                    ))
                                                                    .object_fit(ObjectFit::Cover)
                                                                    .with_loading(move || {
                                                                        artist_portrait_placeholder(
                                                                            portrait_size,
                                                                            &loading_name,
                                                                            &loading_detail,
                                                                        )
                                                                        .into_any_element()
                                                                    })
                                                                    .with_fallback(move || {
                                                                        artist_portrait_placeholder(
                                                                            portrait_size,
                                                                            &failed_name,
                                                                            &failed_detail,
                                                                        )
                                                                        .into_any_element()
                                                                    }),
                                                            )
                                                            },
                                                        )
                                                        .when(portrait.is_none(), |cell| {
                                                            cell.child(artist_portrait_placeholder(
                                                                portrait_size,
                                                                &fallback_name,
                                                                &fallback_detail,
                                                            ))
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .mt(px(8.0))
                                                        .max_w_full()
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .whitespace_nowrap()
                                                        .text_size(px(12.5))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(this.theme.text)
                                                        .child(artist.name),
                                                )
                                                .child(
                                                    div()
                                                        .mt(px(2.0))
                                                        .text_size(px(9.5))
                                                        .text_color(this.theme.secondary_text)
                                                        .child(fallback_detail),
                                                ),
                                        )
                                    }))
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
    }

    fn render_diagnostics_controls(&self, cx: &mut Context<Self>) -> Div {
        div()
            .px(px(18.0))
            .pb(px(14.0))
            .flex()
            .items_center()
            .gap(px(7.0))
            .child(
                Self::chrome_button("diagnostics-export", "Export…")
                    .on_click(cx.listener(|this, _, _, cx| this.export_diagnostics(cx))),
            )
            .child(
                Self::chrome_button("cover-cache-reveal", "Reveal artwork")
                    .on_click(cx.listener(|this, _, _, cx| this.reveal_cover_cache(cx))),
            )
            .child(
                Self::chrome_button("cover-cache-clear", "Clear artwork")
                    .on_click(cx.listener(|this, _, _, cx| this.clear_cover_cache(cx))),
            )
            .child(
                Self::chrome_button("library-repair", "Repair database")
                    .on_click(cx.listener(|this, _, _, cx| this.repair_library(cx))),
            )
            .child(
                Self::chrome_button("indexed-cleanup", "Clean leftovers").on_click(
                    cx.listener(|this, _, _, cx| this.request_indexed_leftovers_cleanup(cx)),
                ),
            )
    }

    fn render_indexed_leftovers(&self, cx: &mut Context<Self>) -> Div {
        let count = self.source_audit.hidden_tracks.len();
        let mut panel = div()
            .mx(px(18.0))
            .mb(px(14.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if count == 0 {
                self.theme.separator
            } else {
                self.theme.warning
            })
            .bg(self.theme.elevated_surface)
            .overflow_hidden()
            .child(
                div()
                    .min_h(px(46.0))
                    .px(px(11.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(if count == 0 {
                                        "Library sources are clean".to_string()
                                    } else {
                                        format!("{count} indexed leftovers")
                                    }),
                            )
                            .child(
                                div()
                                    .mt(px(3.0))
                                    .text_size(px(10.0))
                                    .text_color(self.theme.secondary_text)
                                    .child(if count == 0 {
                                        "Every visible record belongs to the managed library or an explicit reference folder."
                                    } else {
                                        "These records are hidden because their paths are outside configured library roots. Media files are untouched."
                                    }),
                            ),
                    )
                    .when(count > 0, |header| {
                        header.child(
                            div()
                                .flex()
                                .gap(px(5.0))
                                .child(Self::chrome_button("leftovers-add-source", "Add Folder…").on_click(
                                    cx.listener(|this, _, _, cx| this.choose_reference_source(cx)),
                                ))
                                .child(Self::chrome_button("leftovers-remove", "Remove Records…").on_click(
                                    cx.listener(|this, _, _, cx| this.request_indexed_leftovers_cleanup(cx)),
                                )),
                        )
                    }),
            );
        for (index, root) in self.source_audit.grouped_roots.iter().take(4).enumerate() {
            panel = panel.child(
                div()
                    .id(("leftover-root", index))
                    .min_h(px(38.0))
                    .px(px(11.0))
                    .border_t_1()
                    .border_color(self.theme.separator)
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.0))
                    .child(root.root.display().to_string())
                    .child(div().text_color(self.theme.secondary_text).child(format!(
                        "{} records · {}",
                        root.count,
                        if root.mounted { "mounted" } else { "offline" }
                    ))),
            );
        }
        panel
    }

    fn render_settings_content(&self, cx: &mut Context<Self>) -> Div {
        let navigation = [
            (SettingsCategory::General, "General"),
            (SettingsCategory::Library, "Library sources"),
            (SettingsCategory::Metadata, "Metadata & artwork"),
            (SettingsCategory::Playback, "Playback"),
            (SettingsCategory::Formats, "Sync & formats"),
            (SettingsCategory::Appearance, "Appearance"),
            (SettingsCategory::Preview, "Device Preview"),
            (SettingsCategory::Advanced, "Advanced"),
        ];
        let content = match self.settings_category {
            SettingsCategory::General => self.render_general_settings(),
            SettingsCategory::Library => self.render_library_source_settings(cx),
            SettingsCategory::Metadata => self.render_metadata_settings(cx),
            SettingsCategory::Playback => self.render_playback_settings(),
            SettingsCategory::Formats => self.render_format_settings(cx),
            SettingsCategory::Appearance => self.render_appearance_settings(cx),
            SettingsCategory::Preview => self.render_preview_settings(cx),
            SettingsCategory::Advanced => self.render_advanced_settings(cx),
        };
        div()
            .min_h(px(410.0))
            .flex()
            .border_t_1()
            .border_color(self.theme.separator)
            .child(
                div()
                    .w(px(162.0))
                    .flex_none()
                    .p(px(10.0))
                    .bg(self.theme.sidebar)
                    .border_r_1()
                    .border_color(self.theme.separator)
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .children(navigation.into_iter().map(|(category, label)| {
                        let selected = self.settings_category == category;
                        div()
                            .id(("settings-category", category as usize))
                            .h(px(29.0))
                            .px(px(9.0))
                            .rounded(px(5.0))
                            .flex()
                            .items_center()
                            .text_size(px(11.0))
                            .font_weight(if selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if selected {
                                rgb(0xffffff).into()
                            } else {
                                self.theme.text
                            })
                            .when(selected, |row| row.bg(self.theme.selected))
                            .when(!selected, |row| {
                                row.hover(|style| style.bg(self.theme.hover))
                            })
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.settings_category = category;
                                cx.notify();
                            }))
                            .child(label)
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .p(px(18.0))
                    .bg(self.theme.surface)
                    .text_color(self.theme.text)
                    .child(content),
            )
    }

    fn settings_heading(&self, title: &str, detail: &str) -> Div {
        div()
            .pb(px(13.0))
            .border_b_1()
            .border_color(self.theme.separator)
            .child(
                div()
                    .text_size(px(17.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .mt(px(4.0))
                    .max_w(px(560.0))
                    .text_size(px(10.5))
                    .text_color(self.theme.secondary_text)
                    .child(detail.to_string()),
            )
    }

    fn settings_row(&self, title: &str, detail: &str) -> Div {
        div()
            .min_h(px(48.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(self.theme.separator)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .mt(px(3.0))
                            .text_size(px(10.0))
                            .text_color(self.theme.secondary_text)
                            .child(detail.to_string()),
                    ),
            )
    }

    fn render_general_settings(&self) -> Div {
        div()
            .child(self.settings_heading(
                "General",
                "The native desktop library owns its database and playback. Device Preview is optional.",
            ))
            .child(self.settings_row(
                "Library database",
                &self.config.db_path.display().to_string(),
            ))
            .child(self.settings_row(
                "Managed media",
                &self.config.media_dir.display().to_string(),
            ))
            .child(self.settings_row("Connected device", &self.config.device_target))
    }

    fn render_library_source_settings(&self, cx: &mut Context<Self>) -> Div {
        let references = self
            .snapshot
            .sources
            .iter()
            .filter(|source| source.kind == "reference")
            .cloned()
            .collect::<Vec<_>>();
        let mut list = div()
            .mt(px(12.0))
            .border_1()
            .border_color(self.theme.separator)
            .rounded(px(6.0))
            .overflow_hidden();
        if references.is_empty() {
            list = list.child(
                div()
                    .min_h(px(72.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .text_size(px(10.5))
                    .text_color(self.theme.secondary_text)
                    .child("No referenced folders. Add a folder to index music in place without copying it."),
            );
        } else {
            for source in references {
                let id = source.id;
                let enabled = source.enabled;
                let remove_name = source.display_name.clone();
                let source_path = source.path.clone();
                let available = source.path.is_dir();
                list = list.child(
                    div()
                        .min_h(px(62.0))
                        .px(px(11.0))
                        .flex()
                        .items_center()
                        .border_b_1()
                        .border_color(self.theme.separator)
                        .when(!enabled, |row| row.opacity(0.58))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(7.0))
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(source.display_name)
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .text_color(if available {
                                                    self.theme.success
                                                } else {
                                                    self.theme.error
                                                })
                                                .child(if available {
                                                    "Online"
                                                } else {
                                                    "Offline"
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .mt(px(3.0))
                                        .text_size(px(10.0))
                                        .text_color(self.theme.secondary_text)
                                        .child(source.path.display().to_string()),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(5.0))
                                .child(
                                    Self::chrome_button(("source-rescan", id as usize), "Rescan")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.rescan_source(id, cx)
                                        })),
                                )
                                .child(
                                    Self::chrome_button(("source-full-scan", id as usize), "Full")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.full_rescan_source(id, cx)
                                        })),
                                )
                                .child(
                                    Self::chrome_button(
                                        ("source-toggle", id as usize),
                                        if enabled { "Pause" } else { "Enable" },
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| this.toggle_source(id, !enabled, cx),
                                    )),
                                )
                                .child(
                                    Self::chrome_button(("source-remove", id as usize), "Remove")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.request_remove_source(id, remove_name.clone(), cx)
                                        })),
                                )
                                .child(
                                    Self::chrome_button(("source-reveal", id as usize), "Show")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.reveal_library_path(source_path.clone(), cx)
                                        })),
                                ),
                        ),
                );
            }
        }
        let watch_enabled = self.config.source_preferences.watch_reference_folders;
        let scan_on_startup = self.config.source_preferences.scan_on_startup;
        div()
            .child(self.settings_heading(
                "Library sources",
                "Referenced folders stay in place. MusicOS watches enabled sources, ignores system/application bundles, and never deletes their files.",
            ))
            .child(
                self.settings_row(
                    "Managed library",
                    &format!("{} · app-owned imports", self.config.media_dir.display()),
                )
                .child(Self::chrome_button("source-managed-reveal", "Reveal").on_click(
                    cx.listener(|this, _, _, cx| this.reveal_managed_library(cx)),
                )),
            )
            .child(
                self.settings_row(
                    "Watch reference folders",
                    "Debounces filesystem changes and rescans enabled sources automatically.",
                )
                .child(
                    Self::chrome_button(
                        "source-watch-toggle",
                        if watch_enabled { "On" } else { "Off" },
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.config.source_preferences.watch_reference_folders = !watch_enabled;
                        this.save_preferences(cx);
                    })),
                ),
            )
            .child(
                self.settings_row(
                    "Scan when MusicOS opens",
                    "When off, the cached library appears immediately; Refresh still checks every enabled source.",
                )
                .child(
                    Self::chrome_button(
                        "source-startup-scan-toggle",
                        if scan_on_startup { "On" } else { "Off" },
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.config.source_preferences.scan_on_startup = !scan_on_startup;
                        this.save_preferences(cx);
                    })),
                ),
            )
            .child(
                div()
                    .mt(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Referenced folders"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(5.0))
                            .child(
                                Self::chrome_button("source-rescan-all", "Rescan All").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.rescan_all_reference_sources(cx)
                                    }),
                                ),
                            )
                            .child(Self::chrome_button("source-add", "Add Folder…").on_click(
                                cx.listener(|this, _, _, cx| this.choose_reference_source(cx)),
                            )),
                    ),
            )
            .child(list)
    }

    fn render_metadata_settings(&self, cx: &mut Context<Self>) -> Div {
        let enabled = self.config.enrichment.enabled;
        let review_count = self
            .enrichment_counts
            .get("review")
            .copied()
            .unwrap_or_default();
        let applied_count = self
            .enrichment_counts
            .get("applied")
            .copied()
            .unwrap_or_default();
        let unmatched_count = self
            .enrichment_counts
            .get("unmatched")
            .copied()
            .unwrap_or_default();
        let mut panel = div()
            .child(self.settings_heading(
                "Metadata & artwork",
                "Re-evaluates the full library with MusicBrainz. High-confidence matches become reversible database overlays; uncertain matches remain reviewable.",
            ))
            .child(
                self.settings_row(
                    "Automatic enrichment",
                    "Runs after library changes. File paths and audio files are never uploaded.",
                )
                .child(
                    Self::chrome_button("enrichment-toggle", if enabled { "On" } else { "Off" })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.config.enrichment.enabled = !enabled;
                            this.save_preferences(cx);
                            if !enabled {
                                this.start_metadata_enrichment(true, cx);
                            }
                        })),
                ),
            )
            .child(
                self.settings_row(
                    "Match confidence",
                    &format!(
                        "Apply at {:.0}% · review from {:.0}% · user edits always win",
                        self.config.enrichment.auto_apply_threshold * 100.0,
                        self.config.enrichment.review_threshold * 100.0
                    ),
                )
                .child(if self.enrichment_running {
                    Self::chrome_button("enrichment-cancel", "Cancel").on_click(
                        cx.listener(|this, _, _, cx| this.cancel_metadata_enrichment(cx)),
                    )
                } else {
                    Self::chrome_button("enrichment-run", "Run Now").on_click(
                        cx.listener(|this, _, _, cx| this.start_metadata_enrichment(true, cx)),
                    )
                }),
            )
            .child(self.settings_row(
                "Artwork",
                &format!(
                    "Embedded and folder artwork first · upgrade below {} px · {} MB cache",
                    self.config.artwork.upgrade_below_pixels,
                    self.config.artwork.disk_cache_megabytes
                ),
            ))
            .child(self.settings_row(
                "Results",
                &format!(
                    "{applied_count} applied · {review_count} need review · {unmatched_count} unmatched"
                ),
            ));
        if !self.metadata_reviews.is_empty() {
            panel = panel.child(
                div()
                    .mt(px(13.0))
                    .mb(px(5.0))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Needs review"),
            );
            for (index, item) in self.metadata_reviews.iter().take(6).cloned().enumerate() {
                let accept_id = item.track_id;
                let keep_id = item.track_id;
                panel = panel.child(
                    div()
                        .id(("metadata-review-row", index))
                        .min_h(px(58.0))
                        .py(px(7.0))
                        .border_b_1()
                        .border_color(self.theme.separator)
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .child(
                                    div()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(format!(
                                            "{} — {}",
                                            item.candidate.artist, item.candidate.title
                                        )),
                                )
                                .child(
                                    div()
                                        .mt(px(3.0))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .text_size(px(9.5))
                                        .text_color(self.theme.secondary_text)
                                        .child(format!(
                                            "Current: {} — {} · {:.0}% confidence",
                                            item.current_artist,
                                            item.current_title,
                                            item.candidate.confidence * 100.0
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(5.0))
                                .child(
                                    Self::chrome_button(
                                        ("metadata-review-accept", index),
                                        "Accept",
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.resolve_metadata_review(accept_id, true, cx)
                                        },
                                    )),
                                )
                                .child(
                                    Self::chrome_button(
                                        ("metadata-review-keep", index),
                                        "Keep Current",
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.resolve_metadata_review(keep_id, false, cx)
                                        },
                                    )),
                                ),
                        ),
                );
            }
        }
        panel
    }

    fn render_playback_settings(&self) -> Div {
        div()
            .child(self.settings_heading(
                "Playback",
                "Local playback is owned by the desktop app and remains available when Device Preview is closed.",
            ))
            .child(self.settings_row(
                "Audio backend",
                if self.playback.backend.is_empty() {
                    "Starting local audio"
                } else {
                    &self.playback.backend
                },
            ))
            .child(self.settings_row(
                "Saved volume",
                &format!("{:.0}%", self.playback.volume.clamp(0.0, 1.0) * 100.0),
            ))
    }

    fn render_format_settings(&self, cx: &mut Context<Self>) -> Div {
        div()
            .child(self.settings_heading(
                "Sync & formats",
                "Local originals are never converted. The selected profile applies only to device sync and exports.",
            ))
            .children(self.config.format_profiles.iter().cloned().enumerate().map(|(index, profile)| {
                let selected = self.config.selected_sync_profile == profile.id;
                let id = profile.id.clone();
                let detail = if profile.codec == musicos_desktop::models::AudioCodec::Vorbis {
                    let sample_rate = match profile.sample_rate {
                        musicos_desktop::models::SampleRatePolicy::Auto => {
                            "source sample rate".to_string()
                        }
                        value => format!("{} Hz", value.as_transcode_value()),
                    };
                    format!(
                        "OGG Vorbis quality {:.0} · {} · {} · metadata and artwork preserved",
                        profile.vorbis_quality,
                        sample_rate,
                        match profile.channels {
                            musicos_desktop::models::ChannelPolicy::Auto => "source channels",
                            musicos_desktop::models::ChannelPolicy::Stereo => "stereo",
                            musicos_desktop::models::ChannelPolicy::Mono => "mono",
                        }
                    )
                } else {
                    "Keep the source codec and file unchanged".to_string()
                };
                self.settings_row(&profile.name, &detail).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .when(
                            profile.codec == musicos_desktop::models::AudioCodec::Vorbis,
                            |controls| {
                                controls
                                    .child(
                                        Self::chrome_button(("format-quality-down", index), "Q−")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.adjust_format_quality(index, -1.0, cx)
                                            })),
                                    )
                                    .child(
                                        Self::chrome_button(("format-quality-up", index), "Q+")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.adjust_format_quality(index, 1.0, cx)
                                            })),
                                    )
                                    .child(
                                        Self::chrome_button(("format-rate", index), "Rate")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.cycle_format_sample_rate(index, cx)
                                            })),
                                    )
                                    .child(
                                        Self::chrome_button(("format-channels", index), "Channels")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.cycle_format_channels(index, cx)
                                            })),
                                    )
                            },
                        )
                        .child(
                            Self::chrome_button(
                                ("format-profile", index),
                                if selected { "Selected" } else { "Use" },
                            )
                            .when(selected, |button| button.bg(self.theme.hover))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.config.selected_sync_profile = id.clone();
                                this.save_preferences(cx);
                            })),
                        ),
                )
            }))
    }

    fn render_appearance_settings(&self, cx: &mut Context<Self>) -> Div {
        let mode = self.config.appearance.mode;
        let density = self.config.appearance.density;
        div()
            .child(self.settings_heading(
                "Appearance",
                "Choose how MusicOS follows the desktop and how much information fits in the library.",
            ))
            .child(
                self.settings_row("Color scheme", "System follows the active operating-system appearance.")
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .children([
                                (AppearanceMode::System, "System"),
                                (AppearanceMode::Light, "Light"),
                                (AppearanceMode::Dark, "Dark"),
                            ].into_iter().map(|(next, label)| {
                                Self::chrome_button(("appearance-mode", next as usize), label)
                                    .when(mode == next, |button| button.bg(self.theme.hover))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.config.appearance.mode = next;
                                        this.save_preferences(cx);
                                    }))
                            })),
                    ),
            )
            .child(
                self.settings_row("Density", "Adjusts library rows and control spacing without changing content.")
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .children([
                                (InterfaceDensity::Compact, "Compact"),
                                (InterfaceDensity::Standard, "Standard"),
                                (InterfaceDensity::Comfortable, "Comfortable"),
                            ].into_iter().map(|(next, label)| {
                                Self::chrome_button(("interface-density", next as usize), label)
                                    .when(density == next, |button| button.bg(self.theme.hover))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.config.appearance.density = next;
                                        this.config.appearance.row_height = match next {
                                            InterfaceDensity::Compact => 27,
                                            InterfaceDensity::Standard => 30,
                                            InterfaceDensity::Comfortable => 34,
                                        };
                                        this.save_preferences(cx);
                                    }))
                            })),
                    ),
            )
            .child(
                self.settings_row(
                    "Accent",
                    "Selection, focus, progress, and active controls share one restrained accent.",
                )
                .child(
                    div()
                        .flex()
                        .gap(px(4.0))
                        .children([
                            ("cobalt", "Cobalt"),
                            ("graphite", "Graphite"),
                            ("jade", "Jade"),
                            ("amber", "Amber"),
                            ("berry", "Berry"),
                        ].into_iter().enumerate().map(|(index, (value, label))| {
                            let selected = self.config.appearance.accent == value;
                            Self::chrome_button(("accent-choice", index), label)
                                .when(selected, |button| button.bg(self.theme.hover))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.config.appearance.accent = value.to_string();
                                    this.save_preferences(cx);
                                }))
                        })),
                ),
            )
    }

    fn render_preview_settings(&self, cx: &mut Context<Self>) -> Div {
        let status = self.preview.status();
        div()
            .child(self.settings_heading(
                "Device Preview",
                "The embedded preview is optional, read-only, and cannot scan or compete for desktop audio.",
            ))
            .child(
                self.settings_row(
                    "Preview process",
                    if status.running { "Running" } else { "Stopped" },
                )
                .child(
                    div()
                        .flex()
                        .gap(px(5.0))
                        .child(
                            Self::chrome_button("settings-preview-start", "Launch").on_click(
                                cx.listener(|this, _, _, cx| this.start_preview(cx)),
                            ),
                        )
                        .child(
                            Self::chrome_button("settings-preview-controls", "Controls").on_click(
                                cx.listener(|this, _, _, cx| this.show_preview_controls(cx)),
                            ),
                        ),
                ),
            )
    }

    fn render_advanced_settings(&self, cx: &mut Context<Self>) -> Div {
        div()
            .child(self.settings_heading(
                "Advanced",
                "Repair and cleanup actions create backups and never delete media files.",
            ))
            .child(
                self.settings_row(
                    "Indexed leftovers",
                    &format!(
                        "{} records outside configured library roots",
                        self.source_audit.hidden_tracks.len()
                    ),
                )
                .child(
                    Self::chrome_button("settings-leftovers", "Review…").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.navigate(DesktopSection::Logs, cx);
                        },
                    )),
                ),
            )
            .child(
                self.settings_row(
                    "Database reset",
                    "Preserves the current database as a recoverable backup.",
                )
                .child(
                    Self::chrome_button("database-reset", "Reset…")
                        .on_click(cx.listener(|this, _, _, cx| this.request_database_reset(cx))),
                ),
            )
    }

    fn save_preferences(&mut self, cx: &mut Context<Self>) {
        self.config.version = musicos_desktop::models::DESKTOP_CONFIG_VERSION;
        match save_config(&self.config) {
            Ok(()) => self.notice = Some("Preferences saved".to_string()),
            Err(error) => self.error = Some(format!("Could not save preferences: {error:#}")),
        }
        cx.notify();
    }

    fn adjust_format_quality(&mut self, index: usize, delta: f32, cx: &mut Context<Self>) {
        if let Some(profile) = self.config.format_profiles.get_mut(index) {
            profile.vorbis_quality = (profile.vorbis_quality + delta).clamp(-1.0, 10.0);
            self.save_preferences(cx);
        }
    }

    fn cycle_format_sample_rate(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(profile) = self.config.format_profiles.get_mut(index) {
            profile.sample_rate = match profile.sample_rate {
                musicos_desktop::models::SampleRatePolicy::Auto => {
                    musicos_desktop::models::SampleRatePolicy::Hz44100
                }
                musicos_desktop::models::SampleRatePolicy::Hz44100 => {
                    musicos_desktop::models::SampleRatePolicy::Hz48000
                }
                musicos_desktop::models::SampleRatePolicy::Hz48000 => {
                    musicos_desktop::models::SampleRatePolicy::Auto
                }
            };
            self.save_preferences(cx);
        }
    }

    fn cycle_format_channels(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(profile) = self.config.format_profiles.get_mut(index) {
            profile.channels = match profile.channels {
                musicos_desktop::models::ChannelPolicy::Auto => {
                    musicos_desktop::models::ChannelPolicy::Stereo
                }
                musicos_desktop::models::ChannelPolicy::Stereo => {
                    musicos_desktop::models::ChannelPolicy::Mono
                }
                musicos_desktop::models::ChannelPolicy::Mono => {
                    musicos_desktop::models::ChannelPolicy::Auto
                }
            };
            self.save_preferences(cx);
        }
    }

    fn resolve_metadata_review(&mut self, track_id: i64, accept: bool, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let task = cx.background_executor().spawn(async move {
            if accept {
                repository.accept_metadata_review(track_id)
            } else {
                repository.dismiss_metadata_review(track_id)
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.notice = Some(if accept {
                            "Metadata match accepted".to_string()
                        } else {
                            "Current metadata kept".to_string()
                        });
                        this.refresh_library(cx);
                    }
                    Err(error) => {
                        this.error = Some(format!("Could not update metadata review: {error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn render_confirmation(&self, cx: &mut Context<Self>) -> Option<Div> {
        match self.pending_confirmation.clone()? {
            PendingConfirmation::ClearQueue => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(18.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(self.theme.warning)
                    .bg(self.theme.elevated_surface)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(self.theme.text)
                            .child(format!(
                                "Clear all {} songs from the queue? Playback will stop.",
                                self.playback.queue.len()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("confirm-queue-clear", "Clear queue")
                                    .on_click(cx.listener(|this, _, _, cx| this.clear_queue(cx))),
                            )
                            .child(
                                Self::chrome_button("confirm-queue-cancel", "Cancel").on_click(
                                    cx.listener(|this, _, _, cx| this.cancel_confirmation(cx)),
                                ),
                            ),
                    ),
            ),
            PendingConfirmation::DeletePlaylist { id, name } => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(10.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(self.theme.warning)
                    .bg(self.theme.elevated_surface)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(self.theme.text)
                            .child(format!(
                                "Delete “{name}”? Its {} tracks stay in your library.",
                                self.playlist_tracks.len()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button(("confirm-playlist-delete", id as usize), "Delete")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.delete_active_playlist(cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("confirm-playlist-cancel", "Cancel").on_click(
                                    cx.listener(|this, _, _, cx| this.cancel_confirmation(cx)),
                                ),
                            ),
                    ),
            ),
            PendingConfirmation::RemoveSource { id, name } => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(18.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(rgb(0xb56a6a))
                    .bg(rgb(ui_color(0xfff5f5, 0x3d2428)))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ui_color(0x6f2525, 0xffc2bc)))
                            .child(format!(
                                "Remove “{name}”? Music files stay on disk; indexed entries are removed."
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("confirm-source-remove", "Remove")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirm_remove_source(id, cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("confirm-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_confirmation(cx)
                                    })),
                            ),
                    ),
            ),
            PendingConfirmation::ResetDatabase => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(18.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(rgb(0xb56a6a))
                    .bg(rgb(ui_color(0xfff5f5, 0x3d2428)))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ui_color(0x6f2525, 0xffc2bc)))
                            .child("Reset the local library database? The old database will be preserved and Device Preview will remain optional."),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("confirm-database-reset", "Reset")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_database_reset(cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("confirm-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_confirmation(cx)
                                    })),
                            ),
                    ),
            ),
            PendingConfirmation::RestoreBackup { path } => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(18.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(rgb(0xb08b54))
                    .bg(rgb(ui_color(0xfffaf0, 0x3a3022)))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ui_color(0x674814, 0xffd69a)))
                            .child(format!(
                                "Restore settings and playlists from {}?",
                                path.display()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("confirm-backup-restore", "Restore")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirm_restore_backup(path.clone(), cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("confirm-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_confirmation(cx)
                                    })),
                            ),
                    ),
            ),
            PendingConfirmation::InstallUpdate => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(18.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(rgb(0xb08b54))
                    .bg(rgb(ui_color(0xfffaf0, 0x3a3022)))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ui_color(0x674814, 0xffd69a)))
                            .child("Install the staged MusicOS update from removable storage?"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("confirm-update-install", "Install")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_update_install(cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("confirm-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_confirmation(cx)
                                    })),
                            ),
                    ),
            ),
            PendingConfirmation::CleanupIndexedLeftovers {
                revision,
                count,
                playlist_references,
            } => Some(
                div()
                    .mx(px(18.0))
                    .mb(px(18.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(rgb(0xb08b54))
                    .bg(rgb(ui_color(0xfffaf0, 0x3a3022)))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ui_color(0x674814, 0xffd69a)))
                            .child(format!(
                                "Remove {count} indexed leftover{}?{} A database backup is created first.",
                                if count == 1 { "" } else { "s" },
                                if playlist_references > 0 {
                                    format!(" {playlist_references} playlist references will be cleaned.")
                                } else {
                                    String::new()
                                }
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_button("confirm-cleanup", "Remove")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirm_indexed_leftovers_cleanup(revision, cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("confirm-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_confirmation(cx)
                                    })),
                            ),
                    ),
            ),
        }
    }

    fn render_operation_page(&self, cx: &mut Context<Self>) -> Div {
        let (title, detail) = match self.section() {
            DesktopSection::Devices => (
                "Device Preview",
                "Launch the optional embedded MusicOS preview. The desktop library and audio keep working when it is closed.",
            ),
            DesktopSection::Sync => (
                "Sync music",
                "Plan, transcode, and copy selected music while preserving artist and album folders.",
            ),
            DesktopSection::Updates => (
                "Software updates",
                "Stage a verified MusicOS update on removable storage and install it on the player.",
            ),
            DesktopSection::Logs => (
                "Diagnostics",
                "Runtime, synchronization, scan, and performance logs appear here.",
            ),
            DesktopSection::Settings => (
                "Preferences",
                "Library locations, playback output, device target, and synchronization defaults.",
            ),
            DesktopSection::Playlists => (
                "Playlists",
                "Create, reorder, export, import, and reconcile playlists with a connected player.",
            ),
            DesktopSection::Queue => (
                "Queue",
                "The local playback queue is available from the Library source list.",
            ),
            _ => ("Library", "Browse the music stored on this computer."),
        };
        let panel_width = if self.section() == DesktopSection::Settings {
            1020.0
        } else {
            720.0
        };
        let mut panel = div()
            .max_w(px(panel_width))
            .w_full()
            .border_1()
            .border_color(self.theme.separator)
            .bg(self.theme.surface)
            .shadow_sm()
            .child(
                div()
                    .h(px(34.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .bg(self.theme.chrome)
                    .border_b_1()
                    .border_color(self.theme.separator)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(12.0))
                    .text_color(self.theme.text)
                    .child(title),
            )
            .child(
                div()
                    .p(px(18.0))
                    .text_size(px(13.0))
                    .text_color(self.theme.secondary_text)
                    .child(detail),
            );
        panel = match self.section() {
            DesktopSection::Devices => {
                let status = self.preview.status();
                panel
                    .child(
                        div()
                        .px(px(18.0))
                        .pb(px(12.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            Self::chrome_button("runtime-start", "Launch / Restart Preview")
                                .on_click(cx.listener(|this, _, _, cx| this.start_preview(cx))),
                        )
                        .child(
                            Self::chrome_button("runtime-stop", "Stop")
                                .on_click(cx.listener(|this, _, _, cx| this.stop_preview(cx))),
                        )
                        .child(
                            Self::chrome_button("runtime-controls", "Show Controls")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_preview_controls(cx)
                                })),
                        )
                        .child(
                            div()
                                .ml(px(8.0))
                                .text_size(px(11.0))
                                .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                .child(if status.running {
                                    format!("Running · PID {}", status.pid.unwrap_or_default())
                                } else {
                                    "Stopped".to_string()
                                }),
                        ),
                    )
                    .child(
                        div()
                            .mx(px(18.0))
                            .mb(px(18.0))
                            .pt(px(12.0))
                            .border_t_1()
                            .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .child(
                                Self::chrome_button("library-backup", "Back Up…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.backup_library(cx)
                                    })),
                            )
                            .child(
                                Self::chrome_button("library-restore", "Restore…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.choose_restore_backup(cx)
                                    })),
                            )
                            .child(
                                div()
                                    .ml(px(8.0))
                                    .text_size(px(10.0))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child("Backups include settings, playlists, track membership, and history."),
                            ),
                    )
            }
            DesktopSection::Sync => {
                let progress = &self.sync_progress;
                panel.child(
                    div()
                        .px(px(18.0))
                        .pb(px(18.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            Self::chrome_button("sync-start", "Sync Folder")
                                .on_click(cx.listener(|this, _, _, cx| this.start_sync(cx))),
                        )
                        .child(
                            Self::chrome_button("sync-cancel", "Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_operation(cx))),
                        )
                        .child(
                            div()
                                .ml(px(8.0))
                                .text_size(px(11.0))
                                .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                .child(format!(
                                    "{} uploaded · {} skipped · {} failed",
                                    progress.uploaded, progress.skipped, progress.failed
                                )),
                        ),
                )
            }
            DesktopSection::Logs => {
                panel
                    .child(self.render_diagnostics_controls(cx))
                    .child(self.render_indexed_leftovers(cx))
                    .child(
                    div()
                        .mx(px(18.0))
                        .mb(px(18.0))
                        .p(px(9.0))
                        .min_h(px(160.0))
                        .border_1()
                        .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
                        .bg(rgb(ui_color(0xf6f7f8, DARK_ROW_ALT)))
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .children(self.preview.logs().into_iter().rev().take(18).map(|line| {
                            div().text_size(px(10.0)).text_color(rgb(ui_color(TEXT, DARK_TEXT))).child(line)
                        }))
                        .when(self.preview.logs().is_empty(), |logs| {
                            logs.child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child("No runtime log entries yet."),
                            )
                        }),
                    )
            }
            DesktopSection::Updates => panel.child(
                div()
                    .px(px(18.0))
                    .pb(px(18.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Self::chrome_button("update-install", "Install Staged Update…")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_update_install(cx)
                            })),
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .text_size(px(10.5))
                            .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                            .child("The player verifies and installs an update staged on removable storage."),
                    ),
            ),
            DesktopSection::Settings => panel.child(self.render_settings_content(cx)),
            _ => panel,
        };
        if let Some(confirmation) = self.render_confirmation(cx) {
            panel = panel.child(confirmation);
        }
        div()
            .flex_1()
            .min_h(px(0.0))
            .bg(rgb(ui_color(CONTENT, DARK_CONTENT)))
            .p(px(28.0))
            .child(panel)
    }

    fn context_menu_action(
        id: impl Into<gpui::ElementId>,
        label: &str,
        icon: &'static str,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .mx(px(5.0))
            .px(px(9.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .gap(px(9.0))
            .rounded(px(6.0))
            .text_size(px(12.0))
            .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
            .cursor_pointer()
            .hover(|style| {
                style
                    .bg(rgb(ui_color(BLUE, DARK_BLUE)))
                    .text_color(rgb(0xffffff))
            })
            .child(svg().path(icon).size(px(14.0)).flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(label.to_string()),
            )
    }

    fn context_menu_destructive(
        id: impl Into<gpui::ElementId>,
        label: &str,
        icon: &'static str,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .mx(px(5.0))
            .px(px(9.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .gap(px(9.0))
            .rounded(px(6.0))
            .text_size(px(12.0))
            .text_color(rgb(ui_color(0xb43a31, 0xff8a80)))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(ui_color(0xfdecea, 0x3a2326))))
            .child(svg().path(icon).size(px(14.0)).flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(label.to_string()),
            )
    }

    fn context_menu_header(track: &Track) -> Div {
        let artwork = track.cover_art_path.clone();
        div()
            .px(px(10.0))
            .pt(px(4.0))
            .pb(px(8.0))
            .flex()
            .items_center()
            .gap(px(9.0))
            .child(
                div()
                    .size(px(36.0))
                    .flex_none()
                    .when_some(artwork.clone(), |cell, path| {
                        cell.child(
                            img(path)
                                .size(px(36.0))
                                .rounded(px(5.0))
                                .object_fit(ObjectFit::Cover)
                                .with_loading(|| artwork_placeholder_sized(36.0).into_any_element())
                                .with_fallback(|| {
                                    artwork_placeholder_sized(36.0).into_any_element()
                                }),
                        )
                    })
                    .when(artwork.is_none(), |cell| {
                        cell.child(artwork_placeholder_sized(36.0))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
                            .child(track.title.clone()),
                    )
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(px(10.5))
                            .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                            .child(track.artist.clone()),
                    ),
            )
    }

    fn render_context_menu(
        &self,
        viewport: gpui::Size<gpui::Pixels>,
        _cx: &mut Context<Self>,
    ) -> Div {
        let Some(menu) = self.context_menu.clone() else {
            return div();
        };
        let track_id = menu.track_id;
        let track = self
            .snapshot
            .tracks
            .iter()
            .find(|track| track.id == track_id)
            .cloned();
        let favorited = track.as_ref().is_some_and(|track| track.favorited);
        let in_playlist = self.active_playlist.is_some();
        let favorite_label = if favorited { "Unfavorite" } else { "Favorite" };
        let playlists_open = menu.playlists_open;
        let playlist_count = self.snapshot.playlists.len();

        let menu_width = 225.0;
        let item_count = 3 // Play, Play Next, Add to Queue
            + 1 // Add to Playlist
            + if playlists_open { playlist_count.max(1) } else { 0 }
            + 1 // Favorite
            + usize::from(in_playlist);
        let estimated_height = 58.0 // track header block
            + item_count as f32 * 28.0
            + 3.0 * 9.0 // separators
            + 12.0; // vertical padding
        let x = f32::from(menu.position.x)
            .min(f32::from(viewport.width) - menu_width - 8.0)
            .max(8.0);
        let y = f32::from(menu.position.y)
            .min(f32::from(viewport.height) - estimated_height - 8.0)
            .max(8.0);

        let separator = || {
            div()
                .h(px(1.0))
                .mx(px(6.0))
                .my(px(4.0))
                .bg(rgb(ui_color(BORDER, DARK_BORDER)))
        };
        let mut menu_panel = div()
            .id("context-menu")
            .absolute()
            .top(px(y))
            .left(px(x))
            .w(px(menu_width))
            .py(px(6.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(rgb(ui_color(BORDER_DARK, DARK_BORDER_DARK)))
            .bg(rgb(ui_color(CONTENT, DARK_CONTENT)))
            .shadow_lg();
        if let Some(track) = track.as_ref() {
            menu_panel = menu_panel.child(Self::context_menu_header(track));
        }
        menu_panel = menu_panel
            .child(
                Self::context_menu_action("ctx-play", "Play", "icons/desktop/play.svg")
                    .on_click(cx.listener(move |this, _, _, cx| this.context_play(track_id, cx))),
            )
            .child(
                Self::context_menu_action("ctx-play-next", "Play Next", "icons/desktop/next.svg")
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.context_play_next(track_id, cx)),
                    ),
            )
            .child(
                Self::context_menu_action("ctx-queue", "Add to Queue", "icons/desktop/queue.svg")
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.context_add_to_queue(track_id, cx)),
                    ),
            )
            .child(separator())
            .child(
                div()
                    .id("ctx-add-playlist")
                    .mx(px(5.0))
                    .px(px(9.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .gap(px(9.0))
                    .rounded(px(6.0))
                    .text_size(px(12.0))
                    .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(rgb(ui_color(BLUE, DARK_BLUE)))
                            .text_color(rgb(0xffffff))
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_context_menu_playlists(cx)))
                    .child(
                        svg()
                            .path("icons/desktop/playlist.svg")
                            .size(px(14.0))
                            .flex_none(),
                    )
                    .child(div().flex_1().min_w(px(0.0)).child("Add to Playlist"))
                    .child(
                        svg()
                            .path("icons/desktop/disclosure.svg")
                            .size(px(13.0))
                            .flex_none(),
                    ),
            );
        if playlists_open {
            if self.snapshot.playlists.is_empty() {
                menu_panel = menu_panel.child(
                    div()
                        .mx(px(5.0))
                        .px(px(9.0))
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .text_size(px(11.0))
                        .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                        .child("No playlists yet"),
                );
            }
            for playlist in &self.snapshot.playlists {
                let playlist_id = playlist.id;
                menu_panel = menu_panel.child(
                    div()
                        .id(("ctx-playlist", playlist_id as usize))
                        .mx(px(5.0))
                        .pl(px(32.0))
                        .pr(px(9.0))
                        .h(px(26.0))
                        .flex()
                        .items_center()
                        .rounded(px(6.0))
                        .text_size(px(11.5))
                        .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
                        .cursor_pointer()
                        .hover(|style| {
                            style
                                .bg(rgb(ui_color(BLUE, DARK_BLUE)))
                                .text_color(rgb(0xffffff))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.context_add_to_playlist(playlist_id, track_id, cx)
                        }))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(playlist.name.clone()),
                        ),
                );
            }
        }
        menu_panel = menu_panel.child(separator()).child(
            Self::context_menu_action("ctx-favorite", favorite_label, "icons/desktop/favorite.svg")
                .on_click(
                    cx.listener(move |this, _, _, cx| this.toggle_favorite_track(track_id, cx)),
                ),
        );
        if in_playlist {
            menu_panel = menu_panel.child(
                Self::context_menu_destructive(
                    "ctx-remove",
                    "Remove from Playlist",
                    "icons/desktop/trash.svg",
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.context_remove_from_playlist(track_id, cx)
                })),
            );
        }
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                div()
                    .id("context-menu-dismiss")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.close_context_menu(cx)),
                    ),
            )
            .child(menu_panel)
    }

    #[allow(dead_code)]
    fn render_playback_bar(&self, cx: &mut Context<Self>) -> Div {
        let now_playing = self.playback.track.as_ref();
        let play_icon = if self.playback.is_playing {
            "icons/desktop/pause.svg"
        } else {
            "icons/desktop/play.svg"
        };
        let title = now_playing
            .map(|track| track.title.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Nothing playing");
        let artist = now_playing
            .map(|track| track.artist.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Choose something from your library");
        let artwork_path = now_playing.and_then(|track| track.cover_art_path.clone());
        let duration = now_playing.map(|track| track.duration).unwrap_or(0);
        let timeline_fraction = if duration > 0 {
            (self.playback.position / duration as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let elapsed = format_duration(self.playback.position as i32);
        let remaining = format_duration((duration as f32 - self.playback.position).max(0.0) as i32);
        let volume = self.playback.volume.clamp(0.0, 1.0);
        let shuffle = self.playback.shuffle;
        let repeat = self.playback.repeat.clone();
        let repeat_icon = if repeat == "one" {
            "icons/desktop/repeat-one.svg"
        } else {
            "icons/desktop/repeat.svg"
        };
        let repeat_active = repeat != "none";

        let seek_bounds = Rc::new(RefCell::new(Bounds::default()));
        let volume_bounds = Rc::new(RefCell::new(Bounds::default()));

        div()
            .h(px(74.0))
            .flex_none()
            .flex()
            .items_center()
            .px(px(14.0))
            .gap(px(16.0))
            .bg(rgb(ui_color(CHROME_BOTTOM, DARK_CHROME_BOTTOM)))
            .border_t_1()
            .border_color(rgb(ui_color(BORDER_DARK, DARK_BORDER_DARK)))
            .child(
                div()
                    .w(px(250.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .size(px(52.0))
                            .flex_none()
                            .when_some(artwork_path.clone(), |cell, path| {
                                cell.child(
                                    img(path)
                                        .size(px(52.0))
                                        .rounded(px(5.0))
                                        .object_fit(ObjectFit::Cover)
                                        .with_loading(|| artwork_placeholder().into_any_element())
                                        .with_fallback(|| artwork_placeholder().into_any_element()),
                                )
                            })
                            .when(artwork_path.is_none(), |cell| {
                                cell.child(artwork_placeholder_sized(52.0))
                            }),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(ui_color(TEXT, DARK_TEXT)))
                                    .child(title.to_string()),
                            )
                            .child(
                                div()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_size(px(10.5))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(artist.to_string()),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                Self::chrome_icon_button("bar-shuffle", "icons/desktop/shuffle.svg")
                                    .when(shuffle, |button| {
                                        button.bg(rgb(ui_color(0xd6e8ff, 0x1f3a5c)))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.player_command("shuffle", cx)
                                    })),
                            )
                            .child(
                                Self::chrome_icon_button("bar-previous", "icons/desktop/previous.svg")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.player_command("prev", cx)
                                    })),
                            )
                            .child(
                                Self::chrome_icon_button("bar-play", play_icon)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.player_command("play_pause", cx)
                                    })),
                            )
                            .child(
                                Self::chrome_icon_button("bar-next", "icons/desktop/next.svg")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.player_command("next", cx)
                                    })),
                            )
                            .child(
                                Self::chrome_icon_button("bar-repeat", repeat_icon)
                                    .when(repeat_active, |button| {
                                        button.bg(rgb(ui_color(0xd6e8ff, 0x1f3a5c)))
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.player_command("repeat", cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .w(px(38.0))
                                    .flex_none()
                                    .text_size(px(9.5))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(elapsed),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .h(px(16.0))
                                    .on_children_prepainted({
                                        let seek_bounds = seek_bounds.clone();
                                        move |bounds, _, _| {
                                            if let Some(bounds) = bounds.first() {
                                                *seek_bounds.borrow_mut() = *bounds;
                                            }
                                        }
                                    })
                                    .child(
                                        div()
                                            .id("seek-bar")
                                            .w_full()
                                            .h(px(5.0))
                                            .rounded(px(2.5))
                                            .bg(rgb(ui_color(0xd2d2d7, 0x3a3a3c)))
                                            .cursor_pointer()
                                            .on_mouse_down(MouseButton::Left, {
                                                let seek_bounds = seek_bounds.clone();
                                                cx.listener(
                                                    move |this, event: &gpui::MouseDownEvent, _, cx| {
                                                        let bounds = *seek_bounds.borrow();
                                                        let width = f32::from(bounds.size.width);
                                                        if width > 0.0 {
                                                            let fraction = f32::from(
                                                                event.position.x - bounds.origin.x,
                                                            ) / width;
                                                            this.seek_to(fraction, cx);
                                                        }
                                                    },
                                                )
                                            })
                                            .child(
                                                div()
                                                    .h_full()
                                                    .w(relative(timeline_fraction))
                                                    .rounded(px(2.5))
                                                    .bg(rgb(ui_color(BLUE, DARK_BLUE))),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(38.0))
                                    .flex_none()
                                    .text_size(px(9.5))
                                    .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
                                    .child(remaining),
                            ),
                    ),
            )
            .child(
                div()
                    .w(px(172.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(
                        svg()
                            .path("icons/desktop/volume.svg")
                            .size(px(15.0))
                            .flex_none()
                            .text_color(rgb(ui_color(MUTED, DARK_MUTED))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .h(px(16.0))
                            .on_children_prepainted({
                                let volume_bounds = volume_bounds.clone();
                                move |bounds, _, _| {
                                    if let Some(bounds) = bounds.first() {
                                        *volume_bounds.borrow_mut() = *bounds;
                                    }
                                }
                            })
                            .child(
                                div()
                                    .id("volume-bar")
                                    .w_full()
                                    .h(px(5.0))
                                    .rounded(px(2.5))
                                    .bg(rgb(ui_color(0xd2d2d7, 0x3a3a3c)))
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, {
                                        let volume_bounds = volume_bounds.clone();
                                        cx.listener(
                                            move |this, event: &gpui::MouseDownEvent, _, cx| {
                                                let bounds = *volume_bounds.borrow();
                                                let width = f32::from(bounds.size.width);
                                                if width > 0.0 {
                                                    let fraction =
                                                        f32::from(event.position.x - bounds.origin.x)
                                                            / width;
                                                    this.set_volume(fraction, cx);
                                                }
                                            },
                                        )
                                    })
                                    .child(
                                        div()
                                            .h_full()
                                            .w(relative(volume))
                                            .rounded(px(2.5))
                                            .bg(rgb(ui_color(BLUE, DARK_BLUE))),
                                    ),
                            ),
                    ),
            )
    }

    #[allow(dead_code)]
    fn render_status_bar(&self) -> Div {
        let selected = usize::from(self.selected_track_id.is_some());
        div()
            .h(px(23.0))
            .flex_none()
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(rgb(ui_color(CHROME_BOTTOM, DARK_CHROME_BOTTOM)))
            .border_t_1()
            .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
            .text_size(px(10.5))
            .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
            .child(format!(
                "{} songs   •   {} selected   •   {}",
                self.snapshot.tracks.len(),
                selected,
                total_duration(&self.snapshot.tracks)
            ))
            .child(if let Some(error) = &self.error {
                error.clone()
            } else if self.import_progress.running {
                format!(
                    "Importing {} / {}",
                    self.import_progress.processed, self.import_progress.found
                )
            } else if self.sync_progress.running {
                format!(
                    "Syncing {} / {}",
                    self.sync_progress.uploaded
                        + self.sync_progress.skipped
                        + self.sync_progress.failed,
                    self.sync_progress.found
                )
            } else if self.loading {
                "Updating library…".to_string()
            } else if self.playback_intent.is_some() {
                "Starting local playback…".to_string()
            } else {
                self.notice.clone().unwrap_or_else(|| "Ready".to_string())
            })
    }
}

impl Render for ControlCenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = match self.config.appearance.mode {
            AppearanceMode::System => matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
            AppearanceMode::Light => false,
            AppearanceMode::Dark => true,
        };
        DARK_APPEARANCE.store(dark, Ordering::Relaxed);
        self.theme = if dark {
            ThemePalette::dark()
        } else {
            ThemePalette::light()
        }
        .with_accent(&self.config.appearance.accent, dark);
        if self.compact_player {
            return self.render_compact_player(cx);
        }
        if self.task_center.running_count() > 0 {
            window.request_animation_frame();
        }
        let viewport_size = window.viewport_size();

        div()
            .id("control-center-root")
            .relative()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &ImportMusic, _, cx| this.choose_import(cx)))
            .on_action(cx.listener(|this, _: &RefreshLibrary, _, cx| this.refresh_library(cx)))
            .on_action(
                cx.listener(|this, _: &TogglePlayback, _, cx| {
                    this.player_command("play_pause", cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &PreviousTrack, _, cx| this.player_command("previous", cx)),
            )
            .on_action(cx.listener(|this, _: &NextTrack, _, cx| this.player_command("next", cx)))
            .on_action(
                cx.listener(|this, _: &ShowSongs, _, cx| this.navigate(DesktopSection::Songs, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ShowAlbums, _, cx| {
                    this.navigate(DesktopSection::Albums, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ShowArtists, _, cx| {
                this.navigate(DesktopSection::Artists, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowFlow, _, cx| {
                this.navigate(DesktopSection::Albums, cx);
                this.set_view(LibraryPresentation::Flow, cx);
            }))
            .on_action(cx.listener(|this, _: &ShowPreferences, _, cx| {
                this.navigate(DesktopSection::Settings, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowDiagnostics, _, cx| {
                this.navigate(DesktopSection::Logs, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleActivity, _, cx| this.toggle_activity(cx)))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                window.focus(&this.search_input.focus_handle(cx));
            }))
            .on_action(cx.listener(|_, _: &ToggleFullscreen, window, _| {
                window.toggle_fullscreen();
            }))
            .on_action(cx.listener(|this, _: &ToggleMiniPlayer, window, cx| {
                this.toggle_mini_player(window, cx)
            }))
            .on_action(cx.listener(|this, _: &NewPlaylist, _, cx| this.create_playlist(cx)))
            .on_action(cx.listener(|this, _: &DeleteFromPlaylist, _, cx| {
                this.remove_selected_from_playlist(cx)
            }))
            .on_action(
                cx.listener(|this, _: &SaveQueueAsPlaylist, _, cx| this.save_queue_as_playlist(cx)),
            )
            .on_action(cx.listener(|this, _: &ClearQueue, _, cx| this.request_clear_queue(cx)))
            .on_action(
                cx.listener(|this, _: &ExportPlaylist, _, cx| this.export_active_playlist(cx)),
            )
            .on_action(cx.listener(|this, _: &ImportPlaylist, _, cx| this.import_playlist_file(cx)))
            .on_key_down(cx.listener(|this, event, window, cx| this.key_down(event, window, cx)))
            .w(px(720.0))
            .h(px(480.0))
            .relative()
            .font_family("FOT-RodinNTLG Pro")
            .bg(rgb(0x80d3ff))
            .text_color(rgb(0x000000))
            .child(self.render_toolbar(cx))
            .when(!self.devices_mode, |root| {
                root
                    // Sidebar: left: 2, top: 64, w: 196, h: 414
                    .child(
                        div()
                            .absolute()
                            .left(px(2.0))
                            .top(px(64.0))
                            .w(px(196.0))
                            .h(px(414.0))
                            .child(self.render_sidebar(cx))
                    )
                    // Subheader: left: 200, top: 64, w: 518, h: 32
                    .child(
                        div()
                            .absolute()
                            .left(px(200.0))
                            .top(px(64.0))
                            .w(px(518.0))
                            .h(px(32.0))
                            .child(self.render_browser_header())
                    )
                    // Content box: left: 200, top: 100, w: 515, h: 378
                    .child(
                        div()
                            .absolute()
                            .left(px(200.0))
                            .top(px(100.0))
                            .w(px(515.0))
                            .h(px(378.0))
                            .border_1()
                            .border_color(rgb(0x0018ff))
                            .bg(rgb(0x00aeff))
                            .overflow_hidden()
                            .child(self.render_collection_browser(720.0, cx))
                    )
            })
            .when(self.devices_mode, |root| {
                root
                    // Subheader spans width: left: 2, top: 65, w: 716, h: 32
                    .child(
                        div()
                            .absolute()
                            .left(px(2.0))
                            .top(px(65.0))
                            .w(px(716.0))
                            .h(px(32.0))
                            .child(self.render_browser_header())
                    )
                    // Sidebar: left: 2, top: 100, w: 255, h: 378
                    .child(
                        div()
                            .absolute()
                            .left(px(2.0))
                            .top(px(100.0))
                            .w(px(255.0))
                            .h(px(378.0))
                            .child(self.render_sidebar(cx))
                    )
                    // Content box: left: 259, top: 100, w: 456, h: 378
                    .child(
                        div()
                            .absolute()
                            .left(px(259.0))
                            .top(px(100.0))
                            .w(px(456.0))
                            .h(px(378.0))
                            .border_1()
                            .border_color(rgb(0x0018ff))
                            .bg(rgb(0x00aeff))
                            .overflow_hidden()
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_center()
                                            .text_size(px(20.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x000000))
                                            .child("choosing what to sync\nis coming soon."),
                                    )
                            )
                    )
            })
            .when(self.context_menu.is_some(), |root| {
                root.child(self.render_context_menu(viewport_size, cx))
            })
            .when(self.sparklist_modal_open, |root| {
                root.child(self.render_sparklist_modal(cx))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AlbumGridMetrics {
    columns: usize,
    card_width: f32,
    artwork_size: f32,
    card_height: f32,
    row_height: f32,
    gap: f32,
}

fn album_grid_metrics(
    viewport_width: f32,
    sidebar_width: f32,
    preferred_card_width: f32,
) -> AlbumGridMetrics {
    const HORIZONTAL_PADDING: f32 = 24.0;
    const GAP: f32 = 12.0;
    const CARD_PADDING: f32 = 8.0;
    const METADATA_HEIGHT: f32 = 45.0;
    const ROW_PADDING: f32 = 10.0;

    let available_width = (viewport_width - sidebar_width - HORIZONTAL_PADDING).max(320.0);
    let preferred_card_width = preferred_card_width.clamp(128.0, 220.0);
    let columns =
        (((available_width + GAP) / (preferred_card_width + GAP)).ceil() as usize).clamp(2, 12);
    let card_width = ((available_width - GAP * columns.saturating_sub(1) as f32) / columns as f32)
        .clamp(128.0, 230.0)
        .floor();
    let artwork_size = (card_width - CARD_PADDING).max(120.0);
    let card_height = artwork_size + METADATA_HEIGHT + CARD_PADDING;
    AlbumGridMetrics {
        columns,
        card_width,
        artwork_size,
        card_height,
        row_height: card_height + ROW_PADDING,
        gap: GAP,
    }
}

fn route_from_config(config: &ControlCenterConfig) -> AppRoute {
    if config.last_section == DesktopSection::Music {
        return AppRoute::Library(DesktopLibraryRoute::new(
            LibraryScope::Albums,
            LibraryPresentation::Grid,
        ));
    }
    if let Some(scope) = LibraryScope::from_section(config.last_section) {
        let legacy_presentation = match config.library_view {
            LibraryViewMode::Songs => LibraryPresentation::Table,
            LibraryViewMode::Albums | LibraryViewMode::Artists => LibraryPresentation::Grid,
            LibraryViewMode::CoverFlow => LibraryPresentation::Flow,
        };
        let presentation = config
            .library_ui
            .presentation_by_scope
            .get(&scope.key())
            .copied()
            .unwrap_or(legacy_presentation);
        AppRoute::Library(DesktopLibraryRoute::new(scope, presentation))
    } else {
        AppRoute::Section(config.last_section)
    }
}

fn column(text: &str, width: f32) -> Div {
    div()
        .w(px(width))
        .min_w(px(width))
        .h_full()
        .px(px(7.0))
        .flex()
        .items_center()
        .overflow_hidden()
        .border_r_1()
        .border_color(rgb(0xc6cad0))
        .child(text.to_string())
}

fn collection_column(text: &str, width: f32, weight: FontWeight) -> Div {
    div()
        .w(px(width))
        .min_w(px(width))
        .h_full()
        .px(px(9.0))
        .flex()
        .items_center()
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .border_r_1()
        .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
        .text_size(px(11.5))
        .font_weight(weight)
        .child(text.to_string())
}

fn collection_list_header(
    primary: &'static str,
    secondary: &'static str,
    tertiary: &'static str,
    fourth: &'static str,
) -> Div {
    div()
        .h(px(28.0))
        .flex_none()
        .flex()
        .items_center()
        .bg(rgb(ui_color(CHROME_BOTTOM, DARK_CHROME_BOTTOM)))
        .border_b_1()
        .border_color(rgb(ui_color(BORDER, DARK_BORDER)))
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
        .child(column(primary, 420.0))
        .child(column(secondary, 100.0))
        .child(column(tertiary, 100.0))
        .child(column(fourth, 110.0))
}

fn compare_tracks(left: &Track, right: &Track, field: SortField) -> std::cmp::Ordering {
    let ordering = match field {
        SortField::Title => left.title.cmp(&right.title),
        SortField::Artist => left.artist.cmp(&right.artist),
        SortField::Album => left.album.cmp(&right.album),
        SortField::Genre => left.genre.cmp(&right.genre),
        SortField::Year => left.year.cmp(&right.year),
        SortField::TrackNumber => {
            (left.disc_number, left.track_number).cmp(&(right.disc_number, right.track_number))
        }
        SortField::Duration => left.duration.cmp(&right.duration),
        SortField::Rating => left.rating.cmp(&right.rating),
        SortField::PlayCount => left.play_count.cmp(&right.play_count),
        SortField::DateAdded => left.indexed_at.cmp(&right.indexed_at),
    };
    ordering
        .then_with(|| left.artist.cmp(&right.artist))
        .then_with(|| left.album.cmp(&right.album))
        .then_with(|| left.disc_number.cmp(&right.disc_number))
        .then_with(|| left.track_number.cmp(&right.track_number))
        .then_with(|| left.id.cmp(&right.id))
}

fn artwork_placeholder() -> Div {
    artwork_placeholder_sized(31.0)
}

fn artwork_placeholder_sized(size: f32) -> Div {
    div()
        .size(px(size))
        .rounded(px(4.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(ui_color(0xcbd3dc, 0x34404e)))
        .text_color(rgb(ui_color(MUTED, DARK_MUTED)))
        .child(
            svg()
                .path("icons/desktop/record-placeholder.svg")
                .size(px((size * 0.58).max(14.0)))
                .text_color(rgb(ui_color(MUTED, DARK_MUTED))),
        )
}

fn collection_artwork_placeholder(size: f32, title: &str, subtitle: &str) -> Div {
    const COLORS: [u32; 8] = [
        0x31558f, 0x326b6f, 0x78527f, 0x8a5d3b, 0x3f6382, 0x645c8c, 0x476c55, 0x7a4d58,
    ];
    let hash = title
        .bytes()
        .chain(subtitle.bytes())
        .fold(2_166_136_261_u32, |value, byte| {
            (value ^ u32::from(byte)).wrapping_mul(16_777_619)
        });
    let initials = title
        .split_whitespace()
        .filter_map(|word| word.chars().find(|character| character.is_alphanumeric()))
        .take(2)
        .collect::<String>()
        .to_uppercase();
    div()
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(COLORS[hash as usize % COLORS.len()]))
        .text_color(rgb(0xf5f7fb))
        .text_size(px((size * 0.24).clamp(22.0, 48.0)))
        .font_weight(FontWeight::SEMIBOLD)
        .child(if initials.is_empty() {
            "?".to_string()
        } else {
            initials
        })
}

fn artist_portrait_placeholder(size: f32, name: &str, detail: &str) -> Div {
    div()
        .size(px(size))
        .rounded(px(size / 2.0))
        .overflow_hidden()
        .child(collection_artwork_placeholder(size, name, detail))
}

fn source_icon_path(section: DesktopSection) -> &'static str {
    match section {
        DesktopSection::Music => "icons/desktop/music.svg",
        DesktopSection::Albums => "icons/desktop/album.svg",
        DesktopSection::Artists => "icons/desktop/artist.svg",
        DesktopSection::Genres => "icons/desktop/genre.svg",
        DesktopSection::Songs => "icons/desktop/song.svg",
        DesktopSection::Favorites => "icons/desktop/favorite.svg",
        DesktopSection::Queue => "icons/desktop/queue.svg",
        DesktopSection::Playlists => "icons/desktop/playlist.svg",
        DesktopSection::Devices => "icons/desktop/device.svg",
        DesktopSection::Sync => "icons/desktop/sync.svg",
        DesktopSection::Updates => "icons/desktop/update.svg",
        DesktopSection::Logs => "icons/desktop/diagnostics.svg",
        DesktopSection::Settings => "icons/desktop/settings.svg",
    }
}

fn task_kind_icon(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::LibraryImport => "↘",
        TaskKind::LibraryScan | TaskKind::ReferenceScan => "⌕",
        TaskKind::SyncDryRun | TaskKind::Hashing | TaskKind::Verification => "◇",
        TaskKind::Transcoding => "≋",
        TaskKind::Upload => "↗",
        TaskKind::PlaylistComparison | TaskKind::PlaylistApply | TaskKind::PlaylistRollback => "☷",
        TaskKind::Backup | TaskKind::Restore => "▤",
        TaskKind::DiagnosticsExport | TaskKind::PerformanceSmoke => "⌁",
        TaskKind::Update => "↓",
        TaskKind::EmulatorStart | TaskKind::EmulatorStop | TaskKind::EmulatorRestart => "▯",
        TaskKind::MetadataLookup => "◎",
        TaskKind::ArtworkDownload => "▧",
        TaskKind::Fingerprinting => "≈",
        TaskKind::LibraryRelocation => "⇢",
        TaskKind::MetadataWrite => "✎",
    }
}

fn task_phase_label(phase: &musicos_desktop::models::TaskPhase) -> &'static str {
    use musicos_desktop::models::TaskPhase;
    match phase {
        TaskPhase::Queued => "Queued",
        TaskPhase::Discovering => "Discovering",
        TaskPhase::Processing => "Processing",
        TaskPhase::Hashing => "Hashing",
        TaskPhase::Transcoding => "Transcoding",
        TaskPhase::Uploading => "Uploading",
        TaskPhase::Verifying => "Verifying",
        TaskPhase::Applying => "Applying",
        TaskPhase::RollingBack => "Rolling back",
        TaskPhase::Starting => "Starting",
        TaskPhase::Stopping => "Stopping",
        TaskPhase::Finishing => "Finishing",
        TaskPhase::Custom(_) => "Working",
    }
}

fn activity_pulse() -> f32 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f32();
    0.32 + ((seconds * 3.2).sin() * 0.5 + 0.5) * 0.5
}

fn blend_color(from: u32, to: u32, amount: f32) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let a = ((from >> shift) & 0xff) as f32;
        let b = ((to >> shift) & 0xff) as f32;
        (a + (b - a) * amount).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn ui_color(light: u32, dark: u32) -> u32 {
    if DARK_APPEARANCE.load(Ordering::Relaxed) {
        dark
    } else {
        light
    }
}

fn format_duration(seconds: i32) -> String {
    let seconds = seconds.max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn total_duration(tracks: &[Track]) -> String {
    let total = tracks
        .iter()
        .map(|track| track.duration.max(0) as i64)
        .sum::<i64>();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    if hours > 0 {
        format!("{hours} hr {minutes} min")
    } else {
        format!("{minutes} min")
    }
}

fn main() {
    struct StderrLogger;
    impl log::Log for StderrLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::max_level()
        }
        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                eprintln!(
                    "[{:5}] {}: {}",
                    record.level(),
                    record.target(),
                    record.args()
                );
            }
        }
        fn flush(&self) {}
    }
    static LOGGER: StderrLogger = StderrLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(match std::env::var("RUST_LOG").as_deref() {
        Ok("trace") => log::LevelFilter::Trace,
        Ok("debug") => log::LevelFilter::Debug,
        Ok("warn") => log::LevelFilter::Warn,
        Ok("error") => log::LevelFilter::Error,
        Ok("off") => log::LevelFilter::Off,
        _ => log::LevelFilter::Info,
    });
    Application::new()
        .with_assets(DesktopAssets)
        .run(|cx: &mut App| {
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([
                KeyBinding::new("cmd-o", ImportMusic, None),
                KeyBinding::new("cmd-r", RefreshLibrary, None),
                KeyBinding::new("cmd-f", FocusSearch, None),
                KeyBinding::new("space", TogglePlayback, None),
                KeyBinding::new("cmd-left", PreviousTrack, None),
                KeyBinding::new("cmd-right", NextTrack, None),
                KeyBinding::new("cmd-1", ShowSongs, None),
                KeyBinding::new("cmd-2", ShowAlbums, None),
                KeyBinding::new("cmd-3", ShowArtists, None),
                KeyBinding::new("cmd-4", ShowFlow, None),
                KeyBinding::new("cmd-,", ShowPreferences, None),
                KeyBinding::new("alt-cmd-i", ShowDiagnostics, None),
                KeyBinding::new("alt-cmd-a", ToggleActivity, None),
                KeyBinding::new("cmd-n", NewPlaylist, None),
                KeyBinding::new("cmd-shift-s", SaveQueueAsPlaylist, None),
                KeyBinding::new("cmd-shift-e", ExportPlaylist, None),
                KeyBinding::new("cmd-shift-i", ImportPlaylist, None),
                KeyBinding::new("ctrl-cmd-f", ToggleFullscreen, None),
                KeyBinding::new("cmd-shift-m", ToggleMiniPlayer, None),
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("backspace", SearchBackspace, Some("MusicOSSearchInput")),
                KeyBinding::new("delete", SearchDelete, Some("MusicOSSearchInput")),
                KeyBinding::new("left", SearchLeft, Some("MusicOSSearchInput")),
                KeyBinding::new("right", SearchRight, Some("MusicOSSearchInput")),
                KeyBinding::new("shift-left", SearchSelectLeft, Some("MusicOSSearchInput")),
                KeyBinding::new("shift-right", SearchSelectRight, Some("MusicOSSearchInput")),
                KeyBinding::new("cmd-a", SearchSelectAll, Some("MusicOSSearchInput")),
                KeyBinding::new("cmd-v", SearchPaste, Some("MusicOSSearchInput")),
                KeyBinding::new("cmd-c", SearchCopy, Some("MusicOSSearchInput")),
                KeyBinding::new("cmd-x", SearchCut, Some("MusicOSSearchInput")),
                KeyBinding::new("home", SearchHome, Some("MusicOSSearchInput")),
                KeyBinding::new("end", SearchEnd, Some("MusicOSSearchInput")),
                KeyBinding::new("escape", ClearSearch, Some("MusicOSSearchInput")),
            ]);
            cx.set_menus(vec![
                Menu {
                    name: "MusicOS".into(),
                    items: vec![
                        MenuItem::action("Preferences…", ShowPreferences),
                        MenuItem::os_submenu("Services", SystemMenuType::Services),
                        MenuItem::separator(),
                        MenuItem::action("Quit MusicOS", Quit),
                    ],
                },
                Menu {
                    name: "File".into(),
                    items: vec![
                        MenuItem::action("Import Music…", ImportMusic),
                        MenuItem::action("Refresh Library", RefreshLibrary),
                    ],
                },
                Menu {
                    name: "Playback".into(),
                    items: vec![
                        MenuItem::action("Play or Pause", TogglePlayback),
                        MenuItem::action("Previous Track", PreviousTrack),
                        MenuItem::action("Next Track", NextTrack),
                    ],
                },
                Menu {
                    name: "View".into(),
                    items: vec![
                        MenuItem::action("Songs", ShowSongs),
                        MenuItem::action("Albums", ShowAlbums),
                        MenuItem::action("Artists", ShowArtists),
                        MenuItem::action("Flow", ShowFlow),
                        MenuItem::separator(),
                        MenuItem::action("Mini Player", ToggleMiniPlayer),
                        MenuItem::action("Fullscreen", ToggleFullscreen),
                        MenuItem::separator(),
                        MenuItem::action("Activity", ToggleActivity),
                        MenuItem::action("Diagnostics", ShowDiagnostics),
                    ],
                },
            ]);
            let _ = cx.text_system().add_fonts(vec![
                Cow::Borrowed(include_bytes!("../../../assets/fonts/Figtree-Variable.ttf")),
                Cow::Borrowed(include_bytes!(
                    "../../../assets/fonts/NotoSansSC-Regular.ttf"
                )),
                Cow::Borrowed(include_bytes!(
                    "../../../assets/fonts/RasterGothic12CondBold.ttf"
                )),
            ]);
            let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // The same native window collapses into the 456×164 compact player.
                    window_min_size: Some(size(px(720.0), px(480.0))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("MusicOS".into()),
                        appears_transparent: cfg!(target_os = "macos"),
                        traffic_light_position: if cfg!(target_os = "macos") {
                            Some(gpui::point(px(-200.0), px(-200.0)))
                        } else {
                            None
                        },
                    }),
                    app_id: Some("com.musicos.controlcenter".to_string()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(ControlCenter::new);
                    view.read(cx).focus_handle.focus(window);
                    view
                },
            )
            .expect("opening MusicOS Control Center");
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    #[test]
    fn album_grid_packs_square_artwork_responsively() {
        let minimum = album_grid_metrics(1100.0, 220.0, 190.0);
        let default = album_grid_metrics(1280.0, 220.0, 190.0);
        let wide = album_grid_metrics(1680.0, 220.0, 190.0);

        assert!(minimum.columns >= 4);
        assert!(default.columns > minimum.columns);
        assert!(wide.columns > default.columns);
        assert!(default.artwork_size <= default.card_width);
        assert!(default.card_width <= 190.0);
        assert!(default.row_height < 228.0);
    }

    #[gpui::test]
    fn virtualized_library_filters_ten_thousand_tracks(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| ControlCenter::test_fixture(cx, 10_000));
        window
            .update(cx, |view, _, cx| {
                assert_eq!(view.visible_tracks().len(), 10_000);
                view.search = "Song 09999".to_string();
                view.rebuild_library_index();
                assert_eq!(view.visible_tracks().len(), 1);
                view.set_view(LibraryPresentation::Grid, cx);
                cx.notify();
            })
            .unwrap();
    }

    #[gpui::test]
    fn every_source_destination_is_reachable(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| ControlCenter::test_fixture(cx, 4));
        window
            .update(cx, |view, _, cx| {
                for section in DesktopSection::ALL {
                    view.navigate(section, cx);
                    assert_eq!(view.section(), section);
                    assert!(!section.label().is_empty());
                }
                cx.notify();
            })
            .unwrap();
    }

    #[gpui::test]
    fn album_drilldown_scopes_visible_tracks(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| ControlCenter::test_fixture(cx, 10_000));
        window
            .update(cx, |view, _, cx| {
                view.navigate(DesktopSection::Albums, cx);
                let total = view.visible_tracks().len();
                assert_eq!(total, 10_000);
                let album = view.library_index.albums[0].clone();
                let expected = album.track_indices.len();
                assert!(expected > 0 && expected < total);
                view.open_album(album.id.clone(), cx);
                assert_eq!(view.visible_tracks().len(), expected);
                view.close_drilldown(cx);
                assert_eq!(view.visible_tracks().len(), total);
            })
            .unwrap();
    }

    #[gpui::test]
    fn destructive_actions_require_inline_confirmation(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| ControlCenter::test_fixture(cx, 4));
        window
            .update(cx, |view, _, cx| {
                assert!(view.pending_confirmation.is_none());
                view.request_database_reset(cx);
                assert!(matches!(
                    view.pending_confirmation,
                    Some(PendingConfirmation::ResetDatabase)
                ));
                assert!(view.render_confirmation(cx).is_some());
                view.cancel_confirmation(cx);
                assert!(view.pending_confirmation.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn mini_player_collapses_and_expands_the_existing_window(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, cx| ControlCenter::test_fixture(cx, 4));
        let expanded_size = window
            .update(cx, |view, window, cx| {
                let expanded_size = window.bounds().size;
                view.toggle_mini_player(window, cx);
                assert!(view.compact_player);
                assert_eq!(view.expanded_window_size, Some(expanded_size));
                expanded_size
            })
            .unwrap();

        window
            .update(cx, |view, window, cx| {
                view.toggle_mini_player(window, cx);
                assert!(!view.compact_player);
                assert!(view.expanded_window_size.is_none());
                assert_eq!(window.bounds().size, expanded_size);
            })
            .unwrap();
    }
}
