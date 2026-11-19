//! Deterministic fixtures → production analysis/planner/decoder/mixer, no audio device.
use anyhow::{ensure, Context, Result};
use musicos_desktop::local::{
    decoded_audio,
    dj_audio::{self, Command, Frame, Schedule, Tail, RATE},
};
use musicos_tauri::{
    analysis::{self, Features},
    dj_plan, structure,
};
#[path = "../src/transition_energy.rs"]
mod transition_energy;
use rodio::Source;
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use transition_energy::Envelope;

const LENGTH: f32 = 180.;
const HIT: f32 = LENGTH - 1.;

fn wav(path: &Path, frames: &[[f32; 2]]) -> Result<()> {
    let bytes = u32::try_from(frames.len() * 4)?;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    file.write_all(b"RIFF")?;
    file.write_all(&(bytes + 36).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&RATE.to_le_bytes())?;
    file.write_all(&(RATE * 4).to_le_bytes())?;
    file.write_all(&4u16.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&bytes.to_le_bytes())?;
    for frame in frames {
        for sample in frame {
            file.write_all(&((sample.clamp(-1., 1.) * 32767.) as i16).to_le_bytes())?;
        }
    }
    file.flush()?;
    Ok(())
}

fn fixture(incoming: bool) -> Vec<[f32; 2]> {
    let bpm = if incoming { 122. } else { 120. };
    let mut seed = 0x5eed_u32;
    (0..(LENGTH * RATE as f32) as usize)
        .map(|i| {
            let t = i as f64 / RATE as f64;
            let t = t as f32;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (seed as f64 / u32::MAX as f64 * 2. - 1.) as f32;
            let beat = (t * bpm / 60.).fract() * 60. / bpm;
            let phase = |hz: f32| (std::f32::consts::TAU * hz * t).sin();
            let kick = 0.28
                * (-beat * 32.).exp()
                * (std::f32::consts::TAU * (52. * beat + 2. * (1. - (-beat * 45.).exp()))).sin();
            let pad = 0.09 * (phase(220.) + phase(261.6256) + phase(329.6276));
            let note = [440., 523.251, 659.255, 587.33][((t * 2.) as usize / 2) % 4];
            let voice = 0.10 * (phase(note) + 0.32 * phase(note * 2.) + 0.15 * phase(note * 3.));
            let hat = noise * 0.035 * (-beat * 65.).exp();
            let mut sample = kick + pad + voice + hat;
            if incoming {
                sample *= if t < 1. { 0.08 } else { 1. };
            } else if t >= HIT {
                let age = t - HIT;
                let decay = if age < 0.7 {
                    (-age * 6.).exp() * (1. - age / 0.7)
                } else {
                    0.
                };
                sample = (0.20 * (phase(220.) + phase(261.6256) + phase(329.6276))
                    + 0.25 * phase(55.))
                    * decay;
            }
            [sample, sample * 0.97]
        })
        .collect()
}

fn analyze(path: &Path, id: i64) -> Result<Features> {
    let window = 18.;
    let mut excerpts = Vec::new();
    for start in [0., (LENGTH - window) * 0.5, (LENGTH - window) * 0.85] {
        let mut decoder = decoded_audio::open(path)?;
        if start > 0. {
            decoder
                .try_seek(Duration::from_secs_f32(start))
                .map_err(|e| anyhow::anyhow!("Excerpt seek: {e}"))?;
        }
        excerpts.push(
            rodio::source::UniformSourceIterator::<_, f32>::new(decoder, 1, 11025)
                .take((window * 11025.) as usize)
                .collect(),
        );
    }
    let mut features = analysis::extract(id, &excerpts);
    let scan = structure::scan(path, features.bpm, features.bpm_confidence, || true)?;
    if scan.beat_confidence >= 0.5 {
        features.bpm = scan.tempo;
        features.bpm_confidence = scan.beat_confidence;
    }
    features.structure = Some(scan);
    Ok(features)
}

fn decode(path: &Path, start: f32) -> Result<Vec<[f32; 2]>> {
    let decoder = decoded_audio::open(path)?;
    let mut pcm = Vec::new();
    for sample in rodio::source::UniformSourceIterator::<_, f32>::new(decoder, 2, RATE)
        .skip((start as f64 * RATE as f64).round() as usize * 2)
    {
        pcm.push(sample);
    }
    Ok(pcm.chunks_exact(2).map(|p| [p[0], p[1]]).collect())
}

