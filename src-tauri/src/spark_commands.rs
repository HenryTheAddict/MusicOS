use anyhow::{Result,Context,ensure};
use musicos_desktop::local::{LocalLibraryRepository,LibraryRepository};
use serde::{Deserialize,Serialize};
use std::{collections::BTreeMap,path::{Path,PathBuf}};
use crate::{spark_engine::{self,Generation,Preview},collections::{self,Collection},audio_collections};
#[derive(Deserialize)]#[serde(tag="action",rename_all="snake_case",deny_unknown_fields)]
pub enum Request {
 Status,Choose,InstallStaged,Install{directory:String},CancelInstall,RemoveModel,
 Preview{generation:Generation},
 Save{generation:Generation,name:String,artwork_choice:String,#[serde(default)]preview:Option<Preview>},
 Refresh{id:String},
 AsPlaylist{id:String,name:String},
}
#[derive(Default,Serialize)]pub struct Activity {pub running:bool,pub paused:bool,pub processed:usize,pub total:usize}
#[derive(Serialize)]pub struct State {
 pub analysis:Activity,
 pub generated_artwork:Option<String>,
 pub staged_available:bool,pub ready:bool,pub reason:Option<String>,pub download_bytes:u64,
 pub installation:crate::spark_install::Status,pub beta:bool,
 pub installed:bool,
 pub analyzed:usize,pub available:usize,pub analyzed_ids:Vec<i64>,
 pub preview:Option<Preview>,pub items:Vec<Collection>,pub chosen_id:Option<i64>,
}
pub struct Setup {pub dir:PathBuf,pub runtime:PathBuf,pub evaluation:bool}
pub fn staged_candidate()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../out/rebuild-qa/model-evaluation-general/release-candidate/clap-general-v1")}
impl Setup {
 pub fn for_library(db:&Path,runtime:PathBuf,beta:bool)->Self {
  let dir=db.parent().unwrap_or(Path::new(".")).join("audio-model");
  #[cfg(debug_assertions)]
  if let (Some(dir),Some(runtime))=(std::env::var_os("MUSICOS_SPARK_EVALUATION_DIR"),std::env::var_os("MUSICOS_SPARK_EVALUATION_RUNTIME")){return Self{dir:dir.into(),runtime:runtime.into(),evaluation:true};}
  Self{dir,runtime,evaluation:beta}
 }
}
pub fn execute(repo:&LocalLibraryRepository,setup:&Setup,request:Request)->Result<State>{
 match &request {
  Request::Install{directory}=>{ensure!(setup.evaluation,"Enable experimental audio Sparklist beta first");crate::spark_install::start(directory.into(),setup.dir.clone())?;},
  Request::CancelInstall=>crate::spark_install::cancel(),
  Request::InstallStaged=>{ensure!(setup.evaluation,"Enable experimental audio Sparklist beta first.");let source=staged_candidate();ensure!(source.join("manifest.json").is_file(),"The reviewed staged candidate is not available on this machine.");crate::spark_install::start(source,setup.dir.clone())?;},
  Request::RemoveModel=>crate::spark_install::remove(&setup.dir)?,_=>{}
 }
 let manifest=spark_engine::read_manifest(&setup.dir,setup.evaluation);
 let candidate:spark_engine::Manifest=serde_json::from_str(include_str!("../resources/spark-candidate.json"))?;
 let library=repo.snapshot(&Default::default())?;let available=library.tracks.iter().filter(|t|Path::new(&t.path).is_file()).count();
 let (model,candidates,reason)=match manifest {
  Ok(manifest)=>{let id=format!("{}@{}",manifest.id,manifest.revision);let candidates=crate::embeddings::candidates(repo.db_path(),&library.tracks,&id);(Some(manifest),candidates,None)},
  Err(error)=>{let reason=if !setup.dir.join("manifest.json").exists(){if setup.evaluation {"Install the checksum-verified staged beta model, then analyze local songs. Public download is not available yet."} else {"Enable experimental audio Sparklist beta in Settings → Audio to evaluate the local model."}.into()}else{format!("{error:#}")};(None,Vec::new(),Some(reason))},
 };
 let mut state=State{installed:setup.dir.join("manifest.json").is_file(),generated_artwork:None,analysis:Default::default(),staged_available:staged_candidate().join("manifest.json").is_file(),installation:crate::spark_install::status(),beta:setup.evaluation,ready:model.is_some(),reason,download_bytes:candidate.files.iter().map(|f|f.bytes).sum(),analyzed:candidates.len(),available,analyzed_ids:candidates.iter().map(|c|c.id).collect(),preview:None,items:collections::list(repo)?,chosen_id:None};
 if matches!(request,Request::Status|Request::InstallStaged|Request::Install{..}|Request::CancelInstall|Request::RemoveModel){return Ok(state);}
 if let Request::AsPlaylist{id,name}=&request {audio_collections::as_manual(repo,id,name)?;state.items=collections::list(repo)?;return Ok(state);}
 ensure!(state.ready,"{}",state.reason.as_deref().unwrap_or("Audio model is unavailable"));
 let moods:BTreeMap<String,Vec<f32>>=serde_json::from_slice(&std::fs::read(setup.dir.join("moods.json"))?)?;
 match request {
  Request::Choose=>{ensure!(!candidates.is_empty(),"Analyze local songs before choosing a seed");let favorites=candidates.iter().filter(|c|c.favorite).collect::<Vec<_>>();let pool=if favorites.is_empty(){candidates.iter().collect::<Vec<_>>()}else{favorites};let clock=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.subsec_nanos() as usize;state.chosen_id=Some(pool[clock%pool.len()].id);},
  Request::Preview{generation}=>{let preview=spark_engine::generate(&candidates,available,generation,&moods)?;
   let tracks=library.tracks.iter().filter(|t|preview.track_ids.contains(&t.id)).collect::<Vec<_>>();
   let features=crate::analysis::cached_many(repo.db_path(),&tracks.iter().map(|t|(t.id,t.path.as_str())).collect::<Vec<_>>());
   let energies=features.iter().flatten().filter_map(|f|f.structure.as_ref().map(|s|s.energy)).collect::<Vec<_>>();
   let tempos=features.iter().flatten().filter(|f|f.bpm_confidence>=0.5).filter_map(|f|f.bpm).collect::<Vec<_>>();
   let average=|v:&Vec<f32>|if v.is_empty(){None}else{Some(v.iter().sum::<f32>()/v.len()as f32)};
   state.generated_artwork=Some(crate::spark_artwork::generate(&candidates,&preview.track_ids,&moods,preview.configuration.variant,average(&energies),average(&tempos)).choice());state.preview=Some(preview);},
  Request::Save{generation,name,artwork_choice,preview}=>{let preview=preserve_preview(preview,&generation,&candidates,available,&moods)?;audio_collections::save(repo.db_path(),None,&name,&artwork_choice,preview)?;state.items=collections::list(repo)?;},
  Request::Refresh{id}=>{let saved=audio_collections::list(repo.db_path())?.into_iter().find(|s|s.id==id).context("Sparklist no longer exists")?;let preview=spark_engine::generate(&candidates,available,saved.generation,&moods)?;audio_collections::save(repo.db_path(),Some(&id),&saved.name,&saved.artwork_choice,preview)?;state.items=collections::list(repo)?;},
  Request::Status|Request::InstallStaged|Request::Install{..}|Request::CancelInstall|Request::RemoveModel|Request::AsPlaylist{..}=>{},
 }
 Ok(state)
}

