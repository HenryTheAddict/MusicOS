//! Local onset/tempo/beat estimates, using a dynamic-programming beat path.
//! Method reference: Ellis, Beat Tracking by Dynamic Programming (2007).
//! Original Rust implementation; no Python or external beat-tracker runtime.
#[derive(Debug, Default)]
pub struct Evidence {
    pub bpm: Option<f32>,
    pub confidence: f32,
    pub beats: Vec<f32>,
}
pub fn track(flux: &[f32], step: f32, hint: Option<f32>) -> Evidence {
    if flux.len() < 32 || !step.is_finite() || step <= 0. {
        return Evidence::default();
    }
    // Remove slow loudness changes before measuring rhythmic periodicity.
    let radius = (0.5 / step).round().max(1.) as usize;
    let mut prefix = vec![0.; flux.len() + 1];
    for (i, &v) in flux.iter().enumerate() {
        prefix[i + 1] = prefix[i] + v;
    }
    let centered = flux
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let a = i.saturating_sub(radius);
            let b = (i + radius + 1).min(flux.len());
            v - (prefix[b] - prefix[a]) / (b - a) as f32
        })
        .collect::<Vec<_>>();
    let variance = centered.iter().map(|v| v * v).sum::<f32>() / flux.len() as f32;
    if variance < 1e-8 {
        return Evidence::default();
    }
    let scale = variance.sqrt();
    let onsets = centered
        .iter()
        .map(|v| (v / scale).max(0.).min(8.))
        .collect::<Vec<_>>();
    let lag_min = (60. / 200. / step).floor().max(2.) as usize;
    let lag_max = (60. / 60. / step).ceil() as usize;
    let correlation = |lag: f32| {
        let low = lag.floor() as usize;
        let part = lag.fract();
        let (mut cross, mut aa, mut bb) = (0., 0., 0.);
        for i in low + 1..centered.len() {
            let a = centered[i];
            let b = centered[i - low] * (1. - part) + centered[i - low - 1] * part;
            cross += a * b;
            aa += a * a;
            bb += b * b;
        }
        if aa * bb > 1e-12 {
            (cross / (aa * bb).sqrt()).max(0.)
        } else {
            0.
        }
    };
    let mut best = (0., 0., f32::NEG_INFINITY);
    for quarter in lag_min * 4..=lag_max * 4 {
        let lag = quarter as f32 / 4.;
        let bpm = 60. / (lag * step);
        if !(60. ..=200.).contains(&bpm) {
            continue;
        }
        let corr = correlation(lag);
        let prior = hint
            .filter(|v| v.is_finite() && (60. ..=200.).contains(v))
            .unwrap_or(120.);
        let score = corr - 0.025 * (bpm / prior).ln().abs();
        if score > best.2 {
            best = (lag, corr, score);
        }
    }
    if best.1 < 0.12 {
        return Evidence::default();
    }
    let period = best.0;
    let min_gap = (period * 0.65).floor().max(1.) as usize;
    let max_gap = (period * 1.4).ceil() as usize;
    let mut scores = vec![0_f32; flux.len()];
    let mut previous = vec![usize::MAX; flux.len()];
    for i in 0..flux.len() {
        let mut best_prev = (0., usize::MAX);
        for gap in min_gap..=max_gap.min(i) {
            let p = i - gap;
            let transition = -35. * (gap as f32 / period).ln().powi(2);
            let value = scores[p] + transition;
            if value > best_prev.0 {
                best_prev = (value, p);
            }
        }
        scores[i] = onsets[i] + best_prev.0;
        previous[i] = best_prev.1;
    }
    let Some(mut i) = (0..scores.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b])) else {
        return Evidence::default();
    };
    let mut indices = vec![i];
    while previous[i] != usize::MAX {
        i = previous[i];
        indices.push(i);
    }
    indices.reverse();
    // Trim unsupported tails, rather than inventing beats into silence.
    while indices.first().is_some_and(|i| onsets[*i] < 0.5) {
        indices.remove(0);
    }
    while indices.last().is_some_and(|i| onsets[*i] < 0.5) {
        indices.pop();
    }
    if indices.len() < 16 {
        return Evidence::default();
    }
    let support =
        indices.iter().filter(|i| onsets[**i] >= 0.5).count() as f32 / indices.len() as f32;
    let intervals = indices
        .windows(2)
        .map(|p| (p[1] - p[0]) as f32)
        .collect::<Vec<_>>();
    let consistent = intervals
        .iter()
        .filter(|v| (**v / period - 1.).abs() < 0.15)
        .count() as f32
        / intervals.len() as f32;
    // Confidence requires independent periodicity, observed attacks, and stable
    // intervals. It is an estimate, not a calibrated probability of correctness.
    let confidence = (best.1.sqrt() * support * consistent).clamp(0., 1.);
    let mut sorted = intervals;
    sorted.sort_by(f32::total_cmp);
    let median = sorted[sorted.len() / 2];
    Evidence {
        bpm: Some(60. / (median * step)),
        confidence,
        beats: indices.into_iter().map(|i| i as f32 * step).collect(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracks_fractional_clicks_through_missing_attacks() {
        let step = 256_f32 / 11025.;
        let period = 60_f32 / 117.5;
        let mut flux = vec![0.; (180. / step) as usize];
        let mut t = 0.2;
        let mut count = 0;
        while t < 180. {
            if count % 7 != 0 {
                flux[(t / step).round() as usize] = 1.;
            }
            count += 1;
            t += period;
        }
        let e = track(&flux, step, Some(117.5));
        assert!(e.confidence > 0.5, "{e:?}");
        assert!((e.bpm.unwrap() - 117.5).abs() < 5.);
        let matched = e
            .beats
            .iter()
            .filter(|b| {
                let phase = ((**b - 0.2) / period).round();
                (**b - (0.2 + phase * period)).abs() < 0.06
            })
            .count();
        assert!(matched as f32 / e.beats.len() as f32 > 0.9);
    }
    #[test]
    fn silence_constant_and_random_noise_are_not_confident() {
        for flux in [vec![0.; 8000], vec![1.; 8000], {
            let mut seed = 13_u32;
            (0..8000)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    (seed >> 8) as f32 / 16777216.
                })
                .collect()
        }] {
            assert!(track(&flux, 256. / 11025., None).confidence < 0.5);
        }
    }
}
