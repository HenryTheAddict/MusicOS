//! Local audio embeddings and deterministic mix ranking. Candidate models are
//! explicitly gated; licensing eligibility alone does not approve production use.
use anyhow::{Context,Result,ensure};
use ort::{session::Session,value::Tensor};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{collections::{BTreeMap,BTreeSet},fs::File,io::Read,path::Path};
use crate::spark_audio;

pub const GENERATION_VERSION:&str="audio-multiseed-refinement-v4";
#[derive(Clone,Debug,Deserialize,Serialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {pub name:String,pub bytes:u64,pub sha256:String}
#[derive(Clone,Debug,Deserialize,Serialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
 pub schema:u32,pub id:String,pub checkpoint:String,pub revision:String,
 pub license:String,pub license_url:String,pub preprocessing:String,pub runtime:String,
 pub dimensions:usize,pub status:String,pub download_base:Option<String>,pub files:Vec<Artifact>,
}
pub fn verify_artifact(dir:&Path,artifact:&Artifact)->Result<()> {
 ensure!(matches!(artifact.name.as_str(),"audio.onnx"|"moods.json"|"preprocessor_config.json"),"Unrecognized model artifact");
 let mut file=File::open(dir.join(&artifact.name)).with_context(||format!("Missing model file: {}. Retry model setup.",artifact.name))?;
 ensure!(file.metadata()?.len()==artifact.bytes,"Model size check failed for {}. Retry model setup.",artifact.name);
 let mut hash=Sha256::new();let mut buffer=[0_u8;65536];
 loop {let n=file.read(&mut buffer)?;if n==0{break;}hash.update(&buffer[..n]);}
 ensure!(format!("{:x}",hash.finalize())==artifact.sha256,"Model checksum failed for {}. Remove and download it again.",artifact.name);Ok(())
}
pub fn verify_candidate(manifest:&Manifest,evaluation:bool)->Result<()> {
 let pinned:Manifest=serde_json::from_str(include_str!("../resources/spark-candidate.json"))?;
 ensure!(manifest==&pinned,"This is not the pinned Sparklist beta model. Install the verified replacement candidate.");
 ensure!(evaluation,"Enable experimental audio Sparklist beta before using this candidate.");Ok(())
}
pub fn read_manifest(dir:&Path,evaluation:bool)->Result<Manifest> {
 let bytes=std::fs::read(dir.join("manifest.json"))?;ensure!(bytes.len()<65536,"Model manifest is too large");
 let manifest:Manifest=serde_json::from_slice(&bytes)?;
 ensure!(manifest.schema==1&&manifest.dimensions==512&&manifest.preprocessing==spark_audio::VERSION&&manifest.runtime=="onnxruntime-1.31.0","This model version needs a newer musicOS runtime");
 verify_candidate(&manifest,evaluation)?;
 ensure!(manifest.status=="approved"||(evaluation&&manifest.status=="evaluation_candidate"),"Audio model is awaiting listening and packaged-runtime evaluation. Generation is unavailable.");
 ensure!(manifest.files.len()==3&&["audio.onnx","moods.json","preprocessor_config.json"].iter().all(|name|manifest.files.iter().filter(|f|f.name==*name).count()==1),"Incomplete model manifest");
 // Reuse verification only while all file identities remain unchanged. Polling
 // status must not hash the entire model every second during playback.
 static VERIFIED:std::sync::OnceLock<std::sync::Mutex<BTreeMap<std::path::PathBuf,Vec<(u64,std::time::SystemTime)>>>>=std::sync::OnceLock::new();
 let identity=manifest.files.iter().map(|f|{let m=std::fs::metadata(dir.join(&f.name))?;Ok((m.len(),m.modified()?))}).collect::<Result<Vec<_>>>()?;
 let key=dir.canonicalize()?;
 let cache=VERIFIED.get_or_init(Default::default);
 let cached=cache.lock().unwrap_or_else(|e|e.into_inner()).get(&key)==Some(&identity);
 if !cached {for artifact in &manifest.files {verify_artifact(dir,artifact)?;}cache.lock().unwrap_or_else(|e|e.into_inner()).insert(key,identity);}
 Ok(manifest)
}
pub struct Model {session:Session,pub manifest:Manifest,pub moods:BTreeMap<String,Vec<f32>>}
impl Model {
 /// Runtime path must be resolved by Rust from the signed application's resources,
 /// never supplied by a webview command or a downloaded model manifest.
 pub fn load(dir:&Path,runtime:&Path,evaluation:bool)->Result<Self> {
  let manifest=read_manifest(dir,evaluation)?;
  ort::init_from(runtime)?.with_name("MusicOS local audio").commit();
  let session=Session::builder()?.with_intra_threads(2).map_err(|e|anyhow::anyhow!(e.to_string()))?.commit_from_file(dir.join("audio.onnx")).map_err(|e|anyhow::anyhow!(e.to_string()))?;
  let moods:BTreeMap<String,Vec<f32>>=serde_json::from_slice(&std::fs::read(dir.join("moods.json"))?)?;
  ensure!(moods.len()==6&&["Chilled","Energetic","Melancholic","Bright","Focused","Dreamy"].iter().all(|name|moods.get(*name).is_some_and(|v|valid_embedding(v))),"Model mood vectors failed validation");
  Ok(Self{session,manifest,moods})
 }
 pub fn embed(&mut self,audio:&[f32])->Result<Vec<f32>> {
  let mel=spark_audio::mel(audio);let input=Tensor::from_array(([1,1,spark_audio::FRAMES,spark_audio::BINS],mel.into_boxed_slice()))?;
  let output=self.session.run(ort::inputs![input])?;let (_,values)=output[0].try_extract_tensor::<f32>()?;
  ensure!(valid_embedding(values),"Audio model returned an invalid embedding");Ok(normalize(values.to_vec()))
 }
 /// Representative excerpts decoded and resampled entirely in Rust, off the audio
 /// callback. Source identities are checked before persistence by the worker.
 pub fn embed_file(&mut self,path:&Path,duration:f32,mut checkpoint:impl FnMut()->bool)->Result<Vec<f32>> {
  use rodio::Source;
  let mut starts=vec![0.];if duration>20. {starts.extend([(duration-10.)*0.5,(duration-10.)*0.85]);}
  let mut mean=vec![0_f32;512];
  for start in starts.iter().copied() {
   ensure!(checkpoint(),"Analysis cancelled");let mut decoder=musicos_desktop::local::decoded_audio::open(path)?;
   if start>0. {decoder.try_seek(std::time::Duration::from_secs_f32(start)).map_err(|e|anyhow::anyhow!("Cannot seek to representative audio: {e}"))?;}
   let source=musicos_desktop::local::resample::ResampledFrames::<_,1>::new(decoder,spark_audio::SAMPLE_RATE);
   let mut audio=Vec::with_capacity(spark_audio::SAMPLES);
   for (i,[sample]) in source.take(spark_audio::SAMPLES).enumerate(){if i%spark_audio::SAMPLE_RATE as usize==0 {ensure!(checkpoint(),"Analysis cancelled");}audio.push(sample);}
   ensure!(!audio.is_empty(),"No decoded audio in representative excerpt");
   let embedding=self.embed(&audio)?;for (sum,value) in mean.iter_mut().zip(embedding){*sum+=value/starts.len() as f32;}
  }
  ensure!(checkpoint(),"Analysis cancelled");Ok(normalize(mean))
 }
}
fn valid_embedding(v:&[f32])->bool {v.len()==512&&v.iter().all(|v|v.is_finite())&&v.iter().map(|v|v*v).sum::<f32>()>1e-8}
fn normalize(mut v:Vec<f32>)->Vec<f32>{let norm=v.iter().map(|v|v*v).sum::<f32>().sqrt().max(1e-8);v.iter_mut().for_each(|v|*v/=norm);v}
fn cosine(a:&[f32],b:&[f32])->f32{a.iter().zip(b).map(|(a,b)|a*b).sum::<f32>().clamp(-1.,1.)}
#[derive(Clone,Debug)]
pub struct Candidate {pub id:i64,pub artist:String,pub album:String,pub favorite:bool,pub embedding:Vec<f32>}
pub fn default_target_count()->usize{25}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct KeptSong { pub track_id:i64, pub position:usize }
fn balanced()->u8 { 50 }
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Refinement {
 #[serde(default)]pub seed_ids:Vec<i64>,
 #[serde(default)]pub moods:Vec<String>,
 #[serde(default="balanced")]pub variety:u8,
 #[serde(default="balanced")]pub discovery:u8,
 #[serde(default)]pub kept:Vec<KeptSong>,
 #[serde(default)]pub excluded_ids:Vec<i64>,
 #[serde(default)]pub more_like:Vec<i64>,
}
impl Default for Refinement {fn default()->Self{Self{seed_ids:vec![],moods:vec![],variety:50,discovery:50,kept:vec![],excluded_ids:vec![],more_like:vec![]}}}
#[derive(Clone,Debug,Deserialize,Serialize,PartialEq)]
pub struct Generation {
 pub seed:Option<i64>,pub mood:Option<String>,pub less_like:Vec<i64>,pub variant:u32,
 #[serde(default="default_target_count")]pub target_count:usize,
 #[serde(default)]pub refinement:Refinement,
}
impl Default for Generation {fn default()->Self{Self{seed:None,mood:None,less_like:Vec::new(),variant:0,target_count:25,refinement:Refinement::default()}}}
pub fn validate_target_count(count:usize)->Result<()>{ensure!((10..=100).contains(&count),"Choose between 10 and 100 songs");Ok(())}
#[derive(Clone,Debug,Deserialize,Serialize,PartialEq)]
pub struct Preview {pub track_ids:Vec<i64>,pub analyzed:usize,pub available:usize,pub generation_version:String,pub configuration:Generation}
/// Explicit feedback only. Ordinary skips and unliking never enter this score.
pub fn generate(candidates:&[Candidate],available:usize,request:Generation,moods:&BTreeMap<String,Vec<f32>>)->Result<Preview> {
 generate_with_listening(candidates,available,request,moods,&BTreeMap::new())
}
pub fn generate_with_listening(candidates:&[Candidate],available:usize,request:Generation,moods:&BTreeMap<String,Vec<f32>>,plays:&BTreeMap<i64,u32>)->Result<Preview>{
 validate_target_count(request.target_count)?;
 let options=&request.refinement;
 ensure!(options.seed_ids.len()<=3&&options.moods.len()<=2,"Choose up to three starting songs and two moods");
 ensure!(options.variety<=100&&options.discovery<=100&&options.kept.len()<=request.target_count&&options.more_like.len()<=32&&options.excluded_ids.len()<=1000,"Sparklist refinement is out of range");
 let seeds=if options.seed_ids.is_empty(){request.seed.into_iter().collect::<Vec<_>>()}else{options.seed_ids.clone()};
 let mood_names=if options.moods.is_empty(){request.mood.iter().cloned().collect::<Vec<_>>()}else{options.moods.clone()};
 ensure!(seeds.iter().collect::<BTreeSet<_>>().len()==seeds.len()&&mood_names.iter().collect::<BTreeSet<_>>().len()==mood_names.len(),"Choose distinct starting songs and moods");
 ensure!(!seeds.is_empty()||!mood_names.is_empty(),"Choose a starting song or mood");
 ensure!(request.less_like.len()<=1000,"Too many feedback entries");
 ensure!(candidates.iter().all(|c|valid_embedding(&c.embedding)),"Audio analysis contains invalid embeddings. Analyze changed songs again.");
 let seed_songs=seeds.iter().map(|id|candidates.iter().find(|c|c.id==*id).context("Prepare every starting song before generating a mix")).collect::<Result<Vec<_>>>()?;
 let mood_vectors=mood_names.iter().map(|name|moods.get(name).filter(|v|valid_embedding(v)).context("Choose an available mood")).collect::<Result<Vec<_>>>()?;
 let mean=|vectors:Vec<&Vec<f32>>|->Option<Vec<f32>>{
  if vectors.is_empty(){return None}
  let mut combined=vec![0.;512];
  for vector in &vectors{for (out,value) in combined.iter_mut().zip(vector.iter()){*out+=value/vectors.len() as f32;}}
  Some(normalize(combined))
 };
 let seed_vector=mean(seed_songs.iter().map(|c|&c.embedding).collect());
 let mood_vector=mean(mood_vectors);
 let positive=options.more_like.iter().filter_map(|id|candidates.iter().find(|c|c.id==*id)).collect::<Vec<_>>();
 let negative=request.less_like.iter().filter_map(|id|candidates.iter().find(|c|c.id==*id)).collect::<Vec<_>>();
 let excluded=request.less_like.iter().chain(&options.excluded_ids).copied().collect::<BTreeSet<_>>();
 let mut positions=BTreeSet::new();let mut kept_ids=BTreeSet::new();
 for kept in &options.kept {
  ensure!(kept.position<request.target_count&&positions.insert(kept.position)&&kept_ids.insert(kept.track_id),"Kept song positions changed. Review your mix again.");
  ensure!(!excluded.contains(&kept.track_id)&&candidates.iter().any(|c|c.id==kept.track_id),"A kept song is unavailable. Remove its Keep setting or prepare it again.");
 }
 ensure!(options.kept.iter().all(|k|k.position<candidates.iter().filter(|c|!excluded.contains(&c.id)).count()),"There are too few eligible songs to retain every kept position. Reduce exclusions or move the kept song earlier.");
 // Compute expensive embedding similarities once; selection only updates bounded diversity terms.
 let groups=candidates.iter().map(|c|(c.id,c.artist.split(';').next().unwrap_or(&c.artist).trim().to_lowercase())).collect::<BTreeMap<_,_>>();
 let bases=candidates.iter().map(|c|{
   let similarity=seed_vector.as_ref().map(|s|cosine(s,&c.embedding));let mood_score=mood_vector.as_ref().map(|m|cosine(m,&c.embedding));
   let base=match(similarity,mood_score){(Some(a),Some(b))=>a*0.65+b*0.35,(Some(a),None)|(None,Some(a))=>a,_=>0.};
   let avoid=negative.iter().map(|n|cosine(&n.embedding,&c.embedding).max(0.)).fold(0_f32,f32::max)*0.25;
   let jitter=if request.variant==0{0.}else{let hash=(c.id as u64).wrapping_mul(6364136223846793005).wrapping_add((request.variant as u64).wrapping_mul(1442695040888963407));((hash>>32)as u32 as f32/u32::MAX as f32-0.5)*0.025};
   let positive_score=positive.iter().map(|p|cosine(&p.embedding,&c.embedding).max(0.)).fold(0_f32,f32::max)*0.08;
   let played=(*plays.get(&c.id).unwrap_or(&0) as f32).ln_1p()/8_f32.ln_1p();
   let preference=(0.5-options.discovery as f32/100.)*0.08*played.min(1.);
   (c.id,base+if c.favorite{0.025}else{0.}-avoid+jitter+positive_score+preference)
 }).collect::<BTreeMap<_,_>>();
 let mut remaining=candidates.iter().filter(|c|!excluded.contains(&c.id)&&!kept_ids.contains(&c.id)).collect::<Vec<_>>();remaining.sort_by_key(|c|c.id);
 let mut selected:Vec<&Candidate>=Vec::new();let mut artists:BTreeMap<&str,usize>=BTreeMap::new();let mut albums:BTreeMap<(&str,&str),usize>=BTreeMap::new();
 let mut pending_seeds=seed_songs.iter().copied().filter(|seed|!excluded.contains(&seed.id)&&!kept_ids.contains(&seed.id)).collect::<Vec<_>>();
 while selected.len()<request.target_count&&(!remaining.is_empty()||selected.len()<options.kept.iter().map(|k|k.position+1).max().unwrap_or(0)) {
  if let Some(kept)=options.kept.iter().find(|k|k.position==selected.len()){
   let song=candidates.iter().find(|c|c.id==kept.track_id).unwrap();
   *artists.entry(groups[&song.id].as_str()).or_default()+=1;
   *albums.entry((groups[&song.id].as_str(),song.album.as_str())).or_default()+=1;
   selected.push(song);continue;
  }
  if remaining.is_empty(){break;}
  if let Some(seed)=pending_seeds.first().copied(){
   pending_seeds.remove(0);remaining.retain(|song|song.id!=seed.id);
   *artists.entry(groups[&seed.id].as_str()).or_default()+=1;
   *albums.entry((groups[&seed.id].as_str(),seed.album.as_str())).or_default()+=1;
   selected.push(seed);continue;
  }
  let score=|c:&Candidate| {
   let group=groups[&c.id].as_str();
   let artist_penalty=(artists.get(group).copied().unwrap_or(0) as f32*0.04).min(0.12);
   let album_penalty=(albums.get(&(group,c.album.as_str())).copied().unwrap_or(0) as f32*0.025).min(0.075);
   let adjacent=selected.last().map(|last|if groups[&last.id]==group{0.035}else{0.}).unwrap_or(0.);
   bases[&c.id]-(artist_penalty+album_penalty+adjacent)*(options.variety as f32/50.)
  };
  let best=(0..remaining.len()).max_by(|a,b|score(remaining[*a]).total_cmp(&score(remaining[*b])).then_with(||remaining[*b].id.cmp(&remaining[*a].id))).unwrap();let chosen=remaining.remove(best);
  *artists.entry(groups[&chosen.id].as_str()).or_default()+=1;*albums.entry((groups[&chosen.id].as_str(),&chosen.album)).or_default()+=1;selected.push(chosen);
 }
 ensure!(!selected.is_empty(),"No analyzed songs match this request. Analyze more local songs or clear feedback.");
 Ok(Preview{track_ids:selected.iter().map(|c|c.id).collect(),analyzed:candidates.len(),available,generation_version:GENERATION_VERSION.into(),configuration:request})
}
/// Playlist suggestions use cached audio only; they never generate a saved sequence.
pub fn suggest(candidates:&[Candidate],members:&[i64])->Vec<i64>{
 let existing=members.iter().copied().collect::<BTreeSet<_>>();let seeds=candidates.iter().filter(|c|existing.contains(&c.id)&&valid_embedding(&c.embedding)).collect::<Vec<_>>();if seeds.is_empty(){return vec![];}
 let mut mean=vec![0_f32;512];for c in &seeds{for (out,value) in mean.iter_mut().zip(&c.embedding){*out+=value;}}
 if !valid_embedding(&mean){mean=seeds.iter().max_by(|a,b|{let score=|c:&&Candidate|seeds.iter().map(|s|cosine(&c.embedding,&s.embedding)).sum::<f32>();score(a).total_cmp(&score(b)).then_with(||b.id.cmp(&a.id))}).unwrap().embedding.clone();}let mean=normalize(mean);
 let mut remaining=candidates.iter().filter(|c|!existing.contains(&c.id)&&valid_embedding(&c.embedding)).collect::<Vec<_>>();remaining.sort_by_key(|c|c.id);let mut selected=Vec::new();let mut artists=BTreeMap::<&str,usize>::new();let mut albums=BTreeMap::<(&str,&str),usize>::new();
 while selected.len()<10&&!remaining.is_empty(){let score=|c:&Candidate|cosine(&mean,&c.embedding)-(artists.get(c.artist.as_str()).copied().unwrap_or(0)as f32*0.04).min(0.12)-(albums.get(&(c.artist.as_str(),c.album.as_str())).copied().unwrap_or(0)as f32*0.025).min(0.075);let best=(0..remaining.len()).max_by(|a,b|score(remaining[*a]).total_cmp(&score(remaining[*b])).then_with(||remaining[*b].id.cmp(&remaining[*a].id))).unwrap();let c=remaining.remove(best);selected.push(c.id);*artists.entry(&c.artist).or_default()+=1;*albums.entry((&c.artist,&c.album)).or_default()+=1;}selected
}

