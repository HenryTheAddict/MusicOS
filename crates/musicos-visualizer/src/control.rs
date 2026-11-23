//! Presentation policy. Source positions come from acknowledged playback; this
//! module never advances audio, creates a tempo estimate, or issues a command.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    #[default]
    Manual,
    EachSong,
    Timed,
    Beats,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub preset: String,
    pub rotation: Rotation,
    pub timed_seconds: u32,
    pub beat_interval: u32,
    pub beat_sync: bool,
    pub album_colors: bool,
    pub lyrics: bool,
    pub show_cover: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            preset: "boxes".into(),
            rotation: Rotation::Manual,
            timed_seconds: 30,
            beat_interval: 32,
            beat_sync: true,
            album_colors: false,
            lyrics: false,
            show_cover: true,
        }
    }
}
impl Preferences {
    pub fn normalize(mut self) -> Self {
        if self.preset != "beat-pulse"
            && !crate::catalog::catalog()
                .iter()
                .any(|p| p.id == self.preset)
        {
            self.preset = "boxes".into();
        }
        if ![15, 30, 60].contains(&self.timed_seconds) {
            self.timed_seconds = 30;
        }
        if ![16, 32, 64].contains(&self.beat_interval) {
            self.beat_interval = 32;
        }
        self
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Clock {
    pub entry: Option<u64>,
    pub position: f64,
    pub speed: f64,
    pub playing: bool,
    pub bpm: Option<f64>,
    pub beat_offset: Option<f64>,
    pub confidence: f64,
}
impl Clock {
    pub fn period(&self) -> Option<f64> {
        let bpm = self.bpm?;
        if self.confidence < 0.5
            || !bpm.is_finite()
            || !(20. ..=400.).contains(&bpm)
            || !self.beat_offset?.is_finite()
            || !self.speed.is_finite()
            || self.speed <= 0.
        {
            None
        } else {
            Some(60. / bpm)
        }
    }
    pub fn beat(&self) -> Option<f64> {
        Some((self.position - self.beat_offset?) / self.period()?)
    }
    pub fn accent(&self) -> f32 {
        if !self.playing {
            return 0.;
        }
        let Some(beat) = self.beat() else { return 0. };
        let phase = beat.rem_euclid(1.);
        ((1. - phase / 0.2).clamp(0., 1.).powi(2) * 0.035) as f32
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Change {
    pub seconds: f64,
    pub fallback: bool,
}
#[derive(Default)]
pub struct Coordinator {
    previous: Option<Clock>,
    elapsed: f64,
    next_beat: Option<f64>,
    policy: Option<Preferences>,
}
impl Coordinator {
    pub fn reset(&mut self) {
        self.previous = None;
        self.elapsed = 0.;
        self.next_beat = None;
    }
    pub fn step(&mut self, prefs: &Preferences, clock: Clock, dt: f64) -> Option<Change> {
        if self.policy.as_ref().is_none_or(|old| {
            old.rotation != prefs.rotation
                || old.timed_seconds != prefs.timed_seconds
                || old.beat_interval != prefs.beat_interval
        }) {
            self.reset();
            self.policy = Some(prefs.clone());
        }
        let old = self.previous.replace(clock);
        let Some(old) = old else {
            self.rebase_beat(clock, prefs);
            return None;
        };
        let changed = clock.entry != old.entry;
        let jump = (clock.position
            - old.position
            - if old.playing {
                dt.max(0.) * old.speed
            } else {
                0.
            })
        .abs()
            > 0.3;
        if changed || jump || clock.period() != old.period() {
            self.elapsed = 0.;
            self.rebase_beat(clock, prefs);
            if changed && prefs.rotation == Rotation::EachSong && clock.entry.is_some() {
                return Some(Change {
                    seconds: 2.,
                    fallback: false,
                });
            }
            return None;
        }
        if !clock.playing {
            return None;
        }
        match prefs.rotation {
            Rotation::Manual | Rotation::EachSong => None,
            Rotation::Timed => self.timed(dt, prefs.timed_seconds as f64, false),
            Rotation::Beats => {
                if let Some(beat) = clock.beat() {
                    self.elapsed = 0.;
                    let next = *self.next_beat.get_or_insert_with(|| {
                        ((beat / prefs.beat_interval as f64).floor() + 1.)
                            * prefs.beat_interval as f64
                    });
                    if beat >= next {
                        self.rebase_beat(clock, prefs);
                        Some(Change {
                            seconds: (4. * clock.period().unwrap() / clock.speed).clamp(0.5, 3.),
                            fallback: false,
                        })
                    } else {
                        None
                    }
                } else {
                    self.next_beat = None;
                    self.timed(dt, 30., true)
                }
            }
        }
    }
    fn rebase_beat(&mut self, clock: Clock, prefs: &Preferences) {
        self.next_beat = clock
            .beat()
            .map(|b| ((b / prefs.beat_interval as f64).floor() + 1.) * prefs.beat_interval as f64);
    }
    fn timed(&mut self, dt: f64, interval: f64, fallback: bool) -> Option<Change> {
        self.elapsed += dt.clamp(0., 0.25);
        if self.elapsed >= interval {
            self.elapsed = 0.;
            Some(Change {
                seconds: 2.,
                fallback,
            })
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn web_preset_survives_normalization_without_entering_native_catalog() {
        let prefs = Preferences {
            preset: "beat-pulse".into(),
            ..Default::default()
        }
        .normalize();
        assert_eq!(prefs.preset, "beat-pulse");
        assert!(!crate::catalog::catalog()
            .iter()
            .any(|p| p.id == prefs.preset));
    }
    fn clock(p: f64) -> Clock {
        Clock {
            entry: Some(1),
            position: p,
            speed: 1.,
            playing: true,
            bpm: Some(120.),
            beat_offset: Some(0.),
            confidence: 0.9,
        }
    }
    #[test]
    fn album_colors_round_trip_and_do_not_reset_rotation() {
        let mut prefs: Preferences = serde_json::from_str(r#"{"preset":"oil-puddle"}"#).unwrap();
        assert!(!prefs.album_colors);
        prefs.album_colors = true;
        assert!(
            serde_json::from_str::<Preferences>(&serde_json::to_string(&prefs).unwrap())
                .unwrap()
                .album_colors
        );
        let mut coordinator = Coordinator::default();
        prefs.rotation = Rotation::Timed;
        prefs.timed_seconds = 15;
        let mut clock = Clock {
            playing: true,
            speed: 1.,
            ..Default::default()
        };
        coordinator.step(&prefs, clock, 0.);
        for _ in 0..59 {
            clock.position += 0.25;
            assert!(coordinator.step(&prefs, clock, 0.25).is_none());
        }
        prefs.album_colors = false;
        clock.position += 0.25;
        assert!(coordinator.step(&prefs, clock, 0.25).is_some());
    }
    #[test]
    fn defaults_and_unknown_tempo_never_invent_beats() {
        let p = Preferences::default();
        assert_eq!(p.preset, "boxes");
        assert!(!p.lyrics);
        assert_eq!(Clock::default().accent(), 0.);
        assert!(Clock {
            bpm: Some(120.),
            ..Default::default()
        }
        .period()
        .is_none());
    }
    #[test]
    fn beats_change_once_on_boundary_and_rebase_after_seek() {
        let p = Preferences {
            rotation: Rotation::Beats,
            beat_interval: 16,
            ..Default::default()
        };
        let mut c = Coordinator::default();
        c.step(&p, clock(7.9), 0.);
        assert_eq!(
            c.step(&p, clock(8.), 0.1),
            Some(Change {
                seconds: 2.,
                fallback: false
            })
        );
        assert!(c.step(&p, clock(8.01), 0.01).is_none());
        assert!(c.step(&p, clock(50.), 0.01).is_none());
        assert!(c.step(&p, clock(50.01), 0.01).is_none());
    }
    #[test]
    fn duplicate_queue_entries_rotate_and_pause_freezes_timer() {
        let p = Preferences {
            rotation: Rotation::EachSong,
            ..Default::default()
        };
        let mut c = Coordinator::default();
        c.step(&p, clock(0.), 0.);
        assert!(c
            .step(
                &p,
                Clock {
                    entry: Some(2),
                    ..clock(0.)
                },
                0.1
            )
            .is_some());
        let p = Preferences {
            rotation: Rotation::Timed,
            timed_seconds: 15,
            ..Default::default()
        };
        let mut c = Coordinator::default();
        c.step(&p, clock(0.), 0.);
        for _ in 0..1000 {
            assert!(c
                .step(
                    &p,
                    Clock {
                        playing: false,
                        ..clock(0.)
                    },
                    0.1
                )
                .is_none());
        }
    }
    #[test]
    fn unknown_bpm_has_thirty_second_fallback_without_bursts() {
        let p = Preferences {
            rotation: Rotation::Beats,
            ..Default::default()
        };
        let mut c = Coordinator::default();
        let mut changes = 0;
        for i in 0..1202 {
            let now = Clock {
                bpm: None,
                ..clock(i as f64 / 20.)
            };
            if let Some(change) = c.step(&p, now, 0.05) {
                assert!(change.fallback);
                changes += 1;
            }
        }
        assert_eq!(changes, 2);
    }
}
