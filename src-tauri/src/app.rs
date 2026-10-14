use crate::{
    integrations::{
        IntegrationService, Request as IntegrationRequest, Response as IntegrationResponse,
        Status as IntegrationStatus,
    },
    library,
    preferences::{self, Preferences},
    providers,
};
use anyhow::{Context, Result};
use musicos_desktop::{
    api_client::ApiClient,
    local::{
        LibraryQuery, LibraryRepository, LocalLibraryRepository, LocalPlaybackCommand,
        LocalPlaybackService,
    },
    models::ControlCenterConfig,
    runtime_manager::RuntimeManager,
    storage,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, Manager, State};
#[derive(Debug, Serialize)]
pub struct AppError {
    code: String,
    message: String,
}
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self {
            code: "operation_failed".into(),
            message: format!("{e:#}"),
        }
    }
}
type ApiResult<T> = std::result::Result<T, AppError>;
#[derive(Debug, Clone, Default, Serialize)]
pub struct Task {
    pub running: bool,
    pub kind: String,
    pub processed: usize,
    pub total: usize,
    pub message: String,
    pub error: Option<String>,
    pub cancelled: bool,
}
pub struct AppState {
    analysis: crate::analysis::Analysis,
    integrations: IntegrationService,
    config: Mutex<ControlCenterConfig>,
    preferences: Mutex<Preferences>,
    preference_error: Option<String>,
    playback: Mutex<LocalPlaybackService>,
    runtime: Mutex<RuntimeManager>,
    cancel: AtomicBool,
    task: Mutex<Task>,
    artwork: Mutex<HashSet<PathBuf>>,
    album_artwork: Mutex<std::collections::HashMap<i64,Option<PathBuf>>>,
    search_generation: AtomicU64,
    stopped: AtomicBool,
    metadata_running: AtomicBool,
    artist_jobs: Mutex<std::collections::BTreeSet<String>>,
    artist_running: AtomicBool,
}
impl AppState {
    fn new() -> Result<Self> {
        let config = storage::load_config()?;
        let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
        repo.ensure_ready()?;
        crate::desktop_metadata::repair_automatic_albums(&config.db_path)?;
        let playback = LocalPlaybackService::start_with_preferences(
            config.db_path.clone(),
            config.playback.clone(),
        )?;
        let runtime = RuntimeManager::new(&config);
        let prefs = preferences::load();
        playback.send(LocalPlaybackCommand::SetDjMode(prefs.dj_config.enabled))?;
        let preference_error = if !preferences::path().exists() {
            preferences::save(&prefs)
                .err()
                .map(|e| format!("Could not save settings. {e:#}"))
        } else {
            None
        };
        let integrations = IntegrationService::start(playback.client())?;
        Ok(Self {
            analysis: Default::default(),
            integrations,
            config: Mutex::new(config),
            preferences: Mutex::new(prefs),
            preference_error,
            playback: Mutex::new(playback),
            runtime: Mutex::new(runtime),
            cancel: AtomicBool::new(false),
            task: Mutex::new(Task::default()),
            artwork: Mutex::new(HashSet::new()),
            album_artwork: Mutex::new(Default::default()),
            search_generation: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
            metadata_running: AtomicBool::new(false),
            artist_jobs: Mutex::new(Default::default()),
            artist_running: AtomicBool::new(false),
        })
    }
    fn config(&self) -> ControlCenterConfig {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
fn emit_task(app: &tauri::AppHandle, task: Task) {
    let state = app.state::<AppState>();
    *state.task.lock().unwrap_or_else(|e| e.into_inner()) = task.clone();
    let _ = app.emit("task", &task);
}
fn allow_art(app: &tauri::AppHandle, path: &str) {
    if let Ok(path) = Path::new(path).canonicalize() {
        if path.is_file() {
            let _ = app.asset_protocol_scope().allow_file(&path);
            app.state::<AppState>()
                .artwork
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(path);
        }
    }
}
fn queue_artist(app: &tauri::AppHandle, name: &str) {
    let state = app.state::<AppState>();
    if !providers::artist_due(name) {
        return;
    }
    state
        .artist_jobs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(name.into());
    if state
        .artist_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || loop {
        let state = app.state::<AppState>();
        if state.stopped.load(Ordering::Relaxed) {
            state.artist_running.store(false, Ordering::Relaxed);
            break;
        }
        let name = {
            let jobs = state.artist_jobs.lock().unwrap_or_else(|e| e.into_inner());
            let name = jobs.first().cloned();
            if name.is_none() {
                state.artist_running.store(false, Ordering::SeqCst);
            }
            name
        };
        let Some(name) = name else {
            break;
        };
        let config = state.config();
        let mut info = providers::revalidate_artist(&config.db_path, &name, &state.stopped)
            .unwrap_or_default();
        info.lookup_status = if info.artist_mbid.is_some() {
            "cached"
        } else {
            "unavailable"
        }
        .into();
        {
            if let Some(path) = &info.portrait {
                allow_art(&app, path);
            }
            // A portrait update is not a catalog change. Refreshing the entire
            // library here invalidated every cover for each background lookup.
            let _ = app.emit("artist-profile", json!({"name":name,"info":info}));
        }
        state
            .artist_jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&name);
    });
}
fn load_library(app: &tauri::AppHandle) -> Result<library::Library> {
    let state = app.state::<AppState>();
    let config = state.config();
    let mut snapshot = LocalLibraryRepository::new(&config.db_path, &config.media_dir)
        .snapshot(&LibraryQuery::default())?;
    // Older imports missed two-digit space-separated filename prefixes. Recover
    // display/play order without changing source tags or explicit user numbers.
    let numbered=rusqlite::Connection::open(&config.db_path)?.prepare("SELECT track_id FROM track_metadata_overrides WHERE origin='user' AND track_number IS NOT NULL")?.query_map([],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
    library::recover_filename_numbers(&mut snapshot.tracks,&numbered);
    let metadata = crate::desktop_metadata::cached_many(&config.db_path, &snapshot.tracks);
    for track in &mut snapshot.tracks {
        if let Some(m) = metadata.get(&track.id) {
            crate::desktop_metadata::apply(track, m.clone());
        }
    }
    if state
        .metadata_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_ok()
    {
        let app = app.clone();
        let tracks = snapshot
            .tracks
            .iter()
            .filter(|t| !metadata.contains_key(&t.id))
            .cloned()
            .collect::<Vec<_>>();
        let db = config.db_path.clone();
        std::thread::spawn(move || {
            let state = app.state::<AppState>();
            let mut changed = false;
            for track in tracks {
                if state.stopped.load(Ordering::Relaxed) {
                    break;
                }
                if crate::desktop_metadata::cached(&db, &track).is_none() {
                    match crate::desktop_metadata::ensure(&db, &track) {
                        Ok(_) => changed = true,
                        Err(e) => eprintln!("Metadata probe {}: {e:#}", track.id),
                    }
                }
            }
            state.metadata_running.store(false, Ordering::Relaxed);
            if changed {
                let _ = app.emit("library-changed", ());
            }
        });
    }
    let mut result = library::group(snapshot);
    library::canonical_artwork(&config.db_path,&mut result)?;
    for album in &mut result.albums {
        let types=album.track_ids.iter().filter_map(|id|metadata.get(id)?.release_type.clone()).collect::<std::collections::BTreeSet<_>>();
        if types.len()==1 {album.release_type=types.first().cloned();album.release_type_source=Some("file_tag".into());}
    }
    if let Ok(db)=rusqlite::Connection::open(&config.db_path){for album in &mut result.albums {
        if album.release_type.is_none(){if let Ok(kind)=db.query_row("SELECT kind FROM desktop_album_type_v1 WHERE album_id=?",[&album.id],|r|r.get::<_,String>(0)){album.release_type=Some(kind);album.release_type_source=Some("verified_release".into());}}
    }}
    // Per-song artwork can differ from the album's chosen cover. Register all
    // actual local artwork before the first frontend image request, avoiding a
    // cached asset-denied response followed by repeated extraction retries.
    for track in &result.tracks {
        if let Some(path) = &track.cover_art_path {
            allow_art(app, &path.to_string_lossy());
        }
    }
    for a in &mut result.artists {
        if providers::cached_artist_info(&a.name).is_none_or(|p|p.artist_mbid.is_none()||p.portrait.as_ref().is_none_or(|path|!Path::new(path).is_file())) && providers::artist_due(&a.name) {
            queue_artist(app, &a.name);
        }
        a.portrait = providers::cached_artist_info(&a.name).and_then(|info| info.portrait);
        if let Some(p) = &a.portrait {
            allow_art(app, p)
        }
    }
    for a in &mut result.albums {
        if a.cover.as_ref().is_some_and(|p| !Path::new(p).is_file()) {
            a.cover = None;
        }
        if let Some(p) = &a.cover {
            allow_art(app, p);
        }
    }
    *state.album_artwork.lock().unwrap_or_else(|e|e.into_inner())=result.tracks.iter().map(|t|(t.id,t.cover_art_path.clone())).collect();
    Ok(result)
}
fn playback_value(state: &AppState) -> Value {
    let s = state
        .playback
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .snapshot();
    json!({"track":s.track,"queue":s.queue.as_ref(),"queue_index":s.queue_index,"is_playing":s.is_playing,"position":s.position,"volume":s.volume,"shuffle":s.shuffle,"dj_mode":s.dj_mode,"queue_entry_ids":s.queue_entry_ids,"pinned_entries":s.pinned_entries,"arranged_entries":s.arranged_entries,"plan_revision":s.plan_revision,"dj_transition":s.dj_transition,"listening":s.listening,"listening_applied":s.listening_applied,"audio_levels":s.audio_levels,"repeat":s.repeat,"error":s.error,"backend":s.backend})
}
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T> + Send + 'static,
) -> ApiResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError {
            code: "worker_failed".into(),
            message: e.to_string(),
        })?
        .map_err(Into::into)
}
#[tauri::command]
async fn bootstrap(app: tauri::AppHandle) -> ApiResult<Value> {
    blocking(move || {
        let library = load_library(&app)?;
        let state = app.state::<AppState>();
        let preferences = state
            .preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let task = state.task.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Ok(json!({
            "library": library,
            "preference_error": state.preference_error,
            "preferences": preferences,
            "playback": playback_value(&state),
            "task": task,
        }))
    })
    .await
}
#[tauri::command]
async fn refresh_library(app: tauri::AppHandle) -> ApiResult<library::Library> {
    blocking(move || load_library(&app)).await
}
#[tauri::command]
fn save_preferences(state: State<AppState>, preferences: Preferences) -> ApiResult<Preferences> {
    preferences.validate().map_err(AppError::from)?;
    let normalized = preferences.normalize();
    let previous = state
        .preferences
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    preferences::save(&normalized).map_err(AppError::from)?;
    *state.preferences.lock().unwrap_or_else(|e| e.into_inner()) = normalized.clone();
    if previous.sparklist_beta && !normalized.sparklist_beta {let _=state.analysis.control("cancel");crate::spark_install::cancel();}
    if previous.listening != normalized.listening {state.playback.lock().unwrap_or_else(|e|e.into_inner()).send(LocalPlaybackCommand::SetListening(normalized.listening.clone())).map_err(AppError::from)?;}
    if previous.dj_config != normalized.dj_config {
        state
            .playback
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(LocalPlaybackCommand::SetDjMode(
                normalized.dj_config.enabled,
            ))
            .map_err(AppError::from)?;
    }
    Ok(normalized)
}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PlaybackAction {
    Play {
        ids: Vec<i64>,
        start_index: usize,
        shuffle: bool,
    },
    Toggle,
    Previous,
    Next,
    Seek {
        position: f32,
    },
    Volume {
        volume: f32,
    },
    Shuffle,
    Repeat,
    Move {
        from: usize,
        to: usize,
    },
    PlayNext { id: i64 },
    AddToQueue { id: i64, index:Option<usize> },
    ClearUpcoming,
    DjMode {
        enabled: bool,
    },
    DjOrder,
    Select {
        index: usize,
    },
    Remove {
        index: usize,
    },
}
fn sequence(ids: Vec<i64>, start: usize, shuffle: bool) -> Result<(Vec<i64>, usize)> {
    anyhow::ensure!(
        !ids.is_empty(),
        "There are no playable tracks in this selection."
    );
    anyhow::ensure!(
        start < ids.len(),
        "The selected track is no longer available."
    );
    let mut ids = ids;
    if shuffle {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u64;
        for i in (1..ids.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            ids.swap(i, (seed as usize) % (i + 1));
        }
        Ok((ids, 0))
    } else {
        Ok((ids, start))
    }
}
#[tauri::command]
async fn playback(app: tauri::AppHandle, action: PlaybackAction) -> ApiResult<Value> {
    blocking(move || {
        let state = app.state::<AppState>();
        let service = state.playback.lock().unwrap_or_else(|e| e.into_inner());
        let command = match action {
            PlaybackAction::Play {
                ids,
                start_index,
                shuffle,
            } => {
                let config = state.config();
                let db = musicos_init::database::Db::open(
                    config.db_path.to_str().context("Invalid library path")?,
                )?;
                for id in &ids {
                    let t = db
                        .get_track_by_id(*id)?
                        .context("A track is no longer in the library.")?;
                    anyhow::ensure!(
                        Path::new(&t.path).is_file(),
                        "{} is unavailable. Reconnect its folder and rescan.",
                        t.title
                    );
                }
                let (mut ids, start_index) = sequence(ids, start_index, shuffle)?;
                let dj_mode = service.snapshot().dj_mode;
                if dj_mode {
                    let tracks = ids
                        .iter()
                        .map(|id| {
                            db.get_track_by_id(*id)?
                                .context("A track is no longer in the library.")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let features = crate::analysis::cached_many(
                        &config.db_path,
                        &tracks
                            .iter()
                            .map(|t| (t.id, t.path.as_str()))
                            .collect::<Vec<_>>(),
                    );
                    let artists = tracks.iter().map(|t| t.artist.as_str()).collect::<Vec<_>>();
                    let order = dj_order(&features, &artists, start_index + 1, Some(start_index));
                    let original = ids.clone();
                    for (offset, index) in order.into_iter().enumerate() {
                        ids[start_index + 1 + offset] = original[index];
                    }
                }
                service.send(LocalPlaybackCommand::SetShuffle(shuffle && !dj_mode))?;
                LocalPlaybackCommand::PlayTracks { ids, start_index }
            }
            PlaybackAction::DjMode { enabled } => {
                let mut prefs = state.preferences.lock().unwrap_or_else(|e| e.into_inner());
                let mut next = prefs.clone();
                next.dj_config.enabled = enabled;
                preferences::save(&next)?;
                *prefs = next;
                LocalPlaybackCommand::SetDjMode(enabled)
            }
            PlaybackAction::DjOrder => {
                anyhow::ensure!(
                    order_dj_queue(&state, &service)? > 0,
                    "Analyze songs in this queue first to order by BPM and key."
                );
                drop(service);
                return Ok(playback_value(&state));
            }
            PlaybackAction::Toggle => LocalPlaybackCommand::PlayPause,
            PlaybackAction::Previous => LocalPlaybackCommand::Previous,
            PlaybackAction::Next => LocalPlaybackCommand::Next,
            PlaybackAction::Seek { position } => {
                anyhow::ensure!(
                    position.is_finite() && position >= 0.,
                    "Invalid playback position"
                );
                LocalPlaybackCommand::SeekAbsolute(position)
            }
            PlaybackAction::Volume { volume } => {
                anyhow::ensure!(volume.is_finite(), "Invalid volume");
                LocalPlaybackCommand::SetVolume(volume.clamp(0., 1.))
            }
            PlaybackAction::Shuffle => LocalPlaybackCommand::ToggleShuffle,
            PlaybackAction::Repeat => LocalPlaybackCommand::CycleRepeat,
            PlaybackAction::Select { index } => LocalPlaybackCommand::SelectQueueIndex(index),
            PlaybackAction::Remove { index } => LocalPlaybackCommand::RemoveQueueIndex(index),
            PlaybackAction::Move { from, to } => LocalPlaybackCommand::MoveQueueItem { from, to },
            PlaybackAction::PlayNext { id } => LocalPlaybackCommand::PlayNext(id),
            PlaybackAction::AddToQueue { id,index } => match index {Some(index)=>LocalPlaybackCommand::InsertQueue{id,index},None=>LocalPlaybackCommand::AddToQueue(id)},
            PlaybackAction::ClearUpcoming => LocalPlaybackCommand::ClearUpcoming,
        };
        service.send(command)?;
        drop(service);
        Ok(playback_value(&state))
    })
    .await
}
fn order_dj_queue(state: &AppState, service: &LocalPlaybackService) -> Result<usize> {
    let snapshot = service.snapshot();
    let config = state.config();
    let start = snapshot.queue_index.map(|i| i + 1).unwrap_or(0);
    let features = crate::analysis::cached_many(
        &config.db_path,
        &snapshot
            .queue
            .iter()
            .map(|t| (t.id, t.path.as_str()))
            .collect::<Vec<_>>(),
    );

    let analyzed = features
        .iter()
        .flatten()
        .filter(|f| f.bpm.is_some() || f.key.is_some())
        .count();
    if analyzed == 0 {
        return Ok(0);
    }
    let artists = snapshot
        .queue
        .iter()
        .map(|t| t.artist.as_str())
        .collect::<Vec<_>>();
    let order = dj_order(&features, &artists, start, snapshot.queue_index);
    let mut positions = (0..snapshot.queue.len()).collect::<Vec<_>>();
    for (offset, index) in order.into_iter().enumerate() {
        let from = positions.iter().position(|p| *p == index).unwrap();
        let to = start + offset;
        if from != to {
            service.send(LocalPlaybackCommand::MoveQueueItem { from, to })?;
            let token = positions.remove(from);
            positions.insert(to, token);
        }
    }
    Ok(analyzed)
}
fn dj_order(
    features: &[Option<crate::analysis::Features>],
    artists: &[&str],
    start: usize,
    mut previous: Option<usize>,
) -> Vec<usize> {
    let mut remaining = (start..features.len()).collect::<Vec<_>>();
    let mut order = Vec::new();
    while !remaining.is_empty() {
        let selected = remaining
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let score = |index: usize| {
                    crate::analysis::compatibility(
                        previous.and_then(|p| features[p].as_ref()),
                        features[index].as_ref(),
                    ) + if previous.is_some_and(|p| artists[p] == artists[index]) {
                        0.25
                    } else {
                        0.
                    }
                };
                score(**a).total_cmp(&score(**b))
            })
            .map(|(i, _)| i)
            .unwrap();
        let index = remaining.remove(selected);
        order.push(index);
        previous = Some(index);
    }
    order
}
#[tauri::command]
async fn sparklist(
    app: tauri::AppHandle,
    request: crate::spark_commands::Request,
) -> ApiResult<crate::spark_commands::State> {
    blocking(move || {
        let state = app.state::<AppState>();
        let config = state.config();
        let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
        let runtime = app
            .path()
            .resource_dir()?
            .join("runtime/libonnxruntime.1.31.0.dylib");
        let setup = crate::spark_commands::Setup::for_library(&config.db_path, runtime,state.preferences.lock().unwrap_or_else(|e|e.into_inner()).sparklist_beta);
        if matches!(request,crate::spark_commands::Request::RemoveModel) {let _=state.analysis.control("cancel");}
        let mut value=crate::spark_commands::execute(&repo, &setup, request)?;let analysis=state.analysis.progress();value.analysis=crate::spark_commands::Activity{running:analysis.running,paused:analysis.paused,processed:analysis.processed,total:analysis.total};Ok(value)
    })
    .await
}
#[tauri::command]
async fn audio_analysis(
    app: tauri::AppHandle,
    action: String,
    ids: Option<Vec<i64>>,
) -> ApiResult<crate::analysis::Status> {
    blocking(move || {
        let state = app.state::<AppState>();
        let config = state.config();
        if action == "analyze_seed" && state.analysis.snapshot().running {
            let repo=LocalLibraryRepository::new(&config.db_path,&config.media_dir);
            let tracks=repo.snapshot(&LibraryQuery::default())?.tracks.into_iter().filter(|t|ids.as_ref().is_some_and(|ids|ids.contains(&t.id))).map(|t|(t.id,t.path,t.duration as i64)).collect();
            let setup=crate::spark_commands::Setup::for_library(&config.db_path,app.path().resource_dir()?.join("runtime/libonnxruntime.1.31.0.dylib"),state.preferences.lock().unwrap_or_else(|e|e.into_inner()).sparklist_beta);
            let model=setup.evaluation.then_some((setup.dir,setup.runtime,setup.evaluation));
            if state.analysis.request_seeds(tracks,model){return Ok(state.analysis.snapshot());}
        }
        if matches!(action.as_str(),"start"|"analyze_seed"|"rescan"|"rescan_all") {
            if action=="analyze_seed" {state.analysis.prioritize_seed(ids.clone().unwrap_or_default());}
            let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
            let snapshot = repo.snapshot(&LibraryQuery::default())?;
            let mut tracks = snapshot
                .tracks
                .into_iter()
                .filter(|t| Path::new(&t.path).is_file())
                .filter(|t| action=="analyze_seed" || ids.as_ref().is_none_or(|ids| ids.contains(&t.id)))
                .collect::<Vec<_>>();
            if action=="rescan" {tracks.retain(|t|crate::analysis::cached(&config.db_path,t.id,&t.path).is_none()||crate::desktop_metadata::cached(&config.db_path,t).is_none_or(|m|m.probe_error.is_some()));}
            if let Some(ids) = &ids {
                tracks.sort_by_key(|t| ids.iter().position(|id| *id == t.id).unwrap_or(usize::MAX));
            }
            let setup = crate::spark_commands::Setup::for_library(
                &config.db_path,
                app.path()
                    .resource_dir()?
                    .join("runtime/libonnxruntime.1.31.0.dylib"),
                state.preferences.lock().unwrap_or_else(|e|e.into_inner()).sparklist_beta,
            );
            let model = (setup.evaluation && setup.dir.join("manifest.json").is_file()).then_some((
                setup.dir,
                setup.runtime,
                setup.evaluation,
            ));
            let tracks=tracks.into_iter().map(|t|(t.id,t.path,t.duration as i64)).collect();
            if action.starts_with("rescan") {
                state.analysis.rescan(config.db_path.clone(),tracks,model,action=="rescan_all")?;
                let notify=app.clone();let worker=state.analysis.clone();
                std::thread::spawn(move||{while worker.progress().running{std::thread::sleep(Duration::from_millis(250));}let _=notify.emit("audio-rescanned",());let _=notify.emit("library-changed",());});
            } else {state.analysis.start_with_model(
                config.db_path.clone(),
                tracks,
                model,
            )?;}
        } else if action != "status" {
            state.analysis.control(&action)?;
        }
        let mut status = state.analysis.snapshot();
        {
            // Coverage is persisted cache state, not the current worker batch.
            let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
            status.features = crate::analysis::cached_many(
                &config.db_path,
                &repo
                    .snapshot(&LibraryQuery::default())?
                    .tracks
                    .iter()
                    .map(|t| (t.id, t.path.as_str()))
                    .collect::<Vec<_>>(),
            )
            .into_iter()
            .flatten()
            .collect();
        }
        Ok(status)
    })
    .await
}
#[derive(Debug, Deserialize)]
struct TaskRequest {
    kind: String,
    #[serde(default)]
    paths: Vec<PathBuf>,
    root: Option<PathBuf>,
}
fn begin_task(state: &AppState, kind: &str) -> Result<()> {
    let mut task = state.task.lock().unwrap_or_else(|e| e.into_inner());
    anyhow::ensure!(
        !task.running,
        "Wait for the current task to finish, or cancel it first."
    );
    state.cancel.store(false, Ordering::Relaxed);
    *task = Task {
        running: true,
        kind: kind.into(),
        message: "Starting…".into(),
        ..Default::default()
    };
    Ok(())
}
#[tauri::command]
async fn inspect_import(paths: Vec<PathBuf>) -> ApiResult<bool> {
    blocking(move || {
        anyhow::ensure!(!paths.is_empty(), "Drop music files or folders.");
        for path in &paths {
            anyhow::ensure!(path.exists(), "{} is unavailable.", path.display());
        }
        Ok(paths.iter().all(|p| p.is_dir()))
    })
    .await
}
#[tauri::command]
fn library_task(app: tauri::AppHandle, request: TaskRequest) -> ApiResult<()> {
    if !["reference", "copy", "rescan", "enrich", "sync"].contains(&request.kind.as_str()) {
        return Err(AppError {
            code: "invalid_task".into(),
            message: "Unknown task".into(),
        });
    }
    begin_task(&app.state::<AppState>(), &request.kind).map_err(AppError::from)?;
    let initial = app
        .state::<AppState>()
        .task
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let _ = app.emit("task", initial);
    let task_app = app.clone();
    std::thread::spawn(move || {
        let app = task_app;
        let state = app.state::<AppState>();
        let config = state.config();
        let kind = request.kind.clone();
        let report = (|| -> Result<String> {
            let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
            if kind == "sync" {
                let root = request.root.as_deref().unwrap_or(&config.media_dir);
                let target = ApiClient::new(&config.device_target)?;
                let profile = config
                    .format_profiles
                    .iter()
                    .find(|p| p.id == config.selected_sync_profile)
                    .or_else(|| config.format_profiles.first())
                    .context("No sync format configured")?;
                let p = musicos_desktop::sync::sync_files(
                    root,
                    None,
                    &target,
                    &profile.to_transcode_options(),
                    &state.cancel,
                    |p| {
                        emit_task(
                            &app,
                            Task {
                                running: true,
                                kind: kind.clone(),
                                processed: p.uploaded + p.skipped + p.failed,
                                total: p.found,
                                message: p.current.clone(),
                                error: None,
                                cancelled: p.cancel_requested,
                            },
                        )
                    },
                )?;
                storage::append_sync_history(&p)?;
                anyhow::ensure!(
                    p.failed == 0,
                    "{} file transfers failed: {}",
                    p.failed,
                    p.errors.join("; ")
                );
                return Ok(format!(
                    "{} files transferred; {} unchanged.",
                    p.uploaded, p.skipped
                ));
            }
            if kind == "enrich" {
                let tracks = repo.snapshot(&LibraryQuery::default())?.tracks;
                let ids: Vec<_> = tracks.iter().map(|t| t.id).collect();
                let p = musicos_desktop::local::enrichment::find_metadata_suggestions(
                    &config.db_path,
                    &ids,
                    &config.enrichment,
                    &config.artwork,
                    &state.cancel,
                    true,
                    |p| {
                        emit_task(
                            &app,
                            Task {
                                running: true,
                                kind: kind.clone(),
                                processed: p.processed,
                                total: p.total,
                                message: p.current,
                                ..Default::default()
                            },
                        )
                    },
                )?;
                return Ok(format!(
                    "{} updated; {} need review; {} unmatched; {} failed.",
                    p.applied, p.review, p.unmatched, p.failed
                ));
            }
            if kind == "reference" {
                anyhow::ensure!(!request.paths.is_empty(), "Choose a folder.");
                for path in &request.paths {
                    if state.cancel.load(Ordering::Relaxed) {
                        return Ok("Cancelled.".into());
                    }
                    repo.add_reference_source(path)?;
                }
            }
            if kind == "copy" {
                let p = musicos_desktop::library_import::import_managed_library(
                    &request.paths,
                    &config.media_dir,
                    &state.cancel,
                    |p| {
                        emit_task(
                            &app,
                            Task {
                                running: true,
                                kind: kind.clone(),
                                processed: p.processed,
                                total: p.found,
                                message: p.current.clone(),
                                ..Default::default()
                            },
                        )
                    },
                )?;
                anyhow::ensure!(
                    p.failed == 0,
                    "{} files could not be imported: {}",
                    p.failed,
                    p.errors.join("; ")
                );
            }
            let p = repo.scan(musicos_init::media::ScanMode::Changed, &state.cancel, |p| {
                emit_task(
                    &app,
                    Task {
                        running: true,
                        kind: kind.clone(),
                        processed: p.processed,
                        total: p.discovered,
                        message: p.current_root.unwrap_or_else(|| "Indexing music…".into()),
                        ..Default::default()
                    },
                )
            })?;
            if config.enrichment.enabled && !state.cancel.load(Ordering::Relaxed) {
                let ids=repo.snapshot(&LibraryQuery::default())?.tracks.iter().map(|t|t.id).collect::<Vec<_>>();
                let _=musicos_desktop::local::enrichment::find_metadata_suggestions(&config.db_path,&ids,&config.enrichment,&config.artwork,&state.cancel,false,|progress|emit_task(&app,Task{running:true,kind:kind.clone(),processed:progress.processed,total:progress.total,message:format!("Finding suggestions · {}",progress.current),..Default::default()}));
            }
            Ok(format!("{} files indexed. Metadata suggestions are ready for review.", p.indexed))
        })();
        let cancelled = state.cancel.load(Ordering::Relaxed);
        let (message, error) = match report {
            Ok(m) => (m, None),
            Err(e) => ("Task failed".into(), Some(format!("{e:#}"))),
        };
        emit_task(
            &app,
            Task {
                kind,
                running: false,
                message,
                error,
                cancelled,
                ..Default::default()
            },
        );
        let _ = app.emit("library-changed", ());
    });
    Ok(())
}
#[tauri::command]
fn cancel_task(state: State<AppState>) {
    state.cancel.store(true, Ordering::Relaxed)
}
#[tauri::command]
async fn metadata_edit(
    app: tauri::AppHandle,
    ids: Vec<i64>,
    patch: crate::metadata::MetadataPatch,
) -> ApiResult<usize> {
    blocking(move || {
        let changed =
            crate::metadata::edit(&app.state::<AppState>().config().db_path, &ids, patch)?;
        let _ = app.emit("library-changed", ());
        Ok(changed)
    })
    .await
}
#[tauri::command]
async fn metadata_workspace(app:tauri::AppHandle,request:crate::metadata::WorkspaceRequest)->ApiResult<Value>{
    blocking(move||{let mutating=matches!(request.action.as_str(),"apply"|"undo");let mut result=crate::metadata::workspace(&app.state::<AppState>().config().db_path,request)?;
    if let Some(items)=result.get_mut("items").and_then(Value::as_array_mut){for item in items {
        if let Some(id)=item["track_id"].as_i64(){if let Some(Some(path))=app.state::<AppState>().album_artwork.lock().unwrap_or_else(|e|e.into_inner()).get(&id){item["current_artwork"]=path.to_string_lossy().into_owned().into();}}
        for path in [item["current_artwork"].as_str(),item["candidate"]["artwork_path"].as_str()].into_iter().flatten(){allow_art(&app,path);}
    }}
    if mutating{let _=app.emit("library-changed",());}Ok(result)}).await
}
#[tauri::command]
async fn metadata_review(
    app: tauri::AppHandle,
    track_id: Option<i64>,
    accept: Option<bool>,
) -> ApiResult<Value> {
    blocking(move || {
        let config = app.state::<AppState>().config();
        if let Some(id) = track_id {
            if accept.unwrap_or(false) {
                musicos_desktop::local::accept_metadata_review(&config.db_path, id)?;
            } else {
                musicos_desktop::local::dismiss_metadata_review(&config.db_path, id)?;
            }
        }
        Ok(serde_json::to_value(
            musicos_desktop::local::load_metadata_review_items(&config.db_path, 200)?,
        )?)
    })
    .await
}
#[tauri::command]
async fn collections(
    app: tauri::AppHandle,
    request: Option<crate::collections::Request>,
) -> ApiResult<Vec<crate::collections::Collection>> {
    blocking(move || {
        let config = app.state::<AppState>().config();
        let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
        let items = if let Some(request) = request {
            crate::collections::change(&repo, request)?
        } else {
            crate::collections::list(&repo)?
        };
        for item in &items {
            if let Some(path) = &item.artwork {
                allow_art(&app, path);
            }
        }
        Ok(items)
    })
    .await
}
#[tauri::command]
async fn playlist_file(
    app: tauri::AppHandle,
    action: String,
    id: Option<String>,
    path: String,
) -> ApiResult<Value> {
    blocking(move || {
        let config = app.state::<AppState>().config();
        let repo = LocalLibraryRepository::new(&config.db_path, &config.media_dir);
        let result =
            crate::collections::file_action(&repo, &action, id.as_deref(), Path::new(&path))?;
        for item in crate::collections::list(&repo)? {
            if let Some(path) = item.artwork {
                allow_art(&app, &path);
            }
        }
        Ok(result)
    })
    .await
}
#[tauri::command]
async fn toggle_favorite(app: tauri::AppHandle, track_id: i64) -> ApiResult<bool> {
    blocking(move || {
        let config = app.state::<AppState>().config();
        let db = musicos_init::database::Db::open(&config.db_path.to_string_lossy())?;
        let value = db.toggle_favorite(track_id)?;
        let _ = app.emit("library-changed", ());
        Ok(value)
    })
    .await
}
#[tauri::command]
async fn album_info(app: tauri::AppHandle, album_id: String) -> ApiResult<providers::AlbumInfo> {
    blocking(move || {
        let state = app.state::<AppState>();
        let data = load_library(&app)?;
        let album = data
            .albums
            .iter()
            .find(|a| a.id == album_id || a.legacy_ids.contains(&album_id))
            .ok_or_else(|| anyhow::anyhow!("Album is no longer in the library"))?;
        let info=providers::album_info(&state.config().db_path, album, &state.stopped)?;
        if let Some(kind)=&info.release_type {
            let db=rusqlite::Connection::open(&state.config().db_path)?;db.execute_batch("CREATE TABLE IF NOT EXISTS desktop_album_type_v1(album_id TEXT PRIMARY KEY,kind TEXT NOT NULL)")?;
            let old=db.query_row("SELECT kind FROM desktop_album_type_v1 WHERE album_id=?",[&album.id],|r|r.get::<_,String>(0)).ok();
            if old.as_ref()!=Some(kind){db.execute("INSERT INTO desktop_album_type_v1 VALUES(?,?) ON CONFLICT(album_id) DO UPDATE SET kind=excluded.kind",rusqlite::params![album.id,kind])?;let _=app.emit("library-changed",());}
        }
        Ok(info)
    })
    .await
}
#[tauri::command]
async fn artist_info(
    app: tauri::AppHandle,
    name: String,
    refresh: Option<bool>,
) -> ApiResult<providers::ArtistInfo> {
    blocking(move || {
        anyhow::ensure!(name.chars().count() <= 200, "Artist name is too long");
        let pending = providers::artist_due(&name)
            || app
                .state::<AppState>()
                .artist_jobs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains(&name);
        if refresh.unwrap_or(false) || providers::cached_artist_info(&name).is_none() {
            queue_artist(&app, &name);
        }
        let info = providers::cached_artist_info(&name).unwrap_or_else(|| providers::ArtistInfo {
            lookup_status: if pending { "loading" } else { "unavailable" }.into(),
            ..Default::default()
        });
        if let Some(p) = &info.portrait {
            allow_art(&app, p)
        }
        Ok(info)
    })
    .await
}
#[derive(Serialize)]
struct MetadataStatus {
    cached: usize,
    checking: usize,
    unavailable: usize,
    retryable: usize,
}
#[tauri::command]
async fn metadata_status(app: tauri::AppHandle, retry: Option<bool>) -> ApiResult<MetadataStatus> {
    blocking(move || {
        let state = app.state::<AppState>();
        let config = state.config();
        let snapshot = LocalLibraryRepository::new(&config.db_path, &config.media_dir)
            .snapshot(&LibraryQuery::default())?;
        let library = library::group(snapshot);
        let mut result = MetadataStatus { cached:0, checking:0, unavailable:0, retryable:0 };
        for artist in &library.artists {
            let cached = providers::cached_artist_info(&artist.name)
                .is_some_and(|info| info.artist_mbid.is_some() || info.biography.is_some() || info.portrait.is_some());
            if retry.unwrap_or(false) && !cached { queue_artist(&app, &artist.name); }
            let checking = state.artist_jobs.lock().unwrap_or_else(|e|e.into_inner()).contains(&artist.name);
            if cached { result.cached += 1; }
            if checking { result.checking += 1; }
            else if !cached {
                result.unavailable += 1;
                if providers::artist_due(&artist.name) { result.retryable += 1; }
            }
        }
        Ok(result)
    }).await
}
#[tauri::command]
async fn library_source(app: tauri::AppHandle, id: i64, enabled: bool) -> ApiResult<()> {
    blocking(move || {
        let state = app.state::<AppState>();
        anyhow::ensure!(
            !state.task.lock().unwrap_or_else(|e| e.into_inner()).running,
            "Wait for the current library task to finish"
        );
        let config = state.config();
        let db = musicos_init::database::Db::open(&config.db_path.to_string_lossy())?;
        let source = db
            .library_sources()?
            .into_iter()
            .find(|s| s.id == id)
            .context("Folder no longer exists")?;
        anyhow::ensure!(
            source.kind != "managed",
            "The managed MusicOS library stays available"
        );
        LocalLibraryRepository::new(&config.db_path, &config.media_dir)
            .update_source(id, enabled)?;
        let _ = app.emit("library-changed", ());
        Ok(())
    })
    .await
}
#[tauri::command]
async fn audio_specs(app: tauri::AppHandle, track_id: i64) -> ApiResult<Value> {
    blocking(move || {
        let config = app.state::<AppState>().config();
        let db = musicos_init::database::Db::open(&config.db_path.to_string_lossy())?;
        let track = musicos_desktop::local::library::track_from_init(
            db.get_track_by_id(track_id)?
                .context("Song no longer exists")?,
        );
        Ok(serde_json::to_value(
            crate::desktop_metadata::ensure(&config.db_path, &track)?
                .quality
                .context("Audio specifications unavailable")?,
        )?)
    })
    .await
}
#[tauri::command]
async fn discover(
    app: tauri::AppHandle,
    term: String,
    kind: String,
    lookup: Option<String>,
) -> ApiResult<Vec<providers::CatalogItem>> {
    let token = app
        .state::<AppState>()
        .search_generation
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    providers::discover(
        &term,
        &kind,
        lookup.as_deref(),
        &app.state::<AppState>().search_generation,
        token,
    )
    .await
    .map_err(Into::into)
}
#[tauri::command]
fn cancel_search(state: State<AppState>) {
    state.search_generation.fetch_add(1, Ordering::Relaxed);
}
#[tauri::command]
async fn library_artwork(app: tauri::AppHandle, track_id: i64, per_track: Option<bool>) -> ApiResult<Option<String>> {
    blocking(move || {
        let config = app.state::<AppState>().config();
        let db = musicos_init::database::Db::open(
            config.db_path.to_str().context("Invalid library path")?,
        )?;
        let Some(_track) = db.get_track_by_id(track_id)? else {
            return Ok(None);
        };
        let _=per_track;
        let grouped=load_library(&app)?;
        let Some(album)=grouped.albums.iter().find(|a|a.track_ids.contains(&track_id)) else{return Ok(None)};
        if let Some(path)=&album.cover {allow_art(&app,path);return Ok(Some(path.clone()));}
        for id in &album.track_ids {
            if let Some(path)=musicos_desktop::local::artwork::ensure_artwork(config.db_path.clone(),*id)? {
                let path=path.to_string_lossy().into_owned();allow_art(&app,&path);
                let _=app.emit("artwork-found",json!({"track_id":track_id,"path":path}));
                let _=app.emit("library-changed",());return Ok(Some(path));
            }
        }
        Ok(None)
    })
    .await
}
#[tauri::command]
async fn cache_artwork(app: tauri::AppHandle, url: String) -> ApiResult<String> {
    blocking(move || {
        let path = providers::cache_artwork(&url)?
            .to_string_lossy()
            .into_owned();
        allow_art(&app, &path);
        Ok(path)
    })
    .await
}
#[tauri::command]
fn dj_diagnostics()->Vec<musicos_desktop::local::dj_diagnostics::Record>{musicos_desktop::local::dj_diagnostics::snapshot()}
#[tauri::command]
async fn artwork_performance(sample:crate::depth::FrameMetrics)->ApiResult<()>{blocking(move||crate::depth::record_metrics(sample)).await}
#[tauri::command]
async fn depth_setup(action:String)->ApiResult<crate::depth::Status>{blocking(move||crate::depth::action(&action)).await}
#[tauri::command]
async fn depth_artwork(app:tauri::AppHandle,path:String)->ApiResult<Option<crate::depth::Artwork>>{
 blocking(move||{let state=app.state::<AppState>();anyhow::ensure!(state.preferences.lock().unwrap_or_else(|e|e.into_inner()).artwork_depth,"Enable Depth artwork in Appearance first");
 let source=Path::new(&path).canonicalize()?;anyhow::ensure!(state.artwork.lock().unwrap_or_else(|e|e.into_inner()).contains(&source),"Depth source is not registered artwork");
 let (map,fresh)=crate::depth::reported(&source)?;map.map(|map|{let mut result=crate::depth::textures(&source,&map)?;result.fresh=fresh;Ok(result)}).transpose()}).await
}
#[tauri::command]
async fn portrait_artwork(app:tauri::AppHandle,path:String)->ApiResult<String>{
 blocking(move||{
  let source=Path::new(&path).canonicalize()?;
  anyhow::ensure!(app.state::<AppState>().artwork.lock().unwrap_or_else(|e|e.into_inner()).contains(&source),"Portrait is not registered artwork");
  let framed=crate::portrait::crop(&source)?.to_string_lossy().into_owned();allow_art(&app,&framed);Ok(framed)
 }).await
}
#[tauri::command]
async fn artwork_palette(app: tauri::AppHandle, path: String) -> ApiResult<providers::Palette> {
    blocking(move || {
        let canonical = Path::new(&path).canonicalize()?;
        anyhow::ensure!(
            app.state::<AppState>()
                .artwork
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains(&canonical),
            "Artwork is not in the library cache."
        );
        providers::palette(&canonical)
    })
    .await
}
#[tauri::command]
async fn device(
    app: tauri::AppHandle,
    action: String,
    target: Option<String>,
    root: Option<PathBuf>,
) -> ApiResult<Value> {
    blocking(move || {
        let state = app.state::<AppState>();
        if let Some(target) = target {
            let url = url::Url::parse(&target)?;
            anyhow::ensure!(["http", "https"].contains(&url.scheme()) && url.host_str().is_some(), "Enter an HTTP device address.");
            let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
            config.device_target = target;
            storage::save_config(&config)?;
        }
        match action.as_str() {
            "status" => {
                let config = state.config();
                let response = ApiClient::with_timeout(&config.device_target, Duration::from_secs(5))?.status();
                let preview = state.runtime.lock().unwrap_or_else(|e| e.into_inner()).refresh();
                Ok(json!({"target": config.device_target, "connected": response.is_ok(), "status": response.as_ref().ok(), "error": response.err().map(|e| e.to_string()), "preview": preview}))
            }
            "preview_start" => Ok(serde_json::to_value(state.runtime.lock().unwrap_or_else(|e| e.into_inner()).start()?)?),
            "preview_stop" => Ok(serde_json::to_value(state.runtime.lock().unwrap_or_else(|e| e.into_inner()).stop()?)?),
            "plan" => {
                let config = state.config();
                let root = root.unwrap_or(config.media_dir);
                let files = musicos_desktop::sync::resolve_sync_files(&root, None)?;
                let bytes: u64 = files.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum();
                Ok(json!({"root": root, "files": files.len(), "bytes": bytes, "target": config.device_target, "profile": config.selected_sync_profile}))
            }
            _ => anyhow::bail!("Unknown device operation"),
        }
    }).await
}
// Keep the OS controls centered in the actual rendered header at every scale.
#[tauri::command]
fn sync_window_chrome(window: tauri::WebviewWindow, center_y: f64) -> ApiResult<()> {
    if !center_y.is_finite() || !(16.0..=150.0).contains(&center_y) {
        return Err(AppError {
            code: "invalid_chrome".into(),
            message: "Invalid titlebar geometry.".into(),
        });
    }
    #[cfg(target_os = "macos")]
    window
        .with_webview(move |webview| {
            use objc2_app_kit::{NSWindow, NSWindowButton};
            use objc2_foundation::NSPoint;
            // Tauri guarantees this callback runs on the UI thread, with a live NSWindow.
            let native: &NSWindow = unsafe { &*webview.ns_window().cast() };
            let Some(close) = native.standardWindowButton(NSWindowButton::CloseButton) else {
                return;
            };
            let Some(mini) = native.standardWindowButton(NSWindowButton::MiniaturizeButton) else {
                return;
            };
            let Some(zoom) = native.standardWindowButton(NSWindowButton::ZoomButton) else {
                return;
            };
            let Some(parent) = (unsafe { close.superview().and_then(|v| v.superview()) }) else {
                return;
            };
            let rect = close.frame();
            let height = center_y + rect.origin.y + rect.size.height / 2.0;
            let mut container = parent.frame();
            container.size.height = height;
            container.origin.y = native.frame().size.height - height;
            parent.setFrame(container);
            let gap = mini.frame().origin.x - rect.origin.x;
            for (index, button) in [close, mini, zoom].into_iter().enumerate() {
                button.setFrameOrigin(NSPoint::new(18.0 + index as f64 * gap, rect.origin.y));
            }
        })
        .map_err(|e| AppError::from(anyhow::Error::from(e)))?;
    #[cfg(not(target_os = "macos"))]
    let _ = window;
    Ok(())
}
#[tauri::command]
async fn integration_status(state: State<'_, AppState>) -> ApiResult<IntegrationStatus> {
    let service = state.integrations.clone();
    blocking(move || Ok(service.status())).await
}
#[tauri::command]
async fn integration_action(
    state: State<'_, AppState>,
    request: IntegrationRequest,
) -> ApiResult<IntegrationResponse> {
    let service = state.integrations.clone();
    blocking(move || service.action(request)).await
}
#[tauri::command]
fn open_provider(url: String) -> ApiResult<()> {
    let u = url::Url::parse(&url).map_err(|e| AppError::from(anyhow::Error::from(e)))?;
    let host = u.host_str().unwrap_or("");
    if u.scheme() != "https"
        || !(host == "music.apple.com"
            || host == "itunes.apple.com"
            || host.ends_with(".wikipedia.org")
            || host == "commons.wikimedia.org"
            || host == "musicbrainz.org"
            || (host == "huggingface.co" && u.path().starts_with("/apple/coreml-depth-anything-v2-small"))
            || host == "www.last.fm"
            || host == "last.fm"
            || (host == "discord.com" && u.path().starts_with("/developers/applications")))
    {
        return Err(AppError {
            code: "invalid_link".into(),
            message: "This provider link is not supported.".into(),
        });
    }
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg(&url).status();
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", &url])
        .status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = std::process::Command::new("xdg-open").arg(&url).status();
    let status = status.map_err(|e| AppError::from(anyhow::Error::from(e)))?;
    if !status.success() {
        return Err(AppError {
            code: "open_failed".into(),
            message: "Could not open the provider link.".into(),
        });
    }
    Ok(())
}
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            audio_analysis, playlist_file, sparklist,
            bootstrap, refresh_library, save_preferences, playback, library_task,
            cancel_task, inspect_import, metadata_review, metadata_workspace, metadata_edit, collections, toggle_favorite, album_info, artist_info, metadata_status, library_source, audio_specs, discover, cancel_search,
            artwork_palette, cache_artwork, portrait_artwork, depth_setup, depth_artwork, dj_diagnostics, artwork_performance, library_artwork, device, open_provider, integration_status, integration_action, sync_window_chrome
        ])
        .setup(|app| {
            #[cfg(target_os="macos")]
            if let Some(window)=app.get_webview_window("main") {
                let _=window.with_webview(|view| {
                    let enabled=unsafe{musicos_desktop::local::decoded_audio::apple::prefer_native_refresh(view.inner().cast())};
                    eprintln!("artwork rendering: native refresh preference {}",if enabled{"applied"}else{"unavailable; platform pacing retained"});
                });
            }

            let state = AppState::new()?;
            let client = state.playback.lock().unwrap_or_else(|e|e.into_inner()).client();
            client.send(LocalPlaybackCommand::SetListening(state.preferences.lock().unwrap_or_else(|e|e.into_inner()).listening.clone()))?;
            #[cfg(target_os="macos")]
            crate::system_media::register(client.clone());
            app.manage(state);
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_queue_revision = u64::MAX;
                let mut last_value = Value::Null;
                let mut planned=None;
                let mut arranged=None;
                let mut analysis_requested=std::collections::BTreeMap::<i64,std::time::Instant>::new();
                let mut background_scan=std::time::Instant::now();
                let mut media_update=std::time::Instant::now()-Duration::from_secs(1);
                loop {
                    if handle.state::<AppState>().stopped.load(Ordering::Relaxed) { break; }
                    let s = client.snapshot();
                    #[cfg(target_os="macos")]
                    if media_update.elapsed()>=Duration::from_millis(250) {
                        media_update=std::time::Instant::now();let mut snapshot=s.clone();
                        if let Some(track)=snapshot.track.as_mut(){if let Some(cover)=handle.state::<AppState>().album_artwork.lock().unwrap_or_else(|e|e.into_inner()).get(&track.id){track.cover_art_path=cover.clone();}}
                        let _=handle.run_on_main_thread(move||crate::system_media::publish(&snapshot));
                    }
                    let state=handle.state::<AppState>();
                    let analysis_progress=state.analysis.progress();
                    let mut config=state.preferences.lock().unwrap_or_else(|e|e.into_inner()).dj_config.clone();config.enabled=s.dj_mode;
                    if s.dj_mode {if let Some(current)=s.queue_index {
                     let db=state.config().db_path;let upcoming=s.queue.iter().skip(current).take(5).collect::<Vec<_>>();state.analysis.prioritize(upcoming.iter().map(|t|t.id).collect());
                     if state.analysis.auto_allowed()&&!analysis_progress.running {analysis_requested.retain(|_,time|time.elapsed()<Duration::from_secs(60));let missing=upcoming.iter().filter(|t|!analysis_requested.contains_key(&t.id)&&crate::analysis::cached(&db,t.id,&t.path).is_none()).map(|t|(t.id,t.path.clone(),t.duration as i64)).collect::<Vec<_>>();if !missing.is_empty(){analysis_requested.extend(missing.iter().map(|t|(t.0,std::time::Instant::now())));let _=state.analysis.start(db.clone(),missing);}else if background_scan.elapsed()>=Duration::from_secs(30){
                        background_scan=std::time::Instant::now();let cfg=state.config();if let Ok(library)=LocalLibraryRepository::new(&cfg.db_path,&cfg.media_dir).snapshot(&LibraryQuery::default()){
                            let cached=crate::analysis::cached_many(&db,&library.tracks.iter().map(|t|(t.id,t.path.as_str())).collect::<Vec<_>>());
                            let stale=library.tracks.iter().zip(cached).filter(|(t,c)|c.is_none()&&t.available&&!analysis_requested.contains_key(&t.id)).take(8).map(|(t,_)|(t.id,t.path.clone(),t.duration as i64)).collect::<Vec<_>>();
                            if !stale.is_empty(){analysis_requested.extend(stale.iter().map(|t|(t.0,std::time::Instant::now())));let _=state.analysis.start(db.clone(),stale);}
                        }
                     }}
                     let queued_ids=s.queue.iter().skip(current).map(|t|t.id).collect::<Vec<_>>();
                     let stamp=(s.queue_revision,s.queue_index,config.preserve_energy,state.analysis.pair_revision(&queued_ids));if config.preserve_energy&&!s.dj_transition.as_ref().is_some_and(|t|t.active)&&arranged!=Some(stamp) {arranged=Some(stamp);
                      let features=crate::analysis::cached_many(&db,&s.queue.iter().map(|t|(t.id,t.path.as_str())).collect::<Vec<_>>());let pins=s.queue_entry_ids.iter().enumerate().filter(|(_,id)|s.pinned_entries.contains(id)).map(|(i,_)|i).collect();
                      let order=crate::dj_plan::order_upcoming(s.queue.as_ref(),current+1,&pins,|previous,index|{let a=previous.and_then(|i|features[i].as_ref());let b=features[index].as_ref();let energy=match(a.and_then(|f|f.structure.as_ref()),b.and_then(|f|f.structure.as_ref())){(Some(a),Some(b))=>(a.energy-b.energy).abs()*2.,_=>0.};crate::analysis::compatibility(a,b)+energy+if previous.is_some_and(|p|s.queue[p].artist==s.queue[index].artist){0.3}else{0.}});
                      let ids=order.iter().map(|i|s.queue_entry_ids[*i]).collect::<Vec<_>>();if ids!=s.queue_entry_ids[current+1..]{let _=client.send(LocalPlaybackCommand::ArrangeUpcoming(ids));std::thread::sleep(Duration::from_millis(100));continue;}
                     }
                    }}
                    let relevant_ids=s.queue_index.map(|i|s.queue.iter().skip(i).take(2).map(|t|t.id).collect::<Vec<_>>()).unwrap_or_default();
                    let stamp=(s.queue_revision,s.plan_revision,s.queue_index,serde_json::to_string(&config).unwrap_or_default(),state.analysis.pair_revision(&relevant_ids));
                    if !s.dj_transition.as_ref().is_some_and(|t|t.active)&&planned.as_ref()!=Some(&stamp) {planned=Some(stamp);if let Some(index)=s.queue_index {if index+1<s.queue.len()&&s.repeat!="one" {
                     let outgoing=&s.queue[index];let incoming=&s.queue[index+1];let db=state.config().db_path;
                     let features=crate::analysis::cached_many(&db,&[(outgoing.id,outgoing.path.as_str()),(incoming.id,incoming.path.as_str())]);
                     let (a,b)=(features[0].as_ref(),features[1].as_ref());
                     // Album adjacency is not evidence of a continuous passage. Until verified
                     // continuity metadata exists, use the normal confidence-aware blend.
                     let contiguous=false;
                     let plan=crate::dj_plan::plan(crate::dj_plan::Track{key_index:a.filter(|f|f.key_confidence>=0.5).and_then(|f|f.key_index),duration:a.and_then(|f|f.structure.as_ref()).map(|s|s.duration).unwrap_or(outgoing.duration as f32),bpm:a.and_then(|f|f.bpm),structure:a.and_then(|f|f.structure.as_ref())},crate::dj_plan::Track{key_index:b.filter(|f|f.key_confidence>=0.5).and_then(|f|f.key_index),duration:b.and_then(|f|f.structure.as_ref()).map(|s|s.duration).unwrap_or(incoming.duration as f32),bpm:b.and_then(|f|f.bpm),structure:b.and_then(|f|f.structure.as_ref())},&config,contiguous);
                     if let Some(plan)=plan{musicos_desktop::local::dj_diagnostics::record("candidate",Some(s.queue_entry_ids[index]),Some(s.queue_entry_ids[index+1]),format!("{}; confidence {:.2}, cue {:.3}s overlap {:.3}s",plan.reason,plan.confidence,plan.outgoing_cue,plan.duration));if let Err(error)=client.send(LocalPlaybackCommand::PlanTransition(musicos_desktop::local::deck_playback::Transition{outgoing:s.queue_entry_ids[index],incoming:s.queue_entry_ids[index+1],outgoing_cue:plan.outgoing_cue,incoming_cue:plan.incoming_cue,duration:plan.duration,tempo:plan.incoming_tempo,effect:serde_json::to_value(plan.effect).ok().and_then(|v|v.as_str().map(str::to_owned)).unwrap_or_default()})){planned=None;musicos_desktop::local::dj_diagnostics::record("delivery_failed",None,None,error.to_string());}}
                    }}}

                    let mut value = json!({"track":s.track,"queue_index":s.queue_index,"is_playing":s.is_playing,"position":s.position,"volume":s.volume,"shuffle":s.shuffle,"dj_mode":s.dj_mode,"pinned_entries":s.pinned_entries,"arranged_entries":s.arranged_entries,"plan_revision":s.plan_revision,"dj_transition":s.dj_transition,"listening":s.listening,"listening_applied":s.listening_applied,"audio_levels":s.audio_levels,"repeat":s.repeat,"error":s.error,"backend":s.backend});
                    if value != last_value || s.queue_revision != last_queue_revision {
                        last_value=value.clone();
                        // Send the potentially large queue only when membership/order changes.
                        if s.queue_revision != last_queue_revision {
                            value["queue"]=json!(s.queue.as_ref());value["queue_entry_ids"]=json!(s.queue_entry_ids);
                            last_queue_revision=s.queue_revision;
                        }
                        let _=handle.emit("playback",value);
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Could not start MusicOS");
    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            let state=handle.state::<AppState>();
            let snapshot=state.playback.lock().unwrap_or_else(|e|e.into_inner()).snapshot();
            let mut config=state.config.lock().unwrap_or_else(|e|e.into_inner());
            config.playback=saved_playback(&snapshot);
            if let Err(error)=storage::save_config(&config) {eprintln!("Could not save playback session: {error:#}");}
            drop(config);
            #[cfg(target_os="macos")]
            crate::system_media::shutdown();
            crate::spark_install::cancel();
            handle.state::<AppState>().integrations.shutdown();
            let _ = handle.state::<AppState>().analysis.control("cancel");
            handle
                .state::<AppState>()
                .stopped
                .store(true, Ordering::Relaxed);
            handle
                .state::<AppState>()
                .cancel
                .store(true, Ordering::Relaxed);
            let _ = handle
                .state::<AppState>()
                .runtime
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .stop();
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn playback_session_retains_source_position() {
        let mut snapshot=musicos_desktop::local::LocalPlaybackSnapshot::default();
        snapshot.queue_index=Some(2);snapshot.position=163.7;snapshot.volume=0.4;
        let saved=saved_playback(&snapshot);assert_eq!(saved.queue_index,2);assert_eq!(saved.position,163.7);assert_eq!(saved.volume,0.4);
    }
    #[test]
    fn dj_order_preserves_anchor_and_prefers_compatible_upcoming_tracks() {
        let feature = |id, bpm, key| {
            Some(crate::analysis::Features {
                track_id: id,
                bpm: Some(bpm),
                bpm_confidence: 1.,
                key: Some("known".into()),
                key_index: Some(key),
                key_confidence: 1.,
                structure: None,
            })
        };
        let features = vec![
            feature(1, 120., 0),
            feature(2, 90., 6),
            feature(3, 122., 0),
            feature(4, 124., 7),
        ];
        assert_eq!(
            dj_order(&features, &["a", "b", "c", "d"], 1, Some(0)),
            vec![2, 3, 1]
        );
        assert_eq!(
            dj_order(&features, &["a", "b", "c", "d"], 3, Some(2)),
            vec![3]
        );
        assert_eq!(
            dj_order(&[None, None, None], &["a", "b", "c"], 1, Some(0)),
            vec![1, 2]
        );
    }
    #[test]
    fn playback_selection_and_shuffle_preserve_members() {
        assert_eq!(
            sequence(vec![1, 2, 3], 1, false).unwrap(),
            (vec![1, 2, 3], 1)
        );
        let (mut shuffled, start) = sequence(vec![1, 2, 3], 0, true).unwrap();
        shuffled.sort();
        assert_eq!(shuffled, vec![1, 2, 3]);
        assert_eq!(start, 0);
        assert!(sequence(vec![], 0, false).is_err());
        assert!(sequence(vec![1], 2, false).is_err());
    }
    #[test]
    fn desktop_flac_decoder_supports_seek_and_rewind() {
        use rodio::Source;
        let bytes = include_bytes!("../tests/fixtures/seek.flac");
        let mut decoder = rodio::Decoder::new(std::io::Cursor::new(bytes.as_slice())).unwrap();
        assert!(decoder.next().is_some());
        decoder.try_seek(Duration::from_millis(800)).unwrap();
        assert!(decoder.next().is_some());
        decoder.try_seek(Duration::ZERO).unwrap();
        assert!(decoder.next().is_some());
    }
    #[test]
    fn reference_scan_and_lazy_cover_use_real_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        // A small PCM WAV fixture needs no external encoder.
        let samples = vec![0u8; 1600];
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&(36u32 + samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&16000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(&samples);
        let song = source.join("song.wav");
        std::fs::write(&song, &wav).unwrap();
        image::RgbImage::from_pixel(8, 8, image::Rgb([30, 90, 200]))
            .save(source.join("cover.png"))
            .unwrap();
        let db = temp.path().join("library.sqlite3");
        let repo = LocalLibraryRepository::new(&db, temp.path().join("managed"));
        repo.ensure_ready().unwrap();
        let reference = repo.add_reference_source(&source).unwrap();
        repo.scan(
            musicos_init::media::ScanMode::Changed,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        let snapshot = repo.snapshot(&LibraryQuery::default()).unwrap();
        assert_eq!(snapshot.tracks.len(), 1);
        let source_id = reference.id;
        repo.update_source(source_id, false).unwrap();
        assert!(repo
            .snapshot(&LibraryQuery::default())
            .unwrap()
            .tracks
            .is_empty());
        repo.update_source(source_id, true).unwrap();
        assert_eq!(
            repo.snapshot(&LibraryQuery::default()).unwrap().tracks[0].id,
            snapshot.tracks[0].id
        );
        let cover = musicos_desktop::local::artwork::ensure_artwork(db, snapshot.tracks[0].id)
            .unwrap()
            .unwrap();
        assert!(cover.is_file());
        assert!(image::open(cover).is_ok());
        assert_eq!(std::fs::read(song).unwrap(), wav);
    }
    #[test]
    fn cancelled_import_does_not_copy_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("song.mp3"), b"test fixture").unwrap();
        let target = temp.path().join("managed");
        let cancelled = AtomicBool::new(true);
        let result = musicos_desktop::library_import::import_managed_library(
            &[source],
            &target,
            &cancelled,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.imported, 0);
        assert!(!target.join("song.mp3").exists());
    }
}

fn saved_playback(snapshot:&musicos_desktop::local::LocalPlaybackSnapshot)->musicos_desktop::models::PlaybackPreferences {
 musicos_desktop::models::PlaybackPreferences{queue_track_ids:snapshot.queue.iter().map(|t|t.id).collect(),queue_index:snapshot.queue_index.unwrap_or(0),position:snapshot.position.max(0.),volume:snapshot.volume,shuffle:snapshot.shuffle,repeat:snapshot.repeat.clone()}
}
