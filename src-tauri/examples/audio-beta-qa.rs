//! Isolated native acceptance probe. Never mutates the user's library/queue.
use anyhow::{ensure,Result};
use musicos_tauri::{spark_install,spark_engine::{Model,Generation,generate,Candidate},embeddings,dj_plan};
use musicos_desktop::{local::{LocalPlaybackService,LocalPlaybackCommand,LocalLibraryRepository,LibraryRepository},models::PlaybackPreferences};
use musicos_init::database::Db;
use serde::Deserialize;
use std::{path::Path,time::{Duration,Instant}};
#[derive(Deserialize)]struct Input{id:i64,path:String,title:String,artist:String,album:String,duration:f32,#[serde(default)]favorite:bool}
fn main()->Result<()>{
 let args=std::env::args().collect::<Vec<_>>();ensure!(args.len()==5,"candidate-directory bundled-runtime tracks-json report-json");
 let temp=tempfile::tempdir()?;let model_dir=temp.path().join("audio-model");
 spark_install::start(args[1].clone().into(),model_dir.clone())?;
 while spark_install::status().running{std::thread::sleep(Duration::from_millis(20));}
 ensure!(spark_install::status().error.is_none(),"{:?}",spark_install::status().error);
 ensure!(Model::load(&model_dir,Path::new(&args[2]),false).is_err(),"Beta consent bypassed");
 let mut model=Model::load(&model_dir,Path::new(&args[2]),true)?;
 let mut inputs:Vec<Input>=serde_json::from_slice(&std::fs::read(&args[3])?)?;
 let cache=temp.path().join("cache.sqlite");let repo=LocalLibraryRepository::new(cache.clone(),temp.path().join("media"));repo.ensure_ready()?;
 let db=Db::open(cache.to_str().unwrap())?;for t in inputs.iter_mut().take(8){let source=db.add_library_source("reference",Path::new(&t.path).parent().unwrap().to_str().unwrap(),"Read-only QA recording")?;t.id=db.insert_track(&t.path,&t.title,&t.artist,&t.album,t.duration as i32,std::fs::metadata(&t.path)?.len() as i64,1)?;db.assign_track_source(t.id,source.id)?;}drop(db);
 let start=Instant::now();let mut candidates=Vec::new();
 for t in inputs.iter().take(8){let vector=embeddings::analyze(&cache,t.id,Path::new(&t.path),t.duration,&mut model,||true)?;candidates.push(Candidate{id:t.id,artist:t.artist.clone(),album:t.album.clone(),favorite:t.favorite,embedding:vector});}
 let inference_seconds=start.elapsed().as_secs_f64();let request=Generation{seed:Some(candidates[0].id),mood:Some("Chilled".into()),..Default::default()};
 let preview=generate(&candidates,inputs.len(),request.clone(),&model.moods)?;ensure!(preview.track_ids==generate(&candidates,inputs.len(),request,&model.moods)?.track_ids,"Nondeterministic ranking");
 let feedback=generate(&candidates,inputs.len(),Generation{seed:Some(candidates[0].id),less_like:vec![candidates[1].id],..Default::default()},&model.moods)?;ensure!(!feedback.track_ids.contains(&candidates[1].id),"Explicit feedback ignored");
 let setup=musicos_tauri::spark_commands::Setup{dir:model_dir.clone(),runtime:args[2].clone().into(),resources:Path::new(&args[2]).parent().and_then(Path::parent).unwrap_or(Path::new(".")).to_owned(),evaluation:true};
 use musicos_tauri::spark_commands::{execute,Request};
 let chosen=execute(&repo,&setup,Request::Choose)?.chosen_id.unwrap();ensure!(candidates.iter().any(|c|c.id==chosen),"Choose For Me selected an unanalyzed song");
 let generation=Generation{seed:Some(chosen),mood:Some("Dreamy".into()),..Default::default()};
 let generated=execute(&repo,&setup,Request::Preview{generation:generation.clone()})?;let generated_artwork=generated.generated_artwork.unwrap();musicos_tauri::spark_artwork::Art::parse(&generated_artwork)?;let cached_preview=generated.preview.unwrap();
 execute(&repo,&setup,Request::Save{generation,preview:Some(cached_preview.clone()),name:"Native audio QA".into(),artwork_choice:generated_artwork.clone()})?;
 let saved=musicos_tauri::audio_collections::list(&cache)?.remove(0);ensure!(saved.track_ids==cached_preview.track_ids,"Save changed preview sequence");ensure!(saved.artwork_choice==generated_artwork,"Generated artwork was not saved");
 let reopened=LocalLibraryRepository::new(cache.clone(),temp.path().join("media"));ensure!(musicos_tauri::audio_collections::list(reopened.db_path())?[0].track_ids==saved.track_ids,"Restart lost sequence");
 execute(&reopened,&setup,Request::Refresh{id:saved.id.clone()})?;
 execute(&reopened,&setup,Request::AsPlaylist{id:saved.id.clone(),name:"Manual audio QA".into()})?;
 let manual=reopened.snapshot(&Default::default())?.playlists.into_iter().find(|p|p.name=="Manual audio QA").unwrap();ensure!(reopened.playlist_tracks(manual.id)?.into_iter().map(|t|t.id).collect::<Vec<_>>()==saved.track_ids,"Manual conversion changed ordering");
 // Exercise the actual two-deck desktop output with real same-album recordings.
 // Master volume is zero so this probe never interrupts the user's listening.
 let pair=inputs.windows(2).find(|p|p[0].album==p[1].album).ok_or_else(||anyhow::anyhow!("Need a same-album pair"))?;
 let db_path=temp.path().join("playback.sqlite");let db=Db::open(db_path.to_str().unwrap())?;let mut ids=Vec::new();
 for t in pair{ids.push(db.insert_track(&t.path,&t.title,&t.artist,&t.album,t.duration as i32,std::fs::metadata(&t.path)?.len() as i64,1)?);}drop(db);
 let service=LocalPlaybackService::start_with_preferences(db_path,PlaybackPreferences{volume:0.,..Default::default()})?;
 service.send(LocalPlaybackCommand::PlayTracks{ids,start_index:0})?;
 while service.snapshot().queue.len()!=2{ensure!(start.elapsed()<Duration::from_secs(60),"Playback setup timed out");std::thread::sleep(Duration::from_millis(20));}
 service.send(LocalPlaybackCommand::SetDjMode(true))?;service.send(LocalPlaybackCommand::SeekAbsolute(pair[0].duration-4.))?;std::thread::sleep(Duration::from_millis(150));
 let snapshot=service.snapshot();ensure!(snapshot.backend=="MusicOS two-deck","Native audio output unavailable");
 let config=dj_plan::Config{enabled:true,..Default::default()};let plan=dj_plan::plan(dj_plan::Track{key_index:None,duration:pair[0].duration,bpm:None,structure:None},dj_plan::Track{key_index:None,duration:pair[1].duration,bpm:None,structure:None},&config,false).unwrap();
 ensure!(plan.duration>0.,"Same-album transition was disabled");
 service.send(LocalPlaybackCommand::PlanTransition(musicos_desktop::local::deck_playback::Transition{request_id:0,outgoing:snapshot.queue_entry_ids[0],incoming:snapshot.queue_entry_ids[1],outgoing_cue:plan.outgoing_cue,incoming_cue:0.,duration:plan.duration,tempo:1.,effect:"clean_blend".into()}))?;
 let deadline=Instant::now()+Duration::from_secs(10);let mut saw_active=false;let mut saw_scheduled=false;
 while service.snapshot().queue_index!=Some(1){let s=service.snapshot();if let Some(t)=s.dj_transition{saw_active|=t.active;saw_scheduled|=t.status=="scheduled";}ensure!(Instant::now()<deadline,"Automatic transition timed out: {:?}",s.error);std::thread::sleep(Duration::from_millis(20));}
 ensure!(saw_active&&saw_scheduled,"Transition did not pass readiness and activation");
 service.send(LocalPlaybackCommand::Pause)?;std::thread::sleep(Duration::from_millis(150));let paused=service.snapshot().position;std::thread::sleep(Duration::from_millis(150));ensure!((service.snapshot().position-paused).abs()<0.01,"Paused source clock moved");
 let report=serde_json::json!({"runtime":"bundled ONNX Runtime 1.31.0; native Apple Silicon Rust service","model":model.manifest.id,"installed_bytes":spark_install::status().copied,"audio_embeddings":candidates.len(),"inference_seconds":inference_seconds,"generated_audio_artwork_persisted":true,"offline_local_generation":true,"deterministic_ranking":true,"feedback":true,"cached_command_generation":true,"choose_for_me":true,"save_refresh_conversion_restart":true,"same_album_sample_clocked_transition":true,"scheduled_and_active_observed":true,"pause_freezes_source_time":true,"test_volume":0,"audible_listening_evaluation":false,"packaged_ui_acceptance":false});
 drop(model);spark_install::remove(&model_dir)?;ensure!(!model_dir.exists(),"Model removal failed");std::fs::write(&args[4],serde_json::to_vec_pretty(&report)?)?;println!("{}",report);Ok(())
}