#[cfg(test)]mod tests {
 use super::*;
 fn candidate(id:i64,axis:usize,artist:&str)->Candidate{let mut embedding=vec![0.;512];embedding[axis]=1.;Candidate{id,artist:artist.into(),album:"Album".into(),favorite:false,embedding}}
 #[test]fn playlist_suggestions_are_cached_audio_ranked_unique_and_deterministic(){let mut candidates=(1..=14).map(|id|candidate(id,0,&format!("Artist {id}"))).collect::<Vec<_>>();candidates.push(candidate(20,1,"Unrelated"));let result=suggest(&candidates,&[1,2]);assert_eq!(result.len(),10);assert!(!result.contains(&1)&&!result.contains(&2)&&!result.contains(&20));assert_eq!(result,suggest(&candidates,&[2,1]));assert!(suggest(&candidates,&[999]).is_empty());assert_eq!(suggest(&candidates[..3],&[1,2]),vec![3]);}
 #[test]fn length_is_bounded_deterministic_and_unique(){
  let candidates=(1..=110).map(|i|candidate(i,0,&format!("Artist {i}"))).collect::<Vec<_>>();
  for count in [10,25,100]{let request=Generation{seed:Some(1),target_count:count,..Default::default()};let a=generate(&candidates,110,request.clone(),&BTreeMap::new()).unwrap();let b=generate(&candidates,110,request,&BTreeMap::new()).unwrap();assert_eq!(a,b);assert_eq!(a.track_ids.len(),count);assert_eq!(a.track_ids.iter().collect::<BTreeSet<_>>().len(),count);}
  for count in [0,9,101,usize::MAX]{assert!(generate(&candidates,110,Generation{seed:Some(1),target_count:count,..Default::default()},&BTreeMap::new()).is_err());}
  let partial=generate(&candidates[..3],110,Generation{seed:Some(1),target_count:100,less_like:vec![2],..Default::default()},&BTreeMap::new()).unwrap();assert_eq!(partial.track_ids,vec![1,3]);
 }
 #[test]fn legacy_generation_defaults_to_twenty_five(){let request:Generation=serde_json::from_str(r#"{"seed":1,"mood":null,"less_like":[],"variant":0}"#).unwrap();assert_eq!(request.target_count,25);assert_eq!(Generation::default().target_count,25);}
 #[test]fn deterministic_ranking_is_audio_driven_and_respects_feedback(){let candidates=vec![candidate(1,0,"A"),candidate(2,1,"B"),candidate(3,0,"C")];let request=Generation{seed:Some(1),..Default::default()};let a=generate(&candidates,7,request.clone(),&BTreeMap::new()).unwrap();let b=generate(&candidates,7,request,&BTreeMap::new()).unwrap();assert_eq!(a.track_ids,vec![1,3,2]);assert_eq!(a.track_ids,b.track_ids);assert_eq!(a.available,7);let request=Generation{seed:Some(1),less_like:vec![3],..Default::default()};assert_eq!(generate(&candidates,7,request,&BTreeMap::new()).unwrap().track_ids,vec![1,2]);}
 #[test]fn diversity_cannot_force_unrelated_audio_ahead_of_relevant_songs(){
  let mut candidates=(1..=24).map(|id|candidate(id,0,"Same artist")).collect::<Vec<_>>();
  candidates.push(candidate(25,1,"Unrelated artist"));
  let result=generate(&candidates,25,Generation{seed:Some(1),..Default::default()},&BTreeMap::new()).unwrap();
  assert_eq!(result.track_ids.last(),Some(&25));
  assert_eq!(result.generation_version,"audio-multiseed-refinement-v4");
 }
 #[test]fn missing_seed_and_invalid_mood_do_not_substitute_metadata(){let candidates=vec![candidate(1,0,"A")];assert!(generate(&candidates,1,Generation{seed:Some(2),..Default::default()},&BTreeMap::new()).is_err());assert!(generate(&candidates,1,Generation{mood:Some("Bright".into()),..Default::default()},&BTreeMap::new()).is_err());}
 #[test]fn refinement_keeps_positions_while_combining_seeds_moods_and_feedback(){
  let candidates=(1..=30).map(|id|candidate(id,(id%2) as usize,&format!("Artist {}",id%4))).collect::<Vec<_>>();let mut moods=BTreeMap::new();moods.insert("Bright".into(),candidates[0].embedding.clone());moods.insert("Dreamy".into(),candidates[1].embedding.clone());
  let request=Generation{refinement:Refinement{seed_ids:vec![1,2,3],moods:vec!["Bright".into(),"Dreamy".into()],kept:vec![KeptSong{track_id:15,position:7}],excluded_ids:vec![20],more_like:vec![21],variety:80,discovery:75},..Default::default()};
  let a=generate_with_listening(&candidates,30,request.clone(),&moods,&BTreeMap::from([(4,80)])).unwrap();let b=generate_with_listening(&candidates,30,request,&moods,&BTreeMap::from([(4,80)])).unwrap();assert_eq!(a,b);assert_eq!(a.track_ids[7],15);assert_eq!(&a.track_ids[..3],&[1,2,3]);assert!(!a.track_ids.contains(&20));assert_eq!(a.track_ids.len(),25);
 }
 #[test]fn unreachable_kept_position_and_duplicate_seeds_are_rejected(){let candidates=vec![candidate(1,0,"A"),candidate(2,0,"B")];let mut generation=Generation{seed:Some(1),refinement:Refinement{kept:vec![KeptSong{track_id:2,position:8}],..Default::default()},..Default::default()};assert!(generate(&candidates,2,generation.clone(),&BTreeMap::new()).is_err());generation.refinement.kept.clear();generation.refinement.seed_ids=vec![1,1];assert!(generate(&candidates,2,generation,&BTreeMap::new()).is_err());}
 #[test]fn checksum_and_path_traversal_are_rejected(){let dir=tempfile::tempdir().unwrap();std::fs::write(dir.path().join("audio.onnx"),b"model").unwrap();let mut artifact=Artifact{name:"audio.onnx".into(),bytes:5,sha256:format!("{:x}",Sha256::digest(b"model"))};verify_artifact(dir.path(),&artifact).unwrap();artifact.sha256="bad".into();assert!(verify_artifact(dir.path(),&artifact).is_err());artifact.name="../model".into();assert!(verify_artifact(dir.path(),&artifact).is_err());}
 #[test]fn beta_requires_consent_and_exact_replacement_checkpoint(){let pinned:Manifest=serde_json::from_str(include_str!("../resources/spark-candidate.json")).unwrap();assert!(verify_candidate(&pinned,true).is_ok());assert!(verify_candidate(&pinned,false).is_err());let mut old=pinned.clone();old.checkpoint="laion/larger_clap_music".into();assert!(verify_candidate(&old,true).is_err());let mut changed=pinned;changed.files[0].sha256="unreviewed".into();assert!(verify_candidate(&changed,true).is_err());}
 #[test]fn primary_artist_variants_do_not_bypass_spacing(){let candidates=vec![candidate(1,0,"Artist; Guest A"),candidate(2,0,"ARTIST; Guest B"),candidate(3,0,"Another")];let result=generate(&candidates,3,Generation{seed:Some(1),..Default::default()},&BTreeMap::new()).unwrap();assert_eq!(result.track_ids,vec![1,3,2]);}
 #[test]fn artist_diversity_breaks_equal_similarity(){let candidates=vec![candidate(1,0,"A"),candidate(2,0,"A"),candidate(3,0,"B")];let result=generate(&candidates,3,Generation{seed:Some(1),..Default::default()},&BTreeMap::new()).unwrap();assert_eq!(result.track_ids,vec![1,3,2]);}
}
