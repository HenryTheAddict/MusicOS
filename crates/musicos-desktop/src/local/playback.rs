use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

use super::deck_playback::{DesktopPlayer as AudioPlayer, Transition};
use anyhow::{Context, Result};
use musicos_init::audio::LoopMode;
use musicos_init::database::Db;

use crate::models::{PlaybackPreferences, Track};

use super::library::track_from_init;

#[derive(Debug, Clone)]
pub enum LocalPlaybackCommand {
    PlayTrack(i64),
    PlayTracks { ids: Vec<i64>, start_index: usize },
    PlayPause,
    Pause,
    Previous,
    Next,
    SeekAbsolute(f32),
    SeekRelative(f32),
    SetVolume(f32),
    SetListening(super::listening::Listening),
    SetDjMode(bool),
    PlanTransition(Transition),
    ArrangeUpcoming(Vec<u64>),
    ToggleShuffle,
    SetShuffle(bool),
    CycleRepeat,
    SelectQueueIndex(usize),
    RemoveQueueIndex(usize),
    MoveQueueItem { from: usize, to: usize },
    PlayNext(i64),
    AddToQueue(i64),
    InsertQueue {id:i64,index:usize},
    ClearUpcoming,
    ClearQueue,
    RefreshPaths,
    Shutdown,
}

impl LocalPlaybackCommand {
    fn mutates_queue(&self) -> bool {
        matches!(
            self,
            Self::PlayTrack(_)
                | Self::PlayTracks { .. }
                | Self::RemoveQueueIndex(_)
                | Self::MoveQueueItem { .. }
                | Self::PlayNext(_)
                | Self::AddToQueue(_)
                | Self::InsertQueue {..}
                | Self::ClearUpcoming
                | Self::ClearQueue
                | Self::RefreshPaths
        )
    }
}

#[derive(Debug, Clone)]
pub struct LocalPlaybackSnapshot {
    pub revision: u64,
    pub track: Option<Track>,
    pub queue: Arc<[Track]>,
    pub queue_revision: u64,
    pub queue_entry_ids: Vec<u64>,
    pub pinned_entries: Vec<u64>,
    pub arranged_entries: Vec<u64>,
    pub plan_revision: u64,
    pub listened: Option<f32>,
    pub listening: super::listening::Listening,
    pub listening_applied: bool,
    pub applied_listening_revision:u64,
    pub requested_listening_revision:u64,
    pub applied_reverb:f32,
    pub applied_reverb_depth:f32,
    pub reverb_processed_frames:u64,
    pub transition_outcome:Option<super::deck_playback::TransitionOutcome>,
    pub failed_entries:Vec<u64>,
    pub audio_levels: [f32; 3],
    pub dj_transition: Option<super::deck_playback::TransitionStatus>,
    pub queue_index: Option<usize>,
    pub is_playing: bool,
    pub position: f32,
    pub volume: f32,
    pub dj_mode: bool,
    pub shuffle: bool,
    pub repeat: String,
    pub backend: String,
    pub error: Option<String>,
}

impl Default for LocalPlaybackSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            track: None,
            queue: Arc::from([]),
            queue_revision: 0,
            queue_entry_ids: vec![],
            pinned_entries: vec![],
            arranged_entries: vec![],
            plan_revision: 0,
            listened: None,
            audio_levels: [0.; 3],
            listening:Default::default(),
            listening_applied:true,applied_listening_revision:0,requested_listening_revision:0,applied_reverb:0.,applied_reverb_depth:0.5,reverb_processed_frames:0,transition_outcome:None,failed_entries:vec![],
            dj_transition: None,
            queue_index: None,
            is_playing: false,
            position: 0.0,
            volume: 1.0,
            dj_mode: false,
            shuffle: false,
            repeat: "none".to_string(),
            backend: "starting".to_string(),
            error: None,
        }
    }
}

pub struct LocalPlaybackService {
    sender: mpsc::Sender<LocalPlaybackCommand>,
    snapshot: Arc<RwLock<LocalPlaybackSnapshot>>,
    worker: Option<thread::JoinHandle<()>>,
}

