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
impl Plan {
    /// A weak ending blend is provisional until preparation is actually due.
    /// Otherwise analysis completing during the song can never improve it:
    /// accepted mixer schedules correctly remain protected from replacement.
    pub fn ready_to_schedule(&self,mode:Mode,position:f32,speed:f32)->bool {
        mode==Mode::FullSong || self.confidence>=0.5 || self.duration==0.
            || self.outgoing_cue-position<=12.*speed
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
                outgoing_cue: tail.cue,
                incoming_cue,
                duration: tail.overlap.min(incoming_length / 4.),
                incoming_tempo: 1.,
                effect: Effect::CleanBlend,
                confidence: 0.58,
                reason: "Sustained drop in outro energy".into(),
            };
        }
        Plan { incoming_cue, ..fallback(reason) }
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
    // A nearby beat can refine the entrance if the skipped audio was silent.
    // A confident phrase later in the song is still the wrong entrance.
    let latest_entrance = if incoming_cue > 0. {
        transition_energy::first_audible(envelope(sb)).unwrap_or(incoming_cue) + 0.03
    } else {
        0.03
    };
    let allowed = |cue: f32| {
        cue >= (length - 12.).max(earliest) && cue < length
            && preserves_section(sa,cue)
            && outgoing_tail.is_none_or(|tail| cue >= tail.drop - 2. && cue <= tail.drop + 3.)
    };
    let earliest_end = outgoing_tail.map_or(length - 1.5, |tail| tail.drop);
    let max_overlap = if config.character == Character::Subtle { 8. } else { 12. };
    let mut best: Option<(f32, Plan)> = None;
    let (mut tempo_rejected,mut unstable_rejected,mut drift_rejected)=(0,0,0);
    let exits=transition_boundaries(sa);let entries=transition_boundaries(sb);
    for exit in exits.iter().filter(|p|p.confidence>=0.5&&allowed(p.seconds)) {
        let Some(ai)=boundary_beat(&sa.beats,exit.seconds).filter(|&i| allowed(sa.beats[i])) else {continue};
        for count in [4_usize,8,16,32] {
            let Some(&end)=sa.beats.get(ai+count) else {continue};
            let duration=end-sa.beats[ai];
            if !(2. ..=max_overlap).contains(&duration) || end<earliest_end || end>length || !preserves_section(sa,end) {continue;}
            // The overlap starts on measured structural evidence. Its end is
            // an actual beat of the same stable grid, not a second independently
            // detected section change that needlessly rejects usable blends.
            for entry in entries.iter().filter(|p|p.confidence>=0.5) {
                let Some(bi)=boundary_beat(&sb.beats,entry.seconds) else {continue};
                let entrance = sb.beats[bi];
                if entrance < incoming_cue || entrance > latest_entrance || !preserves_section(sb,entrance) {continue;}
                let Some(&incoming_end)=sb.beats.get(bi+count) else {continue};
                if incoming_end>=incoming_length {continue;}
                let incoming_duration=incoming_end-sb.beats[bi];
                let tempo=incoming_duration/duration;
                if !(0.94..=1.06).contains(&tempo) {tempo_rejected+=1;continue;}
                // Reject unstable or wrongly phased local grids, even if global BPM matches.
                if !stable_window(&sa.beats[ai..=ai+count],ba)
                    ||!stable_window(&sb.beats[bi..=bi+count],bb) {unstable_rejected+=1;continue;}
                let drift=(0..=count).map(|j|((sa.beats[ai+j]-sa.beats[ai])
                    -(sb.beats[bi+j]-sb.beats[bi])/tempo).abs()).fold(0_f32,f32::max);
                if drift>0.05 {drift_rejected+=1;continue;}
                let outgoing_density=density(sa,sa.beats[ai],duration);
                let incoming_density=density(sb,sb.beats[bi],incoming_duration);
                let dense_overlap=outgoing_density*incoming_density;
                let harmonic=harmonic_distance(a.key_index,b.key_index);
                let confidence=sa.beat_confidence.min(sb.beat_confidence)
                    .min(exit.confidence).min(entry.confidence);
                let energy=(outgoing_density-incoming_density).abs();
                let skipped=(length-end)/length+sb.beats[bi]/incoming_length;
                let transition_distance = outgoing_tail.map_or(0., |tail| (sa.beats[ai] - tail.drop).abs() * 0.08);
                let score=transition_distance+dense_overlap*0.35+harmonic*0.15+energy*0.2
                    +(1.-confidence)*0.2+drift/0.05*0.2+skipped*0.25+(duration-8.).abs()/16.*0.08;
                let effect=if config.character==Character::Subtle || dense_overlap<0.2 {Effect::CleanBlend}
                    else if outgoing_density>incoming_density*1.5 {Effect::EchoBounce}
                    else if outgoing_density>incoming_density {Effect::EchoTail}else{Effect::ReverbTail};
                let p=Plan{outgoing_cue:sa.beats[ai],incoming_cue:sb.beats[bi],duration,
                    incoming_tempo:tempo,effect,confidence,
                    reason:format!("{count}-beat measured boundary blend; drift {:.1}ms, confidence {confidence:.2}, density {dense_overlap:.3}, energy {energy:.3}, harmony {harmonic:.2}, skipped {skipped:.3}, score {score:.3}",drift*1000.)};
                if best.as_ref().is_none_or(|(old,_)|score<*old){best=Some((score,p));}
            }
        }
    }
    Some(best.map(|(_, p)| p).unwrap_or_else(||ending(&format!("No compatible measured section boundaries (tempo {tempo_rejected}, unstable {unstable_rejected}, drift {drift_rejected})"))))
}
/// A recurring phrase is useful evidence, but not the only evidence: a strong
/// measured timbre/energy change near an observed beat also supports a cue.
/// Do not infer a bar phase from BPM or manufacture an evenly spaced beat grid.
fn transition_boundaries(s:&Structure)->Vec<crate::structure::Boundary>{
    let mut cues=s.phrases.iter().filter(|p|p.confidence>=0.5).cloned().collect::<Vec<_>>();
    for change in s.changes.iter().filter(|p|p.confidence>=0.5){
        let i=s.beats.partition_point(|b|*b<change.seconds);
        let beat=[i.checked_sub(1),Some(i)].into_iter().flatten().filter_map(|n|s.beats.get(n))
            .filter(|b|(**b-change.seconds).abs()<=0.15)
            .min_by(|a,b|(*a-change.seconds).abs().total_cmp(&(*b-change.seconds).abs()));
        if let Some(&seconds)=beat{if let Some(old)=cues.iter_mut().find(|p|(p.seconds-seconds).abs()<0.04){old.confidence=old.confidence.max(change.confidence);}else{cues.push(crate::structure::Boundary{seconds,confidence:change.confidence});}}
    }
    cues.sort_by(|a,b|a.seconds.total_cmp(&b.seconds));cues
}
fn preserves_section(s:&Structure,time:f32)->bool{s.regions.iter().all(|r|!r.repeated||r.confidence<0.5||time<=r.start+0.05||time>=r.end-0.05)}
fn valid_beats(beats:&[f32])->bool {beats.len()>=5&&beats.iter().all(|t|t.is_finite()&&*t>=0.)&&beats.windows(2).all(|p|p[1]>p[0])}
fn boundary_beat(beats:&[f32],time:f32)->Option<usize>{
    let i=beats.partition_point(|b|*b<time);[i.checked_sub(1),Some(i)].into_iter().flatten()
        .filter(|i|beats.get(*i).is_some_and(|b|(*b-time).abs()<=0.05))
        .min_by(|i,j|(beats[*i]-time).abs().total_cmp(&(beats[*j]-time).abs()))
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
        let p=plan_after(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false,177.).unwrap();
        assert_eq!(p.confidence,0.);assert_eq!(p.outgoing_cue,178.);
    }
    #[test]fn overlap_must_not_finish_inside_a_protected_section(){
        let mut a=measured(0.5);let b=measured(0.5);
        a.phrases.retain(|p|p.seconds==176.);
        a.regions=vec![Region{start:176.,end:181.,repeated:true,confidence:0.9}];
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false).unwrap();assert_eq!(p.confidence,0.);
    }
    #[test]fn measured_changes_support_a_blend_without_a_second_section_at_its_end(){
        let mut a=measured(0.5);let mut b=measured(0.49);a.phrases.clear();b.phrases.clear();
        a.changes=vec![Boundary{seconds:176.08,confidence:0.8}];b.changes=vec![Boundary{seconds:7.9,confidence:0.8}];
        b.energy_step=0.02;
        b.envelope=vec![0.3;9000];
        b.envelope[..392].fill(0.);
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(122.),Some(&b)),&config(),false).unwrap();
        assert!(p.confidence>=0.5);assert_eq!(p.outgoing_cue,176.);assert!((p.incoming_cue-7.84).abs()<0.001);assert!((0.94..=1.06).contains(&p.incoming_tempo));
        a.changes[0].confidence=0.49;let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(122.),Some(&b)),&config(),false).unwrap();assert_eq!(p.confidence,0.);
    }
    #[test]
    fn measured_phrases_do_not_skip_an_audible_intro_or_outro() {
        let mut a = measured(0.5);
        let mut b = measured(0.5);
        a.energy_step = 0.25;
        a.envelope = vec![0.7; 720];
        b.energy_step = 0.25;
        b.envelope = vec![0.5; 720];
        let plan_for = |a: &Structure, b: &Structure| {
            plan(track(180., Some(120.), Some(a)), track(180., Some(120.), Some(b)), &config(), false).unwrap()
        };

        a.phrases.retain(|p| p.seconds == 128.);
        assert_eq!(plan_for(&a, &b).outgoing_cue, 178.);
        a.phrases[0].seconds = 176.;
        b.phrases.retain(|p| p.seconds > 0.);
        let plan = plan_for(&a, &b);
        assert_eq!(plan.incoming_cue, 0.);
        assert_eq!(plan.confidence, 0.);
        b.phrases.insert(0, Boundary { seconds: 0., confidence: 0.9 });
        let plan = plan_for(&a, &b);
        assert!(plan.confidence >= 0.5);
        assert!(plan.outgoing_cue + plan.duration >= 178.5);
        assert_eq!(plan.incoming_cue, 0.);
    }

    #[test]fn beats_without_structural_evidence_never_force_a_mix(){
        let mut a=measured(0.5);a.phrases.clear();let b=measured(0.5);
        let p=plan(track(180.,Some(120.),Some(&a)),track(180.,Some(120.),Some(&b)),&config(),false).unwrap();assert_eq!(p.confidence,0.);
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
        assert!(measured.outgoing_cue + measured.duration < 60.);
        assert_eq!(measured.incoming_tempo, 1.);
        let passed = plan_after(a, b, &config(), false, 56.).unwrap();
        assert_eq!(passed.outgoing_cue, 58.);
        assert_eq!(passed.duration, 2.);
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
 #[test]fn weak_smart_blends_wait_but_confident_and_explicit_full_song_plans_do_not(){
  let mut plan=Plan{outgoing_cue:98.,incoming_cue:0.,duration:2.,incoming_tempo:1.,effect:Effect::CleanBlend,confidence:0.,reason:String::new()};
  assert!(!plan.ready_to_schedule(Mode::SmartTransition,0.,1.));
  assert!(!plan.ready_to_schedule(Mode::SmartTransition,85.,1.));
  assert!(plan.ready_to_schedule(Mode::SmartTransition,86.,1.));
  assert!(plan.ready_to_schedule(Mode::SmartTransition,74.,2.));
  assert!(plan.ready_to_schedule(Mode::FullSong,0.,1.));
  plan.confidence=0.7;assert!(plan.ready_to_schedule(Mode::SmartTransition,0.,1.));
 }
}
