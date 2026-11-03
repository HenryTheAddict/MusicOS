use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Beat {
    pub track_id: Option<i64>,
    pub bpm: Option<f64>,
    pub offset: Option<f64>,
    pub confidence: f64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub request_id: u64,
    pub preferences: musicos_visualizer::control::Preferences,
    pub beat: Beat,
    pub reduced: bool,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub request_id: u64,
    pub phase: String,
    pub preset: String,
    pub error: Option<String>,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
    pub missed_frames: u64,
    pub submission_ms: f64,
    pub beat_fallback: bool,
    pub display_pacing: Option<DisplayPacing>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPacing {
    pub frames: u64,
    pub samples: usize,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub missed: u64,
}
#[derive(Default)]
struct Pacing {
    last: Option<f64>,
    frames: u64,
    missed: u64,
    intervals: std::collections::VecDeque<f64>,
}
impl Pacing {
    fn present(&mut self, time: f64) {
        if !time.is_finite() || time <= 0. || self.last.is_some_and(|last| time <= last) {
            return;
        }
        self.frames += 1;
        if let Some(last) = self.last {
            let delta = (time - last) * 1000.;
            if delta < 250. {
                if self.intervals.len() == 600 {
                    self.intervals.pop_front();
                }
                self.intervals.push_back(delta);
                if delta > 25. {
                    self.missed += 1;
                }
            }
        }
        self.last = Some(time);
    }
    fn report(&self) -> DisplayPacing {
        let mut samples: Vec<_> = self.intervals.iter().copied().collect();
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        DisplayPacing {
            frames: self.frames,
            samples: n,
            missed: self.missed,
            mean_ms: if n > 0 {
                samples.iter().sum::<f64>() / n as f64
            } else {
                0.
            },
            p50_ms: samples.get(n / 2).copied().unwrap_or(0.),
            p95_ms: samples
                .get(n.saturating_sub(1) * 95 / 100)
                .copied()
                .unwrap_or(0.),
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub action: String,
    pub settings: Option<Settings>,
}
/// Native surface ownership. The audio engine and ordinary webview stay intact.
#[cfg(target_os = "macos")]
pub mod surface {
    use super::{Pacing, Settings, Status};
    use anyhow::{Context, Result};
    use musicos_desktop::local::playback::LocalPlaybackClient;
    use musicos_visualizer::gpu::Display;
    use musicos_visualizer::wgpu;
    use musicos_visualizer::{
        control::{Clock, Coordinator},
        engine::{internal_size, Engine},
        model::Audio,
        renderer::Presenter,
    };
    use std::{
        sync::{
            atomic::{AtomicBool, AtomicU32, Ordering},
            Arc, Condvar, Mutex,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };
    use tauri::{Emitter, WebviewWindow};
    // Advance an absolute deadline instead of adding each scheduler wakeup's
    // delay to the next frame. Never run catch-up bursts after a slow frame.
    fn pace(next: &mut Instant) {
        *next += Duration::from_secs_f64(1. / 60.);
        let now = Instant::now();
        if now > *next {
            *next = now;
        }
        thread::sleep(next.saturating_duration_since(now));
    }
    struct State {
        width: AtomicU32,
        height: AtomicU32,
        visible: AtomicBool,
        stop: AtomicBool,
        wake: (Mutex<()>, Condvar),
        changed: std::sync::atomic::AtomicU64,
        settings: Mutex<Settings>,
        status: Mutex<Status>,
        pacing: Mutex<Pacing>,
        events: Option<tauri::AppHandle>,
    }
    extern "C" fn update(raw: *mut std::ffi::c_void, w: u32, h: u32, visible: bool) {
        let s = unsafe { &*raw.cast::<State>() };
        let _lock = s.wake.0.lock().unwrap_or_else(|e| e.into_inner());
        s.changed.fetch_add(1, Ordering::Release);
        s.width.store(w, Ordering::Relaxed);
        s.height.store(h, Ordering::Relaxed);
        s.visible.store(visible, Ordering::Relaxed);
        s.wake.1.notify_all();
    }
    extern "C" fn presented(raw: *mut std::ffi::c_void, time: f64) {
        let state = unsafe { &*raw.cast::<State>() };
        state
            .pacing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .present(time);
    }
    extern "C" fn release(raw: *mut std::ffi::c_void) {
        unsafe {
            drop(Arc::from_raw(raw.cast::<State>()));
        }
    }
    unsafe extern "C" {
        fn musicos_visualizer_attach(
            web: *mut std::ffi::c_void,
            context: *mut std::ffi::c_void,
            update: extern "C" fn(*mut std::ffi::c_void, u32, u32, bool),
            release: extern "C" fn(*mut std::ffi::c_void),
            presented: extern "C" fn(*mut std::ffi::c_void, f64),
        ) -> *mut std::ffi::c_void;
        fn musicos_visualizer_detach(view: *mut std::ffi::c_void);
    }
    pub struct Session {
        state: Arc<State>,
        thread: Option<JoinHandle<()>>,
        view: usize,
    }
    impl Session {
        /// Must be called through with_webview (AppKit main thread).
        pub unsafe fn proof(web: *mut std::ffi::c_void) -> Result<Self> {
            let state = Arc::new(State {
                width: AtomicU32::new(1),
                height: AtomicU32::new(1),
                visible: AtomicBool::new(false),
                stop: AtomicBool::new(false),
                wake: (Mutex::new(()), Condvar::new()),
                changed: std::sync::atomic::AtomicU64::new(1),
                settings: Mutex::new(Settings::default()),
                status: Mutex::new(Status::default()),
                pacing: Mutex::new(Pacing::default()),
                events: None,
            });
            let context = Arc::into_raw(state.clone()).cast_mut().cast();
            let view =
                unsafe { musicos_visualizer_attach(web, context, update, release, presented) };
            if view.is_null() {
                release(context);
                anyhow::bail!("Could not attach a native GPU surface");
            }
            let result = unsafe {
                Display::attach(
                    view,
                    state.width.load(Ordering::Relaxed),
                    state.height.load(Ordering::Relaxed),
                )
            };
            let mut display = match result {
                Ok(d) => d,
                Err(e) => {
                    unsafe { musicos_visualizer_detach(view) };
                    return Err(e);
                }
            };
            let control = state.clone();
            let thread = thread::Builder::new()
                .name("musicos-visualizer".into())
                .spawn(move || {
                    let start = Instant::now();
                    let mut last = Instant::now();
                    let mut deadline = last;
                    let (w, h) = internal_size(display.config.width, display.config.height, 1.);
                    let mut engine = match Engine::new(&display.gpu, "boxes", w, h, 42) {
                        Ok(engine) => engine,
                        Err(e) => {
                            eprintln!("Native preset proof failed: {e}");
                            return;
                        }
                    };
                    let presenter = Presenter::new(&display.gpu.device, display.config.format);
                    let mut audio = Audio::default();
                    while !control.stop.load(Ordering::Acquire) {
                        let now = Instant::now();
                        if !control.visible.load(Ordering::Relaxed) {
                            let lock = control.wake.0.lock().unwrap();
                            if !control.visible.load(Ordering::Relaxed)
                                && !control.stop.load(Ordering::Acquire)
                            {
                                drop(control.wake.1.wait(lock));
                            }
                            last = Instant::now();
                            deadline = last;
                            continue;
                        }
                        display.resize(
                            control.width.load(Ordering::Relaxed),
                            control.height.load(Ordering::Relaxed),
                        );
                        let (w, h) = internal_size(display.config.width, display.config.height, 1.);
                        engine.resize(&display.gpu, w, h);
                        let settings = control
                            .settings
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();
                        if settings.preferences.preset != engine.preset() {
                            if let Err(e) =
                                engine.select(&display.gpu, &settings.preferences.preset, 2.)
                            {
                                eprintln!("Preset proof failed: {e}");
                                return;
                            }
                        }
                        for c in 0..2 {
                            for i in 0..512 {
                                audio.waveform[c][i] =
                                    ((start.elapsed().as_secs_f64() * 48000. + i as f64 * 2.)
                                        * std::f64::consts::TAU
                                        * (110. + c as f64 * 55.)
                                        / 48000.)
                                        .sin() as f32
                                        * 0.4;
                                audio.spectrum[c][i] = 0.2;
                            }
                        }
                        audio.bands = [
                            1. + (start.elapsed().as_secs_f64() * 4.).sin() * 0.4,
                            1.,
                            1.,
                        ];
                        if let Err(e) = engine.render(
                            &display.gpu,
                            now.duration_since(last).as_secs_f64().min(0.1),
                            &audio,
                            0.,
                        ) {
                            eprintln!("Preset proof failed: {e}");
                            return;
                        }
                        last = now;
                        match display.surface.get_current_texture() {
                            Ok(frame) => {
                                presenter.draw(
                                    &display.gpu,
                                    &frame.texture.create_view(&Default::default()),
                                    engine.image(),
                                    engine.image(),
                                    0.,
                                    1.,
                                );
                                frame.present();
                            }
                            Err(e) => {
                                eprintln!("Native surface proof failed: {e}");
                                return;
                            }
                        }
                        pace(&mut deadline);
                    }
                    if let Some(path) = std::env::var_os("MUSICOS_VISUALIZER_PROOF_REPORT") {
                        let report = control
                            .pacing
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .report();
                        if let Ok(bytes) = serde_json::to_vec_pretty(&report) {
                            let _ = std::fs::write(path, bytes);
                        }
                    }
                })
                .context("Could not start GPU renderer")?;
            Ok(Self {
                state,
                thread: Some(thread),
                view: view as usize,
            })
        }
        pub fn update(&self, mut settings: Settings) {
            settings.preferences = settings.preferences.normalize();
            let _lock = self.state.wake.0.lock().unwrap_or_else(|e| e.into_inner());
            let mut current = self
                .state
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if settings.request_id < current.request_id {
                return;
            }
            *current = settings;
            drop(current);
            self.state.changed.fetch_add(1, Ordering::Release);
            self.state.wake.1.notify_all();
        }
        pub fn status(&self) -> Status {
            let pacing = self
                .state
                .pacing
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .report();
            let mut status = self
                .state
                .status
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            status.missed_frames = pacing.missed;
            status.display_pacing = Some(pacing);
            status
        }
        /// The NSView and CAMetalLayer are created on AppKit's main thread;
        /// simulation, feature reads, and GPU submission live on this worker.
        pub unsafe fn render(
            web: *mut std::ffi::c_void,
            client: LocalPlaybackClient,
            settings: Settings,
            events: tauri::AppHandle,
        ) -> Result<Self> {
            let request_id = settings.request_id;
            let preset = settings.preferences.preset.clone();
            let state = Arc::new(State {
                width: AtomicU32::new(1),
                height: AtomicU32::new(1),
                visible: AtomicBool::new(false),
                stop: AtomicBool::new(false),
                wake: (Mutex::new(()), Condvar::new()),
                changed: std::sync::atomic::AtomicU64::new(1),
                settings: Mutex::new(settings),
                events: Some(events),
                pacing: Mutex::new(Pacing::default()),
                status: Mutex::new(Status {
                    phase: "starting".into(),
                    request_id,
                    preset,
                    ..Default::default()
                }),
            });
            let context = Arc::into_raw(state.clone()).cast_mut().cast();
            let view =
                unsafe { musicos_visualizer_attach(web, context, update, release, presented) };
            if view.is_null() {
                release(context);
                anyhow::bail!("Could not attach the native visualizer");
            }
            let display = match unsafe {
                Display::attach(
                    view,
                    state.width.load(Ordering::Relaxed),
                    state.height.load(Ordering::Relaxed),
                )
            } {
                Ok(d) => d,
                Err(e) => {
                    unsafe { musicos_visualizer_detach(view) };
                    return Err(e);
                }
            };
            let control = state.clone();
            let failure = control.clone();
            display
                .gpu
                .device
                .on_uncaptured_error(Box::new(move |error| {
                    fail(&failure, &error.to_string());
                }));
            let failure = control.clone();
            display
                .gpu
                .device
                .set_device_lost_callback(move |reason, message| {
                    if reason == wgpu::DeviceLostReason::Destroyed {
                        return;
                    }
                    fail(&failure, &format!("Graphics device unavailable: {message}"));
                });
            let worker = thread::Builder::new()
                .name("musicos-visualizer".into())
                .spawn(move || {
                    let result = run(display, &control, &client);
                    if let Some(tap) = client.visualizer_audio() {
                        tap.set_active(false);
                    }
                    if let Err(error) = result {
                        let mut status = control.status.lock().unwrap_or_else(|e| e.into_inner());
                        status.phase = "unavailable".into();
                        status.error = Some(error.to_string());
                        if let Some(app) = &control.events {
                            let _ =
                                app.emit_to("main", "musicos-visualizer-status", status.clone());
                        }
                    }
                });
            match worker {
                Ok(thread) => Ok(Self {
                    state,
                    thread: Some(thread),
                    view: view as usize,
                }),
                Err(error) => {
                    unsafe { musicos_visualizer_detach(view) };
                    Err(error.into())
                }
            }
        }
        pub unsafe fn close(mut self, _web: *mut std::ffi::c_void) {
            {
                let _lock = self.state.wake.0.lock().unwrap_or_else(|e| e.into_inner());
                self.state.stop.store(true, Ordering::Release);
                self.state.wake.1.notify_all();
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            unsafe { musicos_visualizer_detach(std::mem::take(&mut self.view) as *mut _) };
        }
    }
    impl Drop for Session {
        fn drop(&mut self) {
            {
                let _lock = self.state.wake.0.lock().unwrap_or_else(|e| e.into_inner());
                self.state.stop.store(true, Ordering::Release);
                self.state.wake.1.notify_all();
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            if self.view != 0 {
                unsafe { musicos_visualizer_detach(std::mem::take(&mut self.view) as *mut _) }
            }
        }
    }
    fn fail(state: &State, message: &str) {
        let _wake = state.wake.0.lock().unwrap_or_else(|e| e.into_inner());
        let mut status = state.status.lock().unwrap_or_else(|e| e.into_inner());
        status.phase = "unavailable".into();
        status.error = Some(message.to_owned());
        if let Some(app) = &state.events {
            let _ = app.emit_to("main", "musicos-visualizer-status", status.clone());
        }
        state.stop.store(true, Ordering::Release);
        state.wake.1.notify_all();
    }
    fn wait(state: &State, revision: u64) {
        let lock = state.wake.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.changed.load(Ordering::Acquire) == revision && !state.stop.load(Ordering::Acquire)
        {
            drop(state.wake.1.wait(lock));
        }
    }
    fn run(mut display: Display, state: &State, client: &LocalPlaybackClient) -> Result<()> {
        let initial = state
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let (w, h) = internal_size(
            state.width.load(Ordering::Relaxed),
            state.height.load(Ordering::Relaxed),
            1.,
        );
        let mut engine = Engine::new(&display.gpu, &initial.preferences.preset, w, h, 42)?;
        let present = Presenter::new(&display.gpu.device, display.config.format);
        let mut coordinator = Coordinator::default();
        let mut seen = u64::MAX;
        let mut last = Instant::now();
        let mut deadline = last;
        let mut receipt = Instant::now();
        let mut previous = client.visualizer_clock();
        let mut quality = 1.;
        let mut slow = 0_u32;
        let mut total = 0.;
        let mut paused_at: Option<Instant> = None;
        let catalog = musicos_visualizer::catalog::catalog();
        let mut requested = initial.preferences.preset.clone();
        while !state.stop.load(Ordering::Acquire) {
            let revision = state.changed.load(Ordering::Acquire);
            let settings = state
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let snapshot = client.visualizer_clock();
            let visible = state.visible.load(Ordering::Relaxed);
            let now = Instant::now();
            if snapshot.playing {
                paused_at = None;
            } else if previous.playing || paused_at.is_none() {
                paused_at = Some(now);
            }
            let settling = !settings.reduced
                && paused_at.is_some_and(|at| at.elapsed() < Duration::from_millis(180));
            let moving = visible && snapshot.playing && !settings.reduced;
            if let Some(tap) = client.visualizer_audio() {
                tap.set_active(moving);
            }
            if !visible || (!moving && !settling && !engine.transitioning() && seen == revision) {
                state.status.lock().unwrap_or_else(|e| e.into_inner()).phase =
                    if visible { "frozen" } else { "hidden" }.into();
                wait(state, revision);
                last = Instant::now();
                deadline = last;
                continue;
            }
            if settings.preferences.preset != requested {
                requested = settings.preferences.preset.clone();
                engine.select(
                    &display.gpu,
                    &settings.preferences.preset,
                    if settings.reduced { 0. } else { 2. },
                )?;
            }
            let dt = now.duration_since(last).as_secs_f64().clamp(1. / 144., 0.1);
            last = now;
            seen = revision;
            if snapshot.position != previous.position
                || snapshot.track_id != previous.track_id
                || snapshot.playing != previous.playing
                || snapshot.speed != previous.speed
                || snapshot.entry != previous.entry
            {
                receipt = now;
                previous = snapshot;
            }
            let matched = settings.beat.track_id == snapshot.track_id;
            let clock = Clock {
                entry: snapshot.entry,
                position: snapshot.position
                    + if snapshot.playing {
                        receipt.elapsed().as_secs_f64().min(0.25) * snapshot.speed
                    } else {
                        0.
                    },
                speed: snapshot.speed,
                playing: snapshot.playing,
                bpm: if matched { settings.beat.bpm } else { None },
                beat_offset: if matched { settings.beat.offset } else { None },
                confidence: if matched {
                    settings.beat.confidence
                } else {
                    0.
                },
            };
            if let Some(change) = coordinator.step(&settings.preferences, clock, dt) {
                let current = catalog
                    .iter()
                    .position(|p| p.id == engine.preset())
                    .unwrap_or(0);
                engine.select(
                    &display.gpu,
                    &catalog[(current + 1) % catalog.len()].id,
                    if settings.reduced { 0. } else { change.seconds },
                )?;
            }
            display.resize(
                state.width.load(Ordering::Relaxed),
                state.height.load(Ordering::Relaxed),
            );
            let (w, h) = internal_size(display.config.width, display.config.height, quality);
            engine.resize(&display.gpu, w, h);
            let audio = client
                .visualizer_audio()
                .map(|tap| tap.latest())
                .map(|f| Audio {
                    waveform: f.waveform,
                    spectrum: f.spectrum,
                    bands: f.bands,
                    attenuated: f.attenuated,
                })
                .unwrap_or_default();
            let begin = Instant::now();
            engine.render_timed(
                &display.gpu,
                if moving || settling { dt } else { 0. },
                dt,
                &audio,
                if settings.preferences.beat_sync {
                    clock.accent()
                } else {
                    0.
                },
            )?;
            let cpu = begin.elapsed().as_secs_f64() * 1000.;
            total += cpu;
            let frame = match display.surface.get_current_texture() {
                Ok(f) => f,
                Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                    display
                        .surface
                        .configure(&display.gpu.device, &display.config);
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            let view = frame.texture.create_view(&Default::default());
            present.draw(&display.gpu, &view, engine.image(), engine.image(), 0., 1.);
            frame.present();
            let mut status = state.status.lock().unwrap_or_else(|e| e.into_inner());
            let old = (
                status.phase.clone(),
                status.preset.clone(),
                status.beat_fallback,
                status.request_id,
            );
            status.phase = if moving { "running" } else { "frozen" }.into();
            status.preset = engine.preset().into();
            status.request_id = settings.request_id;
            status.frames = engine.rendered;
            status.width = w;
            status.height = h;
            status.submission_ms = total / engine.rendered.max(1) as f64;
            status.beat_fallback = settings.preferences.rotation
                == musicos_visualizer::control::Rotation::Beats
                && clock.period().is_none();
            status.missed_frames = state
                .pacing
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .missed;
            if old
                != (
                    status.phase.clone(),
                    status.preset.clone(),
                    status.beat_fallback,
                    status.request_id,
                )
            {
                if let Some(app) = &state.events {
                    let _ = app.emit_to("main", "musicos-visualizer-status", status.clone());
                }
            }
            drop(status);
            // Drawable acquisition can carry the GPU back-pressure while CPU
            // encoding stays cheap. Include it before reducing quality, so a
            // GPU-bound preset cannot remain stuck below the target cadence.
            if cpu > 14. || now.elapsed().as_secs_f64() * 1000. > 22. {
                slow += 1;
            } else {
                slow = slow.saturating_sub(1);
            }
            if slow >= 30 && quality > 0.4 {
                quality = (quality * 0.8_f64).max(0.4);
                slow = 0;
            }
            if moving || settling || engine.transitioning() {
                pace(&mut deadline);
            }
        }
        Ok(())
    }
    pub fn close(window: &WebviewWindow, session: Session) {
        let _ = window.with_webview(move |w| unsafe { session.close(w.inner()) });
    }
}

#[cfg(test)]
mod pacing_tests {
    #[test]
    fn display_samples_are_bounded_and_exclude_hidden_or_paused_gaps() {
        let mut p = super::Pacing::default();
        for i in 0..1000 {
            p.present(1. + i as f64 / 60.);
        }
        let report = p.report();
        assert_eq!(report.samples, 600);
        assert_eq!(report.missed, 0);
        assert!((report.p50_ms - 1000. / 60.).abs() < 0.01);
        p.present(40.);
        assert_eq!(p.report().missed, 0);
        p.present(40.05);
        assert_eq!(p.report().missed, 1);
        let count = p.report().frames;
        p.present(f64::NAN);
        p.present(1.);
        assert_eq!(p.report().frames, count);
    }
}