/// A lightweight handle for secondary windows and other UI entities.
///
/// The service itself remains owned by the main application entity, while this
/// handle lets independent GPUI windows issue commands and observe the same
/// authoritative playback snapshot without creating another audio backend.
#[derive(Clone)]
pub struct LocalPlaybackClient {
    sender: mpsc::Sender<LocalPlaybackCommand>,
    snapshot: Arc<RwLock<LocalPlaybackSnapshot>>,
}

impl LocalPlaybackClient {
    pub fn send(&self, command: LocalPlaybackCommand) -> Result<()> {
        self.sender
            .send(command)
            .context("local playback worker stopped")
    }

    pub fn snapshot(&self) -> LocalPlaybackSnapshot {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }
}

impl LocalPlaybackService {
    pub fn start(db_path: PathBuf) -> Result<Self> {
        Self::start_with_preferences(db_path, PlaybackPreferences::default())
    }

    pub fn start_with_preferences(
        db_path: PathBuf,
        preferences: PlaybackPreferences,
    ) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Validate the database before starting the long-lived audio worker.
        let _ = Db::open(
            db_path
                .to_str()
                .context("library database path is not valid UTF-8")?,
        )?;
        let (sender, receiver) = mpsc::channel();
        let snapshot = Arc::new(RwLock::new(LocalPlaybackSnapshot::default()));
        let worker_snapshot = snapshot.clone();
        let worker = thread::Builder::new()
            .name("musicos-local-playback".to_string())
            .spawn(move || playback_loop(db_path, preferences, receiver, worker_snapshot))
            .context("starting local playback worker")?;
        Ok(Self {
            sender,
            snapshot,
            worker: Some(worker),
        })
    }

    pub fn send(&self, command: LocalPlaybackCommand) -> Result<()> {
        self.sender
            .send(command)
            .context("local playback worker stopped")
    }

    pub fn snapshot(&self) -> LocalPlaybackSnapshot {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }

    pub fn client(&self) -> LocalPlaybackClient {
        LocalPlaybackClient {
            sender: self.sender.clone(),
            snapshot: self.snapshot.clone(),
        }
    }
}

