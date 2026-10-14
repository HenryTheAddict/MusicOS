//! Versioned local embedding cache, separate from structural-analysis estimates.
use anyhow::{Result,ensure};
use serde::{Deserialize,Serialize};
use std::{path::Path,time::{Duration,SystemTime}};
use crate::spark_engine::{Model,Candidate};
#[derive(Serialize,Deserialize)]pub struct Cached {pub vector:Vec<f32>,pub model:String,pub preprocessing:String}
fn connection(db:&Path)->Result<rusqlite::Connection>{let c=rusqlite::Connection::open(db)?;c.busy_timeout(Duration::from_secs(5))?;c.execute_batch("CREATE TABLE IF NOT EXISTS desktop_audio_embeddings_v1(track_id INTEGER PRIMARY KEY,identity TEXT NOT NULL,model TEXT NOT NULL,preprocessing TEXT NOT NULL,vector TEXT NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS desktop_audio_embeddings_track_v1 ON desktop_audio_embeddings_v1(track_id)")?;Ok(c)}
fn identity(path:&Path)->Result<String>{let m=std::fs::metadata(path)?;Ok(format!("{}:{}:{}",path.display(),m.len(),m.modified()?.duration_since(SystemTime::UNIX_EPOCH)?.as_nanos()))}
pub fn model_id(model:&Model)->String{format!("{}@{}",model.manifest.id,model.manifest.revision)}
pub fn cached(db:&Path,id:i64,path:&Path,model:&str)->Option<Vec<f32>>{
 let signature=identity(path).ok()?;let vector=connection(db).ok()?.query_row("SELECT vector FROM desktop_audio_embeddings_v1 WHERE track_id=? AND identity=? AND model=? AND preprocessing=?",rusqlite::params![id,signature,model,crate::spark_audio::VERSION],|r|r.get::<_,String>(0)).ok()?;let values:Vec<f32>=serde_json::from_str(&vector).ok()?;
 (values.len()==512&&values.iter().all(|v|v.is_finite())).then_some(values)
}
pub fn analyze(db:&Path,id:i64,path:&Path,duration:f32,model:&mut Model,checkpoint:impl FnMut()->bool)->Result<Vec<f32>>{
 let version=model_id(model);if let Some(values)=cached(db,id,path,&version){return Ok(values);}
 let signature=identity(path)?;let values=model.embed_file(path,duration,checkpoint)?;
 ensure!(identity(path)?==signature,"File changed during audio analysis. Analyze it again.");
 connection(db)?.execute("INSERT INTO desktop_audio_embeddings_v1 VALUES (?,?,?,?,?) ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity,model=excluded.model,preprocessing=excluded.preprocessing,vector=excluded.vector",rusqlite::params![id,signature,version,crate::spark_audio::VERSION,serde_json::to_string(&values)?])?;Ok(values)
}
pub fn candidates(db:&Path,tracks:&[musicos_desktop::models::Track],model:&str)->Vec<Candidate>{
 tracks.iter().filter(|t|Path::new(&t.path).is_file()).filter_map(|t|cached(db,t.id,Path::new(&t.path),model).map(|embedding|Candidate{id:t.id,artist:t.artist.clone(),album:t.album.clone(),favorite:t.favorited,embedding})).collect()
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn cache_without_primary_key_gets_unique_track_index(){let dir=tempfile::tempdir().unwrap();let db=dir.path().join("library.db");let c=rusqlite::Connection::open(&db).unwrap();c.execute_batch("CREATE TABLE desktop_audio_embeddings_v1(track_id INTEGER,identity TEXT,model TEXT,preprocessing TEXT,vector TEXT); INSERT INTO desktop_audio_embeddings_v1 VALUES(7,'a','b','c','[]')").unwrap();drop(c);let c=connection(&db).unwrap();c.execute("INSERT INTO desktop_audio_embeddings_v1 VALUES(7,'new','b','c','[]') ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity",[]).unwrap();assert_eq!(c.query_row("SELECT identity FROM desktop_audio_embeddings_v1 WHERE track_id=7",[],|r|r.get::<_,String>(0)).unwrap(),"new");}
 #[test]fn file_and_version_changes_invalidate_embeddings(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("song.wav");let db=dir.path().join("library.db");std::fs::write(&path,b"one").unwrap();let mut vector=vec![0_f32;512];vector[0]=1.;connection(&db).unwrap().execute("INSERT INTO desktop_audio_embeddings_v1 VALUES(?,?,?,?,?)",rusqlite::params![7,identity(&path).unwrap(),"test@1",crate::spark_audio::VERSION,serde_json::to_string(&vector).unwrap()]).unwrap();assert!(cached(&db,7,&path,"test@1").is_some());assert!(cached(&db,7,&path,"test@2").is_none());std::fs::write(&path,b"changed").unwrap();assert!(cached(&db,7,&path,"test@1").is_none());}
}
