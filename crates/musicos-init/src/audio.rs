use std::time::{Duration, Instant};

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
use rodio::{Decoder, OutputStream, Sink, Source};
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
use std::fs::File;
#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
use std::io::BufReader;
#[cfg(all(feature = "device-audio", target_os = "linux"))]
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc, Mutex,
};

use crate::database::Track;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcmCapture {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: usize,
    pub peak: f32,
    pub non_silent_samples: usize,
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
pub fn decode_pcm_capture(
    path: &std::path::Path,
    max_samples: usize,
) -> anyhow::Result<PcmCapture> {
    let source = Decoder::new(BufReader::new(File::open(path)?))?;
    let sample_rate = source.sample_rate();
    let channels = source.channels();
    let mut peak = 0.0_f32;
    let mut samples = 0;
    let mut non_silent_samples = 0;
    for sample in source.take(max_samples.max(1)) {
        let magnitude = f32::from(sample).abs() / f32::from(i16::MAX);
        peak = peak.max(magnitude);
        non_silent_samples += usize::from(magnitude > 0.000_01);
        samples += 1;
    }
    Ok(PcmCapture {
        sample_rate,
        channels,
        samples,
        peak,
        non_silent_samples,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    None,
    All,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EqPreset {
    Flat,
    Bass,
    Vocal,
    Treble,
    Night,
}

impl Default for EqPreset {
    fn default() -> Self {
        Self::Flat
    }
}

impl EqPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::Bass => "Bass",
            Self::Vocal => "Vocal",
            Self::Treble => "Treble",
            Self::Night => "Night",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Flat => Self::Bass,
            Self::Bass => Self::Vocal,
            Self::Vocal => Self::Treble,
            Self::Treble => Self::Night,
            Self::Night => Self::Flat,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Flat => Self::Night,
            Self::Night => Self::Treble,
            Self::Treble => Self::Vocal,
            Self::Vocal => Self::Bass,
            Self::Bass => Self::Flat,
        }
    }

    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    fn gains(self) -> EqGains {
        match self {
            Self::Flat => EqGains::new(1.0, 1.0, 1.0, 1.0),
            Self::Bass => EqGains::new(1.38, 0.96, 0.92, 0.84),
            Self::Vocal => EqGains::new(0.92, 1.24, 1.02, 0.88),
            Self::Treble => EqGains::new(0.90, 0.98, 1.34, 0.86),
            Self::Night => EqGains::new(0.82, 0.92, 0.78, 0.72),
        }
    }
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
#[derive(Debug, Clone, Copy)]
struct EqGains {
    low: f32,
    mid: f32,
    high: f32,
    preamp: f32,
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
impl EqGains {
    const fn new(low: f32, mid: f32, high: f32, preamp: f32) -> Self {
        Self {
            low,
            mid,
            high,
            preamp,
        }
    }
}

/// An outgoing desktop sink, owned only by the desktop playback worker.
#[cfg(feature = "desktop-audio")]
pub struct DesktopFade { sink: Sink }
#[cfg(feature = "desktop-audio")]
impl DesktopFade {
    pub fn update(&self, volume:f32, playing:bool) { self.sink.set_volume(volume); if playing {self.sink.play()} else {self.sink.pause()} }
}
#[cfg(feature = "desktop-audio")]
impl Drop for DesktopFade { fn drop(&mut self) {self.sink.stop();} }

pub struct AudioPlayer {
    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    _stream: Option<OutputStream>,
    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    stream_handle: Option<rodio::OutputStreamHandle>,
    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    sink: Option<Sink>,
    #[cfg(all(feature = "device-audio", target_os = "linux"))]
    alsa_playback: Option<AlsaPlayback>,
    queue: Vec<Track>,
    current_index: Option<usize>,
    volume: f32,
    eq_preset: EqPreset,
    is_paused: bool,
    loop_mode: LoopMode,
    shuffle: bool,
    play_start: Option<Instant>,
    accumulated_time: Duration,
    current_loaded: bool,
    simulate_output: bool,
    last_backend: String,
    selected_pcm: Option<String>,
    last_error: Option<String>,
    shuffle_seed: u64,
    shuffle_order: Vec<usize>,
    shuffle_cursor: usize,
}

impl AudioPlayer {
    pub fn new() -> Self {
        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        {
            if prefers_silent_backend() {
                return Self::silent("none");
            }

            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            if std::env::var("MUSICOS_AUDIO_BACKEND")
                .map(|value| value.eq_ignore_ascii_case("alsa"))
                .unwrap_or(true)
            {
                // Direct ALSA owns the PCM on device. Opening Rodio here reserves the
                // same hardware and makes every subsequent direct playback fail busy.
                return Self::silent("alsa");
            }

            let mut stream_opt = None;
            let mut handle_opt = None;
            let mut sink_opt = None;

            match OutputStream::try_default() {
                Ok((stream, handle)) => {
                    match Sink::try_new(&handle) {
                        Ok(sink) => {
                            log::info!("audio output stream ready");
                            sink_opt = Some(sink);
                        }
                        Err(err) => log::error!("audio sink create failed: {err}"),
                    }
                    stream_opt = Some(stream);
                    handle_opt = Some(handle);
                }
                Err(default_err) => {
                    log::error!("default audio output stream failed: {default_err}. Trying fallback devices...");
                    use rodio::cpal::traits::{DeviceTrait, HostTrait};
                    let host = rodio::cpal::default_host();
                    let mut fallback_success = false;
                    if let Ok(devices) = host.output_devices() {
                        for device in devices {
                            if let Ok(name) = device.name() {
                                log::info!("trying audio fallback device: {}", name);
                            }
                            match OutputStream::try_from_device(&device) {
                                Ok((stream, handle)) => match Sink::try_new(&handle) {
                                    Ok(sink) => {
                                        log::info!("audio fallback stream ready");
                                        sink_opt = Some(sink);
                                        stream_opt = Some(stream);
                                        handle_opt = Some(handle);
                                        fallback_success = true;
                                        break;
                                    }
                                    Err(err) => {
                                        log::error!("failed to create sink on fallback: {err}")
                                    }
                                },
                                Err(err) => log::error!("failed to initialize device: {err}"),
                            }
                        }
                    }
                    if !fallback_success {
                        log::error!("all audio device initialization attempts failed");
                    }
                }
            }

            let last_backend = if sink_opt.is_some() {
                "rodio"
            } else {
                "unavailable"
            }
            .to_string();
            let last_error = if sink_opt.is_none() {
                Some("no rodio output sink available".to_string())
            } else {
                None
            };
            let shuffle_seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xdeadbeef);

            Self {
                _stream: stream_opt,
                stream_handle: handle_opt,
                sink: sink_opt,
                #[cfg(all(feature = "device-audio", target_os = "linux"))]
                alsa_playback: None,
                queue: Vec::new(),
                current_index: None,
                volume: 1.0,
                eq_preset: EqPreset::Flat,
                is_paused: false,
                loop_mode: LoopMode::None,
                shuffle: false,
                play_start: None,
                accumulated_time: Duration::ZERO,
                current_loaded: false,
                simulate_output: false,
                last_backend,
                selected_pcm: None,
                last_error,
                shuffle_seed,
                shuffle_order: Vec::new(),
                shuffle_cursor: 0,
            }
        }
        #[cfg(not(any(feature = "desktop-audio", feature = "device-audio")))]
        {
            let shuffle_seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xdeadbeef);
            Self {
                queue: Vec::new(),
                current_index: None,
                volume: 1.0,
                eq_preset: EqPreset::Flat,
                is_paused: false,
                loop_mode: LoopMode::None,
                shuffle: false,
                play_start: None,
                accumulated_time: Duration::ZERO,
                current_loaded: false,
                simulate_output: false,
                last_backend: "silent".to_string(),
                selected_pcm: None,
                last_error: None,
                shuffle_seed,
                shuffle_order: Vec::new(),
                shuffle_cursor: 0,
            }
        }
    }

    pub fn set_queue(&mut self, tracks: Vec<Track>, start_index: usize) {
        self.queue = tracks;
        self.current_index = if start_index < self.queue.len() {
            Some(start_index)
        } else {
            None
        };
        self.rebuild_shuffle_order();
        self.play_current();
    }

    pub fn queue_select(&mut self, index: usize) -> bool {
        if index >= self.queue.len() {
            return false;
        }
        self.current_index = Some(index);
        self.rebuild_shuffle_order();
        self.play_current();
        true
    }

    pub fn restore_queue_paused(&mut self, tracks: Vec<Track>, start_index: usize, position: f32) {
        self.stop();
        self.queue = tracks;
        self.current_index = if start_index < self.queue.len() {
            Some(start_index)
        } else {
            None
        };
        self.rebuild_shuffle_order();
        self.accumulated_time = Duration::from_secs_f32(position.max(0.0));
        self.play_start = None;
        self.is_paused = true;
        self.current_loaded = false;
    }

    pub fn play_current(&mut self) {
        let Some(idx) = self.current_index else {
            return;
        };
        if idx >= self.queue.len() {
            return;
        }
        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        let track_path = self.queue[idx].path.clone();
        let track_title = self.queue[idx].title.clone();
        let track_artist = self.queue[idx].artist.clone();

        self.stop();

        if self.simulate_output || prefers_silent_backend() {
            self.is_paused = false;
            self.last_backend = "none".to_string();
            self.selected_pcm = None;
            self.last_error = None;
            self.play_start = Some(Instant::now());
            self.accumulated_time = Duration::ZERO;
            self.current_loaded = true;
            log::info!("audio none backend: {} - {}", track_title, track_artist);
            return;
        }

        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        {
            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            if self.prefers_direct_alsa() {
                if self.start_alsa_playback(
                    track_path.clone(),
                    track_title.clone(),
                    track_artist.clone(),
                    0.0,
                ) {
                    return;
                }
            }

            if self.sink.is_none() {
                if let Some(ref handle) = self.stream_handle {
                    if let Ok(new_sink) = Sink::try_new(handle) {
                        self.sink = Some(new_sink);
                    } else {
                        let message = "audio sink recreate failed".to_string();
                        log::error!("{message}");
                        self.last_error = Some(message);
                    }
                }
            }

            if let Some(ref sink) = self.sink {
                match File::open(&track_path) {
                    Ok(file) => {
                        let reader = BufReader::new(file);
                        match Decoder::new(reader) {
                            Ok(source) => {
                                sink.append(EqSource::new(source, self.eq_preset));
                                sink.set_volume(self.volume);
                                sink.play();
                                self.is_paused = false;
                                self.last_backend = "rodio".to_string();
                                self.selected_pcm = None;
                                self.last_error = None;
                                self.play_start = Some(Instant::now());
                                self.accumulated_time = Duration::ZERO;
                                self.current_loaded = true;
                                log::info!("Playing track: {} - {}", track_title, track_artist);
                                return;
                            }
                            Err(err) => {
                                let message = format!("decoder failed for {}: {err}", track_path);
                                log::error!("{message}");
                                self.last_error = Some(message);
                            }
                        }
                    }
                    Err(err) => {
                        let message = format!("failed to open file {}: {err}", track_path);
                        log::error!("{message}");
                        self.last_error = Some(message);
                    }
                }
            } else {
                let message = "audio playback skipped: no output sink available".to_string();
                log::error!("{message}");
                self.last_error = Some(message);
            }

            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            if !self.prefers_direct_alsa()
                && self.start_alsa_playback(
                    track_path.clone(),
                    track_title.clone(),
                    track_artist.clone(),
                    0.0,
                )
            {
                return;
            }

            self.is_paused = true;
            self.play_start = None;
            self.accumulated_time = Duration::ZERO;
            let message = format!(
                "audio playback did not start: {} - {}",
                track_title, track_artist
            );
            log::error!("{message}");
            self.last_error = Some(message);
            return;
        }

        #[cfg(not(any(feature = "desktop-audio", feature = "device-audio")))]
        {
            self.is_paused = false;
            self.last_backend = "silent".to_string();
            self.selected_pcm = None;
            self.last_error = None;
            self.play_start = Some(Instant::now());
            self.accumulated_time = Duration::ZERO;
            self.current_loaded = true;
            log::info!("audio silent fallback: {} - {}", track_title, track_artist);
        }
    }

    pub(crate) fn for_snapshot() -> Self {
        let mut player = Self::silent("none");
        player.simulate_output = true;
        player
    }

    fn silent(backend: &str) -> Self {
        let shuffle_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        Self {
            #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
            _stream: None,
            #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
            stream_handle: None,
            #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
            sink: None,
            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            alsa_playback: None,
            queue: Vec::new(),
            current_index: None,
            volume: 1.0,
            eq_preset: EqPreset::Flat,
            is_paused: false,
            loop_mode: LoopMode::None,
            shuffle: false,
            play_start: None,
            accumulated_time: Duration::ZERO,
            current_loaded: false,
            simulate_output: false,
            last_backend: backend.to_string(),
            selected_pcm: None,
            last_error: None,
            shuffle_seed,
            shuffle_order: Vec::new(),
            shuffle_cursor: 0,
        }
    }

    /// Start the following queue item while retaining the outgoing sink for a finite fade.
    /// Decode and allocate first; a failed transition leaves current playback untouched.
    #[cfg(feature = "desktop-audio")]
    pub fn desktop_crossfade_next(&mut self) -> Result<Option<DesktopFade>, String> {
        if self.last_backend != "rodio" || !self.is_playing() || self.shuffle || self.loop_mode == LoopMode::One {return Ok(None);}
        let Some(index) = self.current_index else {return Ok(None)};
        let next = if index+1 < self.queue.len() {index+1} else if self.loop_mode == LoopMode::All && self.queue.len()>1 {0} else {return Ok(None)};
        if self.queue[next].duration < 12 {return Ok(None);}
        let source = Decoder::new(BufReader::new(File::open(&self.queue[next].path).map_err(|e|e.to_string())?)).map_err(|e|e.to_string())?;
        let handle=self.stream_handle.as_ref().ok_or("Audio output is unavailable")?;
        let incoming=Sink::try_new(handle).map_err(|e|e.to_string())?;
        incoming.set_volume(0.);incoming.append(EqSource::new(source,self.eq_preset));incoming.play();
        let outgoing=self.sink.replace(incoming);
        self.current_index=Some(next);self.play_start=Some(Instant::now());self.accumulated_time=Duration::ZERO;self.current_loaded=true;self.is_paused=false;self.last_error=None;
        Ok(outgoing.map(|sink|DesktopFade{sink}))
    }
    #[cfg(feature = "desktop-audio")]
    pub fn desktop_set_gain(&self,gain:f32) {if let Some(sink)=&self.sink {sink.set_volume(self.volume*gain.clamp(0.,1.));}}

    pub fn pause(&mut self) {
        if !self.is_paused {
            #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
            if let Some(ref sink) = self.sink {
                sink.pause();
            }
            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            if let Some(ref playback) = self.alsa_playback {
                playback.set_paused(true);
            }
            if let Some(start) = self.play_start.take() {
                self.accumulated_time += start.elapsed();
            }
            self.is_paused = true;
        }
    }

    pub fn play_pause(&mut self) {
        if self.current_index.is_none() {
            if !self.queue.is_empty() {
                self.current_index = Some(0);
                self.play_current();
            }
            return;
        }

        if self.is_paused && !self.current_loaded {
            let resume_position = self.accumulated_time.as_secs_f32();
            self.play_current();
            if self.current_loaded && resume_position > 0.0 {
                self.seek(resume_position);
            }
        } else if self.is_paused {
            #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
            if let Some(ref sink) = self.sink {
                sink.play();
            }
            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            if let Some(ref playback) = self.alsa_playback {
                playback.set_paused(false);
            }
            self.play_start = Some(Instant::now());
            self.is_paused = false;
        } else {
            self.pause();
        }
    }

    pub fn stop(&mut self) {
        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        {
            // rodio's `Sink::stop` is permanent: `play` cannot revive that sink.
            // Drop it after stopping so the next track receives a fresh sink.
            if let Some(sink) = self.sink.take() {
                sink.stop();
            }
            #[cfg(all(feature = "device-audio", target_os = "linux"))]
            {
                self.alsa_playback = None;
            }
        }
        self.play_start = None;
        self.accumulated_time = Duration::ZERO;
        self.current_loaded = false;
        self.is_paused = false;
    }

    pub fn next(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        if let Some(idx) = self.current_index {
            let next_idx = if self.shuffle {
                if self.shuffle_order.len() != self.queue.len() {
                    self.rebuild_shuffle_order();
                }
                if self.shuffle_cursor + 1 < self.shuffle_order.len() {
                    self.shuffle_cursor += 1;
                    self.shuffle_order[self.shuffle_cursor]
                } else if self.loop_mode == LoopMode::All {
                    self.rebuild_shuffle_order();
                    if self.shuffle_order.len() > 1 {
                        self.shuffle_cursor = 1;
                    }
                    self.shuffle_order[self.shuffle_cursor]
                } else {
                    self.finish_current();
                    return;
                }
            } else if idx + 1 < self.queue.len() {
                idx + 1
            } else if self.loop_mode == LoopMode::All {
                0
            } else {
                self.finish_current();
                return;
            };
            self.current_index = Some(next_idx);
            self.play_current();
        }
    }

    pub fn prev(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        if self.get_position() > 3.0 {
            self.seek(0.0);
            return;
        }
        if let Some(idx) = self.current_index {
            let prev_idx = if self.shuffle && self.shuffle_cursor > 0 {
                self.shuffle_cursor -= 1;
                self.shuffle_order[self.shuffle_cursor]
            } else if idx > 0 {
                idx - 1
            } else if self.loop_mode == LoopMode::All {
                self.queue.len() - 1
            } else {
                0
            };
            self.current_index = Some(prev_idx);
            self.play_current();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        if let Some(ref sink) = self.sink {
            sink.set_volume(self.volume);
        }
        #[cfg(all(feature = "device-audio", target_os = "linux"))]
        if let Some(ref playback) = self.alsa_playback {
            playback.set_volume(self.volume);
        }
        crate::system::apply_system_volume(self.volume);
    }

    pub fn set_eq_preset(&mut self, preset: EqPreset) {
        if self.eq_preset == preset {
            return;
        }
        self.eq_preset = preset;
        if self.current_index.is_some() && !self.is_paused {
            self.play_current();
        }
    }

    pub fn get_eq_preset(&self) -> EqPreset {
        self.eq_preset
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.rebuild_shuffle_order();
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
        self.rebuild_shuffle_order();
    }

    pub fn shuffle_enabled(&self) -> bool {
        self.shuffle
    }

    pub fn toggle_loop_mode(&mut self) {
        self.loop_mode = match self.loop_mode {
            LoopMode::None => LoopMode::All,
            LoopMode::All => LoopMode::One,
            LoopMode::One => LoopMode::None,
        };
    }

    pub fn set_loop_mode(&mut self, loop_mode: LoopMode) {
        self.loop_mode = loop_mode;
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn get_position(&self) -> f32 {
        #[cfg(all(feature = "device-audio", target_os = "linux"))]
        if let Some(ref alsa) = self.alsa_playback {
            let frames = alsa.written_frames.load(Ordering::Relaxed);
            let rate = alsa.sample_rate.load(Ordering::Relaxed).max(1);
            return frames as f32 / rate as f32;
        }

        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        if self.last_backend == "rodio" && self.current_loaded {
            if let Some(sink) = &self.sink {
                return sink.get_pos().as_secs_f32();
            }
        }

        let elapsed = if !self.is_paused && self.play_start.is_some() {
            self.accumulated_time + self.play_start.unwrap().elapsed()
        } else {
            self.accumulated_time
        };
        elapsed.as_secs_f32()
    }

    pub fn seek(&mut self, seconds: f32) {
        let duration = self.get_current_track().map(|t| t.duration).unwrap_or(0);
        let seconds = seconds.clamp(0.0, duration as f32);
        let seek_dur = Duration::from_secs_f32(seconds);
        self.accumulated_time = seek_dur;

        #[cfg(all(feature = "device-audio", target_os = "linux"))]
        if self.alsa_playback.is_some() {
            if let Some(track) = self.get_current_track() {
                let path = track.path.clone();
                let title = track.title.clone();
                let artist = track.artist.clone();
                self.alsa_playback = None;
                self.start_alsa_playback(path, title, artist, seconds);
            }
        }

        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        if let Some(ref sink) = self.sink {
            let _ = sink.try_seek(seek_dur);
        }

        if !self.is_paused {
            self.play_start = Some(Instant::now());
        }
    }

    pub fn get_current_track(&self) -> Option<&Track> {
        let idx = self.current_index?;
        if idx < self.queue.len() {
            Some(&self.queue[idx])
        } else {
            None
        }
    }

    pub fn queue_tracks(&self) -> Vec<Track> {
        self.queue.clone()
    }

    pub fn queue_track_ids(&self) -> Vec<i64> {
        self.queue.iter().map(|track| track.id).collect()
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index.filter(|idx| *idx < self.queue.len())
    }

    pub fn is_playing(&self) -> bool {
        #[cfg(all(feature = "device-audio", target_os = "linux"))]
        if self
            .alsa_playback
            .as_ref()
            .is_some_and(|playback| playback.finished.load(Ordering::Relaxed))
        {
            return false;
        }
        !self.is_paused && self.current_index.is_some()
    }

    pub fn queue_add_next(&mut self, track: Track) {
        let insert_index = self
            .current_index()
            .map(|index| index + 1)
            .unwrap_or(0)
            .min(self.queue.len());
        self.queue.insert(insert_index, track);
        if self.current_index.is_none() {
            self.current_index = Some(0);
            self.is_paused = true;
        }
        self.rebuild_shuffle_order();
    }

    pub fn queue_add_later(&mut self, track: Track) {
        self.queue.push(track);
        if self.current_index.is_none() {
            self.current_index = Some(0);
            self.is_paused = true;
        }
        self.rebuild_shuffle_order();
    }

    pub fn queue_remove(&mut self, index: usize) -> Option<Track> {
        if index >= self.queue.len() {
            return None;
        }
        let current_index = self.current_index;
        let was_paused = self.is_paused;
        let removed = self.queue.remove(index);
        if self.queue.is_empty() {
            self.stop();
            self.current_index = None;
            self.is_paused = true;
        } else if let Some(current) = current_index {
            if index == current {
                self.current_index = Some(index.min(self.queue.len() - 1));
                self.play_current();
                if was_paused {
                    self.pause();
                }
            } else if index < current {
                self.current_index = Some(current - 1);
            }
        }
        self.rebuild_shuffle_order();
        Some(removed)
    }

    pub fn queue_move(&mut self, from: usize, to: usize) -> bool {
        if from >= self.queue.len() || to >= self.queue.len() || from == to {
            return false;
        }
        let current_index = self.current_index;
        let track = self.queue.remove(from);
        self.queue.insert(to, track);
        if let Some(current) = current_index {
            self.current_index = Some(if current == from {
                to
            } else if from < current && to >= current {
                current - 1
            } else if from > current && to <= current {
                current + 1
            } else {
                current
            });
        }
        self.rebuild_shuffle_order();
        true
    }

    pub fn queue_clear(&mut self) {
        self.stop();
        self.queue.clear();
        self.current_index = None;
        self.is_paused = true;
        self.shuffle_order.clear();
        self.shuffle_cursor = 0;
    }

    pub fn queue_clear_upcoming(&mut self) {
        if let Some(current) = self.current_index() {
            self.queue.truncate(current + 1);
        } else {
            self.queue.clear();
        }
        self.rebuild_shuffle_order();
    }

    pub fn update_track_favorite(&mut self, id: i64, favorited: bool) {
        for track in &mut self.queue {
            if track.id == id {
                track.favorited = favorited;
            }
        }
    }

    pub fn update_track_rating(&mut self, id: i64, rating: u8) {
        for track in &mut self.queue {
            if track.id == id {
                track.rating = rating.min(5);
            }
        }
    }

    pub fn get_status(&self) -> PlaybackStatus {
        PlaybackStatus {
            track: self.get_current_track().cloned(),
            is_playing: self.is_playing(),
            position: self.get_position(),
            volume: self.volume,
            eq_preset: self.eq_preset,
            shuffle: self.shuffle,
            loop_mode: self.loop_mode,
            queue_len: self.queue.len(),
            queue_index: self.current_index(),
            backend: self.last_backend.clone(),
            selected_pcm: self.selected_pcm.clone(),
            last_error: self.last_error.clone().or_else(|| {
                #[cfg(all(feature = "device-audio", target_os = "linux"))]
                if let Some(playback) = &self.alsa_playback {
                    return playback.error.lock().ok().and_then(|error| error.clone());
                }
                None
            }),
        }
    }

    pub fn tick(&mut self) {
        #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
        if self.last_backend == "rodio" && self.current_loaded && !self.is_paused {
            if self.sink.as_ref().is_some_and(Sink::empty) {
                if self.loop_mode == LoopMode::One {
                    self.play_current();
                } else {
                    self.next();
                }
            }
            return;
        }
        let duration = self.get_current_track().map(|t| t.duration).unwrap_or(0);
        if !self.is_paused && duration > 0 && self.get_position() >= duration as f32 {
            if self.loop_mode == LoopMode::One {
                self.play_current();
            } else {
                self.next();
            }
        }
    }

    fn finish_current(&mut self) {
        let duration = self
            .get_current_track()
            .map(|track| track.duration)
            .unwrap_or(0);
        self.pause();
        self.accumulated_time = Duration::from_secs(duration.max(0) as u64);
    }

    fn rebuild_shuffle_order(&mut self) {
        self.shuffle_order = (0..self.queue.len()).collect();
        let Some(current) = self.current_index.filter(|index| *index < self.queue.len()) else {
            self.shuffle_cursor = 0;
            return;
        };
        if let Some(position) = self
            .shuffle_order
            .iter()
            .position(|index| *index == current)
        {
            self.shuffle_order.swap(0, position);
        }
        for index in (2..=self.shuffle_order.len()).rev() {
            self.shuffle_seed = self
                .shuffle_seed
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let swap_with = 1 + (self.shuffle_seed as usize % (index - 1));
            self.shuffle_order.swap(index - 1, swap_with);
        }
        self.shuffle_cursor = 0;
    }

    #[cfg(all(feature = "device-audio", target_os = "linux"))]
    fn prefers_direct_alsa(&self) -> bool {
        std::env::var("MUSICOS_AUDIO_BACKEND")
            .map(|value| value.eq_ignore_ascii_case("alsa"))
            .unwrap_or(true)
    }

    #[cfg(all(feature = "device-audio", target_os = "linux"))]
    fn start_alsa_playback(
        &mut self,
        path: String,
        title: String,
        artist: String,
        start_seconds: f32,
    ) -> bool {
        let was_paused = self.is_paused;
        match AlsaPlayback::start(path.clone(), self.volume, self.eq_preset, start_seconds) {
            Ok(playback) => {
                if was_paused {
                    playback.paused.store(true, Ordering::Relaxed);
                }
                self.alsa_playback = Some(playback);
                self.is_paused = was_paused;
                self.last_backend = "alsa".to_string();
                self.selected_pcm =
                    alsa_pcm_candidates(std::env::var("MUSICOS_ALSA_DEVICE").ok().as_deref())
                        .into_iter()
                        .next();
                self.last_error = None;
                self.play_start = if was_paused {
                    None
                } else {
                    Some(Instant::now())
                };
                self.accumulated_time = Duration::from_secs_f32(start_seconds);
                self.current_loaded = true;
                log::info!(
                    "Playing track through ALSA from {}s: {} - {}",
                    start_seconds,
                    title,
                    artist
                );
                true
            }
            Err(err) => {
                let message = format!("direct ALSA playback could not start for {path}: {err}");
                log::error!("{message}");
                self.last_error = Some(message);
                false
            }
        }
    }
}

fn prefers_silent_backend() -> bool {
    std::env::var("MUSICOS_TEST_AUDIO_BACKEND")
        .or_else(|_| std::env::var("MUSICOS_AUDIO_BACKEND"))
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "none" | "silent" | "stub" | "off"
            )
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub track: Option<Track>,
    pub is_playing: bool,
    pub position: f32,
    pub volume: f32,
    pub eq_preset: EqPreset,
    pub shuffle: bool,
    pub loop_mode: LoopMode,
    pub queue_len: usize,
    pub queue_index: Option<usize>,
    pub backend: String,
    pub selected_pcm: Option<String>,
    pub last_error: Option<String>,
}

#[cfg(all(feature = "device-audio", target_os = "linux"))]
struct AlsaPlayback {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    volume_bits: Arc<AtomicU32>,
    written_frames: Arc<AtomicU64>,
    sample_rate: Arc<AtomicU32>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(all(feature = "device-audio", target_os = "linux"))]
impl AlsaPlayback {
    fn start(
        path: String,
        volume: f32,
        preset: EqPreset,
        start_seconds: f32,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let volume_bits = Arc::new(AtomicU32::new(volume.clamp(0.0, 1.0).to_bits()));
        let written_frames = Arc::new(AtomicU64::new(0));
        let sample_rate = Arc::new(AtomicU32::new(44100));
        let finished = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));

        let thread_stop = Arc::clone(&stop);
        let thread_paused = Arc::clone(&paused);
        let thread_volume = Arc::clone(&volume_bits);
        let thread_written = Arc::clone(&written_frames);
        let thread_rate = Arc::clone(&sample_rate);
        let thread_finished = Arc::clone(&finished);
        let thread_error = Arc::clone(&error);
        let thread_path = path.clone();

        let worker = std::thread::Builder::new()
            .name("musicos-alsa-playback".to_string())
            .spawn(move || {
                if let Err(err) = play_file_with_alsa(
                    &thread_path,
                    preset,
                    thread_stop,
                    thread_paused,
                    thread_volume,
                    thread_written,
                    thread_rate,
                    start_seconds,
                ) {
                    log::error!("ALSA playback thread stopped for {thread_path}: {err}");
                    if let Ok(mut playback_error) = thread_error.lock() {
                        *playback_error = Some(err);
                    }
                }
                thread_finished.store(true, Ordering::Relaxed);
            })
            .map_err(|err| format!("thread spawn failed: {err}"))?;

        Ok(Self {
            stop,
            paused,
            volume_bits,
            written_frames,
            sample_rate,
            finished,
            error,
            worker: Some(worker),
        })
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    fn set_volume(&self, volume: f32) {
        self.volume_bits
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

#[cfg(all(feature = "device-audio", target_os = "linux"))]
impl Drop for AlsaPlayback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(all(feature = "device-audio", target_os = "linux"))]
fn play_file_with_alsa(
    path: &str,
    preset: EqPreset,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    volume_bits: Arc<AtomicU32>,
    written_frames: Arc<AtomicU64>,
    sample_rate: Arc<AtomicU32>,
    start_seconds: f32,
) -> Result<(), String> {
    let file = File::open(path).map_err(|err| format!("open failed: {err}"))?;
    let reader = BufReader::new(file);
    let mut decoder = Decoder::new(reader).map_err(|err| format!("decode failed: {err}"))?;
    if start_seconds > 0.0 {
        let _ = decoder.try_seek(Duration::from_secs_f32(start_seconds));
    }
    let mut source = EqSource::new(decoder, preset);
    let rate = source.sample_rate();
    sample_rate.store(rate, Ordering::Relaxed);
    written_frames.store((start_seconds * rate as f32) as u64, Ordering::Relaxed);
    let channels = source.channels().max(1);
    let device = std::env::var("MUSICOS_ALSA_DEVICE").ok();
    let candidates = alsa_pcm_candidates(device.as_deref());

    let mut last_err = String::new();
    for candidate in candidates {
        match write_source_to_alsa(
            &candidate,
            &mut source,
            rate,
            channels,
            &stop,
            &paused,
            &volume_bits,
            &written_frames,
        ) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_err = format!("{candidate}: {err}");
                log::error!("ALSA device failed: {last_err}");
            }
        }

        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
    }

    Err(if last_err.is_empty() {
        "no ALSA devices tried".to_string()
    } else {
        last_err
    })
}

