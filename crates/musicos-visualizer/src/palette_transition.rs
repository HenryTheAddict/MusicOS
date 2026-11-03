//! One finite artwork-color handoff. Missing in-flight artwork holds the
//! displayed palette; it never flashes the preset's original colors.
pub type Palette = [[f32; 3]; 3];
#[derive(Default)]
pub struct PaletteTransition {
    from: Option<Palette>,
    target: Option<Palette>,
    started: f64,
}
impl PaletteTransition {
    pub fn sample(&self, now: f64) -> Option<Palette> {
        let target = self.target?;
        let from = self.from.unwrap_or(target);
        let t = ((now - self.started) / 0.32).clamp(0., 1.) as f32;
        let t = t * t * (3. - 2. * t);
        Some(std::array::from_fn(|i| {
            std::array::from_fn(|j| from[i][j] + (target[i][j] - from[i][j]) * t)
        }))
    }
    pub fn update(
        &mut self,
        target: Option<Palette>,
        pending: bool,
        reduced: bool,
        now: f64,
    ) -> Option<Palette> {
        let target = target.filter(|p| {
            p.iter()
                .flatten()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        });
        if !pending && target != self.target {
            self.from = self.sample(now).or(target);
            self.target = target;
            self.started = now;
        }
        if reduced {
            self.from = self.target;
            self.started = now - 0.32;
        }
        self.sample(now)
    }
    pub fn transitioning(&self, now: f64) -> bool {
        self.from != self.target && self.target.is_some() && now - self.started < 0.32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_art_and_rapid_skips_continue_from_displayed_colors() {
        let mut t = PaletteTransition::default();
        let a = [[0.1; 3]; 3];
        let b = [[0.9; 3]; 3];
        let c = [[0.3; 3]; 3];
        assert_eq!(t.update(Some(a), false, false, 0.), Some(a));
        assert_eq!(t.update(None, true, false, 1.), Some(a));
        assert_eq!(t.update(Some(b), false, false, 2.), Some(a));
        let mid = t.update(Some(b), false, false, 2.16).unwrap();
        assert!((mid[0][0] - 0.5).abs() < 0.001);
        assert_eq!(t.update(Some(c), false, false, 2.16), Some(mid));
        assert_eq!(t.update(Some(c), false, false, 2.5), Some(c));
        assert!(!t.transitioning(2.5));
        assert_eq!(t.update(None, false, false, 3.), None);
    }
    #[test]
    fn reduced_motion_finishes_immediately_and_never_loops() {
        let mut t = PaletteTransition::default();
        t.update(Some([[0.; 3]; 3]), false, false, 0.);
        assert_eq!(
            t.update(Some([[1.; 3]; 3]), false, true, 1.),
            Some([[1.; 3]; 3])
        );
        assert!(!t.transitioning(1.));
    }
}
