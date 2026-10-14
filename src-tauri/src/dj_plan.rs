//! Pure desktop transition planning. Plans are estimates and are not audio
//! scheduling; the sample-clocked engine must validate them before execution.
use crate::structure::Structure;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    FullSong,
    #[default]
    SmartTransition,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Character {
    #[default]
    Subtle,
    Creative,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub mode: Mode,
    pub character: Character,
    pub preserve_energy: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: Mode::SmartTransition,
            character: Character::Subtle,
            preserve_energy: true,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    CleanBlend,
    EchoTail,
    ReverbTail,
    EchoBounce,
    Brake,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub outgoing_cue: f32,
    pub incoming_cue: f32,
    pub duration: f32,
    pub incoming_tempo: f32,
    pub effect: Effect,
    pub confidence: f32,
    pub reason: String,
}
#[derive(Clone, Copy)]
pub struct Track<'a> {
    pub key_index:Option<usize>,
    pub duration: f32,
    pub bpm: Option<f32>,
    pub structure: Option<&'a Structure>,
}
fn harmonic_distance(a:Option<usize>,b:Option<usize>)->f32{match(a,b){(Some(a),Some(b))if a<24&&b<24=>{let relative=(a<12&&b==(a+9)%12+12)||(b<12&&a==(b+9)%12+12);let fifth=a/12==b/12&&((a%12+7)%12==b%12||(b%12+7)%12==a%12);if a==b||relative{0.}else if fifth{0.2}else{0.7}},_=>0.}}
fn safe_duration(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.)
    } else {
        0.
    }
}
pub fn plan(a: Track<'_>, b: Track<'_>, config: &Config, contiguous_album: bool) -> Option<Plan> {
    if !config.enabled {
        return None;
    }
    let length = safe_duration(a.duration);
    let incoming_length = safe_duration(b.duration);
    if length == 0. || incoming_length == 0. {
        return None;
    }
    if contiguous_album {
        return Some(Plan {
            outgoing_cue: length,
            incoming_cue: 0.,
            duration: 0.,
            incoming_tempo: 1.,
            effect: Effect::CleanBlend,
            confidence: 1.,
            reason: "Contiguous album tracks retain their natural handoff".into(),
        });
    }
    let fallback = |reason:&str| {
        let duration = 2_f32.min(length / 4.).min(incoming_length / 4.);
        Plan {
            outgoing_cue: length - duration,
            incoming_cue: 0.,
            duration,
            incoming_tempo: 1.,
            effect: Effect::CleanBlend,
            confidence: 0.,
            reason: format!("{reason}: unwarped ending blend"),
        }
    };
    if config.mode == Mode::FullSong {
        return Some(Plan {
            reason: "Full song: overlap only within the ending; incoming starts at zero".into(),
            ..fallback("Full song")
        });
    }
    if length < 45. || incoming_length < 30. {
        return Some(fallback("Short track"));
    }
    let (Some(sa), Some(sb), Some(ba), Some(bb)) = (a.structure, b.structure, a.bpm, b.bpm) else {
        return Some(fallback("Structural analysis unavailable"));
    };
    if sa.beat_confidence < 0.5
        || sb.beat_confidence < 0.5
        || !ba.is_finite()
        || !bb.is_finite()
        || ba <= 0.
        || bb <= 0.
    {
        return Some(fallback("Beat grid uncertain"));
    }
    let tempo = ba / bb;
    if !(0.94..=1.06).contains(&tempo) {
        return Some(fallback("Tempo difference exceeds six percent"));
    }
    let allowed = |cue: f32| {
        cue >= length * 2. / 3.
            && cue < length
            && sa.regions.iter().all(|r| {
                !r.repeated || r.confidence < 0.5 || cue <= r.start + 0.15 || cue >= r.end - 0.15
            })
    };
    let mut best: Option<(f32, Plan)> = None;
    for beats in [4., 8., 16.] {
        let duration = beats * 60. / ba;
        if !(2. ..=16.).contains(&duration) {
            continue;
        }
        for end in &sa.phrases {
            let cue = end.seconds - duration;
            if end.confidence < 0.5 || !allowed(cue) || end.seconds > length {
                continue;
            }
            // A cue must itself land at a supported boundary: do not cut the preceding phrase.
            if !sa
                .phrases
                .iter()
                .any(|p| p.confidence >= 0.5 && (p.seconds - cue).abs() < 0.15)
            {
                continue;
            }
            for start in sb.phrases.iter().filter(|p| {
                p.confidence >= 0.5
                    && p.seconds <= incoming_length / 3.
                    && p.seconds + duration * tempo < incoming_length
            }) {
                let outgoing_density = density(sa, cue, duration);
                let incoming_density = density(sb, start.seconds, duration * tempo);
                let dense_overlap = outgoing_density * incoming_density;
                let harmonic=harmonic_distance(a.key_index,b.key_index);
                let score = dense_overlap + harmonic*duration/16.
                    + (sa.energy-sb.energy).abs()*duration/16.
                    + (length - end.seconds) / length * 0.15
                    + start.seconds / incoming_length * 0.1;
                let effect = if dense_overlap < 0.2 {
                    Effect::CleanBlend
                } else if config.character == Character::Creative
                    && outgoing_density > incoming_density * 1.5
                {
                    Effect::EchoBounce
                } else if outgoing_density > incoming_density {
                    Effect::EchoTail
                } else {
                    Effect::ReverbTail
                };
                let p = Plan {
                    outgoing_cue: cue,
                    incoming_cue: start.seconds,
                    duration,
                    incoming_tempo: tempo,
                    effect,
                    confidence: sa
                        .beat_confidence
                        .min(sb.beat_confidence)
                        .min(end.confidence)
                        .min(start.confidence),
                    reason: format!(
                        "{}-beat blend at supported phrase boundaries",
                        beats as usize
                    ),
                };
                if best.as_ref().is_none_or(|(old, _)| score < *old) {
                    best = Some((score, p));
                }
            }
        }
    }
    Some(best.map(|(_, p)| p).unwrap_or_else(||fallback("No compatible supported phrase boundaries")))
}
fn density(s: &Structure, start: f32, duration: f32) -> f32 {
    if !s.energy_step.is_finite() || s.energy_step <= 0. {
        return 1.;
    }
    let a = (start / s.energy_step).max(0.) as usize;
    let b = ((start + duration) / s.energy_step).max(0.) as usize;
    let Some(values) = s.envelope.get(a..b.min(s.envelope.len())) else {
        return 1.;
    };
    if values.is_empty() {
        1.
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}
/// Return upcoming positions only. Pins identify queue positions, not track IDs;
/// duplicate queue occurrences and the current/past queue remain stable.
pub fn order_upcoming<T>(
    items: &[T],
    start: usize,
    pins: &std::collections::BTreeSet<usize>,
    mut cost: impl FnMut(Option<usize>, usize) -> f32,
) -> Vec<usize> {
    let start = start.min(items.len());
    let mut remaining = (start..items.len())
        .filter(|i| !pins.contains(i))
        .collect::<Vec<_>>();
    let mut result = Vec::with_capacity(items.len() - start);
    let mut previous = start.checked_sub(1);
    for position in start..items.len() {
        let index = if pins.contains(&position) {
            position
        } else {
            let best = (0..remaining.len())
                .min_by(|a, b| {
                    cost(previous, remaining[*a]).total_cmp(&cost(previous, remaining[*b]))
                })
                .unwrap();
            remaining.remove(best)
        };
        result.push(index);
        previous = Some(index);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{Boundary, Region};
    fn config() -> Config {
        Config {
            enabled: true,
            ..Default::default()
        }
    }
    fn track<'a>(duration: f32, bpm: Option<f32>, structure: Option<&'a Structure>) -> Track<'a> {
        Track {
            key_index:None,
            duration,
            bpm,
            structure,
        }
    }
    #[test]
    fn same_album_adjacency_does_not_imply_a_continuous_passage() {
        let p=plan(track(180.,None,None),track(180.,None,None),&config(),false).unwrap();
        assert_eq!(p.duration,2.);assert_eq!(p.outgoing_cue,178.);
        let p=plan(track(180.,None,None),track(180.,None,None),&config(),true).unwrap();
        assert_eq!(p.duration,0.);assert_eq!(p.incoming_cue,0.);
    }
    #[test]
    fn defaults_are_off_smart_subtle_energy() {
        let c = Config::default();
        assert!(!c.enabled && c.preserve_energy);
        assert_eq!(c.mode, Mode::SmartTransition);
        assert_eq!(c.character, Character::Subtle);
    }
    #[test]
    fn full_song_starts_at_zero_and_finishes_at_ending() {
        let mut c = config();
        c.mode = Mode::FullSong;
        let p = plan(track(180., None, None), track(90., None, None), &c, false).unwrap();
        assert_eq!(p.outgoing_cue + p.duration, 180.);
        assert_eq!(p.incoming_cue, 0.);
        assert_eq!(p.incoming_tempo, 1.);
    }
    #[test]
    fn short_low_confidence_and_extreme_tempos_fall_back() {
        let s = Structure {
            beat_confidence: 1.,
            ..Default::default()
        };
        for (length, tempo) in [(20., 120.), (180., 180.)] {
            let p = plan(
                track(length, Some(120.), Some(&s)),
                track(180., Some(tempo), Some(&s)),
                &config(),
                false,
            )
            .unwrap();
            assert_eq!(p.confidence, 0.);
            assert_eq!(p.incoming_tempo, 1.);
            assert_eq!(p.outgoing_cue + p.duration, length);
        }
    }
    #[test]
    fn repeated_region_prevents_early_cut() {
        let s = Structure {
            beat_confidence: 1.,
            phrases: vec![
                Boundary {
                    seconds: 120.,
                    confidence: 1.,
                },
                Boundary {
                    seconds: 128.,
                    confidence: 1.,
                },
            ],
            regions: vec![Region {
                start: 100.,
                end: 170.,
                confidence: 1.,
                repeated: true,
            }],
            ..Default::default()
        };
        let p = plan(
            track(180., Some(120.), Some(&s)),
            track(180., Some(120.), Some(&s)),
            &config(),
            false,
        )
        .unwrap();
        assert_eq!(p.confidence, 0.);
    }
    #[test]
    fn compatible_boundaries_plan_bounded_pitch_preserving_tempo() {
        let a = Structure {
            beat_confidence: 1.,
            phrases: vec![
                Boundary {
                    seconds: 120.,
                    confidence: 1.,
                },
                Boundary {
                    seconds: 128.,
                    confidence: 1.,
                },
            ],
            ..Default::default()
        };
        let b = Structure {
            beat_confidence: 1.,
            phrases: vec![Boundary {
                seconds: 0.,
                confidence: 1.,
            }],
            ..Default::default()
        };
        let p = plan(
            track(180., Some(120.), Some(&a)),
            track(180., Some(124.), Some(&b)),
            &config(),
            false,
        )
        .unwrap();
        assert!(p.confidence >= 0.5 && p.outgoing_cue >= 120. && (2. ..=16.).contains(&p.duration));
        assert!((0.94..=1.06).contains(&p.incoming_tempo));
    }
    #[test]
    fn album_handoff_has_no_overlap() {
        let p = plan(
            track(180., None, None),
            track(180., None, None),
            &config(),
            true,
        )
        .unwrap();
        assert_eq!(p.duration, 0.);
        assert_eq!(p.outgoing_cue, 180.);
    }
    #[test]
    fn manual_positions_are_pinned_without_moving_current_or_past() {
        let pins = [3].into_iter().collect();
        let order = order_upcoming(&[0, 1, 2, 3, 4, 5], 2, &pins, |_, i| -(i as f32));
        assert_eq!(order, vec![5, 3, 4, 2]);
        assert!(!order.contains(&0) && !order.contains(&1));
    }
}
