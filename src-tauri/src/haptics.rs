//! Discrete main-thread feedback. No audio callback or delayed pulse queue.
use serde::Deserialize;
use std::{
    sync::{Mutex, OnceLock},
    time::Instant,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Beat,
    Pickup,
    Target,
    Reorder,
    Drop,
    Favorite,
    Toggle,
    Step,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub kind: Kind,
    pub key: String,
    pub track_id: Option<i64>,
}
#[derive(Default)]
pub struct Gate {
    serial: u64,
    pending: Option<(u64, Kind, Instant)>,
    last: Option<(Kind, String, Instant)>,
}
impl Gate {
    fn enqueue(&mut self, kind: Kind, key: &str, now: Instant) -> Option<u64> {
        if key.len() > 160 {
            return None;
        }
        if let Some((k, old, at)) = &self.last {
            let elapsed = now.saturating_duration_since(*at);
            if (old == key && *k == kind && elapsed.as_millis() < 120)
                || elapsed.as_millis() < (if kind == Kind::Beat { 250 } else { 80 })
            {
                return None;
            }
        }
        if self.pending.is_some_and(|(_, k, at)| {
            k != Kind::Beat
                && kind == Kind::Beat
                && now.saturating_duration_since(at).as_millis() < 80
        }) {
            return None;
        }
        self.serial += 1;
        self.pending = Some((self.serial, kind, now));
        Some(self.serial)
    }
    fn accept(&mut self, token: u64, key: String, now: Instant) -> Option<Kind> {
        let (id, kind, at) = self.pending?;
        if id != token || now.saturating_duration_since(at).as_millis() > 80 {
            return None;
        }
        self.pending = None;
        self.last = Some((kind, key, now));
        Some(kind)
    }
}
fn gate() -> &'static Mutex<Gate> {
    static GATE: OnceLock<Mutex<Gate>> = OnceLock::new();
    GATE.get_or_init(Default::default)
}
pub async fn request(app: tauri::AppHandle, window: tauri::WebviewWindow, req: Request) {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, window, req);
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let Some(token) = gate()
            .lock()
            .unwrap()
            .enqueue(req.kind, &req.key, Instant::now())
        else {
            return;
        };
        tokio::time::sleep(std::time::Duration::from_millis(8)).await;
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            use tauri::Manager;
            let state = handle.state::<crate::app::AppState>();
            if !state.haptic_allowed(req.kind, req.track_id)
                || !window.is_focused().unwrap_or(false)
                || !window.is_visible().unwrap_or(false)
            {
                return;
            }
            if let Some(kind) = gate()
                .lock()
                .unwrap()
                .accept(token, req.key, Instant::now())
            {
                unsafe {
                    musicos_haptic(match kind {
                        Kind::Target | Kind::Reorder => 1,
                        Kind::Step => 2,
                        _ => 0,
                    });
                }
            }
        });
    }
}
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn musicos_haptic(pattern: i32);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interaction_replaces_beat_and_never_queues_old_ticks() {
        let now = Instant::now();
        let mut g = Gate::default();
        let beat = g.enqueue(Kind::Beat, "1", now).unwrap();
        let action = g.enqueue(Kind::Drop, "drop", now).unwrap();
        assert!(g.accept(beat, "1".into(), now).is_none());
        assert_eq!(g.accept(action, "drop".into(), now), Some(Kind::Drop));
        assert!(g
            .enqueue(Kind::Beat, "2", now + std::time::Duration::from_millis(100))
            .is_none());
        assert!(g
            .enqueue(
                Kind::Drop,
                "drop",
                now + std::time::Duration::from_millis(100)
            )
            .is_none());
        let late = g
            .enqueue(Kind::Beat, "3", now + std::time::Duration::from_millis(300))
            .unwrap();
        assert!(g
            .accept(
                late,
                "3".into(),
                now + std::time::Duration::from_millis(500)
            )
            .is_none());
    }
}
