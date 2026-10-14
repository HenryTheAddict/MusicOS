//! Developer evaluation only. Reports stay local; production has no Python.
use musicos_tauri::{spark_engine::{Model,Candidate,Generation,generate},embeddings};
use serde::Deserialize;
use std::{path::Path,time::Instant};
#[derive(Deserialize)]struct Input{id:i64,path:String,title:String,artist:String,album:String,duration:f32,#[serde(default)]favorite:bool}
fn main()->anyhow::Result<()> {
 let args=std::env::args().collect::<Vec<_>>();anyhow::ensure!(args.len()==6,"model-dir runtime-dylib tracks-json cache-db report-json");
 let mut model=Model::load(Path::new(&args[1]),Path::new(&args[2]),true)?;
 let inputs:Vec<Input>=serde_json::from_slice(&std::fs::read(&args[3])?)?;let mut candidates=Vec::new();let mut timings=Vec::new();
 for t in &inputs {let started=Instant::now();let embedding=embeddings::analyze(Path::new(&args[4]),t.id,Path::new(&t.path),t.duration,&mut model,||true)?;timings.push(serde_json::json!({"id":t.id,"seconds":started.elapsed().as_secs_f64(),"duration":t.duration}));candidates.push(Candidate{id:t.id,artist:t.artist.clone(),album:t.album.clone(),favorite:t.favorite,embedding});}
 let mut comparisons=Vec::new();
 for seed in candidates.iter().take(6) {
  let audio=generate(&candidates,candidates.len(),Generation{seed:Some(seed.id),..Default::default()},&model.moods)?;
  let mut baseline=candidates.iter().filter(|c|c.id!=seed.id).collect::<Vec<_>>();baseline.sort_by_key(|c|(!(c.artist==seed.artist),!(c.album==seed.album),c.id));
  let top=audio.track_ids.iter().skip(1).take(5).filter_map(|id|candidates.iter().find(|c|c.id==*id)).collect::<Vec<_>>();
  comparisons.push(serde_json::json!({"seed":seed.id,"audio":audio.track_ids,"metadata_baseline":baseline.iter().map(|c|c.id).take(24).collect::<Vec<_>>(),"distinct_artists_top5":top.iter().map(|c|&c.artist).collect::<std::collections::BTreeSet<_>>().len(),"listening_result":null}));
 }
 let moods=model.moods.keys().map(|mood|generate(&candidates,candidates.len(),Generation{mood:Some(mood.clone()),..Default::default()},&model.moods)).collect::<anyhow::Result<Vec<_>>>()?;
 let report=serde_json::json!({"checkpoint":model.manifest,"preprocessing":musicos_tauri::spark_audio::VERSION,"generation":musicos_tauri::spark_engine::GENERATION_VERSION,"runtime":"Rust/ONNX Runtime 1.31.0","timings":timings,"comparisons":comparisons,"moods":moods,"songs":inputs.iter().map(|t|serde_json::json!({"id":t.id,"title":t.title,"artist":t.artist,"album":t.album})).collect::<Vec<_>>(),"blind_listening_complete":false,"packaged_acceptance":false});
 std::fs::write(&args[5],serde_json::to_vec_pretty(&report)?)?;println!("Analyzed {} songs. Local report: {}",candidates.len(),args[5]);Ok(())
}
