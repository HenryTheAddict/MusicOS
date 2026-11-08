//! Disposable tones and database: exercise a delayed controller against the
//! native decoder/mixer without seeking, changing the user's queue, or sound.
use anyhow::{ensure,Result};
use musicos_desktop::{local::{LocalPlaybackService,LocalPlaybackCommand,deck_playback::Transition,dj_diagnostics},models::PlaybackPreferences};
use musicos_init::database::Db;
use std::{io::Write,time::{Duration,Instant},path::Path};
fn tone(path:&Path,hz:f32)->Result<()> {
 let frames=14*48000_u32;let bytes=frames*4;let mut file=std::io::BufWriter::new(std::fs::File::create(path)?);
 file.write_all(b"RIFF")?;file.write_all(&(bytes+36).to_le_bytes())?;file.write_all(b"WAVEfmt ")?;file.write_all(&16_u32.to_le_bytes())?;file.write_all(&1_u16.to_le_bytes())?;file.write_all(&2_u16.to_le_bytes())?;file.write_all(&48000_u32.to_le_bytes())?;file.write_all(&192000_u32.to_le_bytes())?;file.write_all(&4_u16.to_le_bytes())?;file.write_all(&16_u16.to_le_bytes())?;file.write_all(b"data")?;file.write_all(&bytes.to_le_bytes())?;
 for i in 0..frames {let sample=((i as f32*std::f32::consts::TAU*hz/48000.).sin()*0.15*i16::MAX as f32) as i16;file.write_all(&sample.to_le_bytes())?;file.write_all(&sample.to_le_bytes())?;}file.flush()?;Ok(())
}
fn main()->Result<()> {
 let temp=tempfile::tempdir()?;let path=temp.path().join("queue.sqlite");let db=Db::open(path.to_str().unwrap())?;let mut ids=Vec::new();
 for (i,hz) in [220.,440.].into_iter().enumerate(){let file=temp.path().join(format!("tone-{i}.wav"));tone(&file,hz)?;ids.push(db.insert_track(file.to_str().unwrap(),&format!("Fixture {i}"),"QA","Disposable",14,std::fs::metadata(&file)?.len() as i64,i as i64)?);}drop(db);
 let service=LocalPlaybackService::start_with_preferences(path,PlaybackPreferences{volume:0.,..Default::default()})?;
 service.send(LocalPlaybackCommand::SetDjMode(true))?;service.send(LocalPlaybackCommand::PlayTracks{ids,start_index:0})?;
 let started=Instant::now();let mut sent=false;let mut accepted=false;let mut active=false;let mut queue_ids=Vec::new();let request_id=9_123_456;
 loop {
  let state=service.snapshot();
  if !sent&&state.queue_entry_ids.len()==2&&state.position>=0.3 {
   ensure!(state.backend=="MusicOS two-deck","Native audio unavailable");
   ensure!(state.dj_transition.is_none(),"A safety blend preempted the controller: {:?}",state.dj_transition);
   queue_ids=state.queue_entry_ids.clone();service.send(LocalPlaybackCommand::PlanTransition(Transition{request_id,outgoing:queue_ids[0],incoming:queue_ids[1],outgoing_cue:6.,incoming_cue:0.,duration:3.,tempo:1.,effect:"clean_blend".into()}))?;sent=true;
  }
  accepted|=state.dj_transition.as_ref().is_some_and(|plan|plan.plan_id==request_id&&plan.status=="scheduled");
  active|=state.dj_transition.as_ref().is_some_and(|plan|plan.plan_id==request_id&&plan.active);
  if state.queue_index==Some(1) {
   ensure!(accepted&&active,"No accepted/active controller overlap");ensure!(state.queue_entry_ids==queue_ids,"Queue identities changed");ensure!((state.position-1.5).abs()<0.4,"Dominant-deck handoff must publish the real midpoint source position");service.send(LocalPlaybackCommand::Pause)?;
   println!("{}",serde_json::to_string_pretty(&serde_json::json!({"fixtures_only":true,"volume":0,"seeking":false,"accepted":accepted,"active_overlap":active,"promoted":true,"incoming_position":state.position,"elapsed_seconds":started.elapsed().as_secs_f32(),"audible_listening_verified":false,"diagnostics":dj_diagnostics::snapshot()}))?);break;
  }
  ensure!(started.elapsed()<Duration::from_secs(20),"Controller transition failed: {:?}",state.transition_outcome);std::thread::sleep(Duration::from_millis(10));
 }
 Ok(())
}
