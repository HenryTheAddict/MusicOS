use std::time::Instant;

use crate::models::Track;

#[derive(Debug, Clone)]
pub struct PlaybackIntent {
    pub command_id: u64,
    pub expected_revision: u64,
    pub track_id: Option<i64>,
    pub issued_at: Instant,
}

impl PlaybackIntent {
    pub fn is_satisfied(&self, revision: u64, track: Option<&Track>) -> bool {
        revision >= self.expected_revision
            && self
                .track_id
                .is_none_or(|track_id| track.is_some_and(|track| track.id == track_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_revisions_cannot_complete_a_new_intent() {
        let intent = PlaybackIntent {
            command_id: 2,
            expected_revision: 7,
            track_id: Some(42),
            issued_at: Instant::now(),
        };
        let track = Track {
            id: 42,
            ..Track::default()
        };
        assert!(!intent.is_satisfied(6, Some(&track)));
        assert!(intent.is_satisfied(7, Some(&track)));
    }
}
