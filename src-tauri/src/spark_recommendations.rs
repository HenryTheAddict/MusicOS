//! Read-only saved-mix ranking. No generation, model inference, or queue mutation.
use crate::{collections::Collection, spark_engine::Candidate};
use musicos_desktop::models::Track;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub id: String,
    pub reason: String,
}
fn unit(values: impl Iterator<Item = (Vec<f32>, f32)>) -> Option<Vec<f32>> {
    let mut sum = vec![0.; 512];
    let mut count = 0;
    for (v, w) in values {
        if v.len() != 512 || !v.iter().all(|x| x.is_finite()) {
            continue;
        }
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm < 1e-6 {
            continue;
        }
        for (out, x) in sum.iter_mut().zip(v) {
            *out += x / norm * w;
        }
        count += 1;
    }
    let norm = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
    if count == 0 || norm < 1e-6 {
        return None;
    }
    sum.iter_mut().for_each(|x| *x /= norm);
    Some(sum)
}
fn artists(t: &Track) -> BTreeSet<String> {
    t.artist
        .split(';')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}
/// Current music dominates; recent listening decays by rank, favorites are a quiet fallback.
/// Diversity reduces near-identical mixes, rather than changing any saved sequence.
pub fn rank(
    items: &[Collection],
    tracks: &[Track],
    audio: &[Candidate],
    current: Option<i64>,
    recent: &[i64],
) -> Vec<Recommendation> {
    let tracks: BTreeMap<_, _> = tracks
        .iter()
        .filter(|t| t.available)
        .map(|t| (t.id, t))
        .collect();
    let audio: BTreeMap<_, _> = audio.iter().map(|c| (c.id, &c.embedding)).collect();
    let mut signals = BTreeMap::<i64, f32>::new();
    if let Some(id) = current.filter(|id| tracks.contains_key(id)) {
        signals.insert(id, 4.);
    }
    for (i, id) in recent
        .iter()
        .filter(|id| tracks.contains_key(id))
        .take(12)
        .enumerate()
    {
        signals.entry(*id).or_insert(1.5 / (1. + i as f32 * 0.35));
    }
    if signals.is_empty() {
        let mut favorites = tracks.values().filter(|t| t.favorited).collect::<Vec<_>>();
        favorites.sort_by_key(|t| (std::cmp::Reverse(t.play_count), t.id));
        for t in favorites.into_iter().take(32) {
            signals.insert(t.id, 1.);
        }
    }
    if signals.is_empty() {
        return vec![];
    }
    let target = unit(
        signals
            .iter()
            .filter_map(|(id, w)| audio.get(id).map(|v| ((*v).clone(), *w))),
    );
    let total = signals.values().sum::<f32>();
    let mut scored = Vec::new();
    for item in items.iter().filter(|c| c.kind == "audio") {
        let ids: BTreeSet<_> = item
            .track_ids
            .iter()
            .filter(|id| tracks.contains_key(id))
            .copied()
            .collect();
        if ids.is_empty() {
            continue;
        }
        let mean = unit(
            ids.iter()
                .filter_map(|id| audio.get(id).map(|v| ((*v).clone(), 1.))),
        );
        let similarity = target
            .as_ref()
            .zip(mean.as_ref())
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| a * b).sum::<f32>().max(0.));
        let mut tag_score = 0.;
        for (id, w) in &signals {
            let seed = tracks[id];
            let credits = artists(seed);
            let best = ids
                .iter()
                .map(|id| {
                    let t = tracks[id];
                    if t.id == seed.id {
                        1.
                    } else if !credits.is_disjoint(&artists(t)) {
                        0.65
                    } else if !seed.genre.trim().is_empty()
                        && seed.genre.eq_ignore_ascii_case(&t.genre)
                    {
                        0.3
                    } else {
                        0.
                    }
                })
                .fold(0_f32, f32::max);
            tag_score += best * w / total;
        }
        let score = similarity.map_or(tag_score, |s| s * 0.85 + tag_score * 0.15);
        if score < 0.12 {
            continue;
        }
        let reason = if current.is_some_and(|id| ids.contains(&id)) {
            "Includes what’s playing"
        } else if similarity.is_some() && current.is_some_and(|id| audio.contains_key(&id)) {
            "Fits the sound of what’s playing"
        } else if current.is_some_and(|id| tracks.contains_key(&id)) {
            "Similar to what’s playing"
        } else if !recent.is_empty() {
            "From your recent listening"
        } else {
            "From your favorites"
        };
        scored.push((item.id.clone(), reason.to_string(), score, ids));
    }
    let mut selected = Vec::new();
    let mut chosen: Vec<BTreeSet<i64>> = Vec::new();
    while selected.len() < 3 && !scored.is_empty() {
        let adjusted = |row: &(String, String, f32, BTreeSet<i64>)| {
            row.2
                - chosen
                    .iter()
                    .map(|old| {
                        old.intersection(&row.3).count() as f32 / old.union(&row.3).count() as f32
                            * 0.12
                    })
                    .fold(0_f32, f32::max)
        };
        let best = (0..scored.len())
            .max_by(|a, b| {
                adjusted(&scored[*a])
                    .total_cmp(&adjusted(&scored[*b]))
                    .then_with(|| scored[*b].0.cmp(&scored[*a].0))
            })
            .unwrap();
        let (id, reason, _, ids) = scored.remove(best);
        chosen.push(ids);
        selected.push(Recommendation { id, reason });
    }
    selected
}
#[cfg(test)]
mod tests {
    use super::*;
    fn track(id: i64, artist: &str) -> Track {
        Track {
            id,
            artist: artist.into(),
            available: true,
            ..Default::default()
        }
    }
    fn mix(id: &str, ids: Vec<i64>) -> Collection {
        Collection {
            updated_at: 0,
            id: id.into(),
            name: id.into(),
            track_ids: ids,
            spark: None,
            kind: "audio".into(),
            description: String::new(),
            artwork: None,
            artwork_choice: None,
            target_count: None,
        }
    }
    fn audio(id: i64, axis: usize) -> Candidate {
        let mut embedding = vec![0.; 512];
        embedding[axis] = 1.;
        Candidate {
            id,
            artist: String::new(),
            album: String::new(),
            favorite: false,
            embedding,
        }
    }
    #[test]
    fn current_audio_outweighs_recent_and_names() {
        let tracks = vec![track(1, "A"), track(2, "B"), track(3, "C")];
        let items = vec![mix("unrelated", vec![3]), mix("similar", vec![2])];
        let result = rank(
            &items,
            &tracks,
            &[audio(1, 0), audio(2, 0), audio(3, 1)],
            Some(1),
            &[3],
        );
        assert_eq!(result[0].id, "similar");
        assert!(result[0].reason.contains("sound"));
        assert_eq!(items[0].track_ids, vec![3]);
    }
    #[test]
    fn cold_start_favorites_unavailable_empty_and_manual_are_honest() {
        let mut tracks = vec![track(1, "A"), track(2, "A"), track(3, "B")];
        let mut items = vec![
            mix("same", vec![2]),
            mix("empty", vec![]),
            mix("missing", vec![999]),
            mix("manual", vec![1]),
        ];
        items[3].kind = "manual".into();
        assert!(rank(&items, &tracks, &[], None, &[]).is_empty());
        tracks[0].favorited = true;
        let result = rank(&items, &tracks, &[], None, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "same");
        assert_eq!(result[0].reason, "From your favorites");
        tracks[1].available = false;
        assert!(rank(&items, &tracks, &[], None, &[]).is_empty());
    }
    #[test]
    fn rankings_are_deterministic_and_ignore_duplicate_members() {
        let tracks = vec![track(1, "A"), track(2, "A")];
        let items = vec![mix("b", vec![2, 2]), mix("a", vec![2])];
        let result = rank(&items, &tracks, &[], Some(1), &[]);
        assert_eq!(result[0].id, "a");
        assert_eq!(result[1].id, "b");
    }
}
