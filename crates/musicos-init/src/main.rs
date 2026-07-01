use std::time::{Duration, Instant};

use anyhow::Result;
use log::{error, info};
use musicos_init::app::MusicOs;
use musicos_init::input::{InputEvent, InputSystem};
use musicos_init::renderer::{self, Renderer};
use musicos_init::server::CompanionServer;
use musicos_init::{init, media};

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };
        let location = info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let log_msg = format!("Panic occurred: {msg} at {location}\nBacktrace:\n{backtrace}\n");
        eprintln!("{log_msg}");
        let _ = std::fs::create_dir_all("/var/lib/musicos");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/var/lib/musicos/panic.log")
        {
            use std::io::Write;
            let _ = write!(file, "{log_msg}");
        } else if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("panic.log")
        {
            use std::io::Write;
            let _ = write!(file, "{log_msg}");
        }
    }));

    if let Err(err) = run() {
        error!("{err:?}");
        eprintln!("musicos-init: {err:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    init::install_logger();
    init::prepare_pid1_environment()?;

    match musicos_init::usb::install_pending_update() {
        Ok(Some(path)) => info!("installed pending MusicOS update to {}", path.display()),
        Ok(None) => {}
        Err(err) => info!("pending update install skipped: {err:#}"),
    }

    let resolution = renderer::Resolution::from_env_or_default();
    let mut renderer = Renderer::open(resolution)?;
    let mut inputs = InputSystem::open();
    musicos_init::system::init_audio();
    let mut app = MusicOs::new(resolution);
    let started_at = Instant::now();
    let target_fps = std::env::var("MUSICOS_TARGET_FPS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(15, 60);
    let frame_budget = Duration::from_micros(1_000_000 / target_fps);
    let idle_frame_budget = Duration::from_millis(180);

    info!(
        "MusicOS starting at {}x{} target_fps={}",
        resolution.width, resolution.height, target_fps
    );

    let (tx, rx) = std::sync::mpsc::channel();
    let db_path = std::env::var("MUSICOS_MEDIA_DB")
        .unwrap_or_else(|_| "/var/lib/musicos/media.sqlite3".to_string());
    CompanionServer::start(8080, tx, db_path)?;

    if let Err(err) = media::index_default_locations(&app.db) {
        info!("media indexing skipped: {err:#}");
    }
    app.refresh_library();

    renderer.draw(|canvas| app.draw(canvas, started_at.elapsed().as_secs_f32()))?;

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    if let Err(err) = ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }) {
        error!("failed to set Ctrl-C handler: {err}");
    }

    let mut last_frame = Instant::now();
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        app.tick();

        let mut screen_changed = false;
        for event in inputs.poll() {
            if matches!(event, InputEvent::Power) {
                info!("power event ignored by UI");
                continue;
            }
            if app.handle_input(event) {
                screen_changed = true;
            }
        }

        while let Ok(event) = rx.try_recv() {
            if app.handle_server_event(event) {
                screen_changed = true;
            }
        }

        let realtime_redraw = app.animation_in_progress() || app.player.get_status().is_playing;
        let redraw_budget = if realtime_redraw {
            frame_budget
        } else {
            idle_frame_budget
        };
        if screen_changed || last_frame.elapsed() >= redraw_budget {
            renderer.draw(|canvas| app.draw(canvas, started_at.elapsed().as_secs_f32()))?;
            last_frame = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(2));
    }

    info!("MusicOS stopping gracefully...");
    app.save_preferences(true);
    Ok(())
}
