use std::time::{Duration, Instant};

use anyhow::Result;
use log::{error, info};
use musicos_init::app::MusicOs;
use musicos_init::init;
use musicos_init::input::InputSystem;
use musicos_init::renderer::{self, Renderer};
use musicos_init::server::CompanionServer;

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
        let location = info
            .location()
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

    if let Err(err) = musicos_init::usb::cleanup_interrupted_updates() {
        log::warn!("interrupted update cleanup skipped: {err:#}");
    }
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
        .unwrap_or(30)
        .clamp(15, 60);
    let frame_budget = Duration::from_micros(1_000_000 / target_fps);
    // While idle (no animation, nothing playing) only the clock/battery change, and
    // those tick at ~1 Hz. Redrawing at that cadence instead of every 180 ms keeps the
    // CPU nearly idle without the display ever looking stale.
    let idle_frame_budget = Duration::from_millis(1000);

    info!(
        "MusicOS starting at {}x{} target_fps={}",
        resolution.width, resolution.height, target_fps
    );

    let (tx, rx) = std::sync::mpsc::channel();
    let db_path = std::env::var("MUSICOS_MEDIA_DB")
        .unwrap_or_else(|_| "/var/lib/musicos/media.sqlite3".to_string());
    CompanionServer::start(8080, tx, db_path)?;
    app.start_library_scan();

    let draw_start = Instant::now();
    renderer.draw(|canvas| app.draw(canvas, started_at.elapsed().as_secs_f32()))?;
    app.set_last_frame_ms(draw_start.elapsed().as_secs_f32() * 1000.0);

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    if let Err(err) = ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }) {
        error!("failed to set Ctrl-C handler: {err}");
    }

    let mut next_frame = Instant::now();
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        app.tick();

        let mut screen_changed = false;
        for event in inputs.poll() {
            if app.handle_input(event) {
                screen_changed = true;
            }
        }

        while let Ok(event) = rx.try_recv() {
            if app.handle_server_event(event) {
                screen_changed = true;
            }
        }

        let animating = app.animation_in_progress();
        let realtime_redraw = animating || app.player.is_playing();
        let redraw_budget = if realtime_redraw {
            frame_budget
        } else {
            idle_frame_budget
        };
        let now = Instant::now();
        if screen_changed || now >= next_frame {
            let draw_start = Instant::now();
            renderer.draw(|canvas| app.draw(canvas, started_at.elapsed().as_secs_f32()))?;
            app.set_last_frame_ms(draw_start.elapsed().as_secs_f32() * 1000.0);
            next_frame = now + redraw_budget;
        }

        inputs.wait(wait_timeout(Instant::now(), next_frame, animating))?;
    }

    info!("MusicOS stopping gracefully...");
    app.save_preferences(true);
    Ok(())
}

fn wait_timeout(now: Instant, next_frame: Instant, animating: bool) -> Duration {
    // Animation state in tick() advances per call, so retain its existing cadence.
    let tick_budget = Duration::from_millis(if animating { 2 } else { 20 });
    next_frame.saturating_duration_since(now).min(tick_budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_respects_frames_ticks_and_overdue_work() {
        let now = Instant::now();
        assert_eq!(wait_timeout(now, now, false), Duration::ZERO);
        assert_eq!(
            wait_timeout(now, now - Duration::from_millis(1), false),
            Duration::ZERO
        );
        assert_eq!(
            wait_timeout(now, now + Duration::from_secs(1), false),
            Duration::from_millis(20)
        );
        assert_eq!(
            wait_timeout(now, now + Duration::from_millis(5), false),
            Duration::from_millis(5)
        );
        assert_eq!(
            wait_timeout(now, now + Duration::from_secs(1), true),
            Duration::from_millis(2)
        );
    }
}