fn rms(frames: &[[f32; 2]]) -> Option<f64> {
    (!frames.is_empty()).then(|| {
        (frames
            .iter()
            .flatten()
            .map(|v| (*v as f64).powi(2))
            .sum::<f64>()
            / (frames.len() * 2) as f64)
            .sqrt()
    })
}

fn track(f: &Features) -> dj_plan::Track<'_> {
    dj_plan::Track {
        key_index: f.key_index,
        duration: f.structure.as_ref().map_or(LENGTH, |s| s.duration),
        bpm: f.bpm,
        structure: f.structure.as_ref(),
    }
}

fn comparisons(features: &[Features; 2]) -> Result<serde_json::Value> {
    let config = dj_plan::Config {
        enabled: true,
        ..Default::default()
    };
    let mut plans = serde_json::Map::new();
    for name in [
        "gradual_fade",
        "sustained_drop",
        "abrupt_loud_cut",
        "break_then_hit",
        "long_trailing_silence",
    ] {
        let mut a = features[0].clone();
        let s = a.structure.as_mut().context("Missing structure")?;
        s.duration = LENGTH;
        s.energy_step = 0.025;
        s.envelope = (0..7200)
            .map(|i| {
                let t = i as f32 * 0.025;
                match name {
                    "gradual_fade" => 0.7 * ((LENGTH - t) / 8.).clamp(0., 1.),
                    "sustained_drop" if t >= 174. => 0.10,
                    "break_then_hit" if (170. ..174.).contains(&t) => 0.07,
                    "long_trailing_silence" if t >= 174. => 0.,
                    _ => 0.7,
                }
            })
            .collect();
        plans.insert(
            name.into(),
            serde_json::to_value(dj_plan::plan(
                track(&a),
                track(&features[1]),
                &config,
                false,
            ))?,
        );
    }
    let started = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(dj_plan::plan(
            std::hint::black_box(track(&features[0])),
            std::hint::black_box(track(&features[1])),
            &config,
            false,
        ));
    }
    Ok(json!({"plans":plans,"planner_10000_ms":started.elapsed().as_secs_f64()*1000.}))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut dir = PathBuf::from("out/qa/dj-cold-ending/");
    let mut plan_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--plan" => {
                plan_path = Some(PathBuf::from(
                    args.next().context("--plan needs a JSON path")?,
                ))
            }
            "--compare" => {
                let features: [Features; 2] = serde_json::from_slice(&std::fs::read(
                    args.next().context("--compare needs analysis.json")?,
                )?)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&comparisons(&features)?)?
                );
                return Ok(());
            }
            _ => {
                ensure!(!arg.starts_with('-'), "Unknown option: {arg}");
                dir = PathBuf::from(arg);
            }
        }
    }
    std::fs::create_dir_all(&dir)?;
    let outgoing = dir.join("outgoing.wav");
    let incoming = dir.join("incoming.wav");
    wav(&outgoing, &fixture(false))?;
    wav(&incoming, &fixture(true))?;
    let a = analyze(&outgoing, 1)?;
    let b = analyze(&incoming, 2)?;
    std::fs::write(dir.join("analysis.json"), serde_json::to_vec(&[&a, &b])?)?;
    let plan: dj_plan::Plan = if let Some(path) = plan_path {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        dj_plan::plan(
            track(&a),
            track(&b),
            &dj_plan::Config {
                enabled: true,
                ..Default::default()
            },
            false,
        )
        .context("No production plan")?
    };
    ensure!(
        plan.outgoing_cue.is_finite()
            && (HIT - 15. ..=LENGTH + 1.).contains(&plan.outgoing_cue)
            && plan.incoming_cue.is_finite()
            && (0. ..LENGTH - 30.).contains(&plan.incoming_cue)
            && plan.duration.is_finite()
            && (0. ..=16.).contains(&plan.duration)
            && plan.incoming_tempo.is_finite()
            && (0.94..=1.06).contains(&plan.incoming_tempo),
        "Invalid fixture transition plan"
    );
    std::fs::write(dir.join("plan.json"), serde_json::to_vec_pretty(&plan)?)?;
    let s = a.structure.as_ref().unwrap();
    let tail = transition_energy::tail(
        Envelope {
            start: 0.,
            step: s.energy_step,
            values: &s.envelope,
        },
        s.duration,
    );
    let mean = |lo: f32, hi: f32| {
        let v = &s.envelope[(lo / s.energy_step).round() as usize
            ..((hi / s.energy_step).round() as usize).min(s.envelope.len())];
        v.iter().sum::<f32>() / v.len() as f32
    };
    let start = (plan.outgoing_cue - 20.).max(0.);
    let sources = [
        decode(&outgoing, start)?,
        decode(&incoming, plan.incoming_cue)?,
    ];
    let mut incoming_pcm = sources[1].iter().flatten().copied().collect::<Vec<_>>();
    if (plan.incoming_tempo - 1.).abs() > 0.0001 {
        incoming_pcm = dj_audio::stretch_stereo(&incoming_pcm, plan.incoming_tempo)?;
    }
    let sources = [
        sources[0].clone(),
        incoming_pcm.chunks_exact(2).map(|p| [p[0], p[1]]).collect(),
    ];
    let (mut control, mut mixer) = dj_audio::create(4096);
    let clock = control.clock.clone();
    for (deck, seconds) in [start, plan.incoming_cue].into_iter().enumerate() {
        control
            .commands
            .push(Command::Reset {
                deck,
                generation: 1,
                source_frame: seconds as f64 * RATE as f64,
            })
            .unwrap();
    }
    let tail_effect = match plan.effect {
        dj_plan::Effect::EchoTail => Tail::Echo,
        dj_plan::Effect::ReverbTail => Tail::Reverb,
        dj_plan::Effect::EchoBounce => Tail::Bounce,
        _ => Tail::Clean,
    };
    let schedule = Schedule {
        cancel_revision: clock.cancel_revision.load(Ordering::Acquire),
        id: 1,
        cue: plan.outgoing_cue as f64 * RATE as f64,
        frames: (plan.duration.min(16.) * RATE as f32) as u64,
        incoming: 1,
        tail: tail_effect,
    };
    control.commands.push(Command::Plan(schedule)).unwrap();
    control.commands.push(Command::Pause(false)).unwrap();
    let count = ((plan.outgoing_cue + plan.duration + 15. - start) * RATE as f32).ceil() as usize;
    let mut positions = [0; 2];
    let mut output = Vec::with_capacity(count);
    let (mut activation, mut promotion, mut ready_at_cue) = (None, None, None);
    for i in 0..count {
        for deck in 0..2 {
            while control.decks[deck].slots() > 0 && positions[deck] < sources[deck].len() {
                let n = positions[deck];
                let source_frame = if deck == 0 {
                    start as f64 * RATE as f64 + n as f64
                } else {
                    plan.incoming_cue as f64 * RATE as f64 + n as f64 * plan.incoming_tempo as f64
                };
                control.decks[deck]
                    .push(Frame {
                        samples: sources[deck][n],
                        source_frame,
                        generation: 1,
                        listening_revision: 0,
                    })
                    .unwrap();
                positions[deck] += 1;
            }
            if positions[deck] == sources[deck].len() {
                clock.finished[deck].store(1, Ordering::Release);
            }
        }
        output.push([mixer.next().unwrap(), mixer.next().unwrap()]);
        if ready_at_cue.is_none() && start as f64 * RATE as f64 + i as f64 >= schedule.cue {
            ready_at_cue = Some(clock.ready[1].load(Ordering::Acquire));
        }
        if activation.is_none() && clock.mixing.load(Ordering::Acquire) {
            activation = Some(clock.rendered_frames.load(Ordering::Acquire) - 1);
        }
        if promotion.is_none() && clock.active.load(Ordering::Acquire) == 1 {
            promotion = Some(clock.rendered_frames.load(Ordering::Acquire) - 1);
        }
    }
    wav(&dir.join("transition.wav"), &output)?;
    let hit_frame = ((HIT - start) as f64 * RATE as f64).round() as u64;
    let gain = if schedule.frames == 0 {
        Some(if promotion.is_some_and(|at| at <= hit_frame) {
            0.
        } else {
            1.
        })
    } else {
        activation.map(|at| {
            if hit_frame < at {
                1.
            } else {
                (((hit_frame - at) as f32 / schedule.frames as f32).clamp(0., 1.)
                    * std::f32::consts::FRAC_PI_2)
                    .cos()
            }
        })
    };
    let window_rms = |lo: f32, hi: f32| {
        rms(
            &output[((lo - start).max(0.) * RATE as f32).round() as usize
                ..(((hi - start).max(0.) * RATE as f32).round() as usize).min(output.len())],
        )
    };
    let mut silence = 0usize;
    let mut longest = 0usize;
    for frame in &output {
        if frame.iter().all(|v| v.abs() < 0.0001) {
            silence += 1;
            longest = longest.max(silence);
        } else {
            silence = 0;
        }
    }
    let audible = |f: &[f32; 2]| f.iter().any(|v| v.abs() >= 0.0001);
    let last_outgoing = sources[0]
        .iter()
        .rposition(audible)
        .context("Silent outgoing")?;
    let first_incoming = sources[1]
        .iter()
        .position(audible)
        .context("Silent incoming")?;
    let natural_gap = (sources[0].len() - last_outgoing - 1 + first_incoming) as f64 / RATE as f64;
    let mut ring_gap = 0usize;
    let mut longest_ring_gap = 0usize;
    let gap_end = promotion
        .unwrap_or(count as u64)
        .saturating_add(first_incoming as u64 + RATE as u64 / 10)
        .min(count as u64) as usize;
    for frame in &output[(hit_frame as usize).min(gap_end)..gap_end] {
        if audible(frame) {
            ring_gap = 0;
        } else {
            ring_gap += 1;
            longest_ring_gap = longest_ring_gap.max(ring_gap);
        }
    }
    let measured_ring_gap = longest_ring_gap as f64 / RATE as f64;
    let evidence = json!({"seed":0x5eed_u32,"duration":s.duration,"final_hit_seconds":HIT,"hit_decay_seconds":0.7,"envelope_mean_L_minus_8_to_L_minus_2":mean(LENGTH-8.,LENGTH-2.),"envelope_mean_L_minus_2_to_L":mean(LENGTH-2.,LENGTH),"tail":tail.map(|t|json!({"drop":t.drop,"cue":t.cue,"overlap":t.overlap}))});
    let result = json!({
        "plan":plan,"energy_evidence":evidence,
        "analysis":{"outgoing_bpm":a.bpm,"incoming_bpm":b.bpm,"outgoing_key":a.key,"incoming_key":b.key,"outgoing_beat_confidence":a.bpm_confidence,"incoming_beat_confidence":b.bpm_confidence},
        "render":{"file":dir.join("transition.wav"),"source_start_seconds":start,"seconds":output.len() as f64/RATE as f64,"activation_clock_frame":activation,"incoming_active_clock_frame":promotion,"incoming_ready_at_cue":ready_at_cue,"underruns":clock.underruns.load(Ordering::Acquire),"failed_schedule":clock.failed_schedule.load(Ordering::Acquire),"scheduled_overlap_seconds":schedule.frames as f64/RATE as f64,"observed_overlap_seconds":if schedule.frames==0 {Some(0.)} else {activation.zip(promotion).map(|(a,b)|(b-a) as f64/RATE as f64)}},
        "measurements":{"outgoing_gain_at_final_hit_from_observed_activation":gain,"final_hit_render_frame":hit_frame,"rms_2s_before_hit":window_rms(HIT-2.,HIT),"rms_100ms_windows":(-5..=9).map(|n|{let t=HIT+n as f32*0.1;json!({"source_start_seconds":t,"rms":window_rms(t,t+0.1)})}).collect::<Vec<_>>(),"peak":output.iter().flatten().map(|v|v.abs()).fold(0_f32,f32::max),"clipped_samples":output.iter().flatten().filter(|v|v.abs()>=1.).count(),"longest_silent_gap_seconds":longest as f64/RATE as f64,"silent_gap_over_50ms":longest>RATE as usize/20,"ring_to_incoming_silent_gap_seconds":measured_ring_gap,"source_ring_silence_and_pickup_seconds":natural_gap,"extra_gap_seconds":(measured_ring_gap-natural_gap).max(0.)}
    });
    std::fs::write(dir.join("result.json"), serde_json::to_vec_pretty(&result)?)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    ensure!(
        output.iter().flatten().all(|v| v.is_finite()),
        "Nonfinite mixer output"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rms_handles_signal_and_empty() {
        assert_eq!(rms(&[]), None);
        assert_eq!(rms(&[[0.; 2]]), Some(0.));
        assert_eq!(rms(&[[0.5; 2]; 10]), Some(0.5));
    }
    #[test]
    fn wav_reports_invalid_destination() {
        let temp = tempfile::tempdir().unwrap();
        assert!(wav(&temp.path().join("missing/file.wav"), &[]).is_err());
    }
    #[test]
    fn wav_round_trips_empty_and_signal() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("fixture.wav");
        wav(&path, &[])?;
        assert_eq!(std::fs::metadata(&path)?.len(), 44);
        wav(&path, &[[0.25, -0.25]; 4800])?;
        let frames = decode(&path, 0.)?;
        assert_eq!(frames.len(), 4800);
        assert!((rms(&frames).unwrap() - 0.25).abs() < 0.0001);
        Ok(())
    }
}
