use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionLevel {
    Visuals,
    Balanced,
    BatterySaver,
    Reduced,
}

impl Default for MotionLevel {
    fn default() -> Self {
        Self::Visuals
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MotionSpec {
    pub control: Duration,
    pub navigation: Duration,
    pub artwork: Duration,
    pub completion: Duration,
    pub ambient_fps: u16,
    pub overshoot: f32,
}

impl MotionLevel {
    pub fn spec(self) -> MotionSpec {
        match self {
            Self::Visuals => MotionSpec {
                control: Duration::from_millis(160),
                navigation: Duration::from_millis(260),
                artwork: Duration::from_millis(420),
                completion: Duration::from_millis(560),
                ambient_fps: 60,
                overshoot: 0.04,
            },
            Self::Balanced => MotionSpec {
                control: Duration::from_millis(140),
                navigation: Duration::from_millis(210),
                artwork: Duration::from_millis(320),
                completion: Duration::from_millis(420),
                ambient_fps: 30,
                overshoot: 0.02,
            },
            Self::BatterySaver => MotionSpec {
                control: Duration::from_millis(90),
                navigation: Duration::from_millis(120),
                artwork: Duration::from_millis(140),
                completion: Duration::from_millis(180),
                ambient_fps: 12,
                overshoot: 0.0,
            },
            Self::Reduced => MotionSpec {
                control: Duration::from_millis(60),
                navigation: Duration::from_millis(80),
                artwork: Duration::from_millis(80),
                completion: Duration::from_millis(80),
                ambient_fps: 0,
                overshoot: 0.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Timeline {
    started_at_ms: u64,
    duration_ms: u64,
    from: f32,
    to: f32,
}

impl Timeline {
    pub fn new(now_ms: u64, duration: Duration, from: f32, to: f32) -> Self {
        Self {
            started_at_ms: now_ms,
            duration_ms: duration.as_millis().max(1) as u64,
            from,
            to,
        }
    }

    pub fn value(self, now_ms: u64) -> f32 {
        let t = now_ms
            .saturating_sub(self.started_at_ms)
            .min(self.duration_ms) as f32
            / self.duration_ms as f32;
        self.from + (self.to - self.from) * ease_out_cubic(t)
    }

    pub fn finished(self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.started_at_ms) >= self.duration_ms
    }

    pub fn reverse(self, now_ms: u64, duration: Duration) -> Self {
        Self::new(now_ms, duration, self.value(now_ms), self.from)
    }
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t.powi(3)
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn damped_spring(t: f32, overshoot: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if overshoot <= 0.0 {
        return ease_out_cubic(t);
    }
    let decay = (-7.5 * t).exp();
    (1.0 - decay * (10.0 * t).cos() * (1.0 + overshoot)).clamp(0.0, 1.0 + overshoot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_uses_deterministic_time_and_finishes() {
        let timeline = Timeline::new(100, Duration::from_millis(200), 0.0, 1.0);
        assert_eq!(timeline.value(100), 0.0);
        assert!(timeline.value(200) > 0.5);
        assert_eq!(timeline.value(300), 1.0);
        assert!(timeline.finished(300));
    }

    #[test]
    fn reduced_motion_has_no_ambient_loop() {
        let spec = MotionLevel::Reduced.spec();
        assert_eq!(spec.ambient_fps, 0);
        assert!(spec.navigation <= Duration::from_millis(80));
    }

    #[test]
    fn reversing_starts_from_current_value() {
        let forward = Timeline::new(0, Duration::from_millis(200), 0.0, 1.0);
        let current = forward.value(100);
        let reverse = forward.reverse(100, Duration::from_millis(200));
        assert!((reverse.value(100) - current).abs() < f32::EPSILON);
        assert_eq!(reverse.value(300), 0.0);
    }
}
