//! Saved audio mixes have explicit refresh semantics. Changes never mutate a
//! playing queue, and membership remains intact when sources are hidden.
use anyhow::{Context,Result,ensure};
use musicos_desktop::{local::{LibraryRepository,LocalLibraryRepository},storage::atomic_write_json};
use serde::{Deserialize,Serialize};
use std::path::Path;
use crate::spark_engine::{Generation,Preview,GENERATION_VERSION};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Saved {#[serde(default)]pub updated_at:i64,pub id:String,pub name:String,pub description:String,pub artwork_choice:String,pub track_ids:Vec<i64>,pub generation:Generation,pub generation_version:String}
fn path(db:&Path)->std::path::PathBuf{db.with_extension("audio-sparklists-v1.json")}
pub fn list(db:&Path)->Result<Vec<Saved>>{let path=path(db);if !path.exists(){return Ok(Vec::new());}Ok(serde_json::from_slice(&std::fs::read(path)?)?)}
fn valid_name(name:&str)->Result<String>{let name=name.trim();ensure!(!name.is_empty()&&name.chars().count()<=120,"Use a name between 1 and 120 characters");Ok(name.into())}
fn valid_cover(choice:&str)->Result<()> {if choice.starts_with("vibe/"){crate::spark_artwork::Art::parse(choice)?;return Ok(());}ensure!((1..=9).any(|n|choice==format!("auto/{n}"))||choice=="collage","Choose an automatic Sparklist cover or collage");Ok(())}
pub fn save(db:&Path,id:Option<&str>,name:&str,cover:&str,preview:Preview)->Result<Saved>{
 valid_cover(cover)?;let name=valid_name(name)?;ensure!(preview.generation_version==GENERATION_VERSION&&preview.track_ids.len()<=25&&!preview.track_ids.is_empty(),"Generate a preview before saving");
 let mut items=list(db)?;let id=if let Some(id)=id {ensure!(items.iter().any(|s|s.id==id),"Sparklist no longer exists");id.into()}else{format!("audio/{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos())};
 let old=items.iter().position(|s|s.id==id);let saved=Saved{updated_at:std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()as i64,id,name,description:old.map(|i|items[i].description.clone()).unwrap_or_default(),artwork_choice:cover.into(),track_ids:preview.track_ids,generation:preview.configuration,generation_version:preview.generation_version};
 if let Some(i)=old{items[i]=saved.clone();}else{items.push(saved.clone());}atomic_write_json(&path(db),&items)?;Ok(saved)
}
pub fn edit(db:&Path,id:&str,name:Option<&str>,description:Option<&str>,cover:Option<&str>,delete:bool)->Result<()> {
 let mut items=list(db)?;let item=items.iter_mut().find(|s|s.id==id).context("Sparklist no longer exists")?;
 if let Some(name)=name {item.name=valid_name(name)?;}
 if let Some(description)=description {ensure!(description.chars().count()<=2000,"Use a description under 2,000 characters");item.description=description.into();}
 if let Some(cover)=cover{valid_cover(cover)?;item.artwork_choice=cover.into();}
 item.updated_at=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()as i64;
 if delete{items.retain(|s|s.id!=id);}atomic_write_json(&path(db),&items)?;Ok(())
}
pub fn as_manual(repo:&LocalLibraryRepository,id:&str,name:&str)->Result<i64>{
 let item=list(repo.db_path())?.into_iter().find(|s|s.id==id).context("Sparklist no longer exists")?;
 let playlist=repo.create_playlist(&valid_name(name)?)?;
 // Preserve hidden sources, while deleted database records cannot become new
 // playable songs. No library import takes place here.
 let db=musicos_init::database::Db::open(&repo.db_path().to_string_lossy())?;
 let ids=item.track_ids.into_iter().filter(|id|db.get_track_by_id(*id).ok().flatten().is_some()).collect::<Vec<_>>();
 if let Err(error)=repo.add_tracks(playlist.id,&ids){repo.delete_playlist(playlist.id)?;return Err(error);}Ok(playlist.id)
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn saved_membership_changes_only_on_explicit_save(){let dir=tempfile::tempdir().unwrap();let db=dir.path().join("library.db");let preview=Preview{track_ids:vec![3,1,2],analyzed:3,available:5,generation_version:GENERATION_VERSION.into(),configuration:Generation{seed:Some(3),mood:Some("Dreamy".into()),less_like:vec![7],variant:0}};let saved=save(&db,None,"  Night sky  ","auto/4",preview.clone()).unwrap();edit(&db,&saved.id,Some("Sky"),None,Some("auto/6"),false).unwrap();let reloaded=list(&db).unwrap().remove(0);assert_eq!(reloaded.track_ids,vec![3,1,2]);assert_eq!(reloaded.generation.less_like,vec![7]);assert_eq!(reloaded.artwork_choice,"auto/6");assert_eq!(reloaded.name,"Sky");let mut refreshed=preview;refreshed.track_ids=vec![1,2];save(&db,Some(&saved.id),"Sky","auto/6",refreshed).unwrap();assert_eq!(list(&db).unwrap()[0].track_ids,vec![1,2]);}
}
