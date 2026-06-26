#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn interpolate(&self, target: Position, t: f32) -> Position {
        let t = t.clamp(0.0, 1.0);
        // Smooth cosine interpolation
        let t = (1.0 - (t * std::f32::consts::PI).cos()) / 2.0;
        let inv = 1.0 - t;
        Position {
            x: self.x * inv + target.x * t,
            y: self.y * inv + target.y * t,
            w: self.w * inv + target.w * t,
            h: self.h * inv + target.h * t,
        }
    }
}

pub struct AnimatedState {
    pub current: Position,
    pub target: Position,
    pub progress: f32,
}

impl AnimatedState {
    pub fn new(pos: Position) -> Self {
        Self {
            current: pos,
            target: pos,
            progress: 1.0,
        }
    }

    pub fn set_target(&mut self, target: Position) {
        if self.target.x != target.x
            || self.target.y != target.y
            || self.target.w != target.w
            || self.target.h != target.h
        {
            self.current = self.get_position();
            self.target = target;
            self.progress = 0.0;
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.progress < 1.0 {
            self.progress = (self.progress + dt * 6.0).min(1.0); // fast transition: ~0.16s
        }
    }

    pub fn get_position(&self) -> Position {
        self.current.interpolate(self.target, self.progress)
    }
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn stagger(parent_t: f32, index: usize, delay: f32, span: f32) -> f32 {
    ((parent_t - index as f32 * delay) / span).clamp(0.0, 1.0)
}

pub fn breathe(seconds: f32, speed: f32, amount: f32) -> f32 {
    1.0 + (seconds * speed).sin() * amount
}

pub fn pulse01(seconds: f32, speed: f32, phase: f32) -> f32 {
    (seconds * speed + phase).sin() * 0.5 + 0.5
}
