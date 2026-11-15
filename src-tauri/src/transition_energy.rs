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

/// Prefer a sustained decline, and leave the original ending alone if energy
/// comes back (a break before the last chorus, for example).
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
        // The window test confirms the decline. Refine the cue near the first
        // quiet bin so averaging across a sharp ending does not start the fade early.
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
        let overlap = (duration - cue).min(if near_silence { 3. } else { 4.5 });
        if overlap >= 2. {
            return Some(Tail { drop, cue, overlap });
        }
    }
    None
}

/// Skip only the measured dead air before the first audible event. A quiet
/// intro or a single pickup remains part of the song.
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

    #[test]
    fn tail_tracks_the_outro_and_ignores_a_break_before_the_final_hit() {
        let mut samples = vec![0.7; 240];
        samples[200..216].fill(0.07);
        assert!(tail(envelope(&samples), 60.).is_none());
        samples[220..].fill(0.12);
        let fade = tail(envelope(&samples), 60.).unwrap();
        assert!((54.5..=55.5).contains(&fade.drop));
        assert!(fade.cue + fade.overlap < 60.);
        samples[220..].fill(0.);
        let fade = tail(envelope(&samples), 60.).unwrap();
        assert!(fade.cue < fade.drop);
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
