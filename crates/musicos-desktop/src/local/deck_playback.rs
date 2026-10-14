//! One desktop output, two streaming decoder workers, and a sample-clocked mixer.
use super::dj_audio::{self, Command, Frame, Schedule, Tail, RATE};
use anyhow::{Context, Result};
use musicos_init::{audio::LoopMode, database::Track};
use rodio::{source::UniformSourceIterator, OutputStream, Sink, Source};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};
#[derive(Clone, Debug)]
pub struct Transition {
    pub outgoing: u64,
    pub incoming: u64,
    pub outgoing_cue: f32,
    pub incoming_cue: f32,
    pub duration: f32,
    pub tempo: f32,
    pub effect: String,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct TransitionStatus {
    pub outgoing_entry: u64,
    pub plan_id:u64,
    pub incoming_entry: u64,
    pub active: bool,
    pub status: &'static str,
    pub outgoing_cue: f32,
    pub incoming_cue: f32,
    pub seconds: f32,
    pub source_seconds:f32,
    pub tempo: f32,
}
struct Load {
    path: String,
    start: f32,
    transpose: f32,
    generation: u64,
}
struct Deck {
    load: mpsc::Sender<Load>,
    parameters: Arc<Mutex<(f32,f32,u64)>>,
    generation: Arc<AtomicU64>,
    worker: Option<thread::JoinHandle<()>>,
}
#[derive(Clone)]
struct Entry {
    key: u64,
    track: Track,
}
pub struct Status {
    pub track: Option<Track>,
    pub queue_index: Option<usize>,
    pub is_playing: bool,
    pub position: f32,
    pub volume: f32,
    pub shuffle: bool,
    pub loop_mode: LoopMode,
    pub backend: String,
    pub last_error: Option<String>,
}
pub struct DesktopPlayer {
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
    control: dj_audio::Control,
    decks: Vec<Deck>,
    errors: Arc<Mutex<Option<String>>>,
    queue: Vec<Entry>,
    current: Option<usize>,
    next_key: u64,
    active: usize,
    loaded: [Option<u64>; 2],
    heard: [u64; 2],
    listens: std::collections::BTreeMap<u64, f32>,
    activation_pending: Option<usize>,
    prepared: Option<(u64, usize)>,
    transition: Option<Transition>,
    schedule_id:u64,
    scheduled_reported:bool,
    retry_after:std::time::Instant,
    paused: bool,
    was_mixing: bool,
    volume: f32,
    pub listening: super::listening::Listening,
    pub listening_revision:u64,
    dj_mode:bool,
    shuffle: bool,
    repeat: LoopMode,
    pub pins: std::collections::BTreeSet<u64>,
    pub arranged: std::collections::BTreeSet<u64>,
    pub plan_revision: u64,
}
impl DesktopPlayer {
    pub fn new() -> Self {
        let (mut control, mixer) = dj_audio::create(RATE as usize / 12);
        let producers = std::mem::replace(
            &mut control.decks,
            [rtrb::RingBuffer::new(1).0, rtrb::RingBuffer::new(1).0],
        );
        let errors = Arc::new(Mutex::new(None));
        let mut decks = Vec::new();
        for (index, mut producer) in producers.into_iter().enumerate() {
            let (tx, rx) = mpsc::channel::<Load>();
            let generation = Arc::new(AtomicU64::new(0));
            let token = generation.clone();
            let parameters=Arc::new(Mutex::new((1_f32,1_f32,0_u64)));
            let worker_parameters=parameters.clone();
            let errors = errors.clone();
            let clock = control.clock.clone();
            let worker = thread::Builder::new()
                .name(format!("musicos-deck-{index}"))
                .spawn(move || {
                    while let Ok(load) = rx.recv() {
                        let result = (|| -> Result<()> {
                            let mut decoder =
                                super::decoded_audio::open(std::path::Path::new(&load.path))?;
                            let mut skip = 0;
                            if load.start > 0.
                                && decoder
                                    .try_seek(Duration::from_secs_f32(load.start))
                                    .is_err()
                            {
                                decoder=super::decoded_audio::open(std::path::Path::new(&load.path))?;
                                skip = (load.start * RATE as f32) as usize * 2;
                            }
                            let mut source = UniformSourceIterator::new(
                                decoder.convert_samples::<f32>(),
                                2,
                                RATE,
                            )
                            .skip(skip);
                            let mut stretch = signalsmith_stretch::Stretch::preset_default(2, RATE);
                            stretch.set_transpose_factor(load.transpose, None);
                            let mut tempo = (worker_parameters.lock().unwrap_or_else(|e|e.into_inner()).0).clamp(0.47, 2.12);
                            let mut transpose=load.transpose;
                            let mut processing=(tempo-1.).abs()>0.0001||(transpose-1.).abs()>0.0001;
                            let mut position = load.start as f64 * RATE as f64;
                            let delay=stretch.output_latency();
                            let mut dry=vec![[0_f32;2];delay.max(1)];
                            let mut dry_at=0;
                            let mut wet=if processing {1_f32}else{0_f32};
                            let mut latency=delay;
                            let mut push = |samples: &[f32], rate:f32, revision:u64| -> bool {
                                for pair in samples.chunks_exact(2) {
                                    if latency > 0 {
                                        latency -= 1;
                                        continue;
                                    }
                                    let frame = Frame {
                                        samples: [pair[0], pair[1]],
                                        source_frame: position,
                                        generation: load.generation,
                                        listening_revision:revision,
                                    };
                                    position += rate as f64;
                                    let mut frame = frame;
                                    loop {
                                        if token.load(Ordering::Acquire) != load.generation {
                                            return false;
                                        }
                                        match producer.push(frame) {
                                            Ok(()) => break,
                                            Err(e) => {
                                                frame = match e {
                                                    rtrb::PushError::Full(value) => value,
                                                };
                                                thread::sleep(Duration::from_millis(2));
                                            }
                                        }
                                    }
                                }
                                true
                            };
                            loop {
                                if token.load(Ordering::Acquire) != load.generation {
                                    return Ok(());
                                }
                                // Latest parameters are read off-thread once per 10.7 ms block.
                                // Keep the decoder and stretch history alive across slider changes.
                                let (speed,pitch,revision)=*worker_parameters.lock().unwrap_or_else(|e|e.into_inner());
                                let target=speed.clamp(0.47,2.12);
                                tempo=smooth_parameter(tempo,target);
                                transpose=smooth_parameter(transpose,pitch);
                                stretch.set_transpose_factor(transpose,None);
                                processing=(tempo-1.).abs()>0.0001||(transpose-1.).abs()>0.0001;
                                // Avoid the channel converter size_hint: some decoders report a zero lower bound mid-frame.
                                let mut input=Vec::with_capacity(512*2);
                                for _ in 0..512*2 {match source.next(){Some(sample)=>input.push(sample),None=>break}}
                                if input.is_empty() {
                                    break;
                                }
                                let frames=((input.len()/2) as f32/tempo).round() as usize;
                                let mut output=vec![0.;frames*2];
                                // Warm the processor even during exact PCM bypass, avoiding a cold
                                // processor restart when speed or pitch first changes.
                                stretch.process(&input,&mut output);
                                let mut delayed=Vec::with_capacity(input.len());
                                for pair in input.chunks_exact(2) {delayed.extend_from_slice(&dry[dry_at]);dry[dry_at]=[pair[0],pair[1]];dry_at=(dry_at+1)%dry.len();}
                                let target_wet=if processing {1.}else{0.};
                                for i in 0..frames {
                                    wet=if wet<target_wet {(wet+1./960.).min(target_wet)}else{(wet-1./960.).max(target_wet)};
                                    let d=((i as f32*tempo) as usize).min(delayed.len()/2-1)*2;
                                    for c in 0..2 {output[i*2+c]=if wet==0. {delayed[d+c]}else{output[i*2+c]*wet+delayed[d+c]*(1.-wet)};}
                                }
                                if !push(&output,tempo,revision){return Ok(());}

                            }
                            let mut output=vec![0.;delay*2];
                            if processing {stretch.flush(&mut output);}else{for i in 0..delay {output[i*2..i*2+2].copy_from_slice(&dry[(dry_at+i)%dry.len()]);}}
                            if !push(&output,tempo,worker_parameters.lock().unwrap_or_else(|e|e.into_inner()).2){return Ok(());}
                            clock.finished[index].store(load.generation, Ordering::Release);
                            Ok(())
                        })();
                        if let Err(error) = result {
                            *errors.lock().unwrap_or_else(|e| e.into_inner()) =
                                Some(format!("Cannot play this file: {error:#}"));
                            clock.finished[index].store(load.generation, Ordering::Release);
                        }
                    }
                })
                .expect("decoder worker");
            decks.push(Deck {
                load: tx,
                parameters,
                generation,
                worker: Some(worker),
            });
        }
        let output = (|| -> Result<_> {
            let (stream, handle) = OutputStream::try_default()?;
            let sink = Sink::try_new(&handle)?;
            Ok((stream, sink))
        })();
        let (stream, sink) = match output {
            Ok((stream, sink)) => {
                sink.append(mixer);
                (Some(stream), Some(sink))
            }
            Err(e) => {
                *errors.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("Audio output unavailable: {e}"));
                (None, None)
            }
        };
        Self {
            _stream: stream,
            sink,
            control,
            decks,
            errors,
            queue: vec![],
            current: None,
            next_key: 1,
            active: 0,
            loaded: [None; 2],
            heard: [0; 2],
            listens: Default::default(),
            activation_pending: None,
            prepared: None,
            transition: None,
            schedule_id:0,
            scheduled_reported:false,
            retry_after:std::time::Instant::now(),
            paused: true,
            was_mixing: false,
            volume: 1.,
            listening: Default::default(),
            listening_revision:0,
            dj_mode:false,
            shuffle: false,
            repeat: LoopMode::None,
            pins: Default::default(),
            arranged: Default::default(),
            plan_revision: 0,
        }
    }
    fn command(&mut self, c: Command) {
        if self.control.commands.push(c).is_err() {
            *self.errors.lock().unwrap_or_else(|e| e.into_inner()) =
                Some("Audio controls are busy; try again".into());
        }
    }
    fn account(&mut self) {
        for deck in 0..2 {
            let frames = self.control.clock.audible_frames[deck].load(Ordering::Acquire);
            let delta = frames.saturating_sub(self.heard[deck]);
            self.heard[deck] = frames;
            if let Some(key) = self.loaded[deck] {
                *self.listens.entry(key).or_default() += delta as f32 / RATE as f32;
            }
        }
    }
    pub fn transition_status(&self) -> Option<TransitionStatus> {
        self.transition.as_ref().map(|t| TransitionStatus {
            outgoing_entry: t.outgoing,
            plan_id:self.schedule_id,
            incoming_entry: t.incoming,
            active: self.control.clock.mixing.load(Ordering::Acquire),
            status: if self.control.clock.mixing.load(Ordering::Acquire) {"active"} else if self.control.clock.schedule_id.load(Ordering::Acquire)==self.schedule_id && self.prepared.is_some_and(|(_,deck)|self.control.clock.ready[deck].load(Ordering::Acquire)) {"scheduled"} else {"preparing"},
            outgoing_cue: t.outgoing_cue,
            incoming_cue: t.incoming_cue,
            seconds: (t.duration/self.listening.speed).min(16.),
            source_seconds:t.duration,
            tempo: t.tempo,
        })
    }
    pub fn audio_levels(&self) -> [f32; 3] {
        if self.paused { return [0.; 3]; }
        self.control.clock.levels.each_ref().map(|value| f32::from_bits(value.load(Ordering::Relaxed)))
    }
    pub fn listened(&self) -> f32 {
        self.current
            .and_then(|i| self.listens.get(&self.queue[i].key))
            .copied()
            .unwrap_or(0.)
    }
    fn load(&mut self, deck: usize, track: Track, start: f32, tempo: f32, key: u64) {
        self.account();
        self.loaded[deck] = Some(key);
        self.control.clock.ready[deck].store(false,Ordering::Release);
        let generation = self.decks[deck].generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.command(Command::Reset {
            deck,
            generation,
            source_frame: start as f64 * RATE as f64,
        });
        *self.decks[deck].parameters.lock().unwrap_or_else(|e|e.into_inner())=(tempo*self.listening.speed,self.listening.transpose(),self.listening_revision);
        let _ = self.decks[deck].load.send(Load {
            path: track.path,
            start,
            transpose:self.listening.transpose(),
            generation,
        });
    }
    fn entry(&mut self, track: Track) -> Entry {
        let key = self.next_key;
        self.next_key = self.next_key.wrapping_add(1);
        Entry { key, track }
    }
    fn start(&mut self, index: usize, position: f32, paused: bool) {
        if index >= self.queue.len() {
            return;
        }
        self.cancel();
        self.current = Some(index);
        self.active = 0;
        self.activation_pending = Some(0);
        *self.errors.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.load(
            0,
            self.queue[index].track.clone(),
            position,
            1.,
            self.queue[index].key,
        );
        self.command(Command::Activate(0));
        self.paused = paused;
        self.command(Command::Pause(paused));
        self.plan_revision = self.plan_revision.wrapping_add(1);
    }
    pub fn cancel(&mut self) {
        if let Some(plan)=&self.transition {eprintln!("DJmode: cancelling/replanning {} → {}",plan.outgoing,plan.incoming);}
        self.was_mixing=false;
        super::dj_diagnostics::record("canceled",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),"navigation, queue edit, or replan");
        self.account();
        self.command(Command::Cancel);
        self.prepared = None;
        self.transition = None;
        let other = 1 - self.active;
        self.decks[other].generation.fetch_add(1, Ordering::AcqRel);
        self.plan_revision = self.plan_revision.wrapping_add(1);
    }
    pub fn prepare(&mut self, mut plan: Transition) -> Result<()> {
        let Some(current) = self.current else {
            return Ok(());
        };
        anyhow::ensure!(
            self.queue[current].key == plan.outgoing,
            "Transition belongs to an old queue"
        );
        let next = self
            .queue
            .iter()
            .position(|e| e.key == plan.incoming)
            .context("Incoming queue entry changed")?;
        anyhow::ensure!(next == current + 1, "Transition is no longer next");
        anyhow::ensure!(
            plan.outgoing_cue.is_finite() && plan.outgoing_cue>=0. && plan.incoming_cue.is_finite() && plan.incoming_cue>=0. && plan.duration.is_finite()
                && (0. ..=16.).contains(&plan.duration)
                && plan.tempo.is_finite()
                && (0.94..=1.06).contains(&plan.tempo),
            "Invalid transition"
        );
        if plan.duration>0. && !(2. ..=16.).contains(&(plan.duration/self.listening.speed)) {
            plan.duration=(2.*self.listening.speed).min(self.queue[current].track.duration as f32/3.);
            plan.outgoing_cue=(self.queue[current].track.duration as f32-plan.duration).max(0.);
            plan.incoming_cue=0.;plan.tempo=1.;plan.effect.clear();
            super::dj_diagnostics::record("fallback",Some(plan.outgoing),Some(plan.incoming),"user speed requires a bounded ending blend");
        }
        let position = self.get_position();
        if plan.duration > 0. && position + 0.5*self.listening.speed >= plan.outgoing_cue {
            // Revalidate against the audible source clock. Never activate a missed
            // phrase. Leave preparation time and use an unwarped ending instead.
            let ending = self.queue[current].track.duration as f32;
            let remaining = (ending - position - 0.75*self.listening.speed).max(0.);
            plan.duration = remaining.min(2.*self.listening.speed);
            plan.outgoing_cue = ending - plan.duration;
            plan.incoming_cue = 0.;
            plan.tempo = 1.;
            plan.effect.clear();
            eprintln!("DJmode: missed cue; safe ending replan at {:.2}s ({:.2}s overlap)", plan.outgoing_cue, plan.duration);
        }
        eprintln!("DJmode: preparing {} → {} at {:.2}s, overlap {:.2}s", plan.outgoing, plan.incoming, plan.outgoing_cue, plan.duration);
        let deck = 1 - self.active;
        self.load(
            deck,
            self.queue[next].track.clone(),
            plan.incoming_cue,
            plan.tempo,
            self.queue[next].key,
        );
        let tail = match plan.effect.as_str() {
            "echo_tail" => Tail::Echo,
            "reverb_tail" => Tail::Reverb,
            "echo_bounce" => Tail::Bounce,
            _ => Tail::Clean,
        };
        static NEXT_PLAN:AtomicU64=AtomicU64::new(1);
        self.schedule_id=NEXT_PLAN.fetch_add(1,Ordering::Relaxed);self.scheduled_reported=false;
        if self.control.commands.push(Command::Plan(Schedule {
            id:self.schedule_id,
            cue: plan.outgoing_cue as f64 * RATE as f64,
            frames: ((plan.duration/self.listening.speed).min(16.) * RATE as f32) as u64,
            incoming: deck,
            tail,
        })).is_err(){self.prepared=None;self.transition=None;self.plan_revision=self.plan_revision.wrapping_add(1);
            super::dj_diagnostics::record("delivery_failed",Some(plan.outgoing),Some(plan.incoming),"mixer command ring full");anyhow::bail!("Audio controls are busy; transition will be retried");}
        super::dj_diagnostics::record("preparing",Some(plan.outgoing),Some(plan.incoming),format!("plan {} cue {:.3}s overlap {:.3}s",self.schedule_id,plan.outgoing_cue,plan.duration));
        self.prepared = Some((plan.incoming, deck));
        self.transition = Some(plan);
        Ok(())
    }
    pub fn tick(&mut self) {
        let mixing=self.control.clock.mixing.load(Ordering::Acquire);
        if mixing&&!self.was_mixing {super::dj_diagnostics::record("activated",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),format!("source {:.3}s",self.get_position()));}
        if !self.scheduled_reported && self.control.clock.schedule_id.load(Ordering::Acquire)==self.schedule_id && self.prepared.is_some_and(|(_,d)|self.control.clock.ready[d].load(Ordering::Acquire)) {self.scheduled_reported=true;super::dj_diagnostics::record("accepted",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),format!("mixer acknowledged plan {}, deck ready",self.schedule_id));}
        self.was_mixing=mixing;
        self.account();
        if self.control.clock.plan_failed.swap(false, Ordering::AcqRel) {
            super::dj_diagnostics::record("fallback",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),"preparation deadline missed; revalidate against audible source clock");
            self.retry_after=std::time::Instant::now()+Duration::from_millis(500);
            self.prepared = None;
            self.transition = None;
            self.plan_revision = self.plan_revision.wrapping_add(1);
        }
        let active = self.control.clock.active.load(Ordering::Acquire) as usize;
        if let Some(expected) = self.activation_pending {
            if active != expected {
                return;
            }
            self.activation_pending = None;
        }
        if active != self.active {
            super::dj_diagnostics::record("promoted",None,self.prepared.map(|p|p.0),format!("source {:.3}s",self.get_position()));
            self.active = active;
            if let Some((key, deck)) = self.prepared.take() {
                if deck == active {
                    self.current = self.queue.iter().position(|e| e.key == key);
                    self.transition = None;
                    self.plan_revision = self.plan_revision.wrapping_add(1);
                }
            }
        }
        if self.paused {
            return;
        }
        if self.prepared.is_none() && std::time::Instant::now()>=self.retry_after {
            if let Some(index) = self.current {
                if index + 1 < self.queue.len() && self.repeat != LoopMode::One {
                    let _ = self.prepare(Transition {
                        outgoing: self.queue[index].key,
                        incoming: self.queue[index + 1].key,
                        outgoing_cue: (self.queue[index].track.duration as f32-if self.dj_mode{2_f32.min(self.queue[index].track.duration as f32/3.)}else{0.}).max(0.),
                        incoming_cue: 0.,
                        duration: if self.dj_mode{2_f32.min(self.queue[index].track.duration as f32/3.)}else{0.},
                        tempo: 1.,
                        effect: String::new(),
                    });
                } else if self.control.clock.drained[self.active].load(Ordering::Acquire) {
                    if self.repeat == LoopMode::One {
                        self.start(index, 0., false);
                    } else if self.repeat == LoopMode::All && !self.queue.is_empty() {
                        self.start(0, 0., false);
                    } else {
                        self.pause();
                    }
                }
            }
        }
    }
    pub fn set_queue(&mut self, tracks: Vec<Track>, index: usize) {
        self.queue = tracks.into_iter().map(|t| self.entry(t)).collect();
        self.pins.clear();
        self.arranged.clear();
        self.listens.clear();
        if self.queue.is_empty() {
            self.queue_clear();
        } else {
            self.start(index.min(self.queue.len() - 1), 0., false);
        }
    }
    pub fn restore_queue_paused(&mut self, tracks: Vec<Track>, index: usize, position: f32) {
        self.queue = tracks.into_iter().map(|t| self.entry(t)).collect();
        if !self.queue.is_empty() {
            self.start(index.min(self.queue.len() - 1), position, true);
        }
    }
    pub fn queue_tracks(&self) -> Vec<Track> {
        self.queue.iter().map(|e| e.track.clone()).collect()
    }
    pub fn entry_ids(&self) -> Vec<u64> {
        self.queue.iter().map(|e| e.key).collect()
    }
    pub fn current_index(&self) -> Option<usize> {
        self.current
    }
    pub fn get_current_track(&self) -> Option<&Track> {
        self.current
            .and_then(|i| self.queue.get(i))
            .map(|e| &e.track)
    }
    pub fn get_position(&self) -> f32 {
        self.control.clock.source_frames[self.active].load(Ordering::Acquire) as f32 / RATE as f32
    }
    pub fn is_playing(&self) -> bool {
        !self.paused && self.current.is_some() && self.sink.is_some()
    }
    pub fn pause(&mut self) {
        self.paused = true;
        self.command(Command::Pause(true));
    }
    pub fn play_pause(&mut self) {
        if self.current.is_none() {
            return;
        }
        self.paused = !self.paused;
        self.command(Command::Pause(self.paused));
    }
    pub fn seek(&mut self, position: f32) {
        if let Some(i) = self.current {
            let duration = self.queue[i].track.duration as f32;
            self.start(
                i,
                if position.is_finite() {
                    position.clamp(0., duration.max(0.))
                } else {
                    0.
                },
                self.paused,
            );
        }
    }
    pub fn next(&mut self) {
        if let Some(i) = self.current {
            if i + 1 < self.queue.len() {
                self.start(i + 1, 0., false);
            } else if self.repeat == LoopMode::All {
                self.start(0, 0., false);
            } else {
                self.pause();
            }
        }
    }
    pub fn prev(&mut self) {
        if self.get_position() > 3. {
            self.seek(0.);
        } else if let Some(i) = self.current {
            self.start(i.saturating_sub(1), 0., false);
        }
    }
    pub fn set_dj_mode(&mut self,enabled:bool){self.dj_mode=enabled;self.cancel();}
    pub fn set_listening(&mut self,value:super::listening::Listening)->Result<()> {
        let value=value.normalize();let revision=self.listening_revision.wrapping_add(1);
        self.control.commands.push(Command::Equalizer(value.coefficients(),revision)).map_err(|_|anyhow::anyhow!("Audio controls are busy; retry"))?;
        let changed=value.speed!=self.listening.speed||value.pitch!=self.listening.pitch;
        self.listening=value;self.listening_revision=revision;
        if changed {self.cancel();}
        for deck in &self.decks {let mut p=deck.parameters.lock().unwrap_or_else(|e|e.into_inner());*p=(if changed {self.listening.speed}else{p.0},self.listening.transpose(),revision);}

        Ok(())
    }
    pub fn listening_applied(&self)->bool{self.control.clock.listening_revision.load(Ordering::Acquire)==self.listening_revision&&self.activation_pending.is_none()&&(self.paused||self.current.is_none()||self.control.clock.parameter_revision[self.active].load(Ordering::Acquire)==self.listening_revision)}
    pub fn set_volume(&mut self, value: f32) {
        if value.is_finite() {
            self.volume = value.clamp(0., 1.);
            self.command(Command::Volume(self.volume));
        }
    }
    pub fn set_shuffle(&mut self, value: bool) {
        self.shuffle = value;
    }
    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            let start = self.current.map(|i| i + 1).unwrap_or(0);
            let mut seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as usize;
            for i in (start + 1..self.queue.len()).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let j = start + seed % (i - start + 1);
                self.queue.swap(i, j);
            }
            self.cancel();
        }
    }
    pub fn set_loop_mode(&mut self, value: LoopMode) {
        self.repeat = value;
        self.cancel();
    }
    pub fn toggle_loop_mode(&mut self) {
        self.set_loop_mode(match self.repeat {
            LoopMode::None => LoopMode::All,
            LoopMode::All => LoopMode::One,
            LoopMode::One => LoopMode::None,
        });
    }
    pub fn queue_select(&mut self, index: usize) -> bool {
        if index >= self.queue.len() {
            return false;
        }
        self.start(index, 0., false);
        true
    }
    pub fn queue_move(&mut self, from: usize, to: usize) -> bool {
        if from >= self.queue.len() || to >= self.queue.len() {
            return false;
        }
        let key = self.current.map(|i| self.queue[i].key);
        let entry = self.queue.remove(from);
        self.pins.insert(entry.key);
        self.queue.insert(to, entry);
        self.current = key.and_then(|k| self.queue.iter().position(|e| e.key == k));
        self.cancel();
        true
    }
    pub fn arrange(&mut self, ids: Vec<u64>) -> Result<()> {
        let start = self.current.map(|i| i + 1).unwrap_or(0);
        anyhow::ensure!(
            ids.len() == self.queue.len() - start,
            "Queue changed during arrangement"
        );
        let before = self.queue[start..].to_vec();
        let old = before
            .iter()
            .map(|e| e.key)
            .collect::<std::collections::BTreeSet<_>>();
        anyhow::ensure!(
            old == ids.iter().copied().collect(),
            "Queue changed during arrangement"
        );
        for (offset, key) in ids.iter().enumerate() {
            if self.pins.contains(&self.queue[start + offset].key) {
                anyhow::ensure!(
                    self.queue[start + offset].key == *key,
                    "Pinned queue position changed"
                );
            }
        }
        for (offset, key) in ids.into_iter().enumerate() {
            let entry = before.iter().find(|e| e.key == key).unwrap().clone();
            if self.queue[start + offset].key != key {
                self.arranged.insert(key);
            }
            self.queue[start + offset] = entry;
        }
        self.cancel();
        Ok(())
    }
    pub fn queue_remove(&mut self, index: usize) -> Option<Track> {
        if index >= self.queue.len() {
            return None;
        }
        let current = self.current.map(|i| self.queue[i].key);
        let removed = self.queue.remove(index);
        self.current = current.and_then(|k| self.queue.iter().position(|e| e.key == k));
        if current == Some(removed.key) {
            if self.queue.is_empty() {
                self.queue_clear();
            } else {
                self.start(index.min(self.queue.len() - 1), 0., self.paused);
            }
        } else {
            self.cancel();
        }
        Some(removed.track)
    }
    pub fn queue_add_next(&mut self, track: Track) {
        let entry = self.entry(track);
        let index = self.current.map(|i| i + 1).unwrap_or(self.queue.len());
        self.queue.insert(index, entry);
        if self.current.is_none() {
            self.start(0, 0., true);
        } else {
            self.cancel();
        }
    }
    pub fn queue_insert(&mut self,track:Track,index:usize){
        let index=index.max(self.current.map(|i|i+1).unwrap_or(0)).min(self.queue.len());
        let entry=self.entry(track);self.pins.insert(entry.key);self.queue.insert(index,entry);
        if self.current.is_none(){self.start(0,0.,true);}else{self.cancel();}
    }
    pub fn queue_add_later(&mut self, track: Track) {
        let entry = self.entry(track);
        self.queue.push(entry);
        if self.current.is_none() {
            self.start(0, 0., true);
        } else {
            self.cancel();
        }
    }
    pub fn queue_clear_upcoming(&mut self) {
        self.queue
            .truncate(self.current.map(|i| i + 1).unwrap_or(0));
        self.cancel();
    }
    pub fn queue_clear(&mut self) {
        self.pause();
        self.cancel();
        self.queue.clear();
        self.current = None;
        self.decks[self.active]
            .generation
            .fetch_add(1, Ordering::AcqRel);
    }
    pub fn get_status(&self) -> Status {
        Status {
            track: self.get_current_track().cloned(),
            queue_index: self.current,
            is_playing: self.is_playing(),
            position: self.get_position(),
            volume: self.volume,
            shuffle: self.shuffle,
            loop_mode: self.repeat,
            backend: if self.sink.is_some() {
                "MusicOS two-deck"
            } else {
                "unavailable"
            }
            .into(),
            last_error: self
                .errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        }
    }
}
impl Drop for DesktopPlayer {
    fn drop(&mut self) {
        for deck in &mut self.decks {
            deck.generation.fetch_add(1, Ordering::AcqRel);
            let (tx, _) = mpsc::channel();
            let old = std::mem::replace(&mut deck.load, tx);
            drop(old);
        }
        for deck in &mut self.decks {
            if let Some(worker) = deck.worker.take() {
                let _ = worker.join();
            }
        }
    }
}

// Approximately 45 ms of parameter smoothing, snapped exactly to the baseline.
fn smooth_parameter(current:f32,target:f32)->f32 {if (target-current).abs()<0.0001 {target} else {current+(target-current)*0.25}}
#[cfg(test)]mod parameter_tests {use super::*;#[test]fn reversals_remain_bounded_and_settle(){let mut value=1.;for _ in 0..60{value=smooth_parameter(value,2.);assert!((1. ..=2.).contains(&value));}assert_eq!(value,2.);for _ in 0..60{value=smooth_parameter(value,0.5);}assert_eq!(value,0.5);}}