impl Drop for LocalPlaybackService {
    fn drop(&mut self) {
        let _ = self.sender.send(LocalPlaybackCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn playback_loop(
    db_path: PathBuf,
    preferences: PlaybackPreferences,
    receiver: mpsc::Receiver<LocalPlaybackCommand>,
    snapshot: Arc<RwLock<LocalPlaybackSnapshot>>,
) {
    let db_path = match db_path.to_str() {
        Some(path) => path.to_string(),
        None => {
            set_error(&snapshot, "library database path is not valid UTF-8");
            return;
        }
    };
    let db = match Db::open(&db_path) {
        Ok(db) => db,
        Err(error) => {
            set_error(&snapshot, format!("opening playback library: {error:#}"));
            return;
        }
    };
    let mut player = AudioPlayer::new();
    player.set_volume(preferences.volume);
    player.set_shuffle(preferences.shuffle);
    player.set_loop_mode(match preferences.repeat.as_str() {
        "all" => LoopMode::All,
        "one" => LoopMode::One,
        _ => LoopMode::None,
    });
    if !preferences.queue_track_ids.is_empty() {
        let tracks = preferences
            .queue_track_ids
            .iter()
            .filter_map(|id| db.get_track_by_id(*id).ok().flatten())
            .filter(|track| std::path::Path::new(&track.path).is_file())
            .collect::<Vec<_>>();
        if !tracks.is_empty() {
            player.restore_queue_paused(
                tracks,
                preferences.queue_index,
                preferences.position.max(0.0),
            );
        }
    }
    let mut dj_mode = false;

    let mut revision = 0_u64;
    let mut queue_revision = 0_u64;
    let mut command_error = None;
    let mut queue = queue_snapshot(&player);
    publish(
        &snapshot,
        &player,
        dj_mode,
        queue.clone(),
        queue_revision,
        revision,
        None,
    );

    loop {
        let timeout = if player.is_playing() {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(1)
        };
        match receiver.recv_timeout(timeout) {
            Ok(LocalPlaybackCommand::Shutdown) => break,
            Ok(command) => {
                if let LocalPlaybackCommand::SetDjMode(enabled) = &command {
                    dj_mode = *enabled;
                }
                let mutates_queue = command.mutates_queue();
                match apply_command(&db, &mut player, command) {
                    Ok(()) => {
                        command_error = None;
                        if mutates_queue {
                            queue = queue_snapshot(&player);
                            queue_revision = queue_revision.wrapping_add(1);
                        }
                    }
                    Err(error) => command_error = Some(format!("{error:#}")),
                }
                revision = revision.wrapping_add(1);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        player.tick();
        let latest = player.entry_ids();
        if latest
            != snapshot
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .queue_entry_ids
        {
            queue = queue_snapshot(&player);
            queue_revision = queue_revision.wrapping_add(1);
        }

        revision = revision.wrapping_add(1);
        publish(
            &snapshot,
            &player,
            dj_mode,
            queue.clone(),
            queue_revision,
            revision,
            command_error.as_deref(),
        );
    }
}

fn apply_command(db: &Db, player: &mut AudioPlayer, command: LocalPlaybackCommand) -> Result<()> {
    match command {
        LocalPlaybackCommand::PlayTrack(id) => {
            let track = db
                .get_track_by_id(id)?
                .with_context(|| format!("track {id} was not found"))?;
            anyhow::ensure!(
                std::path::Path::new(&track.path).is_file(),
                "track file is unavailable: {}",
                track.path
            );
            db.increment_play_count(id)?;
            player.set_queue(vec![track], 0);
        }
        LocalPlaybackCommand::PlayTracks { ids, start_index } => {
            let requested_start_id = ids.get(start_index).copied();
            let mut tracks = Vec::with_capacity(ids.len());
            for id in ids {
                if let Some(track) = db.get_track_by_id(id)? {
                    if std::path::Path::new(&track.path).is_file() {
                        tracks.push(track);
                    }
                }
            }
            anyhow::ensure!(!tracks.is_empty(), "no playable tracks were selected");
            let start_index = requested_start_id
                .and_then(|id| tracks.iter().position(|track| track.id == id))
                .unwrap_or_else(|| start_index.min(tracks.len().saturating_sub(1)));
            if let Some(track) = tracks.get(start_index) {
                db.increment_play_count(track.id)?;
            }
            player.set_queue(tracks, start_index);
        }
        LocalPlaybackCommand::PlayPause => player.play_pause(),
        LocalPlaybackCommand::Pause => player.pause(),
        LocalPlaybackCommand::Previous => player.prev(),
        LocalPlaybackCommand::Next => player.next(),
        LocalPlaybackCommand::SeekAbsolute(value) => player.seek(value),
        LocalPlaybackCommand::SeekRelative(delta) => {
            player.seek(player.get_position() + delta);
        }
        LocalPlaybackCommand::SetVolume(value) => player.set_volume(value),
        LocalPlaybackCommand::SetListening(value) => player.set_listening(value)?,
        LocalPlaybackCommand::SetDjMode(enabled) => {
            player.set_dj_mode(enabled);
            if enabled {
                player.set_shuffle(false);
            }
        }
        LocalPlaybackCommand::PlanTransition(plan) => {
            if let Err(error)=player.prepare(plan){player.plan_revision=player.plan_revision.wrapping_add(1);
                super::dj_diagnostics::record("rejected",None,None,error.to_string());return Err(error);}
        }
        LocalPlaybackCommand::ArrangeUpcoming(ids) => {
            player.arrange(ids)?;
        }
        LocalPlaybackCommand::ToggleShuffle => player.toggle_shuffle(),
        LocalPlaybackCommand::SetShuffle(enabled) => player.set_shuffle(enabled),
        LocalPlaybackCommand::CycleRepeat => player.toggle_loop_mode(),
        LocalPlaybackCommand::SelectQueueIndex(index) => {
            anyhow::ensure!(player.queue_select(index), "queue index is out of bounds");
        }
        LocalPlaybackCommand::RemoveQueueIndex(index) => {
            anyhow::ensure!(
                player.queue_remove(index).is_some(),
                "queue index is out of bounds"
            );
        }
        LocalPlaybackCommand::MoveQueueItem { from, to } => {
            anyhow::ensure!(player.queue_move(from, to), "queue move is out of bounds");
        }
        LocalPlaybackCommand::PlayNext(id) => {
            let track = db
                .get_track_by_id(id)?
                .with_context(|| format!("track {id} was not found"))?;
            anyhow::ensure!(
                std::path::Path::new(&track.path).is_file(),
                "track file is unavailable: {}",
                track.path
            );
            player.queue_add_next(track);
        }
        LocalPlaybackCommand::AddToQueue(id) => {
            let track = db
                .get_track_by_id(id)?
                .with_context(|| format!("track {id} was not found"))?;
            anyhow::ensure!(
                std::path::Path::new(&track.path).is_file(),
                "track file is unavailable: {}",
                track.path
            );
            player.queue_add_later(track);
        }
        LocalPlaybackCommand::InsertQueue {id,index} => {
            let track=db.get_track_by_id(id)?.with_context(||format!("track {id} was not found"))?;
            anyhow::ensure!(std::path::Path::new(&track.path).is_file(),"Track file is unavailable");
            player.queue_insert(track,index);
        }
        LocalPlaybackCommand::ClearUpcoming => player.queue_clear_upcoming(),
        LocalPlaybackCommand::ClearQueue => player.queue_clear(),
        LocalPlaybackCommand::RefreshPaths => {let tracks=player.queue_tracks().into_iter().filter_map(|t|db.get_track_by_id(t.id).ok().flatten()).collect::<Vec<_>>();player.refresh_paths(&tracks);}
        LocalPlaybackCommand::Shutdown => {}
    }
    Ok(())
}

fn publish(
    target: &Arc<RwLock<LocalPlaybackSnapshot>>,
    player: &AudioPlayer,
    dj_mode: bool,
    queue: Arc<[Track]>,
    queue_revision: u64,
    revision: u64,
    command_error: Option<&str>,
) {
    let status = player.get_status();
    let repeat = match status.loop_mode {
        LoopMode::None => "none",
        LoopMode::All => "all",
        LoopMode::One => "one",
    };
    if let Ok(mut snapshot) = target.write() {
        *snapshot = LocalPlaybackSnapshot {
            revision,
            track: status.track.map(track_from_init),
            queue,
            queue_revision,
            queue_entry_ids: player.entry_ids(),
            pinned_entries: player.pins.iter().copied().collect(),
            arranged_entries: player.arranged.iter().copied().collect(),
            plan_revision: player.plan_revision,
            listened: Some(player.listened()),
                audio_levels: player.audio_levels(),
            listening:player.listening.clone(),
            listening_applied:player.listening_applied(),
            applied_listening_revision:player.applied_listening_revision(),requested_listening_revision:player.listening_revision,applied_reverb:player.applied_reverb(),applied_reverb_depth:player.applied_reverb_depth(),reverb_processed_frames:player.reverb_processed_frames(),transition_outcome:player.transition_outcome.clone(),failed_entries:player.failed_entries.iter().copied().collect(),
            dj_transition: player.transition_status(),
            queue_index: status.queue_index,
            is_playing: status.is_playing,
            position: status.position,
            volume: status.volume,
            dj_mode,
            shuffle: status.shuffle,
            repeat: repeat.to_string(),
            backend: status.backend,
            error: command_error.map(str::to_string).or(status.last_error),
        };
    }
}

#[cfg(test)]
fn crossfade_gains(progress: f32) -> (f32, f32) {
    let angle = progress.clamp(0., 1.) * std::f32::consts::FRAC_PI_2;
    (angle.sin(), angle.cos())
}

fn queue_snapshot(player: &AudioPlayer) -> Arc<[Track]> {
    Arc::from(
        player
            .queue_tracks()
            .into_iter()
            .map(track_from_init)
            .collect::<Vec<_>>(),
    )
}

fn set_error(target: &Arc<RwLock<LocalPlaybackSnapshot>>, error: impl Into<String>) {
    if let Ok(mut snapshot) = target.write() {
        snapshot.error = Some(error.into());
        snapshot.backend = "error".to_string();
        snapshot.revision = snapshot.revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfade_is_equal_power_and_has_correct_endpoints() {
        assert_eq!(crossfade_gains(0.), (0., 1.));
        let (a, b) = crossfade_gains(0.5);
        assert!((a * a + b * b - 1.).abs() < 1e-6);
        let (a, b) = crossfade_gains(1.);
        assert!((a - 1.).abs() < 1e-6 && b.abs() < 1e-6);
    }
    #[test]
    fn djmode_crossfades_real_decoders_and_keeps_queue_identity() {
        // Zero master volume: verify the real output path without audible test tones.
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("dj.sqlite");
        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let mut ids = Vec::new();
        for i in 0..2 {
            let path = temp.path().join(format!("{i}.wav"));
            let rate = 8000u32;
            let length = rate * 14 * 2;
            let mut wav = Vec::new();
            wav.extend_from_slice(b"RIFF");
            wav.extend_from_slice(&(36 + length).to_le_bytes());
            wav.extend_from_slice(b"WAVEfmt ");
            wav.extend_from_slice(&16u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&rate.to_le_bytes());
            wav.extend_from_slice(&(rate * 2).to_le_bytes());
            wav.extend_from_slice(&2u16.to_le_bytes());
            wav.extend_from_slice(&16u16.to_le_bytes());
            wav.extend_from_slice(b"data");
            wav.extend_from_slice(&length.to_le_bytes());
            wav.resize(44 + length as usize, 0);
            std::fs::write(&path, wav).unwrap();
            ids.push(
                db.insert_track(
                    path.to_str().unwrap(),
                    "DJ test",
                    "Artist",
                    "Album",
                    14,
                    length as i64,
                    i,
                )
                .unwrap(),
            );
        }
        drop(db);
        let mut preferences = PlaybackPreferences::default();
        preferences.volume = 0.;
        let service = LocalPlaybackService::start_with_preferences(db_path, preferences).unwrap();
        service
            .send(LocalPlaybackCommand::PlayTracks {
                ids: ids.clone(),
                start_index: 0,
            })
            .unwrap();
        for _ in 0..100 {
            if service.snapshot().track.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if cfg!(target_os = "macos") {
            assert_eq!(
                service.snapshot().backend,
                "MusicOS two-deck",
                "Native audio backend unavailable"
            );
        }
        if service.snapshot().backend != "MusicOS two-deck" {
            return;
        } // Headless hosts cannot establish native audio acceptance.
        service.send(LocalPlaybackCommand::SetDjMode(true)).unwrap();
        service
            .send(LocalPlaybackCommand::SeekAbsolute(11.))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let entries = service.snapshot().queue_entry_ids;
        service
            .send(LocalPlaybackCommand::PlanTransition(Transition {
                request_id:0,
                outgoing: entries[0],
                incoming: entries[1],
                outgoing_cue: 12.0,
                incoming_cue: 0.,
                duration: 1.,
                tempo: 1.,
                effect: "clean_blend".into(),
            }))
            .unwrap();
        for _ in 0..200 {
            if service.snapshot().queue_index == Some(1) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let s = service.snapshot();
        assert_eq!(
            s.queue_index,
            Some(1),
            "position={} error={:?} entries={:?}",
            s.position,
            s.error,
            s.queue_entry_ids
        );
        assert!(s.dj_mode && s.is_playing);
        assert_eq!(s.queue.iter().map(|t| t.id).collect::<Vec<_>>(), ids);
        assert!(s.position < 4.);
        service.send(LocalPlaybackCommand::Pause).unwrap();
        for _ in 0..100 {
            if !service.snapshot().is_playing {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!service.snapshot().is_playing);
        service
            .send(LocalPlaybackCommand::SetDjMode(false))
            .unwrap();
        service
            .send(LocalPlaybackCommand::SelectQueueIndex(0))
            .unwrap();
        for _ in 0..100 {
            if service.snapshot().queue_index == Some(0) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(service.snapshot().queue_index, Some(0));
    }
    #[test]
    fn snapshot_clones_share_large_queue_storage() {
        let queue = Arc::<[Track]>::from(
            (0..10_000)
                .map(|id| Track {
                    id,
                    title: format!("Track {id}"),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        );
        let snapshot = LocalPlaybackSnapshot {
            queue: queue.clone(),
            ..Default::default()
        };
        let clone = snapshot.clone();
        assert!(Arc::ptr_eq(&snapshot.queue, &clone.queue));
        assert_eq!(clone.queue.len(), 10_000);
    }

    #[test]
    fn playback_worker_accepts_commands_without_a_companion_server() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("playback.sqlite3");
        let service = LocalPlaybackService::start(db_path).unwrap();
        service.send(LocalPlaybackCommand::SetVolume(0.42)).unwrap();
        let mut snapshot = service.snapshot();
        for _ in 0..100 {
            if (snapshot.volume - 0.42).abs() < 0.01 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
            snapshot = service.snapshot();
        }
        assert!((snapshot.volume - 0.42).abs() < 0.01);
        assert!(snapshot.revision > 0);
    }

    #[test]
    fn playback_worker_accepts_queue_insert_commands() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("playback-queue.sqlite3");
        let first_path = temp.path().join("first.ogg");
        let second_path = temp.path().join("second.ogg");
        let mut wav = Vec::new();
        let length = 16000u32;
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + length).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&16000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&length.to_le_bytes());
        wav.resize(44 + length as usize, 0);
        std::fs::write(&first_path, &wav).unwrap();
        std::fs::write(&second_path, &wav).unwrap();
        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let first = db
            .insert_track(
                first_path.to_str().unwrap(),
                "First",
                "Artist",
                "Album",
                120,
                4,
                1,
            )
            .unwrap();
        let second = db
            .insert_track(
                second_path.to_str().unwrap(),
                "Second",
                "Artist",
                "Album",
                120,
                4,
                1,
            )
            .unwrap();
        drop(db);
        let service = LocalPlaybackService::start(db_path).unwrap();
        service
            .send(LocalPlaybackCommand::AddToQueue(first))
            .unwrap();
        service
            .send(LocalPlaybackCommand::PlayNext(second))
            .unwrap();
        service.send(LocalPlaybackCommand::InsertQueue{id:first,index:1}).unwrap();
        service.send(LocalPlaybackCommand::SetVolume(0.5)).unwrap();
        let mut snapshot = service.snapshot();
        for _ in 0..100 {
            if (snapshot.volume - 0.5).abs() < 0.01 && snapshot.queue.len() == 3 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
            snapshot = service.snapshot();
        }
        assert!((snapshot.volume - 0.5).abs() < 0.01);
        assert_eq!(
            snapshot
                .queue
                .iter()
                .map(|track| track.id)
                .collect::<Vec<_>>(),
            vec![first, first, second]
        );
        assert_eq!(snapshot.queue_index, Some(0));
        assert_eq!(snapshot.queue_entry_ids.iter().collect::<std::collections::HashSet<_>>().len(),3);
        assert!(snapshot.error.is_none(), "{:?}", snapshot.error);
        assert!(snapshot.revision > 0);
    }

    #[test]
    fn collection_playback_preserves_visible_order_and_selected_start() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("collection.sqlite3");
        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let mut ids = Vec::new();
        for index in 0..3 {
            let path = temp.path().join(format!("{index}.ogg"));
            std::fs::write(&path, b"test").unwrap();
            ids.push(
                db.insert_track(
                    path.to_str().unwrap(),
                    &format!("Track {index}"),
                    "Artist",
                    "Album",
                    120,
                    4,
                    index,
                )
                .unwrap(),
            );
        }

        drop(db);
        let service = LocalPlaybackService::start(db_path).unwrap();
        service
            .send(LocalPlaybackCommand::PlayTracks {
                ids: ids.clone(),
                start_index: 1,
            })
            .unwrap();
        let mut snapshot = service.snapshot();
        for _ in 0..100 {
            if snapshot.queue.len() == 3 && snapshot.queue_index == Some(1) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
            snapshot = service.snapshot();
        }
        assert_eq!(
            snapshot
                .queue
                .iter()
                .map(|track| track.id)
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(snapshot.queue_index, Some(1));

        service.send(LocalPlaybackCommand::Next).unwrap();
        for _ in 0..100 {
            snapshot = service.snapshot();
            if snapshot.queue_index == Some(2) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(snapshot.queue_index, Some(2));
    }
}

#[cfg(test)]mod repair_tests {
 use super::*;
 fn wave(path:&PathBuf,seconds:u32){let rate=8000u32;let length=rate*seconds*2;let mut w=Vec::new();w.extend(b"RIFF");w.extend((36+length).to_le_bytes());w.extend(b"WAVEfmt ");w.extend(16u32.to_le_bytes());w.extend(1u16.to_le_bytes());w.extend(1u16.to_le_bytes());w.extend(rate.to_le_bytes());w.extend((rate*2).to_le_bytes());w.extend(2u16.to_le_bytes());w.extend(16u16.to_le_bytes());w.extend(b"data");w.extend(length.to_le_bytes());w.resize(44+length as usize,0);std::fs::write(path,w).unwrap();}
 #[test]fn automatic_promotions_skip_corrupt_incoming_without_mutating_queue(){
  let temp=tempfile::tempdir().unwrap();let path=temp.path().join("test.sqlite");let db=Db::open(path.to_str().unwrap()).unwrap();let mut ids=Vec::new();
  for i in 0..4 {let audio=temp.path().join(format!("{i}.wav"));if i==1 {std::fs::write(&audio,b"corrupt audio").unwrap();}else{wave(&audio,3);}ids.push(db.insert_track(audio.to_str().unwrap(),"Automatic test","Artist","Same album",3,100,i).unwrap());}drop(db);
  let service=LocalPlaybackService::start_with_preferences(path,PlaybackPreferences{volume:0.,..Default::default()}).unwrap();service.send(LocalPlaybackCommand::SetDjMode(true)).unwrap();service.send(LocalPlaybackCommand::PlayTracks{ids:ids.clone(),start_index:0}).unwrap();
  let end=std::time::Instant::now()+Duration::from_secs(15);let mut active=false;let mut promoted=false;let mut entries=vec![];
  while std::time::Instant::now()<end {let s=service.snapshot();if entries.is_empty()&&!s.queue_entry_ids.is_empty(){entries=s.queue_entry_ids.clone();}active|=s.dj_transition.as_ref().is_some_and(|p|p.active);promoted|=s.queue_index==Some(2);if s.queue_index==Some(3){assert!(active&&promoted,"Missing automatic overlap or intermediate promotion");assert_eq!(s.queue_entry_ids,entries);assert_eq!(s.queue.iter().map(|t|t.id).collect::<Vec<_>>(),ids);assert!(s.failed_entries.contains(&entries[1]));service.send(LocalPlaybackCommand::Pause).unwrap();return;}std::thread::sleep(Duration::from_millis(20));}
  panic!("Automatic playback stuck: {:?}",service.snapshot());
 }
 #[test]fn reverb_changes_are_applied_without_a_decoder_revision(){
  let temp=tempfile::tempdir().unwrap();let path=temp.path().join("test.sqlite");let db=Db::open(path.to_str().unwrap()).unwrap();let audio=temp.path().join("song.wav");wave(&audio,4);let id=db.insert_track(audio.to_str().unwrap(),"Reverb","Artist","Album",4,64000,0).unwrap();drop(db);
  let service=LocalPlaybackService::start_with_preferences(path,PlaybackPreferences{volume:0.,..Default::default()}).unwrap();service.send(LocalPlaybackCommand::PlayTracks{ids:vec![id],start_index:0}).unwrap();
  for amount in [0.45,1.,0.2,0.] {let settings=super::super::listening::Listening{reverb:amount,reverb_depth:amount,..Default::default()};service.send(LocalPlaybackCommand::SetListening(settings)).unwrap();let end=std::time::Instant::now()+Duration::from_secs(3);loop {let s=service.snapshot();if s.listening_applied&&s.applied_reverb==amount&&s.applied_reverb_depth==amount&&s.requested_listening_revision==s.applied_listening_revision{break;}assert!(std::time::Instant::now()<end,"Mixer did not apply reverb: {s:?}");std::thread::sleep(Duration::from_millis(20));}}
  service.send(LocalPlaybackCommand::Pause).unwrap();
 }
}
