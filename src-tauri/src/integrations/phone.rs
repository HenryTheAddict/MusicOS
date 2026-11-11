use super::{now, persist, threshold, IntegrationService, LocalPlaybackSnapshot, Scrobble, Worker};
use musicos_desktop::models::Track;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

const RECEIPT_AGE: u64 = 14 * 24 * 3600;
const MAX_BATCH: usize = 12;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct ListenJournal {
    pub(super) queue: Vec<Scrobble>,
    receipts: BTreeMap<String, u64>,
}

impl ListenJournal {
    pub(super) fn load(directory: &Path) -> Self {
        let current = directory.join("lastfm-journal-v2.json");
        if let Ok(bytes) = std::fs::read(current) {
            return serde_json::from_slice(&bytes).unwrap_or_default();
        }
        Self {
            queue: std::fs::read(directory.join("lastfm-queue-v1.json"))
                .ok()
                .and_then(|data| serde_json::from_slice(&data).ok())
                .unwrap_or_default(),
            receipts: BTreeMap::new(),
        }
    }

    pub(super) fn save(&self, directory: &Path) -> anyhow::Result<()> {
        persist(&directory.join("lastfm-journal-v2.json"), self)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhoneUpdate {
    pub(crate) presence: Option<PhonePlayback>,
    #[serde(default)]
    pub(crate) listens: Vec<PhoneListen>,
}

impl PhoneUpdate {
    pub(crate) fn valid(&self) -> bool {
        self.listens.len() <= MAX_BATCH
            && self.listens.iter().all(|listen| {
                listen.track_id > 0
                    && listen.id.len() == 36
                    && listen.id.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
            })
            && self.presence.as_ref().is_none_or(|state| {
                state.track_id > 0 && state.position.is_finite() && state.position >= 0.
            })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhonePlayback {
    pub(crate) track_id: i64,
    position: f32,
    observed_at: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhoneListen {
    pub(crate) track_id: i64,
    id: String,
    started_at: u64,
    listened_seconds: u32,
}

#[derive(Serialize)]
pub(crate) struct PhoneStatus {
    pub(crate) lastfm_ready: bool,
    pub(crate) discord_ready: bool,
}

#[derive(Serialize)]
pub(crate) struct PhoneReply {
    #[serde(flatten)]
    services: PhoneStatus,
    accepted: Vec<String>,
    discarded: Vec<String>,
}

pub(super) struct PhonePresence {
    scope: String,
    peer: String,
    track: Track,
    position: f32,
    received: Instant,
}

impl PhonePresence {
    fn new(scope: &str, peer: &str, state: PhonePlayback, track: Track) -> Option<Self> {
        let now = now();
        if now.saturating_sub(state.observed_at) > 15 || state.observed_at > now + 15 {
            return None;
        }
        let position = state.position + now.saturating_sub(state.observed_at) as f32;
        if track.duration <= 0 || position >= track.duration as f32 {
            return None;
        }
        Some(Self {
            scope: scope.into(),
            peer: peer.into(),
            track,
            position,
            received: Instant::now(),
        })
    }

    pub(super) fn snapshot(&self) -> Option<LocalPlaybackSnapshot> {
        let elapsed = self.received.elapsed();
        let position = self.position + elapsed.as_secs_f32();
        if elapsed >= Duration::from_secs(28) || position >= self.track.duration as f32 {
            return None;
        }
        Some(LocalPlaybackSnapshot {
            track: Some(self.track.clone()),
            position,
            is_playing: true,
            ..Default::default()
        })
    }
}

impl IntegrationService {
    pub(crate) fn clear_phone(&self, scope: &str, peer: Option<&str>) {
        let mut state = self.phone_presence.lock().unwrap_or_else(|e| e.into_inner());
        if state.as_ref().is_some_and(|lease| {
            lease.scope == scope && peer.is_none_or(|id| lease.peer == id)
        }) {
            *state = None;
        }
    }

    pub(crate) fn phone_status(&self) -> PhoneStatus {
        self.worker.lock().unwrap_or_else(|e| e.into_inner()).phone_status()
    }

    pub(crate) fn phone_update(
        &self,
        scope: &str,
        peer: &str,
        update: PhoneUpdate,
        tracks: &BTreeMap<i64, Track>,
        allowed: impl FnOnce() -> bool,
    ) -> anyhow::Result<PhoneReply> {
        let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        anyhow::ensure!(allowed(), "Listening access was turned off");
        let services = worker.phone_status();
        let lease = update.presence.and_then(|state| {
            let track = tracks.get(&state.track_id)?.clone();
            PhonePresence::new(scope, peer, state, track)
        });
        if services.discord_ready {
            if let Some(lease) = lease {
                *self.phone_presence.lock().unwrap_or_else(|e| e.into_inner()) = Some(lease);
            } else {
                self.clear_phone(scope, Some(peer));
            }
        } else {
            self.clear_phone(scope, Some(peer));
        }

        let mut reply = PhoneReply { services, accepted: vec![], discarded: vec![] };
        let Some(username) = worker.config.lastfm_username.clone().filter(|_| reply.services.lastfm_ready) else {
            return Ok(reply);
        };
        let mut journal = worker.journal.clone();
        let now = now();
        journal.receipts.retain(|_, timestamp| now.saturating_sub(*timestamp) <= RECEIPT_AGE);
        let mut changed = false;
        for listen in update.listens {
            let key = format!("{peer}:{}", listen.id.to_ascii_lowercase());
            if journal.receipts.contains_key(&key) {
                reply.accepted.push(listen.id);
                continue;
            }
            let track = tracks.get(&listen.track_id).filter(|track| {
                track.duration > 30
                    && listen.listened_seconds as f32 >= threshold(track.duration)
                    && listen.started_at <= now + 15
                    && now.saturating_sub(listen.started_at) <= RECEIPT_AGE
                    && listen.started_at.saturating_add(listen.listened_seconds as u64) <= now + 30
            });
            let Some(track) = track else {
                reply.discarded.push(listen.id);
                continue;
            };
            if journal.queue.len() >= 500 || journal.receipts.len() >= 30_000 {
                continue;
            }
            journal.queue.push(Scrobble {
                artist: track.artist.clone(),
                track: track.title.clone(),
                album: track.album.clone(),
                duration: track.duration,
                timestamp: listen.started_at,
                username: username.clone(),
            });
            journal.receipts.insert(key, listen.started_at);
            reply.accepted.push(listen.id);
            changed = true;
        }
        if changed {
            journal.save(&worker.directory)?;
            worker.journal = journal;
            worker.status.queued = worker.journal.queue.len();
        }
        Ok(reply)
    }
}

impl Worker {
    fn phone_status(&self) -> PhoneStatus {
        PhoneStatus {
            lastfm_ready: self.config.lastfm_enabled && self.config.lastfm_username.is_some(),
            discord_ready: cfg!(unix) && self.config.discord_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_advances_and_expires_without_a_heartbeat() {
        let state = PhonePlayback { track_id: 1, position: 10., observed_at: now() };
        let track = Track { id: 1, duration: 120, ..Default::default() };
        let mut lease = PhonePresence::new("scope", "phone", state, track).unwrap();
        lease.received = Instant::now() - Duration::from_secs(5);
        let playback = lease.snapshot().unwrap();
        assert!(playback.is_playing);
        assert!((15. ..16.).contains(&playback.position));
        lease.received = Instant::now() - Duration::from_secs(30);
        assert!(lease.snapshot().is_none());
    }

    #[test]
    fn older_lastfm_queue_survives_the_journal_upgrade() {
        let root = tempfile::tempdir().unwrap();
        let original = vec![Scrobble {
            artist: "Artist".into(), track: "Song".into(), album: "Album".into(),
            duration: 180, timestamp: now(), username: "fixture".into(),
        }];
        persist(&root.path().join("lastfm-queue-v1.json"), &original).unwrap();
        let mut journal = ListenJournal::load(root.path());
        assert_eq!(journal.queue.len(), 1);
        journal.queue.clear();
        journal.save(root.path()).unwrap();
        assert!(ListenJournal::load(root.path()).queue.is_empty());
    }
}
