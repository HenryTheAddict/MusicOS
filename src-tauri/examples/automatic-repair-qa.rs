//! No seeking: real recordings play from their beginning through two promotions.
//! Muted native output and a disposable database preserve the user's queue.
use anyhow::{Result,ensure,Context};
use musicos_desktop::{local::{LocalPlaybackService,LocalPlaybackCommand,dj_diagnostics},models::PlaybackPreferences};
use musicos_init::database::Db;
use std::time::{Duration,Instant};
fn main()->Result<()> {
 let args=std::env::args().collect::<Vec<_>>();let source=rusqlite::Connection::open_with_flags(args.get(1).context("Pass source library")?,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
 let tracks=source.prepare("select path,title,artist,album,duration from tracks where duration between 20 and 40 and deleted_at is null order by duration limit 3")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i32>(4)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;ensure!(tracks.len()==3,"Need three real short recordings");
 let temp=tempfile::tempdir()?;let path=temp.path().join("test.sqlite");let db=Db::open(path.to_str().unwrap())?;let ids=tracks.iter().enumerate().map(|(i,t)|db.insert_track(&t.0,&t.1,&t.2,&t.3,t.4,std::fs::metadata(&t.0)?.len() as i64,i as i64)).collect::<Result<Vec<_>,_>>()?;drop(db);
 let service=LocalPlaybackService::start_with_preferences(path,PlaybackPreferences{volume:0.,..Default::default()})?;service.send(LocalPlaybackCommand::SetDjMode(true))?;service.send(LocalPlaybackCommand::PlayTracks{ids:ids.clone(),start_index:0})?;
 let start=Instant::now();let mut keys=vec![];let mut active=0;let mut previous=false;let mut seen_second=false;
 loop {let s=service.snapshot();if keys.is_empty()&&!s.queue_entry_ids.is_empty(){keys=s.queue_entry_ids.clone();}let mixing=s.dj_transition.as_ref().is_some_and(|t|t.active);if mixing&&!previous{active+=1;}previous=mixing;seen_second|=s.queue_index==Some(1);if s.queue_index==Some(2){ensure!(seen_second&&active>=2,"Missing automatic overlaps: {active}");ensure!(s.queue_entry_ids==keys&&s.queue.iter().map(|t|t.id).collect::<Vec<_>>()==ids,"Queue changed");service.send(LocalPlaybackCommand::Pause)?;break;}ensure!(start.elapsed()<Duration::from_secs(100),"Automatic playback did not promote: {s:?}");std::thread::sleep(Duration::from_millis(20));}
 let report=serde_json::json!({"tracks":tracks.iter().map(|t|&t.1).collect::<Vec<_>>(),"automatic_overlaps":active,"elapsed_seconds":start.elapsed().as_secs_f32(),"seeking":false,"test_volume":0,"user_queue_modified":false,"listening_acceptance":false,"diagnostics":dj_diagnostics::snapshot()});println!("{report}");if let Some(out)=args.get(2){std::fs::write(out,serde_json::to_vec_pretty(&report)?)?;}Ok(())
}
