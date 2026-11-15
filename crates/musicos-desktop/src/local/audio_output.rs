//! Reconnect only the output, retaining the single owned mixer and its clocks.
use super::dj_audio::Mixer;
use anyhow::{Context, Result};
use rodio::{cpal::{self, traits::HostTrait}, OutputStream, Sink, Source};
#[cfg(not(target_os="macos"))]
use rodio::DeviceTrait;
use std::{sync::{mpsc, Arc, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Route { identity:String, rate:u64 }
fn current_route()->Result<Route> {
    #[cfg(target_os="macos")]
    {
        extern "C" {fn mos_audio_output_route(device:*mut u32,rate:*mut f64)->i32;}
        let (mut device,mut rate)=(0,0.);
        anyhow::ensure!(unsafe{mos_audio_output_route(&mut device,&mut rate)}==0,"Waiting for the selected audio output");
        Ok(Route{identity:device.to_string(),rate:rate as u64})
    }
    #[cfg(not(target_os="macos"))]
    {
        let device=cpal::default_host().default_output_device().context("Waiting for an audio output")?;
        Ok(Route{identity:device.name()?,rate:device.default_output_config()?.sample_rate().0 as u64})
    }
}

// The stream owns this source exclusively. Dropping it returns the entire mixer
// before a new stream can acquire it. No locks/allocations in per-sample pulls.
struct OutputSource {
    mixer:Option<Mixer>,
    returned:mpsc::SyncSender<Mixer>,
    heartbeat:Arc<AtomicU64>,
    samples:u16,
}
impl Iterator for OutputSource {
    type Item=f32;
    fn next(&mut self)->Option<f32> {
        self.samples=self.samples.wrapping_add(1);
        if self.samples%2048==0 {self.heartbeat.fetch_add(1,Ordering::Relaxed);}
        self.mixer.as_mut()?.next()
    }
}
impl Source for OutputSource {
    fn current_frame_len(&self)->Option<usize>{None}
    fn channels(&self)->u16{2}
    fn sample_rate(&self)->u32{super::dj_audio::RATE}
    fn total_duration(&self)->Option<Duration>{None}
}
impl Drop for OutputSource {
    fn drop(&mut self){if let Some(mixer)=self.mixer.take(){let _=self.returned.try_send(mixer);}}
}

struct Health {heartbeat:u64,progress_at:Instant,retry_at:Instant}
impl Health {
    fn new(now:Instant)->Self{Self{heartbeat:0,progress_at:now,retry_at:now}}
    fn stalled(&mut self,now:Instant,heartbeat:u64)->bool {
        if heartbeat!=self.heartbeat {self.heartbeat=heartbeat;self.progress_at=now;}
        now.saturating_duration_since(self.progress_at)>=Duration::from_secs(2)
    }
    fn opened(&mut self,now:Instant,heartbeat:u64){self.heartbeat=heartbeat;self.progress_at=now;}
    fn failed(&mut self,now:Instant){self.retry_at=now+Duration::from_secs(1);}
}

pub(super) struct AudioOutput {
    // Drop sink before stream, so queued as well as active sources are released.
    sink:Option<Sink>,
    stream:Option<OutputStream>,
    parked:Option<Mixer>,
    returned:mpsc::Receiver<Mixer>,
    return_to:mpsc::SyncSender<Mixer>,
    heartbeat:Arc<AtomicU64>,
    route:Option<Route>,
    checked_at:Instant,
    health:Health,
    pub error:Option<String>,
}
impl AudioOutput {
    pub fn new(mixer:Mixer)->Self {
        let now=Instant::now();let (return_to,returned)=mpsc::sync_channel(1);
        Self{sink:None,stream:None,parked:Some(mixer),returned,return_to,heartbeat:Arc::new(AtomicU64::new(0)),route:None,checked_at:now,health:Health::new(now),error:None}
    }
    pub fn available(&self)->bool{self.sink.is_some()}
    pub fn service_parked(&mut self) {
        if self.sink.is_some() { return; }
        if self.parked.is_none() { self.parked=self.returned.try_recv().ok(); }
        if let Some(mixer)=self.parked.as_mut() { mixer.service_parked(); }
    }
    pub fn suspend(&mut self) {
        if self.available() { self.disconnect(); }
        self.error=None;
        self.service_parked();
    }

    pub fn poll(&mut self,force:bool){
        let now=Instant::now();
        if !force&&now.duration_since(self.checked_at)<Duration::from_millis(250){return;}
        self.checked_at=now;self.inspect(now,current_route());
    }
    #[cfg(test)]
    pub(super) fn simulate_disconnect(&mut self){self.disconnect();}
    fn disconnect(&mut self){
        drop(self.sink.take());drop(self.stream.take());
        if self.parked.is_none(){self.parked=self.returned.try_recv().ok();}
    }
    fn inspect(&mut self,now:Instant,route:Result<Route>){
        let route=match route {Ok(route)=>route,Err(error)=>{
            self.disconnect();self.route=None;self.error=Some(format!("Audio output reconnecting: {error}"));return;
        }};
        let changed=self.route.as_ref()!=Some(&route);
        let stalled=self.available()&&self.health.stalled(now,self.heartbeat.load(Ordering::Relaxed));
        if changed||stalled {self.disconnect();self.route=Some(route);self.health.retry_at=now;}
        if self.available()||now<self.health.retry_at{return;}
        if self.parked.is_none(){self.parked=self.returned.try_recv().ok();}
        if self.parked.is_none(){return;} // A previous callback is still releasing its owner.
        let opened=(||->Result<_>{
            // Never fall back to unrelated speakers while headphones negotiate.
            let device=cpal::default_host().default_output_device().context("No default audio output")?;
            let (stream,handle)=OutputStream::try_from_device(&device)?;
            let sink=Sink::try_new(&handle)?;Ok((stream,sink))
        })();
        match opened {
            Ok((stream,sink))=>{
                sink.append(OutputSource{mixer:self.parked.take(),returned:self.return_to.clone(),heartbeat:self.heartbeat.clone(),samples:0});
                self.stream=Some(stream);self.sink=Some(sink);self.health.opened(now,self.heartbeat.load(Ordering::Relaxed));self.error=None;
                super::dj_diagnostics::record("output_connected",None,None,"Connected selected system output; retained mixer, decoder and queue");
            }
            Err(error)=>{self.error=Some(format!("Audio output reconnecting: {error}"));self.health.failed(now);}
        }
    }
}
impl Drop for AudioOutput {fn drop(&mut self){self.disconnect();}}

#[cfg(test)] mod tests {
    use super::*;
    use super::super::dj_audio::{self,Command,Frame};
    #[test]fn watchdog_detects_frozen_callbacks_even_while_paused_and_bounds_retries(){
        let now=Instant::now();let mut health=Health::new(now);
        assert!(!health.stalled(now+Duration::from_secs(1),0));
        assert!(health.stalled(now+Duration::from_secs(2),0));
        assert!(!health.stalled(now+Duration::from_secs(2),1));
        health.failed(now);assert!(health.retry_at>now+Duration::from_millis(999));
        health.opened(now,8);assert!(!health.stalled(now+Duration::from_millis(1900),8));
    }
    #[test]fn moving_source_preserves_pause_position_pending_commands_and_duplicate_decks(){
        let (mut control,mixer)=dj_audio::create(32);let (tx,rx)=mpsc::sync_channel(1);
        let beat=Arc::new(AtomicU64::new(0));let mut source=OutputSource{mixer:Some(mixer),returned:tx.clone(),heartbeat:beat.clone(),samples:0};
        control.commands.push(Command::Reset{deck:0,generation:1,source_frame:48000.}).unwrap();
        control.commands.push(Command::Pause(true)).unwrap();source.next();source.next();
        let position=control.clock.source_frames[0].load(Ordering::Acquire);drop(source);
        let mixer=rx.try_recv().expect("Mixer is returned by value");
        let mut source=OutputSource{mixer:Some(mixer),returned:tx,heartbeat:beat.clone(),samples:0};
        for _ in 0..4096{source.next();}assert_eq!(control.clock.source_frames[0].load(Ordering::Acquire),position);assert!(beat.load(Ordering::Relaxed)>0);
        control.decks[0].push(Frame{samples:[0.1,0.1],source_frame:48001.,generation:1,listening_revision:0}).unwrap();
        control.commands.push(Command::Pause(false)).unwrap();source.next();source.next();
        assert_eq!(control.clock.source_frames[0].load(Ordering::Acquire),48001);
    }
    #[test]fn unavailable_route_keeps_mixer_and_does_not_turn_resume_into_a_restart(){
        let (_,mixer)=dj_audio::create(1);let now=Instant::now();let (tx,rx)=mpsc::sync_channel(1);
        let mut output=AudioOutput{sink:None,stream:None,parked:Some(mixer),returned:rx,return_to:tx,heartbeat:Arc::new(AtomicU64::new(0)),route:None,checked_at:now,health:Health::new(now),error:None};
        for n in 0..10{output.inspect(now+Duration::from_secs(n),Err(anyhow::anyhow!("headphones connecting")));assert!(!output.available());assert!(output.parked.is_some());}
    }
    #[cfg(target_os="macos")]
    #[test]fn native_output_can_reclaim_and_reopen_the_same_muted_mixer(){
        let (mut control,mixer)=dj_audio::create(1);control.commands.push(Command::Volume(0.)).unwrap();
        let mut output=AudioOutput::new(mixer);assert!(!output.available());output.poll(true);assert!(output.available(),"{:?}",output.error);
        std::thread::sleep(Duration::from_millis(100));output.disconnect();
        assert!(output.parked.is_some(),"Output callback must relinquish the mixer on disposal");
        output.health.retry_at=Instant::now();output.poll(true);assert!(output.available(),"{:?}",output.error);
        std::thread::sleep(Duration::from_millis(100));assert!(output.heartbeat.load(Ordering::Relaxed)>0);
    }
}
