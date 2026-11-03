//! xorshift stream used by the pinned Butterchurn deterministic fixture.
#[derive(Clone)]
pub struct Random {
    state: std::sync::Arc<std::sync::Mutex<[u32; 4]>>,
}
impl Random {
    pub fn new(seed: u64) -> Self {
        let seed = seed as u32;
        let mut r = Self {
            state: std::sync::Arc::new(std::sync::Mutex::new([
                seed,
                seed ^ 0x9e3779b9,
                seed ^ 0x6a09e667,
                seed ^ 0xbb67ae85,
            ])),
        };
        for _ in 0..10 {
            r.next();
        }
        r
    }
    /// A clone continues the same preset stream, including component equations.
    pub fn from_state(state: [u32; 4]) -> Self {
        Self {
            state: std::sync::Arc::new(std::sync::Mutex::new(state)),
        }
    }
    pub fn next(&mut self) -> f64 {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut t = state[3];
        let s = state[0];
        state[3] = state[2];
        state[2] = state[1];
        state[1] = s;
        t ^= t << 11;
        t ^= t >> 8;
        state[0] = t ^ s ^ (s >> 19);
        state[0] as f64 / 4294967296.
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn clones_share_one_ordered_stream() {
        let mut reference = super::Random::new(42);
        let mut a = super::Random::new(42);
        let mut b = a.clone();
        assert_eq!(a.next(), reference.next());
        assert_eq!(b.next(), reference.next());
        assert_eq!(a.next(), reference.next());
    }
}
