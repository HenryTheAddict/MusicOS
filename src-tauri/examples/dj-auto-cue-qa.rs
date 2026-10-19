//! Real recordings, no seeking, muted native output and an ephemeral queue.
use anyhow::{Result,Context,ensure};
use musicos_tauri::{analysis::Features,dj_plan};
use musicos_desktop::{local::{LocalPlaybackService,LocalPlaybackCommand,deck_playback::{Transition,next_request_id},dj_diagnostics},models::PlaybackPreferences};
use musicos_init::database::Db;
use std::time::{Duration,Instant};
fn main()->Result<()> {
 let path=std::env::args().nth(1).context("Pass source library")?;let conn=rusqlite::Connection::open_with_flags(path,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
 let rows=conn.prepare("select t.id,t.path,t.title,t.artist,t.album,t.duration,d.result from tracks t join desktop_dsp_v1 d on t.id=d.track_id where t.deleted_at is null order by t.id")?.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i32>(5)?,r.get::<_,String>(6)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
 let features=rows.iter().map(|r|serde_json::from_str::<Features>(&r.6)).collect::<Result<Vec<_>,_>>()?;let config=dj_plan::Config{enabled:true,..Default::default()};let mut best=None;
 let track=|i:usize|{let f=&features[i];dj_plan::Track{key_index:f.key_index,duration:f.structure.as_ref().map(|s|s.duration).unwrap_or(0.),bpm:f.bpm,structure:f.structure.as_ref()}};
 for a in 0..rows.len(){for b in 0..rows.len(){if a==b {continue;}if let Some(p)=dj_plan::plan(track(a),track(b),&config,false){if p.confidence>=0.5&&best.as_ref().is_none_or(|(_,_,old):&(usize,usize,dj_plan::Plan)|p.outgoing_cue<old.outgoing_cue){best=Some((a,b,p));}}}}
 let (a,b,plan)=best.context("No measured musical candidate")?;let temp=tempfile::tempdir()?;let db_path=temp.path().join("playback.sqlite");let db=Db::open(db_path.to_str().unwrap())?;let mut ids=Vec::new();for index in [a,b]{let t=&rows[index];ids.push(db.insert_track(&t.1,&t.2,&t.3,&t.4,t.5,std::fs::metadata(&t.1)?.len() as i64,index as i64)?);}drop(db);
 let service=LocalPlaybackService::start_with_preferences(db_path,PlaybackPreferences{volume:0.,..Default::default()})?;service.send(LocalPlaybackCommand::SetDjMode(true))?;service.send(LocalPlaybackCommand::PlayTracks{ids,start_index:0})?;
 let started=Instant::now();let mut sent=false;let mut accepted=false;let mut active=false;
 loop{let s=service.snapshot();if !sent&&s.queue_entry_ids.len()==2{ensure!(s.backend=="MusicOS two-deck","Native output unavailable");service.send(LocalPlaybackCommand::PlanTransition(Transition{request_id:next_request_id(),outgoing:s.queue_entry_ids[0],incoming:s.queue_entry_ids[1],outgoing_cue:plan.outgoing_cue,incoming_cue:plan.incoming_cue,duration:plan.duration,tempo:plan.incoming_tempo,effect:serde_json::to_value(&plan.effect)?.as_str().unwrap().into()}))?;sent=true;}
  accepted|=s.dj_transition.as_ref().is_some_and(|t|t.status=="scheduled");active|=s.dj_transition.as_ref().is_some_and(|t|t.active);
  if s.queue_index==Some(1){ensure!(accepted&&active,"Missing accepted schedule/active overlap");service.send(LocalPlaybackCommand::Pause)?;println!("{}",serde_json::to_string_pretty(&serde_json::json!({"outgoing":rows[a].2,"incoming":rows[b].2,"plan":plan,"seeking":false,"accepted":accepted,"active":active,"promoted":true,"elapsed_seconds":started.elapsed().as_secs_f32(),"volume":0,"audible_acceptance":false,"diagnostics":dj_diagnostics::snapshot()}))?);break;}
  ensure!(started.elapsed().as_secs_f32()<plan.outgoing_cue+plan.duration+25.,"Transition did not promote: {s:?}");std::thread::sleep(Duration::from_millis(20));
 }Ok(())
}
