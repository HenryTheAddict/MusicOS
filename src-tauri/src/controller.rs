//! One bounded native input/rumble owner. Never runs on the audio callback.
use crate::haptics::Kind;
use gilrs::{
    ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Repeat, Replay, Ticks},
    Axis, Button, EventType, GamepadId, Gilrs, MappingSource,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub enabled: bool,
    pub vibration: bool,
    pub beat_focus: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            enabled: true,
            vibration: true,
            beat_focus: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: usize,
    pub name: String,
    pub mapped: bool,
    pub vibration: bool,
}
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub devices: Vec<Device>,
    pub active: Option<usize>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    device: usize,
    control: &'static str,
    value: f32,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Configure {
        active: bool,
        sequence: u64,
    },
    Status,
    Feedback {
        kind: Kind,
        key: String,
        #[serde(rename = "trackId")]
        track_id: Option<i64>,
    },
    Stop,
}
enum Message {
    Configure {
        window: String,
        active: bool,
        sequence: u64,
        prefs: Preferences,
    },
    Feedback {
        window: String,
        kind: Kind,
        key: String,
        track: Option<i64>,
        at: Instant,
    },
    Stop,
    Shutdown,
}
pub struct Service {
    send: SyncSender<Message>,
    status: Arc<Mutex<Status>>,
}
impl Service {
    pub fn new(app: tauri::AppHandle) -> Self {
        let (send, receive) = mpsc::sync_channel(32);
        let status = Arc::new(Mutex::new(Status::default()));
        let out = status.clone();
        std::thread::Builder::new()
            .name("musicos-controller".into())
            .spawn(move || worker(app, receive, out))
            .expect("controller worker");
        Self { send, status }
    }
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn configure(&self, window: String, active: bool, sequence: u64, prefs: Preferences) {
        let _ = self.send.try_send(Message::Configure {
            window,
            active,
            sequence,
            prefs,
        });
    }
    pub fn feedback(&self, window: String, kind: Kind, key: String, track: Option<i64>) {
        if key.len() <= 160 {
            let _ = self.send.try_send(Message::Feedback {
                window,
                kind,
                key,
                track,
                at: Instant::now(),
            });
        }
    }
    pub fn stop(&self) {
        let _ = self.send.try_send(Message::Stop);
    }
    pub fn shutdown(&self) {
        let _ = self.send.try_send(Message::Shutdown);
    }
}
fn focused(app: &tauri::AppHandle, label: &str) -> bool {
    app.get_webview_window(label).is_some_and(|w| {
        w.is_visible().unwrap_or(false)
            && w.is_focused().unwrap_or(false)
            && !w.is_minimized().unwrap_or(false)
    })
}
fn stop(effect: &mut Option<(Effect, Instant, Kind, Option<i64>)>) {
    if let Some((e, ..)) = effect.take() {
        let _ = e.stop();
    }
}
fn button(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::South => "accept",
        Button::East => "back",
        Button::West => "favorite",
        Button::North => "panel",
        Button::Start => "play",
        Button::Select => "visualizer",
        Button::RightThumb => "context",
        Button::LeftTrigger => "previous",
        Button::RightTrigger => "next",
        Button::LeftTrigger2 => "seek_back",
        Button::RightTrigger2 => "seek_forward",
        Button::DPadUp => "up",
        Button::DPadDown => "down",
        Button::DPadLeft => "left",
        Button::DPadRight => "right",
        _ => return None,
    })
}
fn axis(axis: Axis) -> Option<&'static str> {
    match axis {
        Axis::LeftStickX => Some("focus_x"),
        Axis::LeftStickY => Some("focus_y"),
        Axis::RightStickY => Some("scroll_y"),
        _ => None,
    }
}
#[derive(Default)]
struct Gate {
    last: Option<(Kind, String, Instant)>,
}
impl Gate {
    fn allow(&mut self, kind: Kind, key: &str, now: Instant) -> bool {
        if let Some((k, old, at)) = &self.last {
            let age = now.saturating_duration_since(*at).as_millis();
            if (kind == Kind::Beat && age < 250)
                || (kind != Kind::Beat && age < 65)
                || (*k == kind && old == key && age < 160)
            {
                return false;
            }
        }
        self.last = Some((kind, key.into(), now));
        true
    }
}
fn pulse(
    gilrs: &mut Gilrs,
    id: GamepadId,
    kind: Kind,
) -> Option<(Effect, Instant, Kind, Option<i64>)> {
    if !gilrs.gamepad(id).is_ff_supported() {
        return None;
    }
    let (ms, strong, weak) = match kind {
        Kind::Beat => (35, 3500, 5500),
        Kind::Drop | Kind::Favorite => (65, 12000, 9000),
        Kind::Pickup => (45, 7000, 11000),
        _ => (25, 2500, 6500),
    };
    let ticks = Ticks::from_ms(ms);
    let mut b = EffectBuilder::new();
    for effect in [
        BaseEffectType::Strong { magnitude: strong },
        BaseEffectType::Weak { magnitude: weak },
    ] {
        b.add_effect(BaseEffect {
            kind: effect,
            scheduling: Replay {
                play_for: ticks,
                ..Default::default()
            },
            ..Default::default()
        });
    }
    let e = b
        .gamepads(&[id])
        .repeat(Repeat::For(ticks))
        .finish(gilrs)
        .ok()?;
    e.play().ok()?;
    Some((
        e,
        Instant::now() + Duration::from_millis(ms as u64 + 20),
        kind,
        None,
    ))
}
fn worker(app: tauri::AppHandle, receive: mpsc::Receiver<Message>, out: Arc<Mutex<Status>>) {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(e) => {
            let mut s = out.lock().unwrap_or_else(|e| e.into_inner());
            s.error = Some(format!("Controller input unavailable: {e}"));
            let _ = app.emit("musicos-controller-status", s.clone());
            return;
        }
    };
    let (mut owner, mut enabled, mut prefs) = (String::new(), false, Preferences::default());
    let mut selected = None;
    let mut effect = None;
    let mut gate = Gate::default();
    let mut last_status = Status::default();
    let mut was_focused = false;
    let mut sequences = std::collections::HashMap::<String, u64>::new();
    loop {
        let message = match receive.recv_timeout(Duration::from_millis(
            if enabled && !last_status.devices.is_empty() {
                16
            } else {
                250
            },
        )) {
            Ok(m) => Some(m),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(_) => None,
        };
        if let Some(message) = message {
            match message {
                Message::Shutdown => break,
                Message::Configure {
                    window,
                    active,
                    sequence,
                    prefs: p,
                } => {
                    if sequence >= *sequences.get(&window).unwrap_or(&0) {
                        sequences.insert(window.clone(), sequence);
                        if (active && focused(&app, &window)) || (!active && owner == window) {
                            owner = window;
                            enabled = active && p.enabled;
                            prefs = p;
                            if !enabled {
                                stop(&mut effect);
                            }
                        }
                    }
                }
                Message::Stop => stop(&mut effect),
                Message::Feedback {
                    window,
                    kind,
                    key,
                    track,
                    at,
                } => {
                    let now = Instant::now();
                    if enabled
                        && prefs.vibration
                        && window == owner
                        && focused(&app, &owner)
                        && now.saturating_duration_since(at) < Duration::from_millis(80)
                        && app
                            .state::<crate::app::AppState>()
                            .controller_feedback_allowed(kind, track)
                        && (kind == Kind::Stop || gate.allow(kind, &key, now))
                    {
                        if kind == Kind::Stop {
                            if effect.as_ref().is_some_and(|e| e.2 == Kind::Beat) {
                                stop(&mut effect);
                            }
                        } else if let Some(id) = selected {
                            stop(&mut effect);
                            effect = pulse(&mut gilrs, id, kind);
                            if let Some(e) = effect.as_mut() {
                                e.3 = track;
                            }
                        }
                    }
                }
            }
        }
        let visible = enabled && focused(&app, &owner);
        if !visible {
            stop(&mut effect);
        } else if effect.as_ref().is_some_and(|(_, until, kind, track)| {
            Instant::now() >= *until
                || !app
                    .state::<crate::app::AppState>()
                    .controller_feedback_allowed(*kind, *track)
        }) {
            stop(&mut effect);
        }
        if was_focused && !visible {
            let _ = app.emit_to(
                &owner,
                "musicos-controller-input",
                Input {
                    device: selected.map(usize::from).unwrap_or(0),
                    control: "reset",
                    value: 0.,
                },
            );
        }
        was_focused = visible;
        while let Some(event) = gilrs.next_event() {
            if matches!(event.event, EventType::Disconnected) {
                if selected == Some(event.id) {
                    stop(&mut effect);
                    selected = None;
                    let _ = app.emit_to(
                        &owner,
                        "musicos-controller-input",
                        Input {
                            device: usize::from(event.id),
                            control: "reset",
                            value: 0.,
                        },
                    );
                }
                continue;
            }
            if !visible || gilrs.gamepad(event.id).mapping_source() == MappingSource::None {
                continue;
            }
            let input = match event.event {
                EventType::ButtonPressed(b, _) => button(b).map(|c| (c, 1.)),
                EventType::ButtonReleased(b, _) => button(b).map(|c| (c, 0.)),
                EventType::AxisChanged(a, v, _) => axis(a).map(|c| (c, v)),
                _ => None,
            };
            if let Some((control, value)) = input {
                if selected != Some(event.id) {
                    stop(&mut effect);
                    if selected.is_some() {
                        let _ = app.emit_to(
                            &owner,
                            "musicos-controller-input",
                            Input {
                                device: usize::from(event.id),
                                control: "reset",
                                value: 0.,
                            },
                        );
                    }
                    selected = Some(event.id);
                }
                let _ = app.emit_to(
                    &owner,
                    "musicos-controller-input",
                    Input {
                        device: usize::from(event.id),
                        control,
                        value,
                    },
                );
            }
        }
        let status = Status {
            devices: gilrs
                .gamepads()
                .map(|(id, g)| Device {
                    id: usize::from(id),
                    name: g.name().chars().take(120).collect(),
                    mapped: g.mapping_source() != MappingSource::None,
                    vibration: g.is_ff_supported(),
                })
                .collect(),
            active: selected.map(usize::from),
            error: None,
        };
        if status != last_status {
            *out.lock().unwrap_or_else(|e| e.into_inner()) = status.clone();
            let _ = app.emit("musicos-controller-status", &status);
            last_status = status;
        }
    }
    stop(&mut effect);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_buttons_keep_transport_and_focus_separate() {
        assert_eq!(button(Button::South), Some("accept"));
        assert_eq!(button(Button::Start), Some("play"));
        assert_eq!(button(Button::RightTrigger), Some("next"));
        assert_eq!(button(Button::RightTrigger2), Some("seek_forward"));
        assert_eq!(button(Button::Unknown), None);
    }
    #[test]
    fn rumble_is_bounded_and_interactions_win_over_a_beat() {
        let now = Instant::now();
        let mut g = Gate::default();
        assert!(g.allow(Kind::Drop, "drop", now));
        assert!(!g.allow(Kind::Beat, "beat", now + Duration::from_millis(100)));
        assert!(g.allow(Kind::Beat, "beat", now + Duration::from_millis(251)));
        assert!(!g.allow(Kind::Beat, "next", now + Duration::from_millis(499)));
        assert!(g.allow(Kind::Toggle, "toggle", now + Duration::from_millis(500)));
    }
}