fn preserve_preview(preview:Option<Preview>,generation:&Generation,candidates:&[spark_engine::Candidate],available:usize,moods:&BTreeMap<String,Vec<f32>>)->Result<Preview>{
 spark_engine::validate_target_count(generation.target_count)?;
 let Some(preview)=preview else{return spark_engine::generate(candidates,available,generation.clone(),moods);};
 ensure!(preview.configuration==*generation&&preview.generation_version==spark_engine::GENERATION_VERSION,"Preview settings changed. Generate a new preview before saving.");
 ensure!(!preview.track_ids.is_empty()&&preview.track_ids.len()<=generation.target_count,"Generate a preview before saving.");
 let ids=preview.track_ids.iter().collect::<std::collections::BTreeSet<_>>();
 ensure!(ids.len()==preview.track_ids.len()&&ids.iter().all(|id|candidates.iter().any(|c|c.id==**id)),"A preview song is no longer available or analyzed. Generate a new preview.");
 Ok(preview)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn saving_preserves_preview_when_analysis_coverage_grows(){
  let candidates=(1..=4).map(|id|spark_engine::Candidate{id,artist:id.to_string(),album:"Album".into(),favorite:false,embedding:vec![1.;512]}).collect::<Vec<_>>();
  let generation=Generation{seed:Some(1),..Default::default()};
  let preview=spark_engine::generate(&candidates[..2],4,generation.clone(),&BTreeMap::new()).unwrap();
  assert_eq!(preserve_preview(Some(preview.clone()),&generation,&candidates,4,&BTreeMap::new()).unwrap(),preview);
  assert!(preserve_preview(Some(preview.clone()),&Generation{variant:1,..generation.clone()},&candidates,4,&BTreeMap::new()).is_err());
  assert!(preserve_preview(Some(preview),&generation,&candidates[1..],4,&BTreeMap::new()).is_err());
 }
}
