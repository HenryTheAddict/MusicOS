//! Pure desktop transition planning. Plans are estimates and are not audio
//! scheduling; the sample-clocked engine must validate them before execution.
use crate::{structure::Structure, transition_energy::{self, Envelope}};
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    BeatMatch,
    OutroFade,
    Crossfade,
    Gapless,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub kind: Kind,
    pub outgoing_cue: f32,
    pub incoming_cue: f32,
    pub duration: f32,
    pub incoming_tempo: f32,
    pub effect: Effect,
    pub confidence: f32,
    pub reason: String,
}
impl Plan {
    /// A weak ending blend is provisional until preparation is actually due.
    /// Otherwise analysis completing during the song can never improve it:
    /// accepted mixer schedules correctly remain protected from replacement.
    pub fn ready_to_schedule(&self,mode:Mode,position:f32,speed:f32)->bool {
        mode == Mode::FullSong || self.duration == 0. || self.outgoing_cue - position <= 12. * speed
    }
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
    plan_after(a,b,config,contiguous_album,0.)
}
/// Earliest cue is in audible source seconds, leaving time for deck preparation.
pub fn plan_after(a:Track<'_>,b:Track<'_>,config:&Config,contiguous_album:bool,earliest:f32)->Option<Plan> {
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
            kind: Kind::Gapless,
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
        let limit: f32 = if config.mode == Mode::FullSong { 2. } else { 3.5 };
        let duration = limit.min(length / 4.).min(incoming_length / 4.);
        let duration = if config.mode == Mode::SmartTransition {
            let available = duration.min((length - earliest).max(0.));
            if available < 2. { 0. } else { available }
        } else {
            duration
        };
        Plan {
            kind: if duration > 0. { Kind::Crossfade } else { Kind::Gapless },
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
    let outgoing_tail = a.structure.and_then(|s| transition_energy::tail(envelope(s), length));
    let incoming_cue = b.structure.map_or(0., |s| transition_energy::intro(envelope(s)));
    let ending = |reason: &str| {
        if let Some(tail) = outgoing_tail.filter(|tail| {
            tail.cue >= earliest
                && a.structure.is_some_and(|s| {
                    preserves_section(s, tail.cue) && preserves_section(s, tail.cue + tail.overlap)
                })
        }) {
            return Plan {
                kind: Kind::OutroFade,
                outgoing_cue: tail.cue,
                incoming_cue,
                duration: tail.overlap.min(incoming_length / 4.),
                incoming_tempo: 1.,
                effect: Effect::CleanBlend,
                confidence: 0.58,
                reason: "Sustained drop in outro energy".into(),
            };
        }
        let mut plan = fallback(reason);
        if plan.duration > 0. { plan.incoming_cue = incoming_cue; }
        plan
    };
    if length < 45. || incoming_length < 30. {
        return Some(ending("Short track"));
    }
    let (Some(sa), Some(sb), Some(ba), Some(bb)) = (a.structure, b.structure, a.bpm, b.bpm) else {
        return Some(ending("Structural analysis unavailable"));
    };
    if sa.beat_confidence < 0.5
        || sb.beat_confidence < 0.5
        || !ba.is_finite()
        || !bb.is_finite()
        || ba <= 0.
        || bb <= 0.
    {
        return Some(ending("Beat grid uncertain"));
    }
    // These are evidence-supported beat sequences, not an extrapolated BPM clock.
    if !valid_beats(&sa.beats)||!valid_beats(&sb.beats) {
        return Some(ending("Measured beats unavailable"));
    }
    let earliest_cue = (length - 16.).max(earliest);
    let earliest_anchor_end = outgoing_tail.map_or(length - 2.25, |tail| tail.drop - 1.25);
    let max_overlap = if config.character == Character::Subtle { 12. } else { 16. };
    let tail_start = sa.beats.partition_point(|&beat| beat < earliest_cue);
    let tail_end = sa.beats.partition_point(|&beat| beat <= length);
    let head_start = sb.beats.partition_point(|&beat| beat < incoming_cue);
    let head_end = sb.beats.partition_point(|&beat| beat <= incoming_cue + 2.25);
    let mut best: Option<(f32, Plan)> = None;
    let exits = transition_boundaries(sa);
    let entries = transition_boundaries(sb);
    for count in [4_usize, 8, 12, 16, 24, 32] {
        for (a_step, b_step) in [(1, 1), (1, 2), (2, 1)] {
            let a_count = count * a_step;
            let b_count = count * b_step;
            for bi in head_start..head_end {
                let Some(b_beats) = sb.beats.get(bi..=bi + b_count) else { break };
                if !stable_window(b_beats, bb) { continue; }
                let lead = b_beats[0] - incoming_cue;
                let b_span = b_beats[b_count] - b_beats[0];
                let entry_confidence = boundary_confidence(&entries, b_beats[0]);
                for ai in tail_start..tail_end.saturating_sub(a_count) {
                    let a_beats = &sa.beats[ai..=ai + a_count];
                    let anchor_end = a_beats[a_count];
                    if anchor_end < earliest_anchor_end { continue; }
                    // Let the fade cover the last note after the measured beat window.
                    let end = outgoing_tail.map_or(length, |tail| anchor_end.max(tail.drop));
                    if end < length - 12. || !preserves_section(sa, end) { continue; }
                    let a_span = anchor_end - a_beats[0];
                    let tempo = b_span / a_span;
                    if !(0.94..=1.06).contains(&tempo) || !stable_window(a_beats, ba) { continue; }
                    // Start the incoming deck at its safe entrance. Its pickup plays before
                    // the anchor, which lands on the outgoing beat after time stretching.
                    let cue = a_beats[0] - lead / tempo;
                    let duration = end - cue;
                    if cue < earliest_cue || !(2. ..=max_overlap).contains(&duration)
                        || !preserves_section(sa, cue)
                        || outgoing_tail.is_some_and(|t| cue < t.cue - 0.45 || cue > t.drop + 1.25)
                        || incoming_cue + duration * tempo >= incoming_length
                    {
                        continue;
                    }
                    let exit_confidence = boundary_confidence(&exits, a_beats[0]);
                    let rhythmic_confidence = sa.beat_confidence.min(sb.beat_confidence);
                    // A structural cue strengthens the timing. Without one, rely on a
                    // stronger measured grid and a short overlap near the song ending.
                    if exit_confidence < 0.5 && rhythmic_confidence < (if duration > 7. { 0.70 } else { 0.58 }) {
                        continue;
                    }
                    let drift = (0..=count)
                        .map(|j| ((a_beats[j * a_step] - a_beats[0])
                            - (b_beats[j * b_step] - b_beats[0]) / tempo).abs())
                        .fold(0_f32, f32::max);
                    if drift > 0.05 { continue; }
                    let outgoing_density = density(sa, cue, duration);
                    let incoming_density = density(sb, incoming_cue, duration * tempo);
                    let dense_overlap = outgoing_density * incoming_density;
                    let harmonic = harmonic_distance(a.key_index, b.key_index);
                    let energy = (outgoing_density - incoming_density).abs();
                    // Avoid long stretches with two full arrangements competing. Favor
                    // a longer phrase when the measured outro or intro leaves space.
                    if duration > 8. && dense_overlap > 0.38 { continue; }
                    let preferred_duration = if dense_overlap < 0.25 { 10. } else { 6. };
                    let target = outgoing_tail.map_or(length, |t| t.drop);
                    let cut = if outgoing_tail.is_some() { (cue - target).abs() } else { length - end };
                    let confidence = rhythmic_confidence.min(exit_confidence.max(0.5) + 0.12);
                    let score = cut * 0.10 + dense_overlap * 0.35 + harmonic * 0.15 + energy * 0.2
                        + (1. - confidence) * 0.25 + drift / 0.05 * 0.2 + lead * 0.075
                        + (duration - preferred_duration).abs() * 0.025 - exit_confidence * 0.14 - entry_confidence * 0.05
                        + if a_step == b_step { 0. } else { 0.06 };
                    let effect = if config.character == Character::Subtle || dense_overlap < 0.2 {
                        Effect::CleanBlend
                    } else if outgoing_density > incoming_density * 1.5 {
                        Effect::EchoBounce
                    } else if outgoing_density > incoming_density {
                        Effect::EchoTail
                    } else {
                        Effect::ReverbTail
                    };
                    if best.as_ref().is_none_or(|(old, _)| score < *old) {
                        best = Some((score, Plan {
                            kind: Kind::BeatMatch,
                            outgoing_cue: cue, incoming_cue, duration, incoming_tempo: tempo,
                            effect, confidence,
                            reason: format!("{count}-beat measured alignment, {:.2}s lead-in, {:.1}ms maximum drift", lead, drift * 1000.),
                        }));
                    }
                }
            }
        }
    }
    Some(best.map(|(_, p)| p).unwrap_or_else(|| ending("No stable beat alignment near both cues")))
}

/// Score detected phrases and changes close to the candidate beat.
fn boundary_confidence(boundaries: &[crate::structure::Boundary], beat: f32) -> f32 {
    let first = boundaries.partition_point(|value| value.seconds < beat - 0.15);
    boundaries[first..].iter().take_while(|value| value.seconds <= beat + 0.15)
        .map(|value| value.confidence).fold(0., f32::max)
}

fn transition_boundaries(s: &Structure) -> Vec<crate::structure::Boundary> {
    let mut boundaries = s.phrases.iter().chain(&s.changes)
        .filter(|point| point.confidence >= 0.5).cloned().collect::<Vec<_>>();
    boundaries.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
    boundaries
}

fn preserves_section(s:&Structure,time:f32)->bool{s.regions.iter().all(|r|!r.repeated||r.confidence<0.5||time<=r.start+0.05||time>=r.end-0.05)}
fn valid_beats(beats:&[f32])->bool {beats.len()>=5&&beats.iter().all(|t|t.is_finite()&&*t>=0.)&&beats.windows(2).all(|p|p[1]>p[0])}
#[cfg(test)]
fn boundary_beat(beats: &[f32], time: f32) -> Option<usize> {
    beats.iter().position(|beat| (*beat - time).abs() < 0.001)
}
fn stable_window(beats:&[f32],bpm:f32)->bool{
    let period=(beats[beats.len()-1]-beats[0])/(beats.len()-1)as f32;
    let local=60./period;
    // Octave-compatible global estimates are allowed only with actual local beat evidence.
    let octave_agrees=[0.5,1.,2.].iter().any(|factor|(local/(bpm*factor)-1.).abs()<=0.15);
    octave_agrees&&beats.windows(2).all(|p|((p[1]-p[0])/period-1.).abs()<=0.12)
}

fn envelope(s: &Structure) -> Envelope<'_> {
    Envelope { start: 0., step: s.energy_step, values: &s.envelope }
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
                .map(|index| (index, cost(previous, remaining[index])))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .expect("an unpinned position always has a candidate")
                .0;
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
        assert_eq!(p.duration,3.5);assert_eq!(p.outgoing_cue,176.5);
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
            beats:(0..=360).map(|i|i as f32*0.5).collect(),
            beat_confidence: 1.,
            phrases: vec![
                Boundary {
                    seconds: 168.,
                    confidence: 1.,
                },
                Boundary {
                    seconds: 176.,
                    confidence: 1.,
                },
            ],
            ..Default::default()
        };
        let b = Structure {
            beats:(0..=372).map(|i|i as f32*60./124.).collect(),
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
        assert!(p.confidence >= 0.5 && p.outgoing_cue >= 168. && (2. ..=8.).contains(&p.duration));
        assert!((0.94..=1.06).contains(&p.incoming_tempo));
        assert_eq!(p.effect, Effect::CleanBlend);
    }
    fn measured(period:f32)->Structure {
        Structure{beat_confidence:0.9,beats:(0..=400).map(|i|i as f32*period).collect(),
            phrases:(0..=25).map(|i|Boundary{seconds:i as f32*16.*period,confidence:0.9}).collect(),..Default::default()}
    }
    #[test]fn measured_beats_override_bpm_rounding_and_octave_hints(){
        let a=measured(0.502);let b=measured(0.49);
        let p=plan(track(180.,Some(60.),Some(&a)),track(180.,Some(122.),Some(&b)),&config(),false).unwrap();
        assert!(p.confidence>=0.5);assert!((p.incoming_tempo-0.49/0.502).abs()<0.0001);
        let ai=boundary_beat(&a.beats,p.outgoing_cue).unwrap();let bi=boundary_beat(&b.beats,p.incoming_cue).unwrap();
        for j in 0..=16 {let drift=((a.beats[ai+j]-p.outgoing_cue)-(b.beats[bi+j]-p.incoming_cue)/p.incoming_tempo).abs();assert!(drift<=0.05);}
    }
    #[test]
    fn gives_open_arrangements_longer_blends_than_dense_pairs() {
        let mut a = measured(0.5);
        let mut b = measured(0.5);
        a.energy_step = 0.25;
        b.energy_step = 0.25;
        a.envelope = vec![0.3; 720];
        b.envelope = vec![0.25; 720];
        let blend = |a:&Structure,b:&Structure| plan(
            track(180., Some(120.), Some(a)), track(180., Some(120.), Some(b)), &config(), false,
        ).unwrap();
        let spacious = blend(&a,&b);
        assert_eq!(spacious.kind, Kind::BeatMatch);
        assert!((9. ..=12.).contains(&spacious.duration), "{spacious:?}");
        a.envelope.fill(0.85);
        b.envelope.fill(0.9);
        let dense = blend(&a,&b);
        assert!(dense.duration <= 8.);
        assert_eq!(spacious.outgoing_cue + spacious.duration, 180.);
        assert_eq!(dense.outgoing_cue + dense.duration, 180.);
    }
    #[test]fn constant_ratio_cannot_hide_local_drift(){
        let a=measured(0.5);let mut b=measured(0.5);
        for (i,beat) in b.beats.iter_mut().enumerate(){if i%2==1{*beat+=0.055;}}
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false).unwrap();
        assert_eq!(p.confidence,0.);assert_eq!(p.incoming_tempo,1.);
    }
    #[test]fn future_planning_does_not_reuse_missed_phrase(){
        let a=measured(0.5);let b=measured(0.5);
        let p=plan_after(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false,162.).unwrap();
        assert!(p.outgoing_cue>=162.);assert!(p.confidence>=0.5);
        let p=plan_after(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false,178.1).unwrap();
        assert_eq!(p.kind,Kind::Gapless);assert_eq!(p.outgoing_cue,180.);
    }
    #[test]fn overlap_must_not_finish_inside_a_protected_section(){
        let mut a=measured(0.5);let b=measured(0.5);
        a.phrases.retain(|p|p.seconds==176.);
        a.regions=vec![Region{start:176.,end:181.,repeated:true,confidence:0.9}];
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false).unwrap();assert_eq!(p.confidence,0.);
    }
    #[test]fn measured_changes_support_a_blend_without_a_second_section_at_its_end(){
        let mut a=measured(0.5);let mut b=measured(0.49);a.phrases.clear();b.phrases.clear();
        a.beat_confidence=0.55;
        a.changes=vec![Boundary{seconds:176.08,confidence:0.8}];b.changes=vec![Boundary{seconds:7.9,confidence:0.8}];
        b.energy_step=0.02;
        b.envelope=vec![0.3;9000];
        b.envelope[..392].fill(0.);
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(122.),Some(&b)),&config(),false).unwrap();
        assert_eq!(p.kind,Kind::BeatMatch);
        assert!((p.incoming_cue-7.72).abs()<0.001);
        assert!((p.outgoing_cue+(7.84-p.incoming_cue)/p.incoming_tempo-176.).abs()<0.001);
        assert!((0.94..=1.06).contains(&p.incoming_tempo));
        a.changes[0].confidence=0.49;let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(122.),Some(&b)),&config(),false).unwrap();assert_eq!(p.confidence,0.);
    }
    #[test]
    fn aligns_measured_beats_without_skipping_the_pickup_or_final_note() {
        let mut a = measured(0.5);
        let mut b = measured(0.5);
        a.phrases.retain(|p| p.seconds == 128.);
        a.beats.retain(|beat| *beat <= 179.5);
        b.phrases.clear();
        b.beats.iter_mut().for_each(|beat| *beat += 0.6);
        b.energy_step = 0.25;
        b.envelope = vec![0.5; 720];
        let p = plan(track(180., Some(120.), Some(&a)), track(180., Some(120.), Some(&b)), &config(), false).unwrap();
        assert_eq!(p.kind, Kind::BeatMatch);
        assert_eq!(p.incoming_cue, 0.);
        assert!(p.outgoing_cue >= 168.);
        assert_eq!(p.outgoing_cue + p.duration, 180.);
        let landing = p.outgoing_cue + 0.6 / p.incoming_tempo;
        assert!((landing * 2. - (landing * 2.).round()).abs() < 0.001);
    }

    #[test]
    fn can_mix_half_and_double_time_grids_at_their_shared_beats() {
        let a = measured(0.5);
        let mut b = measured(0.98);
        b.phrases.clear();
        let p = plan(track(180., Some(120.), Some(&a)), track(180., Some(61.), Some(&b)), &config(), false).unwrap();
        assert_eq!(p.kind, Kind::BeatMatch);
        assert!((p.incoming_tempo - 0.98).abs() < 0.0001);
        assert_eq!(p.incoming_cue, 0.);
    }

    #[test]
    fn weak_beats_without_structural_evidence_never_force_a_mix() {
        let mut a = measured(0.5);
        a.phrases.clear();
        a.beat_confidence = 0.55;
        let b = measured(0.5);
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false).unwrap();
        assert_eq!(p.kind, Kind::Crossfade);
    }
    #[test]
    fn measured_energy_supplies_an_unwarped_cue_without_a_beat_grid() {
        let mut outgoing = Structure { energy_step: 0.25, envelope: vec![0.7; 240], ..Default::default() };
        outgoing.envelope[216..].fill(0.1);
        let mut incoming = Structure { energy_step: 0.25, envelope: vec![0.35; 240], ..Default::default() };
        incoming.envelope[..8].fill(0.);
        let a = track(60., None, Some(&outgoing));
        let b = track(60., None, Some(&incoming));
        let measured = plan(a, b, &config(), false).unwrap();
        assert!((53.5..=54.5).contains(&measured.outgoing_cue));
        assert_eq!(measured.incoming_cue, 1.75);
        assert_eq!(measured.outgoing_cue + measured.duration, 60.);
        assert!(measured.duration > 5.);
        assert_eq!(measured.incoming_tempo, 1.);
        let passed = plan_after(a, b, &config(), false, 56.).unwrap();
        assert_eq!(passed.outgoing_cue, 56.5);
        assert_eq!(passed.duration, 3.5);
        let mut full = config();
        full.mode = Mode::FullSong;
        let full = plan(a, b, &full, false).unwrap();
        assert_eq!(full.outgoing_cue + full.duration, 60.);
        assert_eq!(full.incoming_cue, 0.);
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
    fn lineup_scores_each_candidate_once_per_position_and_keeps_ties_stable() {
        let mut evaluations = std::collections::HashSet::new();
        let order = order_upcoming(&[0, 1, 2, 3, 4], 1, &Default::default(), |from, to| {
            assert!(evaluations.insert((from, to)));
            0.
        });
        assert_eq!(order, [1, 2, 3, 4]);
        assert_eq!(evaluations.len(), 10);
    }

    #[test]
    fn manual_positions_are_pinned_without_moving_current_or_past() {
        let pins = [3].into_iter().collect();
        let order = order_upcoming(&[0, 1, 2, 3, 4, 5], 2, &pins, |_, i| -(i as f32));
        assert_eq!(order, vec![5, 3, 4, 2]);
        assert!(!order.contains(&0) && !order.contains(&1));
    }
}

#[cfg(test)] mod dispatch_tests {
 use super::*;
 #[test]fn smart_plans_wait_until_preparation_is_due(){
  let mut plan=Plan{kind:Kind::Crossfade,outgoing_cue:98.,incoming_cue:0.,duration:2.,incoming_tempo:1.,effect:Effect::CleanBlend,confidence:0.,reason:String::new()};
  assert!(!plan.ready_to_schedule(Mode::SmartTransition,0.,1.));
  assert!(!plan.ready_to_schedule(Mode::SmartTransition,85.,1.));
  assert!(plan.ready_to_schedule(Mode::SmartTransition,86.,1.));
  assert!(plan.ready_to_schedule(Mode::SmartTransition,74.,2.));
  assert!(plan.ready_to_schedule(Mode::FullSong,0.,1.));
  plan.confidence=0.7;assert!(!plan.ready_to_schedule(Mode::SmartTransition,0.,1.));
 }
}