#[cfg(all(feature = "device-audio", target_os = "linux"))]
pub fn alsa_pcm_candidates(preferred: Option<&str>) -> Vec<String> {
    let mut devices = Vec::new();
    if let Some(device) = preferred.filter(|value| !value.trim().is_empty()) {
        devices.push(device.to_string());
    }
    for device in [
        "default",
        "plughw:0,0",
        "hw:0,0",
        "sysdefault",
        "plughw:1,0",
        "hw:1,0",
    ] {
        if !devices.iter().any(|existing| existing == device) {
            devices.push(device.to_string());
        }
    }
    devices
}

#[cfg(not(all(feature = "device-audio", target_os = "linux")))]
pub fn alsa_pcm_candidates(preferred: Option<&str>) -> Vec<String> {
    let mut devices = Vec::new();
    if let Some(device) = preferred.filter(|value| !value.trim().is_empty()) {
        devices.push(device.to_string());
    }
    for device in [
        "default",
        "plughw:0,0",
        "hw:0,0",
        "sysdefault",
        "plughw:1,0",
        "hw:1,0",
    ] {
        if !devices.iter().any(|existing| existing == device) {
            devices.push(device.to_string());
        }
    }
    devices
}

#[cfg(all(feature = "device-audio", target_os = "linux"))]
fn write_source_to_alsa<S>(
    device: &str,
    source: &mut S,
    sample_rate: u32,
    channels: u16,
    stop: &AtomicBool,
    paused: &AtomicBool,
    volume_bits: &AtomicU32,
    written_frames: &AtomicU64,
) -> Result<(), String>
where
    S: Source<Item = i16>,
{
    use alsa::pcm::{Access, Format, HwParams, State, PCM};
    use alsa::{Direction, ValueOr};

    let pcm = PCM::new(device, Direction::Playback, false).map_err(|err| err.to_string())?;
    {
        let hwp = HwParams::any(&pcm).map_err(|err| err.to_string())?;
        hwp.set_channels(channels as u32)
            .map_err(|err| format!("channels {channels}: {err}"))?;
        hwp.set_rate(sample_rate, ValueOr::Nearest)
            .map_err(|err| format!("rate {sample_rate}: {err}"))?;
        hwp.set_format(Format::s16())
            .map_err(|err| format!("format s16: {err}"))?;
        hwp.set_access(Access::RWInterleaved)
            .map_err(|err| format!("interleaved access: {err}"))?;
        let _ = hwp.set_period_size_near(1024, ValueOr::Nearest);
        let _ = hwp.set_buffer_size_near(4096);
        pcm.hw_params(&hwp).map_err(|err| err.to_string())?;
    }

    pcm.prepare().map_err(|err| err.to_string())?;
    log::info!("ALSA PCM ready: {device} {sample_rate}Hz {}ch", channels);

    let io = pcm.io_i16().map_err(|err| err.to_string())?;
    let channels_usize = channels as usize;
    let samples_per_chunk = channels_usize * 1024;
    let mut buffer = Vec::with_capacity(samples_per_chunk);
    let mut current_volume = f32::from_bits(volume_bits.load(Ordering::Relaxed)).clamp(0.0, 1.0);

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if paused.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(16));
            continue;
        }

        buffer.clear();
        let target_volume = f32::from_bits(volume_bits.load(Ordering::Relaxed)).clamp(0.0, 1.0);
        for _ in 0..samples_per_chunk {
            let Some(sample) = source.next() else {
                break;
            };
            if (current_volume - target_volume).abs() > 0.0001 {
                let step = 0.0005 * (target_volume - current_volume).signum();
                current_volume += step;
                if (step > 0.0 && current_volume > target_volume)
                    || (step < 0.0 && current_volume < target_volume)
                {
                    current_volume = target_volume;
                }
            } else {
                current_volume = target_volume;
            }
            let scaled = (sample as f32 * current_volume).clamp(i16::MIN as f32, i16::MAX as f32);
            buffer.push(scaled as i16);
        }
        if buffer.is_empty() {
            break;
        }

        let mut offset = 0usize;
        while offset < buffer.len() && !stop.load(Ordering::Relaxed) {
            match io.writei(&buffer[offset..]) {
                Ok(frames) => {
                    offset += frames * channels_usize;
                    written_frames.fetch_add(frames as u64, Ordering::Relaxed);
                    if pcm.state() != State::Running {
                        let _ = pcm.start();
                    }
                }
                Err(err) if err.errno() == libc::EPIPE || pcm.state() == State::XRun => {
                    log::info!("ALSA underrun recovered on {device}");
                    pcm.prepare().map_err(|err| err.to_string())?;
                }
                Err(err) => return Err(err.to_string()),
            }
        }
    }

    let _ = pcm.drain();
    Ok(())
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
struct EqSource<S> {
    input: S,
    preset: EqPreset,
    gains: EqGains,
    low_state: Vec<f32>,
    high_prev_in: Vec<f32>,
    high_prev_out: Vec<f32>,
    sample_index: usize,
    low_alpha: f32,
    high_alpha: f32,
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
impl<S> EqSource<S>
where
    S: Source<Item = i16>,
{
    fn new(input: S, preset: EqPreset) -> Self {
        let sample_rate = input.sample_rate().max(1) as f32;
        let channels = input.channels().max(1) as usize;
        let low_cut = 220.0;
        let high_cut = 4_000.0;
        let dt = 1.0 / sample_rate;
        let low_rc = 1.0 / (std::f32::consts::TAU * low_cut);
        let high_rc = 1.0 / (std::f32::consts::TAU * high_cut);
        Self {
            input,
            preset,
            gains: preset.gains(),
            low_state: vec![0.0; channels],
            high_prev_in: vec![0.0; channels],
            high_prev_out: vec![0.0; channels],
            sample_index: 0,
            low_alpha: (dt / (low_rc + dt)).clamp(0.0, 1.0),
            high_alpha: (high_rc / (high_rc + dt)).clamp(0.0, 1.0),
        }
    }
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
impl<S> Iterator for EqSource<S>
where
    S: Source<Item = i16>,
{
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.input.next()?;
        if self.preset == EqPreset::Flat {
            return Some(sample);
        }

        let ch = self.sample_index % self.low_state.len();
        self.sample_index = self.sample_index.wrapping_add(1);

        let x = sample as f32 / i16::MAX as f32;
        self.low_state[ch] += self.low_alpha * (x - self.low_state[ch]);
        let low = self.low_state[ch];
        let high = self.high_alpha * (self.high_prev_out[ch] + x - self.high_prev_in[ch]);
        self.high_prev_in[ch] = x;
        self.high_prev_out[ch] = high;
        let mid = x - low - high;

        let out = (low * self.gains.low + mid * self.gains.mid + high * self.gains.high)
            * self.gains.preamp;
        Some((out.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

#[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
impl<S> Source for EqSource<S>
where
    S: Source<Item = i16>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.input.try_seek(pos)?;
        self.low_state.fill(0.0);
        self.high_prev_in.fill(0.0);
        self.high_prev_out.fill(0.0);
        self.sample_index = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;
    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    use std::{fs, fs::File, io::BufReader};

    use super::{alsa_pcm_candidates, AudioPlayer};
    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    use super::{Decoder, EqPreset, EqSource, Source};
    use crate::database::Track;

    fn track(id: i64) -> Track {
        Track {
            id,
            path: format!("{id}.wav"),
            title: format!("Track {id}"),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            track_number: id as i32,
            duration: 120,
            file_size: 0,
            modified_at: 0,
            indexed_at: 0,
            cover_art_path: None,
            cover_art_updated_at: None,
            cover_art_source_modified_at: None,
            favorited: false,
            play_count: 0,
            last_played: None,
            rating: 0,
        }
    }

    #[test]
    fn alsa_pcm_candidates_preserve_preferred_device_and_defaults() {
        let candidates = alsa_pcm_candidates(Some("plughw:2,0"));
        assert_eq!(candidates.first().map(String::as_str), Some("plughw:2,0"));
        assert!(candidates.iter().any(|device| device == "default"));
        assert!(candidates.iter().any(|device| device == "plughw:0,0"));
        assert!(candidates.iter().any(|device| device == "hw:0,0"));
    }

    #[cfg(any(feature = "desktop-audio", feature = "device-audio"))]
    #[test]
    fn pcm_capture_proves_the_decoder_produces_non_silent_samples() {
        let path =
            std::env::temp_dir().join(format!("musicos-pcm-capture-{}.wav", std::process::id()));
        let sample_rate = 8_000_u32;
        let sample_count = 800_u32;
        let data_len = sample_count * 2;
        let mut bytes = Vec::with_capacity((44 + data_len) as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for index in 0..sample_count {
            let sample = if index % 2 == 0 {
                12_000_i16
            } else {
                -12_000_i16
            };
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(&path, bytes).unwrap();
        let capture = super::decode_pcm_capture(&path, 2_000).unwrap();
        assert_eq!(capture.sample_rate, sample_rate);
        assert_eq!(capture.channels, 1);
        assert!(capture.samples >= sample_count as usize);
        assert!(capture.peak > 0.1);
        assert!(capture.non_silent_samples > 700);
        let decoder = Decoder::new(BufReader::new(File::open(&path).unwrap())).unwrap();
        let mut equalized = EqSource::new(decoder, EqPreset::Flat);
        assert!(equalized.try_seek(Duration::from_millis(50)).is_ok());
        assert!(equalized.next().is_some());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn alsa_pcm_candidates_do_not_duplicate_preferred_default() {
        let candidates = alsa_pcm_candidates(Some("default"));
        assert_eq!(
            candidates
                .iter()
                .filter(|device| device.as_str() == "default")
                .count(),
            1
        );
    }

    #[test]
    fn shuffle_visits_every_track_before_repeating() {
        let mut player = AudioPlayer::silent("test");
        player.set_queue((1..=6).map(track).collect(), 0);
        player.set_shuffle(true);
        let mut visited = HashSet::from([player.current_index().unwrap()]);
        for _ in 1..6 {
            player.next();
            visited.insert(player.current_index().unwrap());
        }
        assert_eq!(visited.len(), 6);
        let last = player.current_index();
        player.next();
        assert_eq!(player.current_index(), last);
        assert_eq!(player.get_position(), 120.0);
    }

    #[test]
    fn previous_restarts_after_three_seconds() {
        let mut player = AudioPlayer::silent("test");
        player.set_queue(vec![track(1), track(2)], 1);
        player.accumulated_time = Duration::from_secs(5);
        player.prev();
        assert_eq!(player.current_index(), Some(1));
        // The silent backend still measures wall-clock time while playing, so
        // a few microseconds may elapse between the restart and this read.
        assert!(player.get_position() < 0.01);
    }

    #[test]
    fn moving_duplicate_ids_preserves_the_active_queue_slot() {
        let mut player = AudioPlayer::silent("test");
        player.set_queue(vec![track(1), track(1), track(2)], 1);
        assert!(player.queue_move(0, 2));
        assert_eq!(player.current_index(), Some(0));
    }

    #[test]
    fn clear_upcoming_and_rating_updates_preserve_current_playback_context() {
        let mut player = AudioPlayer::silent("test");
        player.set_queue(vec![track(1), track(2), track(3), track(4)], 1);
        player.queue_clear_upcoming();
        assert_eq!(player.queue_track_ids(), vec![1, 2]);
        assert_eq!(player.current_index(), Some(1));

        player.update_track_rating(2, 4);
        assert_eq!(player.get_current_track().unwrap().rating, 4);
    }

    #[cfg(not(any(feature = "desktop-audio", feature = "device-audio")))]
    #[test]
    fn restored_queue_loads_current_track_before_resuming() {
        let mut player = AudioPlayer::silent("test");
        player.restore_queue_paused(vec![track(1), track(2)], 1, 5.0);
        assert!(!player.current_loaded);
        assert!(!player.is_playing());

        player.play_pause();

        assert!(player.current_loaded);
        assert!(player.is_playing());
        assert_eq!(player.current_index(), Some(1));
        assert!((5.0..5.1).contains(&player.get_position()));
    }
}
