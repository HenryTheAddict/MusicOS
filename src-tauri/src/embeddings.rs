//! Versioned local embedding cache, separate from structural-analysis estimates.
use anyhow::{Result,ensure};
use serde::{Deserialize,Serialize};
use std::{path::Path,time::{Duration,SystemTime}};
use crate::spark_engine::{Model,Candidate};
// Invalidate local PCM changes without altering the pinned model's mel contract.
fn cache_preprocessing() -> &'static str {
    static KEY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    KEY.get_or_init(|| format!("{}/continuous-pcm-v2", crate::spark_audio::VERSION))
}
#[derive(Serialize,Deserialize)]pub struct Cached {pub vector:Vec<f32>,pub model:String,pub preprocessing:String}
fn connection(db:&Path)->Result<musicos_init::database::Connection>{let c=musicos_init::database::Connection::open_background(db)?;c.busy_timeout(Duration::from_secs(5))?;musicos_init::database_access::initialize_schema(db,"embeddings-v1",||{c.execute_batch("CREATE TABLE IF NOT EXISTS desktop_audio_embeddings_v1(track_id INTEGER PRIMARY KEY,identity TEXT NOT NULL,model TEXT NOT NULL,preprocessing TEXT NOT NULL,vector TEXT NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS desktop_audio_embeddings_track_v1 ON desktop_audio_embeddings_v1(track_id)")?;Ok(())})?;Ok(c)}
fn identity(path:&Path)->Result<String>{let m=std::fs::metadata(path)?;Ok(format!("{}:{}:{}",path.display(),m.len(),m.modified()?.duration_since(SystemTime::UNIX_EPOCH)?.as_nanos()))}
pub fn model_id(model:&Model)->String{format!("{}@{}",model.manifest.id,model.manifest.revision)}
pub fn cached(db:&Path,id:i64,path:&Path,model:&str)->Option<Vec<f32>>{
 let signature=identity(path).ok()?;let vector=connection(db).ok()?.query_row("SELECT vector FROM desktop_audio_embeddings_v1 WHERE track_id=? AND identity=? AND model=? AND preprocessing=?",rusqlite::params![id,signature,model,cache_preprocessing()],|r|r.get::<_,String>(0)).ok()?;let values:Vec<f32>=serde_json::from_str(&vector).ok()?;
 (values.len()==512&&values.iter().all(|v|v.is_finite())).then_some(values)
}
pub fn analyze(db:&Path,id:i64,path:&Path,duration:f32,model:&mut Model,checkpoint:impl FnMut()->bool)->Result<Vec<f32>>{
 let version=model_id(model);if let Some(values)=cached(db,id,path,&version){return Ok(values);}
 let signature=identity(path)?;let values=model.embed_file(path,duration,checkpoint)?;
 ensure!(identity(path)?==signature,"File changed during audio analysis. Analyze it again.");
 connection(db)?.execute("INSERT INTO desktop_audio_embeddings_v1 VALUES (?,?,?,?,?) ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity,model=excluded.model,preprocessing=excluded.preprocessing,vector=excluded.vector",rusqlite::params![id,signature,version,cache_preprocessing(),serde_json::to_string(&values)?])?;Ok(values)
}
pub fn candidates(db:&Path,tracks:&[musicos_desktop::models::Track],model:&str)->Vec<Candidate>{
 let Ok(c)=connection(db)else{return vec![]};let Ok(mut query)=c.prepare("SELECT track_id,identity,vector FROM desktop_audio_embeddings_v1 WHERE model=? AND preprocessing=?")else{return vec![]};
 let Ok(rows)=query.query_map(rusqlite::params![model,cache_preprocessing()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))else{return vec![]};
 let cache=rows.filter_map(Result::ok).map(|(id,key,vector)|(id,(key,vector))).collect::<std::collections::HashMap<_,_>>();
 tracks.iter().filter(|t|t.available).filter_map(|t|{let (key,json)=cache.get(&t.id)?;if identity(Path::new(&t.path)).ok().as_ref()!=Some(key){return None;}let embedding:Vec<f32>=serde_json::from_str(json).ok()?;if embedding.len()!=512||!embedding.iter().all(|v|v.is_finite()){return None;}Some(Candidate{id:t.id,artist:t.artist.clone(),album:t.album.clone(),favorite:t.favorited,embedding})}).collect()
}

#[cfg(test)]mod tests{use super::*;
 #[test]fn cache_without_primary_key_gets_unique_track_index(){let dir=tempfile::tempdir().unwrap();let db=dir.path().join("library.db");let c=musicos_init::database::Connection::open_background(&db).unwrap();c.execute_batch("CREATE TABLE desktop_audio_embeddings_v1(track_id INTEGER,identity TEXT,model TEXT,preprocessing TEXT,vector TEXT); INSERT INTO desktop_audio_embeddings_v1 VALUES(7,'a','b','c','[]')").unwrap();drop(c);let c=connection(&db).unwrap();c.execute("INSERT INTO desktop_audio_embeddings_v1 VALUES(7,'new','b','c','[]') ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity",[]).unwrap();assert_eq!(c.query_row("SELECT identity FROM desktop_audio_embeddings_v1 WHERE track_id=7",[],|r|r.get::<_,String>(0)).unwrap(),"new");}
 #[test]fn file_and_version_changes_invalidate_embeddings(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("song.wav");let db=dir.path().join("library.db");std::fs::write(&path,b"one").unwrap();let mut vector=vec![0_f32;512];vector[0]=1.;let conn=connection(&db).unwrap();conn.execute("INSERT INTO desktop_audio_embeddings_v1 VALUES(?,?,?,?,?)",rusqlite::params![7,identity(&path).unwrap(),"test@1",crate::spark_audio::VERSION,serde_json::to_string(&vector).unwrap()]).unwrap();assert!(cached(&db,7,&path,"test@1").is_none());conn.execute("UPDATE desktop_audio_embeddings_v1 SET preprocessing=?", [cache_preprocessing()]).unwrap();assert!(cached(&db,7,&path,"test@1").is_some());assert!(cached(&db,7,&path,"test@2").is_none());std::fs::write(&path,b"changed").unwrap();assert!(cached(&db,7,&path,"test@1").is_none());}
}
