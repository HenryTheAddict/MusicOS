//! Desktop-only sample-clocked mixing primitives. Decoders and tempo processing
//! produce frames off-thread; the output source performs bounded, allocation-free
//! ring reads and gain/effect arithmetic only.
use rodio::Source;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc,
};
pub const RATE: u32 = 48000;
#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    pub samples: [f32; 2],
    pub source_frame: f64,
    pub generation: u64,
    pub listening_revision:u64,
}
#[derive(Clone, Copy, Debug)]
pub enum Tail {
    Clean,
    Echo,
    Reverb,
    Bounce,
}
#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    pub cancel_revision:u64,
    pub id:u64,
    pub cue: f64,
    pub frames: u64,
    pub incoming: usize,
    pub tail: Tail,
}
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Reset {
        deck: usize,
        generation: u64,
        source_frame: f64,
    },
    Activate(usize),
    Plan(Schedule),
    Cancel,
    Volume(f32),
    Equalizer(super::listening::Equalizer,u64),
    Pause(bool),
}
#[derive(Default)]
pub struct Clock {
    pub listening_revision: AtomicU64,
    pub applied_reverb:AtomicU32,
    pub applied_reverb_depth:AtomicU32,
    pub room_active:AtomicBool,
    pub reverb_processed_frames:AtomicU64,
    pub failed:[AtomicU64;2],
    pub parameter_revision:[AtomicU64;2],
    pub levels: [AtomicU32; 3],
    pub audible_frames: [AtomicU64; 2],
    pub finished: [AtomicU64; 2],
    pub drained: [AtomicBool; 2],
    pub plan_failed: AtomicBool,
    pub schedule_id:AtomicU64,
    pub cancel_revision:AtomicU64,
    pub source_frames: [AtomicU64; 2],
    pub rendered_frames: AtomicU64,
    pub active: AtomicU64,
    pub mixing: AtomicBool,
    pub ready: [AtomicBool; 2],
    pub underruns: AtomicU64,
}
struct Effects {
    delay: Vec<[f32; 2]>,
    reverb: Vec<[f32; 2]>,
    d: usize,
    r: usize,
}
impl Effects {
    fn new() -> Self {
        Self {
            delay: vec![[0.; 2]; (RATE / 3) as usize],
            reverb: vec![[0.; 2]; (RATE / 19) as usize],
            d: 0,
            r: 0,
        }
    }
    fn clear(&mut self) {
        self.delay.fill([0.; 2]);
        self.reverb.fill([0.; 2]);
        self.d = 0;
        self.r = 0;
    }
    fn process(&mut self, input: [f32; 2], tail: Tail, amount: f32) -> [f32; 2] {
        let delay = self.delay[self.d];
        let room = self.reverb[self.r];
        let mut output = input;
        for c in 0..2 {
            let feedback = match tail {
                Tail::Bounce => 0.55,
                Tail::Echo => 0.3,
                _ => 0.,
            };
            self.delay[self.d][c] = input[c] * amount + delay[1 - c] * feedback;
            self.reverb[self.r][c] = input[c] * amount + room[1 - c] * 0.65;
            output[c] += match tail {
                Tail::Clean => 0.,
                Tail::Echo => delay[c] * 0.15,
                Tail::Bounce => delay[c] * 0.25,
                Tail::Reverb => room[c] * 0.18,
            };
        }
        self.d = (self.d + 1) % self.delay.len();
        self.r = (self.r + 1) % self.reverb.len();
        output
    }
}
pub struct Mixer {
    pending: [Option<Frame>; 2],
    decks: [Consumer<Frame>; 2],
    commands: Consumer<Command>,
    generations: [u64; 2],
    positions: [f64; 2],
    active: usize,
    plan: Option<Schedule>,
    elapsed: Option<u64>,
    effects: Effects,
    paused: bool,
    volume: f32,
    right: Option<f32>,
    history:[[f32;2];960],
    history_at:usize,
    fade_at:usize,
    fade_left:usize,
    clock: Arc<Clock>,
    eq: super::listening::EqProcessor,
    room:super::listening::Reverb,
    meter: Producer<[f32; 2]>,
}
pub struct Control {
    pub decks: [Producer<Frame>; 2],
    pub commands: Producer<Command>,
    pub clock: Arc<Clock>,
}
fn display_level(previous:f32,rms:f32,band:usize)->f32 {
 let target=if rms<0.0001 {0.} else {(rms*[2.2,3.2,5.0][band]).sqrt().clamp(0.,1.)};
 let rate=if target>previous {0.7} else {0.25};let next=previous+(target-previous)*rate;
 if target==0. && next<0.01 {0.} else {next}
}
pub fn create(buffer_frames: usize) -> (Control, Mixer) {
    let (a, ar) = RingBuffer::new(buffer_frames.max(1));
    let (b, br) = RingBuffer::new(buffer_frames.max(1));
    let (commands, cr) = RingBuffer::new(64);
    let clock = Arc::new(Clock::default());
    let (meter, mut samples) = RingBuffer::<[f32; 2]>::new(RATE as usize / 5);
    let meter_clock = Arc::downgrade(&clock);
    // Spectral/envelope work runs here, never in the output callback. The
    // bounded ring drops telemetry on overload; audio never waits for the meter.
    std::thread::spawn(move || {
        let mut low = 0_f32; let mut high_cut = 0_f32; let mut smoothed=[0_f32;3];
        let low_alpha = 1. - (-2. * std::f32::consts::PI * 250. / RATE as f32).exp();
        let high_alpha = 1. - (-2. * std::f32::consts::PI * 2500. / RATE as f32).exp();
        loop {
            let mut energy = [0_f32; 3]; let mut count = 0_u32;
            while let Ok(frame) = samples.pop() {
                let x = (frame[0] + frame[1]) * 0.5;
                low += low_alpha * (x - low); high_cut += high_alpha * (x - high_cut);
                for (sum, band) in energy.iter_mut().zip([low, high_cut - low, x - high_cut]) { *sum += band * band; }
                count += 1;
            }
            if let Some(clock) = meter_clock.upgrade() {
                for (i, (target, sum)) in clock.levels.iter().zip(energy).enumerate() {
                    let rms=if count==0 {0.} else {(sum/count as f32).sqrt()};
                    smoothed[i]=display_level(smoothed[i],rms,i);
                    target.store(smoothed[i].to_bits(), Ordering::Relaxed);
                }
            } else { break; }
            if samples.is_abandoned() { break; }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
    (
        Control {
            decks: [a, b],
            commands,
            clock: clock.clone(),
        },
        Mixer {
            pending: [None, None],
            decks: [ar, br],
            commands: cr,
            generations: [0; 2],
            positions: [0.; 2],
            active: 0,
            plan: None,
            elapsed: None,
            effects: Effects::new(),
            paused: true,
            volume: 1.,
            right: None,
            history:[[0.;2];960],history_at:0,fade_at:0,fade_left:0,
            clock,
            meter,
            eq: Default::default(),
            room:Default::default(),
        },
    )
}
impl Mixer {
    fn cancel(&mut self) {
        self.plan = None;
        self.clock.schedule_id.store(0,Ordering::Release);
        self.elapsed = None;
        self.effects.clear();
        self.clock.mixing.store(false, Ordering::Relaxed);
    }
    fn commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            match command {
                Command::Reset {
                    deck,
                    generation,
                    source_frame,
                } => {
                    if deck < 2 {
                        if deck==self.active {self.fade_at=self.history_at;self.fade_left=960;}
                        self.pending[deck] = None;
                        if deck==self.active {self.room.clear();}
                        self.generations[deck] = generation;
                        self.clock.drained[deck].store(false, Ordering::Relaxed);
                        self.clock.ready[deck].store(false, Ordering::Release);
                        self.positions[deck] = source_frame.max(0.);
                        self.clock.source_frames[deck]
                            .store(self.positions[deck] as u64, Ordering::Relaxed);
                        self.cancel();
                    }
                }
                Command::Activate(deck) => {
                    if deck < 2 {
                        self.active = deck;
                        self.clock.active.store(deck as u64, Ordering::Relaxed);
                        self.cancel();
                    }
                }
                Command::Plan(plan) => {
                    self.cancel();
                    if plan.incoming < 2
                        && plan.incoming != self.active
                        && plan.frames <= RATE as u64 * 16
                        && plan.cue.is_finite()
                    {
                        self.plan = Some(plan);
                        self.clock.schedule_id.store(plan.id,Ordering::Release);
                    }
                }
                Command::Cancel => self.cancel(),
                Command::Pause(paused) => self.paused = paused,
                Command::Equalizer(coeff,revision)=>{self.eq.set(coeff);self.room.set(coeff.reverb);self.room.set_depth(coeff.reverb_depth);self.clock.applied_reverb_depth.store(coeff.reverb_depth.to_bits(),Ordering::Release);self.clock.applied_reverb.store(coeff.reverb.to_bits(),Ordering::Release);self.clock.listening_revision.store(revision,Ordering::Release);}
                Command::Volume(volume) => {
                    if volume.is_finite() {
                        self.volume = volume.clamp(0., 1.);
                    }
                }
            }
        }
        // Cancellation cannot be dropped by a saturated command ring. Old queued
        // plans are invalidated too, while a new plan in the current epoch survives.
        if self.plan.is_some_and(|p|p.cancel_revision!=self.clock.cancel_revision.load(Ordering::Acquire)){self.cancel();}
    }
    fn read(&mut self, deck: usize) -> Option<Frame> {
        if let Some(frame) = self.pending[deck].take() {
            return Some(frame);
        }
        // At most sixteen stale frames per output frame after a decoder generation
        // change, bounding callback work while the producer abandons the old stream.
        for _ in 0..16 {
            match self.decks[deck].pop() {
                Ok(frame) if frame.generation == self.generations[deck] => return Some(frame),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        None
    }
    fn frame(&mut self) -> [f32; 2] {
        self.commands();
        if self.paused {
            return [0.; 2];
        }
        if let Some(plan) = self.plan {
            if self.pending[plan.incoming].is_none() {
                self.pending[plan.incoming] = self.read(plan.incoming);
                self.clock.ready[plan.incoming].store(self.pending[plan.incoming].is_some(), Ordering::Release);
            }
        }
        let outgoing = match self.read(self.active) {
            Some(frame) => frame,
            None => {
                if self.clock.finished[self.active].load(Ordering::Acquire)
                    == self.generations[self.active]
                    && self.generations[self.active] > 0
                {
                    self.clock.drained[self.active].store(true, Ordering::Release);
                    if let Some(plan) = self.plan {
                        if self.pending[plan.incoming].is_some() {
                            self.active = plan.incoming;
                            self.clock
                                .active
                                .store(self.active as u64, Ordering::Release);
                            self.cancel();
                            return self.frame();
                        }
                    }
                }
                self.clock.underruns.fetch_add(1, Ordering::Relaxed);
                return self.process_output([0.;2]);
            }
        };
        self.clock.parameter_revision[self.active].store(outgoing.listening_revision,Ordering::Release);
        self.positions[self.active] = outgoing.source_frame;
        self.clock.source_frames[self.active]
            .store(outgoing.source_frame.max(0.) as u64, Ordering::Relaxed);
        if self.elapsed.is_none() {
            if let Some(plan) = self.plan {
                if plan.frames > 0 && outgoing.source_frame >= plan.cue {
                    // Missed preparation deadlines never trigger a late phrase cut.
                    if outgoing.source_frame - plan.cue <= RATE as f64 * 0.03
                        && self.pending[plan.incoming].is_some()
                    {
                        self.elapsed = Some(0);
                        self.clock.mixing.store(true, Ordering::Relaxed);
                    } else {
                        self.cancel();
                        self.clock.plan_failed.store(true, Ordering::Release);
                    }
                }
            }
        }
        self.clock.audible_frames[self.active].fetch_add(1, Ordering::Relaxed);
        let mut output = outgoing.samples;
        if let (Some(plan), Some(elapsed)) = (self.plan, self.elapsed) {
            if let Some(incoming) = self.read(plan.incoming) {
                self.positions[plan.incoming] = incoming.source_frame;
                self.clock.source_frames[plan.incoming]
                    .store(incoming.source_frame.max(0.) as u64, Ordering::Relaxed);
                self.clock.audible_frames[plan.incoming].fetch_add(1, Ordering::Relaxed);
                let progress = (elapsed as f32 / plan.frames as f32).clamp(0., 1.);
                let angle = progress * std::f32::consts::FRAC_PI_2;
                let incoming_gain = angle.sin();
                let outgoing_gain = angle.cos();
                let tail = self
                    .effects
                    .process(outgoing.samples, plan.tail, 1. - progress);
                for c in 0..2 {
                    output[c] = (tail[c] * outgoing_gain + incoming.samples[c] * incoming_gain)
                        / (incoming_gain + outgoing_gain).max(1.);
                }
                if elapsed >= plan.frames {
                    self.active = plan.incoming;
                    self.clock
                        .active
                        .store(self.active as u64, Ordering::Relaxed);
                    self.cancel();
                } else {
                    self.elapsed = Some(elapsed + 1);
                }
            } else {
                // A decoder starvation cancels the blend rather than moving source time on
                // invented samples. Controller can recover at the natural ending.
                self.clock.underruns.fetch_add(1, Ordering::Relaxed);
                self.cancel();
                self.clock.plan_failed.store(true, Ordering::Release);
            }
        }
        self.clock.rendered_frames.fetch_add(1, Ordering::Relaxed);
        self.process_output(output)
    }
    fn process_output(&mut self,output:[f32;2])->[f32;2]{
        let equalized=self.eq.process(output);
        let processed=self.room.process(equalized);
        if processed.iter().zip(equalized).any(|(a,b)|(a-b).abs()>0.000001){self.clock.reverb_processed_frames.fetch_add(1,Ordering::Relaxed);}
        let output = processed.map(|v| limit(v * self.volume));
        self.clock.room_active.store(self.room.active(),Ordering::Relaxed);
        output
    }
}
fn limit(sample: f32) -> f32 {
    if !sample.is_finite() {
        return 0.;
    }
    let magnitude = sample.abs();
    if magnitude <= 0.9 {
        sample
    } else {
        sample.signum() * (0.9 + 0.1 * ((magnitude - 0.9) * 10.).tanh())
    }
}
impl Iterator for Mixer {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        let mut frame = self.frame();
        if self.paused {self.fade_left=0;}else if self.fade_left>0 {
            let old=self.history[self.fade_at];self.fade_at=(self.fade_at+1)%960;
            let progress=1.-self.fade_left as f32/960.;self.fade_left-=1;
            for c in 0..2{frame[c]=old[c]*(1.-progress)+frame[c]*progress;}
        }
        let _=self.meter.push(frame);
        self.history[self.history_at]=frame;self.history_at=(self.history_at+1)%960;
        self.right = Some(frame[1]);
        Some(frame[0])
    }
}
impl Source for Mixer {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}
/// Worker-side processing. The ratio changes duration while leaving transpose at
/// unity. Creative pitch-changing effects require a separate explicit operation.
pub fn stretch_stereo(input: &[f32], tempo: f32) -> anyhow::Result<Vec<f32>> {
    anyhow::ensure!(
        input.len() % 2 == 0 && !input.is_empty(),
        "Expected nonempty interleaved stereo audio"
    );
    anyhow::ensure!(
        tempo.is_finite() && (0.94..=1.06).contains(&tempo),
        "Normal tempo matching is limited to ±6%"
    );
    let frames = (input.len() / 2) as f64;
    let output_frames = (frames / tempo as f64).round() as usize;
    let mut output = vec![0.; output_frames * 2];
    let mut stretch = signalsmith_stretch::Stretch::preset_default(2, RATE);
    stretch.set_transpose_factor(1., None);
    anyhow::ensure!(stretch.exact(input, &mut output), "Time stretch failed");
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn push(control: &mut Control, deck: usize, count: usize, start: usize, value: f32) {
        for i in 0..count {
            control.decks[deck]
                .push(Frame {
                    samples: [value; 2],
                    source_frame: (start + i) as f64,
                    generation: 0,
                    listening_revision:0,
                })
                .unwrap();
        }
    }
    #[test] fn visual_bands_are_calibrated_bounded_and_decay_to_silence(){
        assert!(display_level(0.,0.04,2)>display_level(0.,0.04,0));
        for band in 0..3 {assert!(display_level(0.,100.,band)<=1.);let mut value=display_level(0.,0.1,band);for _ in 0..40 {value=display_level(value,0.,band);}assert_eq!(value,0.);}
    }
    #[test]
    fn pause_freezes_decks_clock_and_effect_tails() {
        let (mut c, mut mixer) = create(100);
        push(&mut c, 0, 100, 0, 0.2);
        c.commands.push(Command::Pause(false)).unwrap();
        let _ = mixer.by_ref().take(20).collect::<Vec<_>>();
        let before = c.clock.rendered_frames.load(Ordering::Relaxed);
        c.commands.push(Command::Pause(true)).unwrap();
        assert!(mixer.by_ref().take(20).all(|v| v == 0.));
        assert_eq!(c.clock.rendered_frames.load(Ordering::Relaxed), before);
        c.commands.push(Command::Pause(false)).unwrap();
        let _ = mixer.by_ref().take(2).collect::<Vec<_>>();
        assert_eq!(c.clock.rendered_frames.load(Ordering::Relaxed), before + 1);
    }
    #[test]
    fn blend_is_sample_scheduled_bounded_and_source_time_is_actual() {
        let (mut c, mut mixer) = create(100);
        push(&mut c, 0, 100, 0, 0.9);
        push(&mut c, 1, 100, 1000, 0.9);
        c.commands
            .push(Command::Plan(Schedule {
                cancel_revision:0,
                id:1,
                cue: 10.,
                frames: 20,
                incoming: 1,
                tail: Tail::Echo,
            }))
            .unwrap();
        c.commands.push(Command::Pause(false)).unwrap();
        let values = mixer.by_ref().take(70).collect::<Vec<_>>();
        assert!(values.iter().all(|v| v.is_finite() && v.abs() <= 1.));
        assert_eq!(c.clock.active.load(Ordering::Relaxed), 1);
        assert!(c.clock.source_frames[1].load(Ordering::Relaxed) >= 1020);
    }
    #[test]
    fn cancellation_discards_effect_tails_and_transition() {
        let (mut c, mut mixer) = create(100);
        push(&mut c, 0, 100, 0, 0.2);
        push(&mut c, 1, 100, 0, 0.4);
        c.commands
            .push(Command::Plan(Schedule {
                cancel_revision:0,
                id:1,
                cue: 0.,
                frames: 80,
                incoming: 1,
                tail: Tail::Bounce,
            }))
            .unwrap();
        c.commands.push(Command::Pause(false)).unwrap();
        mixer.by_ref().take(20).for_each(drop);
        assert!(c.clock.mixing.load(Ordering::Relaxed));
        c.commands.push(Command::Cancel).unwrap();
        let value = mixer.next().unwrap();
        assert!(!c.clock.mixing.load(Ordering::Relaxed));
        assert!((value - 0.2).abs() < 1e-6);
    }
    #[test]
    fn normal_tempo_processing_preserves_tone_pitch() {
        let frames = RATE as usize * 2;
        let input = (0..frames)
            .flat_map(|i| {
                let sample = (i as f32 * std::f32::consts::TAU * 440. / RATE as f32).sin() * 0.2;
                [sample, sample]
            })
            .collect::<Vec<_>>();
        for tempo in [0.94, 1.06] {
            let output = stretch_stereo(&input, tempo).unwrap();
            let mono = output.iter().step_by(2).copied().collect::<Vec<_>>();
            let middle = &mono[RATE as usize / 2..RATE as usize * 3 / 2];
            let crossings = middle
                .windows(2)
                .filter(|v| v[0] <= 0. && v[1] > 0.)
                .count();
            assert!(
                (crossings as i32 - 440).abs() <= 2,
                "tone changed: {crossings}"
            );
            assert!((output.len() as f64 / (input.len() as f64 / tempo as f64) - 1.).abs() < 0.001);
        }
        assert!(stretch_stereo(&input, 1.07).is_err());
    }
}

/// Deterministic capture of the production mixer, for output-level regression QA.
/// Feed source frames off-thread; this uses the same processing as the device sink.
pub fn capture(input:&[[f32;2]],listening:super::listening::Listening,tail_frames:usize)->Vec<[f32;2]>{
 let(mut control,mut mixer)=create(4096);
 control.commands.push(Command::Equalizer(listening.normalize().coefficients(),1)).unwrap();
 control.commands.push(Command::Pause(false)).unwrap();
 let mut output=Vec::with_capacity(input.len()+tail_frames);
 for (i,samples) in input.iter().enumerate(){control.decks[0].push(Frame{samples:*samples,source_frame:i as f64,generation:0,listening_revision:0}).unwrap();output.push([mixer.next().unwrap(),mixer.next().unwrap()]);}
 // No invented decoder frames: this also verifies that EOF renders room tails.
 for _ in 0..tail_frames {output.push([mixer.next().unwrap(),mixer.next().unwrap()]);}
 output
}
#[cfg(test)]mod output_tests {
 use super::*;
 #[test]fn actual_output_has_reverb_tail_and_dry_bypass(){
  let mut input=vec![[0.;2];4800];input[2400]=[0.5,0.25];
  let dry=capture(&input,Default::default(),48000);
  assert_eq!(&dry[..input.len()],input.as_slice());assert!(dry[input.len()..].iter().all(|x|*x==[0.;2]));
  let wet=capture(&input,super::super::listening::Listening{reverb:0.45,..Default::default()},48000);
  let energy: f32=wet[input.len()..].iter().flatten().map(|v|v*v).sum();assert!(energy>0.00001,"No wet tail in actual output: {energy}");assert_ne!(dry,wet);assert!(wet.iter().flatten().all(|v|v.is_finite()&&v.abs()<=1.));
 }
 #[test]fn pause_freezes_room_and_effect_changes_are_acknowledged(){
  let(mut c,mut m)=create(4096);c.commands.push(Command::Pause(false)).unwrap();c.commands.push(Command::Equalizer(super::super::listening::Listening{reverb:0.8,..Default::default()}.coefficients(),7)).unwrap();
  for i in 0..3000 {c.decks[0].push(Frame{samples:if i==2000{[0.5;2]}else{[0.;2]},..Default::default()}).unwrap();m.next();m.next();}
  assert_eq!(c.clock.listening_revision.load(Ordering::Acquire),7);assert_eq!(f32::from_bits(c.clock.applied_reverb.load(Ordering::Acquire)),0.8);
  c.commands.push(Command::Pause(true)).unwrap();for _ in 0..10000{assert_eq!(m.next(),Some(0.));}
  c.commands.push(Command::Pause(false)).unwrap();let energy:f32=(0..10000).map(|_|m.next().unwrap().powi(2)).sum();assert!(energy>0.00001);
  c.commands.push(Command::Equalizer(Default::default(),8)).unwrap();m.next();m.next();assert_eq!(c.clock.listening_revision.load(Ordering::Acquire),8);
 }
}
#[cfg(test)]mod cancellation_tests {
 use super::*;
 #[test]fn cancellation_epoch_invalidates_a_queued_plan_without_a_command_slot(){let(mut c,mut m)=create(4);c.commands.push(Command::Plan(Schedule{cancel_revision:0,id:9,cue:100.,frames:96000,incoming:1,tail:Tail::Clean})).unwrap();while c.commands.slots()>0 {c.commands.push(Command::Volume(1.)).unwrap();}c.clock.cancel_revision.fetch_add(1,Ordering::AcqRel);m.next();assert_eq!(c.clock.schedule_id.load(Ordering::Acquire),0);assert!(!c.clock.mixing.load(Ordering::Relaxed));}
 #[test]fn new_plan_in_current_epoch_survives_cancellation_of_old_work(){let(mut c,mut m)=create(4);c.clock.cancel_revision.store(2,Ordering::Release);c.commands.push(Command::Plan(Schedule{cancel_revision:2,id:10,cue:100.,frames:96000,incoming:1,tail:Tail::Clean})).unwrap();m.next();assert_eq!(c.clock.schedule_id.load(Ordering::Acquire),10);}
}
