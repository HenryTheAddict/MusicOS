//! Shared cue selection for desktop structural analysis and the phone's
//! bounded head/tail PCM reads. All positions are in source seconds.

#[derive(Clone, Copy)]
pub struct Envelope<'a> {
    pub start: f32,
    pub step: f32,
    pub values: &'a [f32],
}

impl Envelope<'_> {
    fn mean(self, start: f32, end: f32) -> Option<f32> {
        if !self.step.is_finite() || self.step <= 0. || !self.start.is_finite() {
            return None;
        }
        if start < self.start - self.step * 0.1
            || end > self.start + self.values.len() as f32 * self.step + self.step * 0.1
        {
            return None;
        }
        let index = |time: f32| ((time - self.start) / self.step).round().max(0.) as usize;
        let samples = self
            .values
            .get(index(start)..index(end).min(self.values.len()))?;
        if samples.is_empty()
            || samples
                .iter()
                .any(|value| !value.is_finite() || *value < 0.)
        {
            return None;
        }
        Some(samples.iter().sum::<f32>() / samples.len() as f32)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tail {
    pub drop: f32,
    pub cue: f32,
    pub overlap: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ColdEnding {
    pub hit: f32,
}

pub fn cold_ending(envelope: Envelope<'_>, duration: f32) -> Option<ColdEnding> {
    if !duration.is_finite() || duration < 10. || !envelope.step.is_finite()
        || envelope.step <= 0. || envelope.step > 0.1
    {
        return None;
    }
    let body = envelope.mean(duration - 8., duration - 4.)?;
    let late_body = envelope.mean(duration - 4., duration - 1.6)?;
    let ending = envelope.mean(duration - 0.2, duration)?;
    if body < 0.12 || late_body < body * 0.8 || late_body > body * 1.25
        || ending > body * 0.12
    {
        return None;
    }
    for offset in 0..=12 {
        let hit = duration - 1.6 + offset as f32 * 0.1;
        let before = envelope.mean(hit - 0.5, hit)?;
        let attack = envelope.mean(hit, hit + 0.1)?;
        let ring = envelope.mean(hit + 0.2, hit + 0.4)?;
        if before >= body * 0.8 && attack >= body * 0.65
            && ring >= body * 0.025 && ring <= body * 0.55
        {
            return Some(ColdEnding { hit });
        }
    }
    None
}

/// Wait for the energy to stay down. That quiet break might have one more chorus in it.
pub fn tail(envelope: Envelope<'_>, duration: f32) -> Option<Tail> {
    if !duration.is_finite() || duration < 10. {
        return None;
    }
    let first = (duration - 12.)
        .max(duration * 0.75)
        .max(envelope.start + 2.5);
    for offset in 0..=48 {
        let drop = first + 0.25 * offset as f32;
        if drop + 1.5 > duration {
            break;
        }
        let Some(before) = envelope.mean(drop - 2.5, drop - 0.5) else {
            continue;
        };
        let Some(after) = envelope.mean(drop + 0.5, duration) else {
            continue;
        };
        if before < 0.002 || after > before * 0.45 {
            continue;
        }
        let mut window = drop + 0.5;
        let mut stays_quiet = true;
        while window + 0.25 <= duration {
            if envelope
                .mean(window, (window + 0.75).min(duration))
                .is_none_or(|v| v > before * 0.56)
            {
                stays_quiet = false;
                break;
            }
            window += 0.65;
        }
        if !stays_quiet {
            continue;
        }
        // The drop checks out. Find its first quiet bin so the averaging
        // does not talk us into fading before the actual ending.
        let drop = (0..=4)
            .map(|n| drop + 0.25 * n as f32)
            .find(|&time| {
                envelope
                    .mean(time, time + 0.25)
                    .is_some_and(|v| v <= before * 0.45)
            })
            .unwrap_or(drop);
        let near_silence = envelope
            .mean(drop, (drop + 1.).min(duration))
            .is_some_and(|v| v < before * 0.09);
        let cue = if near_silence { drop - 0.65 } else { drop };
        let overlap = (duration - cue).min(if near_silence { 3. } else { 7.5 });
        if overlap >= 2. {
            return Some(Tail { drop, cue, overlap });
        }
    }
    gradual_tail(envelope, duration, first)
}

fn gradual_tail(envelope: Envelope<'_>, duration: f32, first: f32) -> Option<Tail> {
    const WINDOW: f32 = 1.5;
    let ending = envelope.mean(duration - WINDOW, duration)?;
    // Give it a proper listen. A couple of quiet beats do not make an outro.
    let start = (duration - 7.5).max(first);
    let mut cue = start;
    while cue <= duration - 2. {
        let reference = envelope.mean(cue - 5., cue - 3.);
        let before = envelope.mean(cue - 3., cue - WINDOW);
        let current = envelope.mean(cue - WINDOW * 0.5, cue + WINDOW * 0.5);
        if let (Some(reference), Some(before), Some(current)) = (reference, before, current) {
            if reference >= 0.002 && before >= reference * 0.72
                && current <= before * 0.85 && current <= reference * 0.68
                && ending <= current * 0.72
            {
                let mut window = cue + WINDOW * 0.5;
                let mut level = current;
                let mut falling = true;
                while window + 0.5 <= duration {
                    let Some(next) = envelope.mean(window, (window + WINDOW).min(duration)) else {
                        falling = false;
                        break;
                    };
                    if next > level + reference * 0.085 {
                        falling = false;
                        break;
                    }
                    level = level.min(next);
                    window += WINDOW * 0.5;
                }
                if falling {
                    return Some(Tail { drop: cue, cue, overlap: duration - cue });
                }
            }
        }
        cue += 0.25;
    }
    None
}

/// Trim the dead air. The quiet intro and that single pickup still belong to the song.
pub fn intro(envelope: Envelope<'_>) -> f32 {
    let cue = first_audible(envelope)
        .map_or(0., |start| (start - envelope.step.max(0.12)).max(0.));
    if (0.35..=8.).contains(&cue) { cue } else { 0. }
}

pub fn first_audible(envelope: Envelope<'_>) -> Option<f32> {
    if envelope.start != 0. || !envelope.step.is_finite() || envelope.step <= 0. {
        return None;
    }
    let samples = &envelope.values[..envelope
        .values
        .len()
        .min((12. / envelope.step).ceil() as usize)];
    let peak = samples
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .fold(0_f32, f32::max);
    if peak < 0.002 {
        return None;
    }
    let threshold = (peak * 0.012).max(0.0003);
    samples.iter()
        .position(|&x| x.is_finite() && x > threshold)
        .map(|index| index as f32 * envelope.step)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(values: &[f32]) -> Envelope<'_> {
        Envelope {
            start: 0.,
            step: 0.25,
            values,
        }
    }

    fn cold_samples() -> Vec<f32> {
        (0..7200).map(|i| {
            let time = i as f32 * 0.025;
            if time < 179. { 0.7 } else if time < 179.7 {
                0.9 * (-(time - 179.) * 4.).exp() * (1. - (time - 179.) / 0.7)
            } else { 0. }
        }).collect()
    }
    fn fine(values: &[f32]) -> Envelope<'_> { Envelope { start: 0., step: 0.025, values } }

    #[test]
    fn cold_ending_keeps_a_short_final_hit_and_ring() {
        let values = cold_samples();
        assert!(tail(fine(&values), 180.).is_none());
        let ending = cold_ending(fine(&values), 180.).unwrap();
        assert!((ending.hit - 179.).abs() <= 0.15, "{ending:?}");
    }

    #[test]
    fn cold_ending_rejects_cuts_fades_drops_silence_and_breaks() {
        let original = cold_samples();
        for case in ["loud_cut", "fade", "drop", "quiet", "silent", "break", "trailing_silence"] {
            let mut values = original.clone();
            for (i, value) in values.iter_mut().enumerate() {
                let time = i as f32 * 0.025;
                *value = match case {
                    "loud_cut" => 0.7,
                    "fade" => 0.7 * ((180. - time) / 8.).clamp(0., 1.),
                    "drop" if time >= 174. => 0.07,
                    "quiet" => *value * 0.01,
                    "silent" => 0.,
                    "break" if (170. ..174.).contains(&time) => 0.07,
                    "trailing_silence" if time >= 174. => 0.,
                    _ => *value,
                };
            }
            assert!(cold_ending(fine(&values), 180.).is_none(), "{case}");
        }
        let mut legacy = vec![0.7; 240];
        legacy[200..216].fill(0.07);
        assert!(cold_ending(envelope(&legacy), 60.).is_none());
    }

    #[test]
    fn cold_ending_rejects_missing_invalid_and_partial_evidence() {
        assert!(cold_ending(fine(&[]), 180.).is_none());
        let mut values = cold_samples();
        values[7000] = f32::NAN;
        assert!(cold_ending(fine(&values), 180.).is_none());
        assert!(cold_ending(fine(&values), f32::INFINITY).is_none());
        assert!(cold_ending(Envelope { step: 0., ..fine(&values) }, 180.).is_none());
        assert!(cold_ending(fine(&values[..7100]), 180.).is_none());
    }

    #[test]
    fn tail_tracks_the_outro_and_ignores_a_break_before_the_final_hit() {
        let mut samples = vec![0.7; 240];
        samples[200..216].fill(0.07);
        assert!(tail(envelope(&samples), 60.).is_none());
        samples[220..].fill(0.12);
        let fade = tail(envelope(&samples), 60.).unwrap();
        assert!((54.5..=55.5).contains(&fade.drop));
        assert_eq!(fade.cue + fade.overlap, 60.);
        samples[220..].fill(0.);
        let fade = tail(envelope(&samples), 60.).unwrap();
        assert!(fade.cue < fade.drop);
    }

    #[test]
    fn slow_fade_uses_the_energy_decline_and_keeps_its_final_notes() {
        let mut samples = vec![0.7; 240];
        for (i, value) in samples[213..].iter_mut().enumerate() {
            *value = 0.7 - i as f32 * 0.016;
        }
        let fade = tail(envelope(&samples), 60.).expect("gradual fade");
        assert!((56. ..=58.).contains(&fade.cue));
        assert_eq!(fade.cue + fade.overlap, 60.);
        samples[232..].fill(0.7);
        assert!(tail(envelope(&samples), 60.).is_none());
    }

    #[test]
    fn intro_skips_silence_but_keeps_a_pickup_and_ignores_empty_analysis() {
        let mut samples = vec![0.; 48];
        assert_eq!(intro(envelope(&samples)), 0.);
        samples[9..].fill(0.35);
        assert_eq!(intro(envelope(&samples)), 2.);
        samples[0] = 0.007;
        assert_eq!(intro(envelope(&samples)), 0.);
    }
}
