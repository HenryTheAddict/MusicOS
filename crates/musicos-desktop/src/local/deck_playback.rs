//! One desktop output, two streaming decoder workers, and a sample-clocked mixer.
use super::dj_audio::{self, Command, Frame, Schedule, Tail, RATE};
use anyhow::{Context, Result};
use musicos_init::{audio::LoopMode, database::Track};
use rodio::{source::UniformSourceIterator, Source};
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
    pub request_id:u64,
    pub outgoing: u64,
    pub incoming: u64,
    pub outgoing_cue: f32,
    pub incoming_cue: f32,
    pub duration: f32,
    pub tempo: f32,
    pub effect: String,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct TransitionOutcome {pub request_id:u64,pub outgoing_entry:u64,pub incoming_entry:u64,pub status:String,pub reason:String}
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
    entry_key:u64,
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
#[derive(Clone, Debug, serde::Serialize)]
pub struct AuditionStatus {pub track_id:i64,pub position:f32,pub duration:f32,pub preparing:bool}
struct Audition {track_id:i64,generation:u64,deck:usize,start:f32,end:f32,resume:bool,started:std::time::Instant}
pub struct DesktopPlayer {
    audition:Option<Audition>,
    output: super::audio_output::AudioOutput,
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
    preparing_since:std::time::Instant,
    pub transition_outcome:Option<TransitionOutcome>,
    pub failed_entries:std::collections::BTreeSet<u64>,
    prepare_attempts:std::collections::BTreeMap<u64,u8>,
    rate_revision:u64,
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
                        if token.load(Ordering::Acquire)!=load.generation {continue;}
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
                            if token.load(Ordering::Acquire)!=load.generation {continue;}
                            clock.failed[index].store(load.generation,Ordering::Release);
                            super::dj_diagnostics::record("decoder_failed",None,Some(load.entry_key),format!("generation {}: {error:#}",load.generation));
                            *errors.lock().unwrap_or_else(|e| e.into_inner()) =Some("Could not decode an audio file. Playback will try the next available song.".into());
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
        let output = super::audio_output::AudioOutput::new(mixer);
        Self {
            audition:None,
            output,
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
            preparing_since:std::time::Instant::now(),
            transition_outcome:None,failed_entries:Default::default(),prepare_attempts:Default::default(),rate_revision:0,
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
    pub fn visualizer_audio(&self)->Arc<super::visualizer_audio::Tap>{self.control.clock.visualizer.clone()}
    pub fn audio_levels(&self) -> [f32; 3] {
        if self.paused { return [0.; 3]; }
        self.control.clock.levels.each_ref().map(|value| f32::from_bits(value.load(Ordering::Relaxed)))
    }
    pub fn listened(&self) -> f32 {
        self.presentation().0
            .and_then(|i| self.listens.get(&self.queue[i].key))
            .copied()
            .unwrap_or(0.)
    }
    fn load(&mut self, deck: usize, track: Track, start: f32, tempo: f32, key: u64)->Result<()> {
        anyhow::ensure!(self.control.commands.slots()>=2,"Audio controls are busy; retry preparation");
        self.account();
        let generation=self.decks[deck].generation.fetch_add(1,Ordering::AcqRel)+1;
        self.control.commands.push(Command::Reset{deck,generation,source_frame:start as f64*RATE as f64}).map_err(|_|anyhow::anyhow!("Audio reset could not be delivered"))?;
        self.loaded[deck]=Some(key);
        self.control.clock.ready[deck].store(false,Ordering::Release);
        *self.decks[deck].parameters.lock().unwrap_or_else(|e|e.into_inner())=(tempo*self.listening.speed,self.listening.transpose(),self.rate_revision);
        self.decks[deck].load.send(Load{entry_key:key,path:track.path,start,transpose:self.listening.transpose(),generation}).context("Audio decoder stopped")?;
        Ok(())
    }
    fn outcome(&mut self,status:&str,reason:impl Into<String>){
        if let Some(plan)=&self.transition {let reason=reason.into();super::dj_diagnostics::record(status,Some(plan.outgoing),Some(plan.incoming),format!("request {}: {reason}",plan.request_id));self.transition_outcome=Some(TransitionOutcome{request_id:plan.request_id,outgoing_entry:plan.outgoing,incoming_entry:plan.incoming,status:status.into(),reason});}
    }
    pub fn next_playable(&self)->Option<usize>{self.current.and_then(|i|(i+1..self.queue.len()).find(|&n|!self.failed_entries.contains(&self.queue[n].key)))}
    pub fn reverb_processed_frames(&self)->u64{self.control.clock.reverb_processed_frames.load(Ordering::Relaxed)}
    pub fn applied_reverb_depth(&self)->f32{f32::from_bits(self.control.clock.applied_reverb_depth.load(Ordering::Acquire))}
    pub fn applied_reverb(&self)->f32{f32::from_bits(self.control.clock.applied_reverb.load(Ordering::Acquire))}
    pub fn applied_listening_revision(&self)->u64{self.control.clock.listening_revision.load(Ordering::Acquire)}
    fn entry(&mut self, track: Track) -> Entry {
        let key = self.next_key;
        self.next_key = self.next_key.wrapping_add(1);
        Entry { key, track }
    }
    fn start(&mut self, index: usize, position: f32, paused: bool) {
        if index >= self.queue.len() {
            return;
        }
        if self.control.commands.slots()<4 {*self.errors.lock().unwrap_or_else(|e|e.into_inner())=Some("Audio controls are busy; try again.".into());return;}
        self.cancel_reason("seek or manual navigation");
        self.current = Some(index);
        self.active = 0;
        self.activation_pending = Some(0);
        *self.errors.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.failed_entries.remove(&self.queue[index].key);
        self.prepare_attempts.remove(&self.queue[index].key);
        if let Err(error)=self.load(
            0,
            self.queue[index].track.clone(),
            position,
            1.,
            self.queue[index].key,
        ){*self.errors.lock().unwrap_or_else(|e|e.into_inner())=Some(error.to_string());self.paused=true;return;}
        self.command(Command::Activate(0));
        self.paused = paused;
        self.command(Command::Pause(paused));
        self.plan_revision = self.plan_revision.wrapping_add(1);
    }
    pub fn cancel(&mut self) {self.cancel_reason("queue or playback configuration changed");}
    fn cancel_reason(&mut self,reason:&str) {
        self.outcome("canceled",reason);
        self.retry_after=std::time::Instant::now()+Duration::from_millis(500);
        if let Some(plan)=&self.transition {eprintln!("DJmode: cancelling/replanning {} → {}",plan.outgoing,plan.incoming);}
        self.was_mixing=false;
        super::dj_diagnostics::record("canceled",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),"navigation, queue edit, or replan");
        self.account();
        self.control.clock.cancel_revision.fetch_add(1,Ordering::AcqRel);
        self.command(Command::Cancel);
        self.prepared = None;
        self.transition = None;
        let other = 1 - self.active;
        self.decks[other].generation.fetch_add(1, Ordering::AcqRel);
        self.plan_revision = self.plan_revision.wrapping_add(1);
    }
    pub fn prepare(&mut self, mut plan: Transition) -> Result<()> {
        if plan.request_id==0 {static IDS:AtomicU64=AtomicU64::new(1);plan.request_id=IDS.fetch_add(1,Ordering::Relaxed);}
        let request=plan.clone();
        let result=self.prepare_inner(plan);
        if let Err(error)=&result {self.transition_outcome=Some(TransitionOutcome{request_id:request.request_id,outgoing_entry:request.outgoing,incoming_entry:request.incoming,status:"rejected".into(),reason:error.to_string()});super::dj_diagnostics::record("rejected",Some(request.outgoing),Some(request.incoming),format!("request {}: {error}",request.request_id));}
        result
    }
    fn transition_is_accepted(&self)->bool {self.transition.is_some()&&(self.scheduled_reported||self.control.clock.mixing.load(Ordering::Acquire)||(self.control.clock.schedule_id.load(Ordering::Acquire)==self.schedule_id&&self.prepared.is_some_and(|(_,deck)|self.control.clock.ready[deck].load(Ordering::Acquire))))}
    fn prepare_inner(&mut self, mut plan:Transition)->Result<()> {
        anyhow::ensure!(self.audition.is_none(),"A song preview is using the spare deck");
        super::dj_diagnostics::record("requested",Some(plan.outgoing),Some(plan.incoming),format!("request {}",plan.request_id));
        anyhow::ensure!(!self.transition_is_accepted(),"An accepted transition is already protected");
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
        anyhow::ensure!(Some(next)==self.next_playable(), "Transition is no longer next playable entry");
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
        )?;
        let tail = match plan.effect.as_str() {
            "echo_tail" => Tail::Echo,
            "reverb_tail" => Tail::Reverb,
            "echo_bounce" => Tail::Bounce,
            _ => Tail::Clean,
        };
        self.schedule_id=plan.request_id;self.scheduled_reported=false;
        if self.control.commands.push(Command::Plan(Schedule {
            cancel_revision:self.control.clock.cancel_revision.load(Ordering::Acquire),
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
        self.preparing_since=std::time::Instant::now();
        self.outcome("preparing","decoder requested; awaiting mixer acknowledgement and ready audio");
        Ok(())
    }
    /// Auditions borrow the idle deck. The queue, its occurrence identities and
    /// the suspended deck's source position remain untouched.
    pub fn audition(&mut self,track:Track)->Result<()> {
        self.end_audition(true);
        self.settle_audible()?;
        anyhow::ensure!(self.output.available(),"Audio output is unavailable");
        anyhow::ensure!(self.control.commands.slots()>=6,"Audio controls are busy; try again");
        self.cancel_reason("song audition");
        let deck=1-self.active;let length=track.duration.max(0) as f32;
        let start=if length>40. {(length*0.25).min(length-30.)}else{0.};
        let end=(start+30.).min(length);let id=track.id;let resume=!self.paused;
        self.load(deck,track,start,1.,0)?;
        self.loaded[deck]=None; // An excerpt never increments the library's play count.
        self.paused=true;
        self.command(Command::Activate(deck));self.command(Command::Pause(false));
        self.audition=Some(Audition{track_id:id,generation:self.decks[deck].generation.load(Ordering::Acquire),deck,start,end,resume,started:std::time::Instant::now()});
        Ok(())
    }
    pub fn audition_status(&self)->Option<AuditionStatus>{self.audition.as_ref().map(|a|{
        let applied=self.control.clock.generations[a.deck].load(Ordering::Acquire)==a.generation;
        let source=if applied{self.control.clock.source_frames[a.deck].load(Ordering::Acquire) as f32/RATE as f32}else{a.start};
        AuditionStatus{track_id:a.track_id,position:(source-a.start).max(0.),duration:(a.end-a.start).max(0.),preparing:source<=a.start&&a.started.elapsed()<Duration::from_secs(5)}
    })}
    pub fn end_audition(&mut self,restore_playing:bool){
        if let Some(a)=self.audition.take(){
            self.decks[a.deck].generation.fetch_add(1,Ordering::AcqRel);
            self.command(Command::Activate(self.active));self.activation_pending=Some(self.active);
            self.paused=!(restore_playing&&a.resume);self.command(Command::Pause(self.paused));
            self.retry_after=std::time::Instant::now()+Duration::from_millis(250);
        }
    }
    pub fn tick(&mut self) {
        self.output.poll(false);
        // Output outages must not expire a healthy prepared decoder or advance
        // the queue while its sample clock is stopped.
        if !self.output.available(){self.preparing_since=std::time::Instant::now();return;}
        if let Some(a)=&self.audition {
            if self.control.clock.generations[a.deck].load(Ordering::Acquire)!=a.generation || self.control.clock.active.load(Ordering::Acquire)!=a.deck as u64 {
                if a.started.elapsed()>Duration::from_secs(5){self.end_audition(true);}
                return;
            }
            let source=self.control.clock.source_frames[a.deck].load(Ordering::Acquire) as f32/RATE as f32;
            let generation=self.decks[a.deck].generation.load(Ordering::Acquire);
            let failed=self.control.clock.failed[a.deck].load(Ordering::Acquire)==generation;
            let ended=source>=a.end||self.control.clock.drained[a.deck].load(Ordering::Acquire);
            if failed||ended||a.started.elapsed()>Duration::from_secs(36){self.end_audition(true);}
            return;
        }
        let mixing=self.control.clock.mixing.load(Ordering::Acquire);
        if mixing&&!self.was_mixing {super::dj_diagnostics::record("activated",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),format!("source {:.3}s",self.get_position()));}
        if !self.scheduled_reported && self.control.clock.schedule_id.load(Ordering::Acquire)==self.schedule_id && self.prepared.is_some_and(|(_,d)|self.control.clock.ready[d].load(Ordering::Acquire)) {self.scheduled_reported=true;super::dj_diagnostics::record("accepted",self.transition.as_ref().map(|p|p.outgoing),self.transition.as_ref().map(|p|p.incoming),format!("mixer acknowledged plan {}, deck ready",self.schedule_id));}
        self.was_mixing=mixing;
        self.account();
        if self.scheduled_reported&&self.transition_outcome.as_ref().is_none_or(|o|o.status=="preparing") {self.outcome("accepted","mixer schedule acknowledged and incoming audio ready");}
        if mixing&&self.transition_outcome.as_ref().is_some_and(|o|o.status=="accepted"){self.outcome("active","audible overlap started");}
        if self.paused {self.preparing_since=std::time::Instant::now();}
        if let Some((key,deck))=self.prepared {
            let generation=self.decks[deck].generation.load(Ordering::Acquire);
            let failed=self.control.clock.failed[deck].load(Ordering::Acquire)==generation;
            let expired=!self.paused&&!self.control.clock.ready[deck].load(Ordering::Acquire)&&self.preparing_since.elapsed()>Duration::from_secs(5);
            if failed||expired {
                self.outcome("failed",if failed {"incoming decoder failed"}else{"incoming readiness deadline expired"});
                let terminal=self.transition_outcome.clone();
                let attempts=self.prepare_attempts.entry(key).or_default();*attempts+=1;
                if failed||*attempts>=2 {self.failed_entries.insert(key);}
                self.cancel();self.transition_outcome=terminal;self.scheduled_reported=false;
            }
        }
        if self.control.clock.plan_failed.swap(false, Ordering::AcqRel) {
            self.outcome("failed","missed activation or incoming starvation; future ending blend required");
            if let Some((key,_))=self.prepared {let attempts=self.prepare_attempts.entry(key).or_default();*attempts+=1;if *attempts>=2{self.failed_entries.insert(key);}}
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
            self.outcome("promoted","incoming deck became authoritative");
            self.active = active;
            *self.decks[active].parameters.lock().unwrap_or_else(|e|e.into_inner())=(self.listening.speed,self.listening.transpose(),self.rate_revision);
            self.scheduled_reported=false;
            self.retry_after=std::time::Instant::now()+Duration::from_millis(500);
            if let Some((key, deck)) = self.prepared.take() {
                if deck == active {
                    self.current = self.queue.iter().position(|e| e.key == key);
                    self.transition = None;
                    self.plan_revision = self.plan_revision.wrapping_add(1);
                }
            }
        }
        if let Some(index)=self.current {let generation=self.decks[self.active].generation.load(Ordering::Acquire);if generation>0&&self.control.clock.failed[self.active].load(Ordering::Acquire)==generation {self.failed_entries.insert(self.queue[index].key);if self.repeat==LoopMode::One{self.pause();}}}
        if self.paused {
            return;
        }
        if self.prepared.is_none() && std::time::Instant::now()>=self.retry_after && self.automatic_prepare_due() {
            if let Some(index) = self.current {
                if let Some(next)=self.next_playable().filter(|_|self.repeat!=LoopMode::One) {
                    let _ = self.prepare(Transition {
                        request_id:0,
                        outgoing: self.queue[index].key,
                        incoming: self.queue[next].key,
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
                        if let Some(first)=(0..self.queue.len()).find(|&i|!self.failed_entries.contains(&self.queue[i].key)){self.start(first,0.,false);}else{self.pause();}
                    } else if !self.control.clock.room_active.load(Ordering::Relaxed) {
                        self.pause();
                    }
                }
            }
        }
    }
    pub fn set_queue(&mut self, tracks: Vec<Track>, index: usize) {
        self.queue = tracks.into_iter().map(|t| self.entry(t)).collect();
        self.failed_entries.clear();self.prepare_attempts.clear();
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
    pub fn refresh_paths(&mut self, tracks:&[Track]) {self.cancel_reason("music files organized");for entry in &mut self.queue{if let Some(track)=tracks.iter().find(|t|t.id==entry.track.id){entry.track.path=track.path.clone();self.failed_entries.remove(&entry.key);self.prepare_attempts.remove(&entry.key);}}}
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
    // A gapless prefetch is authoritative once accepted by the mixer. In DJmode
    // it must not win the race against the musical planner at track start.
    // Keep a bounded standalone fallback near EOF if no plan was delivered.
    fn automatic_prepare_due(&self)->bool {
        if !self.dj_mode || self.control.clock.drained[self.active].load(Ordering::Acquire) {return true;}
        let generation=self.decks[self.active].generation.load(Ordering::Acquire);
        if generation>0 && self.control.clock.failed[self.active].load(Ordering::Acquire)==generation {return true;}
        self.current.is_some_and(|index|self.queue[index].track.duration as f32-self.get_position()<=8.*self.listening.speed)
    }
    pub fn get_position(&self) -> f32 {
        self.control.clock.source_frames[self.active].load(Ordering::Acquire) as f32 / RATE as f32
    }
    pub fn is_playing(&self) -> bool {
        !self.paused && self.current.is_some() && self.output.available()
    }
    pub fn pause(&mut self) {
        self.paused = true;
        self.command(Command::Pause(true));
    }
    pub fn play_pause(&mut self) {
        if self.current.is_none() {
            return;
        }
        self.output.poll(true);
        self.paused = self.output.available() && !self.paused;
        self.command(Command::Pause(self.paused));
    }
    pub fn seek(&mut self, position: f32) {
        if let Some(i) = self.presentation().0 {
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
        if let Some(i) = self.presentation().0 {
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
        if self.presentation_position() > 3. {
            self.seek(0.);
        } else if let Some(i) = self.presentation().0 {
            self.start(i.saturating_sub(1), 0., false);
        }
    }
    pub fn set_dj_mode(&mut self,enabled:bool){self.dj_mode=enabled;self.cancel_reason("DJmode changed");}
    pub fn set_listening(&mut self,value:super::listening::Listening)->Result<()> {
        let value=value.normalize();let revision=self.listening_revision.wrapping_add(1);
        self.control.commands.push(Command::Equalizer(value.coefficients(),revision)).map_err(|_|anyhow::anyhow!("Audio controls are busy; retry"))?;
        let changed=value.speed!=self.listening.speed||value.pitch!=self.listening.pitch;
        self.listening=value;self.listening_revision=revision;
        if changed {self.rate_revision=revision;self.cancel_reason("listening speed or pitch changed");}
        for deck in &self.decks {let mut p=deck.parameters.lock().unwrap_or_else(|e|e.into_inner());*p=(if changed {self.listening.speed}else{p.0},self.listening.transpose(),self.rate_revision);}

        Ok(())
    }
    pub fn listening_applied(&self)->bool{self.control.clock.listening_revision.load(Ordering::Acquire)==self.listening_revision&&self.activation_pending.is_none()&&(self.paused||self.current.is_none()||self.control.clock.parameter_revision[self.active].load(Ordering::Acquire)==self.rate_revision)}
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
    pub fn settle_audible(&mut self)->Result<()> {
        let(index,deck)=self.presentation();
        if index!=self.current {
            self.control.commands.push(Command::Activate(deck)).map_err(|_|anyhow::anyhow!("Audio controls are busy; try again"))?;
            self.account();self.outcome("cancelled","explicit control kept the dominant deck");
            self.current=index;self.active=deck;self.activation_pending=Some(deck);self.prepared=None;self.transition=None;self.scheduled_reported=false;self.plan_revision=self.plan_revision.wrapping_add(1);
            *self.decks[deck].parameters.lock().unwrap_or_else(|e|e.into_inner())=(self.listening.speed,self.listening.transpose(),self.rate_revision);
        }
        Ok(())
    }
    fn presentation(&self)->(Option<usize>,usize){
        let deck=self.control.clock.dominant.load(Ordering::Acquire) as usize;
        if self.control.clock.mixing.load(Ordering::Acquire) {
            if let Some((key,incoming))=self.prepared.filter(|(_,incoming)|*incoming==deck){return (self.queue.iter().position(|e|e.key==key),incoming);}
        }
        (self.current,self.active)
    }
    fn presentation_position(&self)->f32{let (_,deck)=self.presentation();self.control.clock.source_frames[deck].load(Ordering::Acquire) as f32/RATE as f32}
    pub fn get_status(&self) -> Status {
        let (index,_)=self.presentation();
        Status {
            track: index.and_then(|i|self.queue.get(i)).map(|e|e.track.clone()),
            queue_index: index,
            is_playing: self.is_playing(),
            position: self.presentation_position(),
            volume: self.volume,
            shuffle: self.shuffle,
            loop_mode: self.repeat,
            backend: if self.output.available() {
                "MusicOS two-deck"
            } else {
                "unavailable"
            }
            .into(),
            last_error: self
                .errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone().or_else(||self.output.error.clone()),
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

pub fn next_request_id()->u64{static IDS:AtomicU64=AtomicU64::new(1_000_000);IDS.fetch_add(1,Ordering::Relaxed)}
#[cfg(test)]mod transition_repair_tests {
 use super::*;
 #[test]fn dominant_presentation_and_explicit_controls_keep_incoming_identity(){let mut p=pending();p.queue[1].track.id=2;p.control.clock.mixing.store(true,Ordering::Release);p.control.clock.dominant.store(1,Ordering::Release);p.control.clock.source_frames[1].store(3*RATE as u64,Ordering::Release);p.listens.insert(2,3.);let shown=p.get_status();assert_eq!(shown.queue_index,Some(1));assert_eq!(shown.track.unwrap().id,2);assert_eq!(shown.position,3.);assert_eq!(p.listened(),3.);assert_eq!(p.current,Some(0));p.settle_audible().unwrap();assert_eq!(p.current,Some(1));assert_eq!(p.active,1);assert!(p.transition.is_none());assert_eq!(p.entry_ids(),vec![1,2]);}
 fn pending()->DesktopPlayer {
  let track=Track{id:1,path:String::new(),title:String::new(),artist:String::new(),album:String::new(),track_number:1,duration:100,file_size:0,modified_at:0,indexed_at:0,cover_art_path:None,cover_art_updated_at:None,cover_art_source_modified_at:None,favorited:false,play_count:0,last_played:None,rating:0};let mut p=DesktopPlayer::new();p.queue=vec![Entry{key:1,track:track.clone()},Entry{key:2,track}];p.current=Some(0);p.paused=false;p.prepared=Some((2,1));p.transition=Some(Transition{request_id:42,outgoing:1,incoming:2,outgoing_cue:98.,incoming_cue:0.,duration:2.,tempo:1.,effect:String::new()});p.schedule_id=42;p.decks[1].generation.store(1,Ordering::Release);p
 }
 #[test]fn dj_fallback_must_not_preempt_the_smart_planner_at_track_start(){
  let mut p=pending();p.prepared=None;p.transition=None;p.schedule_id=0;p.dj_mode=true;p.tick();
  assert!(p.transition.is_none(),"an early safety blend must not lock out a later musical plan");
 }
 #[test]fn dj_fallback_still_prepares_near_the_ending_and_scales_with_user_speed(){
  let mut p=pending();p.dj_mode=true;
  p.control.clock.source_frames[0].store(91*RATE as u64,Ordering::Release);assert!(!p.automatic_prepare_due());
  p.control.clock.source_frames[0].store(92*RATE as u64,Ordering::Release);assert!(p.automatic_prepare_due());
  p.listening.speed=2.;p.control.clock.source_frames[0].store(84*RATE as u64,Ordering::Release);assert!(p.automatic_prepare_due());
  p.control.clock.source_frames[0].store(0,Ordering::Release);p.dj_mode=false;assert!(p.automatic_prepare_due());
  p.dj_mode=true;p.control.clock.drained[0].store(true,Ordering::Release);assert!(p.automatic_prepare_due());
 }
 #[test]fn organized_paths_preserve_duplicate_entries_pins_and_position(){let mut p=pending();p.paused=true;p.pins.insert(2);let ids=p.entry_ids();let position=p.get_position();let mut track=p.queue[0].track.clone();track.path="/new/Artist/Album/song.flac".into();p.refresh_paths(&[track]);assert_eq!(p.entry_ids(),ids);assert!(p.pins.contains(&2));assert_eq!(p.current,Some(0));assert_eq!(p.get_position(),position);assert!(p.queue.iter().all(|e|e.track.path=="/new/Artist/Album/song.flac"));assert!(p.transition.is_none());}
 #[test]fn accepted_plan_cannot_be_replaced_by_routine_preparation(){let mut p=pending();p.scheduled_reported=true;let before=p.decks[1].generation.load(Ordering::Acquire);let mut next=p.transition.clone().unwrap();next.request_id=43;assert!(p.prepare(next).is_err());assert_eq!(p.transition.as_ref().unwrap().request_id,42);assert_eq!(p.decks[1].generation.load(Ordering::Acquire),before);assert_eq!(p.transition_outcome.as_ref().unwrap().status,"rejected");}
 #[test]fn saturated_commands_do_not_publish_a_false_navigation(){let mut p=pending();let (commands,_held_consumer)=rtrb::RingBuffer::new(64);p.control.commands=commands;let held=p.control.commands.slots();for _ in 0..held {let _=p.control.commands.push(Command::Pause(true));}p.start(1,0.,false);assert_eq!(p.current,Some(0));}
 #[test]fn acknowledged_schedule_is_protected_before_controller_poll(){let mut p=pending();p.control.clock.schedule_id.store(42,Ordering::Release);p.control.clock.ready[1].store(true,Ordering::Release);let mut next=p.transition.clone().unwrap();next.request_id=99;assert!(p.prepare(next).is_err());assert_eq!(p.transition.as_ref().unwrap().request_id,42);}
 #[test]fn paused_preparation_keeps_its_deadline_frozen(){let mut p=pending();p.paused=true;p.preparing_since=std::time::Instant::now()-Duration::from_secs(6);p.tick();assert!(p.prepared.is_some());assert!(p.failed_entries.is_empty());}
 #[test]fn readiness_expiry_is_bounded_and_never_remains_pending(){let mut p=pending();for attempt in 1..=2{p.prepared=Some((2,1));p.transition=Some(Transition{request_id:attempt,outgoing:1,incoming:2,outgoing_cue:98.,incoming_cue:0.,duration:2.,tempo:1.,effect:String::new()});p.preparing_since=std::time::Instant::now()-Duration::from_secs(6);p.tick();assert!(p.prepared.is_none());assert_eq!(p.transition_outcome.as_ref().unwrap().status,"failed");}assert!(p.failed_entries.contains(&2));assert_eq!(p.queue.len(),2);}
}

#[cfg(all(test,target_os="macos"))]
mod output_recovery_tests {
    use super::*;
    fn wait(player:&mut DesktopPlayer,condition:impl Fn(&DesktopPlayer)->bool){
        let until=std::time::Instant::now()+Duration::from_secs(4);
        while !condition(player){assert!(std::time::Instant::now()<until,"Audio recovery timed out: {:?}",player.get_status().last_error);thread::sleep(Duration::from_millis(20));player.tick();}
    }
    #[test]fn reconnect_retains_playhead_duplicate_queue_controls_and_paused_intent(){
        let temp=tempfile::tempdir().unwrap();let path=temp.path().join("silence.wav");let length=48000_u32*2*2*8;
        let mut wav=Vec::new();wav.extend(b"RIFF");wav.extend((length+36).to_le_bytes());wav.extend(b"WAVEfmt ");wav.extend(16_u32.to_le_bytes());wav.extend(1_u16.to_le_bytes());wav.extend(2_u16.to_le_bytes());wav.extend(48000_u32.to_le_bytes());wav.extend(192000_u32.to_le_bytes());wav.extend(4_u16.to_le_bytes());wav.extend(16_u16.to_le_bytes());wav.extend(b"data");wav.extend(length.to_le_bytes());wav.resize(44+length as usize,0);std::fs::write(&path,wav).unwrap();
        let track=Track{id:1,path:path.to_string_lossy().into(),title:"Output recovery".into(),artist:"Fixture".into(),album:"Fixture".into(),track_number:1,duration:8,file_size:length as i64,modified_at:0,indexed_at:0,cover_art_path:None,cover_art_updated_at:None,cover_art_source_modified_at:None,favorited:false,play_count:0,last_played:None,rating:0};
        let mut player=DesktopPlayer::new();player.set_volume(0.);player.restore_queue_paused(vec![track.clone(),track],1,2.);player.set_shuffle(true);player.pins.insert(player.entry_ids()[0]);
        let identities=player.entry_ids();let pins=player.pins.clone();
        wait(&mut player,|p|p.get_position()>=2.);let position=player.get_position();
        for _ in 0..3{player.output.simulate_disconnect();player.tick();assert!(!player.is_playing());assert_eq!(player.get_position(),position);assert_eq!(player.entry_ids(),identities);}
        player.play_pause();wait(&mut player,|p|p.get_position()>position+0.1);assert!(player.is_playing());assert_eq!(player.current,Some(1));assert_eq!(player.pins,pins);assert!(player.shuffle);assert_eq!(player.volume,0.);
        let before=player.get_position();player.output.simulate_disconnect();player.tick();wait(&mut player,|p|p.get_position()>before+0.1);assert!(player.is_playing());assert_eq!(player.entry_ids(),identities);
        player.pause();thread::sleep(Duration::from_millis(50));player.tick();let paused=player.get_position();player.output.simulate_disconnect();player.tick();thread::sleep(Duration::from_millis(50));player.tick();assert!(!player.is_playing());assert_eq!(player.get_position(),paused);
    }
}
