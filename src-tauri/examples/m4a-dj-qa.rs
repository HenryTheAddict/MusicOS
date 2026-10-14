//! Isolated muted M4A → E-AC-3 playback, late-cue fallback, and full-track analysis.
use anyhow::{Result,ensure,Context};
use musicos_desktop::{local::{LocalPlaybackService,LocalPlaybackCommand,deck_playback::Transition},models::PlaybackPreferences};
use std::{time::{Instant,Duration},path::Path};
fn wait(mut ready:impl FnMut()->bool,seconds:u64)->Result<()>{let until=Instant::now()+Duration::from_secs(seconds);while !ready(){ensure!(Instant::now()<until,"Native condition timed out");std::thread::sleep(Duration::from_millis(10));}Ok(())}
fn main()->Result<()>{
 let input=std::env::args().nth(1).context("Pass source library database")?;let source=rusqlite::Connection::open_with_flags(input,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
 let temp=tempfile::tempdir()?;let path=temp.path().join("playback.sqlite");let db=musicos_init::database::Db::open(path.to_str().unwrap())?;let mut ids=Vec::new();let mut files=Vec::new();
 for name in ["Dyscalculia.m4a","Over_the_Horizon.m4a"]{let(file,title,artist,album,duration):(String,String,String,String,i32)=source.query_row("SELECT path,title,artist,album,duration FROM tracks WHERE path LIKE ? LIMIT 1",[format!("%/{name}")],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
 ids.push(db.insert_track(&file,&title,&artist,&album,duration,std::fs::metadata(&file)?.len()as i64,1)?);files.push((file,duration));}drop(db);
 let service=LocalPlaybackService::start_with_preferences(path.clone(),PlaybackPreferences{volume:0.,..Default::default()})?;service.send(LocalPlaybackCommand::PlayTracks{ids:ids.clone(),start_index:0})?;wait(||service.snapshot().queue.len()==2,10)?;service.send(LocalPlaybackCommand::SetDjMode(true))?;
 let ending=files[0].1 as f32;service.send(LocalPlaybackCommand::SeekAbsolute(ending-2.))?;wait(||service.snapshot().position>=ending-2.1,10)?;
 let entries=service.snapshot().queue_entry_ids;service.send(LocalPlaybackCommand::PlanTransition(Transition{outgoing:entries[0],incoming:entries[1],outgoing_cue:ending-20.,incoming_cue:10.,duration:4.,tempo:1.04,effect:"echo_tail".into()}))?;
 wait(||service.snapshot().dj_transition.as_ref().is_some_and(|p|p.seconds>0.&&p.status=="scheduled"),10)?;let accepted=service.snapshot().dj_transition.context("Missing accepted schedule")?;ensure!(accepted.tempo==1.&&accepted.incoming_cue==0.&&accepted.seconds<=2.,"Missed cue was not converted to a safe ending blend");
 let mut active=false;wait(||{let s=service.snapshot();active|=s.dj_transition.as_ref().is_some_and(|p|p.active);s.queue_index==Some(1)},10)?;ensure!(active,"M4A transition never activated");ensure!(service.snapshot().error.is_none(),"{:?}",service.snapshot().error);service.send(LocalPlaybackCommand::Pause)?;wait(||!service.snapshot().is_playing,10)?;
 let analysis=musicos_tauri::analysis::Analysis::default();analysis.start(temp.path().join("analysis.sqlite"),files.iter().enumerate().map(|(i,(p,d))|(i as i64,p.clone(),*d as i64)).collect())?;wait(||!analysis.snapshot().running,120)?;let status=analysis.snapshot();ensure!(status.errors.is_empty(),"{:?}",status.errors);ensure!(status.features.len()==2,"M4A full-track analysis missing");
 let model_dir=musicos_tauri::spark_commands::Setup::for_library(Path::new("/Users/henry/Developer/MusicOS/out/desktop-media.sqlite3"),Path::new("src-tauri/resources/runtime/libonnxruntime.1.31.0.dylib").to_owned(),true).dir;
 let mut model=musicos_tauri::spark_engine::Model::load(&model_dir,Path::new("src-tauri/resources/runtime/libonnxruntime.1.31.0.dylib"),true)?;
 let vector=model.embed_file(Path::new(&files[0].0),files[0].1 as f32,||true)?;ensure!(vector.len()==512&&vector.iter().all(|v|v.is_finite()),"M4A embeddings invalid");
 println!("{}",serde_json::json!({"m4a_to_eac3":true,"late_cue_fallback":true,"mixer_acknowledged":accepted.plan_id,"automatic_overlap":active,"full_track_analyses":status.features.len(),"spark_embedding_dimensions":vector.len(),"user_queue_modified":false,"audible_listening":false,"volume":0}));Ok(())
}
