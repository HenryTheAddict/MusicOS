use anyhow::{ensure, Result};
use musicos_init::database::{Db, MetadataUpdate};
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataPatch {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist:Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
}
pub fn edit(path:&Path,ids:&[i64],patch:MetadataPatch)->Result<usize>{edit_with_files(path,ids,patch,false,&[])}
pub fn edit_with_files(path: &Path, ids: &[i64], patch: MetadataPatch,write:bool,reviews:&[(i64,i64,String)]) -> Result<usize> {
    ensure!(!ids.is_empty() && ids.len() <= 10000, "Select between 1 and 10,000 songs.");
    let unique: HashSet<_> = ids.iter().copied().collect();
    ensure!(unique.len() == ids.len(), "A song was selected more than once.");
    for value in [&patch.title, &patch.artist, &patch.album, &patch.genre, &patch.album_artist].into_iter().flatten() {
        ensure!(value.trim().chars().count() <= 512 && !value.contains(['\n','\r','\0']), "Use metadata of up to 512 characters on one line.");
    }
    for value in [&patch.title, &patch.artist, &patch.album].into_iter().flatten() {
        ensure!(!value.trim().is_empty(), "Title, artist and album cannot be blank.");
    }
    ensure!(patch.title.is_none() || ids.len() == 1, "Edit titles one song at a time.");
    ensure!(patch.year.is_none_or(|n| (0..=9999).contains(&n)), "Year must be between 0 and 9999.");
    ensure!([patch.track_number,patch.disc_number].into_iter().flatten().all(|n| (0..=9999).contains(&n)), "Track and disc numbers must be between 0 and 9999.");
    ensure!(patch.title.is_some() || patch.artist.is_some() || patch.album.is_some() || patch.genre.is_some() || patch.year.is_some() || patch.album_artist.is_some() || patch.track_number.is_some() || patch.disc_number.is_some(), "Choose a field to change.");
    let db = Db::open(path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid library path."))?)?;
    for id in ids { ensure!(db.get_track_by_id(*id)?.is_some(), "A selected song no longer exists. Refresh your library."); }
    let changes=ids.iter().map(|id|(*id,MetadataUpdate{title:patch.title.clone(), artist:patch.artist.clone(), album:patch.album.clone(), genre:patch.genre.clone(),year:patch.year,track_number:patch.track_number,disc_number:patch.disc_number,artwork_path:None,album_artist:patch.album_artist.clone()})).collect::<Vec<_>>();
    Ok(crate::file_tags::apply(path,&changes,reviews,write)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignored_candidates_persist_and_restore_without_applying_fields(){
        let tmp=tempfile::tempdir().unwrap();let path=tmp.path().join("library.sqlite3");let db=Db::open(path.to_str().unwrap()).unwrap();let id=db.insert_track("/one.flac","Original","Artist","Album",60,1,1).unwrap();
        let candidate=musicos_desktop::local::MetadataCandidate{title:"Suggested".into(),artist:"Artist".into(),album:"Album".into(),year:None,artwork_path:None,recording_mbid:"recording".into(),release_mbid:Some("release".into()),release_group_mbid:None,confidence:0.8,album_artist:None,track_number:None,disc_number:None};let token=serde_json::to_string(&candidate).unwrap();let c=musicos_init::database::Connection::open(&path).unwrap();let revision:i64=c.query_row("SELECT revision FROM tracks WHERE id=?",[id],|r|r.get(0)).unwrap();c.execute("INSERT INTO metadata_enrichment_status(track_id,state,candidate_json,checked_at) VALUES(?,'review',?,1)",rusqlite::params![id,token]).unwrap();
        let request=||WorkspaceRequest{action:"ignore".into(),track_id:Some(id),revision:Some(revision),token:Some(token.clone()),..Default::default()};workspace(&path,request()).unwrap();assert!(workspace(&path,request()).is_err());let identity=musicos_desktop::local::candidate_fingerprint(&candidate);assert!(Db::open(path.to_str().unwrap()).unwrap().metadata_candidate_ignored(id,&identity).unwrap());let list=workspace(&path,WorkspaceRequest{action:"ignored".into(),..Default::default()}).unwrap();assert_eq!(list["total"],1);assert_eq!(db.get_track_by_id(id).unwrap().unwrap().title,"Original");workspace(&path,WorkspaceRequest{action:"restore".into(),track_id:Some(id),fingerprint:Some(identity.clone()),..Default::default()}).unwrap();assert!(!db.metadata_candidate_ignored(id,&identity).unwrap());assert_eq!(c.query_row("SELECT state FROM metadata_enrichment_status WHERE track_id=?",[id],|r|r.get::<_,String>(0)).unwrap(),"review");
    }
    #[test]
    fn batch_overrides_preserve_unselected_fields_and_reject_partial_invalid_batches() {
        let tmp=tempfile::tempdir().unwrap(); let path=tmp.path().join("library.sqlite3");
        let db=Db::open(path.to_str().unwrap()).unwrap();
        let a=db.insert_track("/a.flac","One","Original","Record",1,1,1).unwrap();
        let b=db.insert_track("/b.flac","Two","Original","Record",1,1,1).unwrap();
        assert_eq!(edit(&path,&[a,b],MetadataPatch{artist:Some("Edited".into()),..Default::default()}).unwrap(),2);
        assert_eq!(db.get_track_by_id(a).unwrap().unwrap().title,"One");
        assert_eq!(db.get_track_by_id(b).unwrap().unwrap().artist,"Edited");
        assert_eq!(db.metadata_override_origin(a).unwrap().as_deref(),Some("user"));
        assert!(edit(&path,&[a,99999],MetadataPatch{artist:Some("Wrong".into()),..Default::default()}).is_err());
        assert_eq!(db.get_track_by_id(a).unwrap().unwrap().artist,"Edited");
        assert!(edit(&path,&[a,b],MetadataPatch{title:Some("Same".into()),..Default::default()}).is_err());
        assert!(edit(&path,&[a],MetadataPatch{artist:Some(" ".into()),..Default::default()}).is_err());
        assert!(edit(&path,&[a],MetadataPatch{year:Some(-1),..Default::default()}).is_err());
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct WorkspaceRequest {
    pub action:String,pub search:Option<String>,pub offset:Option<usize>,pub id:Option<i64>,
    pub track_id:Option<i64>,pub revision:Option<i64>,pub token:Option<String>,pub fingerprint:Option<String>,pub fields:Option<Vec<String>>,
}
pub fn workspace(path:&Path,request:WorkspaceRequest)->Result<serde_json::Value>{
    use anyhow::Context;
    use rusqlite::OptionalExtension;
    let db=Db::open(path.to_str().context("Invalid library path")?)?;
    if request.action=="history" {return Ok(serde_json::json!({"history":db.metadata_history()?}));}
    if request.action=="undo" {crate::file_tags::undo(path,request.id.context("Choose a change to undo")?)?;return Ok(serde_json::json!({"history":db.metadata_history()?}));}
    if request.action=="ignored" {
        let c=musicos_init::database::Connection::open(path)?;let search=format!("%{}%",request.search.as_deref().unwrap_or(""));let offset=request.offset.unwrap_or(0).min(100000);
        let total:i64=c.query_row("SELECT count(*) FROM metadata_ignored_candidates i JOIN tracks t ON t.id=i.track_id WHERE t.title||' '||t.artist LIKE ?",[&search],|r|r.get(0))?;
        let mut q=c.prepare("SELECT i.track_id,i.fingerprint,i.candidate_json,t.title,t.artist FROM metadata_ignored_candidates i JOIN tracks t ON t.id=i.track_id WHERE t.title||' '||t.artist LIKE ? ORDER BY ignored_at DESC LIMIT 50 OFFSET ?")?;
        let items=q.query_map(rusqlite::params![search,offset],|r|Ok(serde_json::json!({"track_id":r.get::<_,i64>(0)?,"candidate_identity":r.get::<_,String>(1)?,"candidate_token":r.get::<_,String>(2)?,"current_title":r.get::<_,String>(3)?,"current_artist":r.get::<_,String>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;return Ok(serde_json::json!({"items":items,"total":total,"offset":offset}));
    }
    if request.action=="restore" {
        let id=request.track_id.context("Choose an ignored suggestion")?;let fingerprint=request.fingerprint.context("Choose an ignored match")?;
        let c=musicos_init::database::Connection::open(path)?;let tx=c.unchecked_transaction()?;
        ensure!(tx.execute("DELETE FROM metadata_ignored_candidates WHERE track_id=? AND fingerprint=?",rusqlite::params![id,fingerprint])?>0,"This ignored match changed");
        let token:Option<String>=tx.query_row("SELECT candidate_json FROM metadata_enrichment_status WHERE track_id=? AND state='ignored'",[id],|r|r.get(0)).optional()?;
        if let Some(token)=token {let candidate=serde_json::from_str::<musicos_desktop::local::MetadataCandidate>(&token)?;if musicos_desktop::local::candidate_fingerprint(&candidate)==fingerprint{tx.execute("UPDATE metadata_enrichment_status SET state='review' WHERE track_id=?",[id])?;}}
        tx.commit()?;return Ok(serde_json::json!({}));
    }
    if request.action=="apply" {
        let id=request.track_id.context("Choose a suggestion")?;
        let token=request.token.as_deref().context("Refresh this suggestion")?;
        let candidate:musicos_desktop::local::MetadataCandidate=serde_json::from_str(token)?;
        let fields=request.fields.as_deref().unwrap_or_default();ensure!(!fields.is_empty(),"Choose fields to apply.");
        let mut update=MetadataUpdate::default();
        for field in fields {match field.as_str(){
            "title"=>{ensure!(!candidate.title.trim().is_empty(),"Title cannot be empty");update.title=Some(candidate.title.clone());},
            "artist"=>{ensure!(!candidate.artist.trim().is_empty(),"Artist cannot be empty");update.artist=Some(candidate.artist.clone());},
            "album"=>{ensure!(!candidate.album.trim().is_empty(),"Album cannot be empty");update.album=Some(candidate.album.clone());},
            "year"=>{ensure!(candidate.year.is_some(),"No year was suggested");update.year=candidate.year;},
            "artwork"=>{let art=candidate.artwork_path.as_deref().context("No artwork was suggested")?;let m=std::fs::metadata(art)?;ensure!(m.len()<=20*1024*1024,"Artwork is too large");let mut reader=image::io::Reader::open(art)?.with_guessed_format()?;let mut limits=image::io::Limits::default();limits.max_alloc=Some(128*1024*1024);limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);reader.limits(limits);reader.decode()?;update.artwork_path=Some(art.into());},
            _=>anyhow::bail!("Choose an available field"),
        }}
        db.update_metadata_review(id,&update,request.revision.context("Refresh this comparison")?,token)?;
    } else if matches!(request.action.as_str(),"dismiss"|"ignore") {
        let id=request.track_id.context("Choose a suggestion")?;
        let c=musicos_init::database::Connection::open(path)?;let tx=c.unchecked_transaction()?;
        let current:Option<(String,i64)>=tx.query_row("SELECT s.candidate_json,t.revision FROM metadata_enrichment_status s JOIN tracks t ON t.id=s.track_id WHERE s.track_id=? AND s.state='review'",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        ensure!(current==request.token.as_ref().zip(request.revision).map(|(t,r)|(t.clone(),r)),"This suggestion changed. Refresh before keeping current values.");
        if request.action=="ignore" {let candidate=serde_json::from_str::<musicos_desktop::local::MetadataCandidate>(request.token.as_deref().unwrap())?;let fingerprint=musicos_desktop::local::candidate_fingerprint(&candidate);tx.execute("INSERT OR REPLACE INTO metadata_ignored_candidates VALUES(?,?,?,strftime('%s','now'))",rusqlite::params![id,fingerprint,request.token])?;}
        tx.execute("UPDATE metadata_enrichment_status SET state=? WHERE track_id=?",rusqlite::params![if request.action=="ignore"{"ignored"}else{"kept"},id])?;tx.commit()?;
    } else {ensure!(request.action=="list","Unknown metadata action");}
    let c=musicos_init::database::Connection::open(path)?;
    let search=format!("%{}%",request.search.as_deref().unwrap_or("").trim().replace('%',"\\%").replace('_',"\\_"));
    let condition="FROM metadata_enrichment_status s JOIN tracks t ON t.id=s.track_id LEFT JOIN track_metadata_overrides o ON o.track_id=t.id WHERE s.state='review' AND (coalesce(o.title,t.title)||' '||coalesce(o.artist,t.artist)||' '||coalesce(o.album,t.album)) LIKE ? ESCAPE '\\'";
    let total:i64=c.query_row(&format!("SELECT count(*) {condition}"),[&search],|r|r.get(0))?;
    let offset=request.offset.unwrap_or(0).min(100000);
    let mut stmt=c.prepare(&format!("SELECT s.track_id,s.candidate_json {condition} ORDER BY s.track_id LIMIT 50 OFFSET ?"))?;
    let records=stmt.query_map(rusqlite::params![search,offset],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut items=Vec::new();for (id,token) in records {if let Some(t)=db.get_track_by_id(id)? {items.push(serde_json::json!({"track_id":id,"current_title":t.title,"current_artist":t.artist,"current_album":t.album,"current_year":db.metadata_values(id)?["year"],"current_artwork":t.cover_art_path,"revision":db.metadata_values(id)?["revision"],"candidate_token":token,"candidate":serde_json::from_str::<serde_json::Value>(&token)?}));}}
    Ok(serde_json::json!({"items":items,"total":total,"offset":offset,"lookup":db.metadata_enrichment_counts()?}))
}

#[cfg(test)] mod workspace_tests {
 use super::*;use musicos_init::database::MetadataEnrichmentStatus;
 fn candidate()->String{serde_json::json!({"title":"Suggested title","artist":"Guest artist","album":"Other edition","year":2024,"artwork_path":null,"recording_mbid":"recording","release_mbid":"release","release_group_mbid":null,"confidence":0.95}).to_string()}
 fn seed(db:&Db,id:i64,token:&str){db.record_metadata_enrichment_status(&MetadataEnrichmentStatus{track_id:id,state:"review".into(),provider:"musicbrainz".into(),candidate_json:Some(token.into()),source_modified_at:1,..Default::default()}).unwrap();}
 #[test] fn selected_fields_reject_stale_candidates_and_preserve_edition(){
  let d=tempfile::tempdir().unwrap();let path=d.path().join("db");let db=Db::open(path.to_str().unwrap()).unwrap();let id=db.insert_track("/x.flac","Original","Artist","Deluxe",1,1,1).unwrap();let token=candidate();seed(&db,id,&token);
  let revision=db.metadata_values(id).unwrap()["revision"].as_i64().unwrap();
  assert!(workspace(&path,WorkspaceRequest{action:"apply".into(),track_id:Some(id),revision:Some(revision+1),token:Some(token.clone()),fields:Some(vec!["title".into()]),..Default::default()}).is_err());
  workspace(&path,WorkspaceRequest{action:"apply".into(),track_id:Some(id),revision:Some(revision),token:Some(token.clone()),fields:Some(vec!["title".into()]),..Default::default()}).unwrap();
  let t=db.get_track_by_id(id).unwrap().unwrap();assert_eq!(t.title,"Suggested title");assert_eq!(t.artist,"Artist");assert_eq!(t.album,"Deluxe");
  assert!(workspace(&path,WorkspaceRequest{action:"apply".into(),track_id:Some(id),revision:Some(revision),token:Some(token),fields:Some(vec!["album".into()]),..Default::default()}).is_err());
 }
 #[test] fn history_persists_and_undo_restores_origin_and_allows_sequential_undo(){
  let d=tempfile::tempdir().unwrap();let path=d.path().join("db");let db=Db::open(path.to_str().unwrap()).unwrap();let id=db.insert_track("/x.flac","Original","Artist","Album",1,1,1).unwrap();
  edit(&path,&[id],MetadataPatch{artist:Some("First".into()),..Default::default()}).unwrap();
  edit(&path,&[id],MetadataPatch{genre:Some("Electronic".into()),..Default::default()}).unwrap();drop(db);
  let db=Db::open(path.to_str().unwrap()).unwrap();let h=db.metadata_history().unwrap();let first=h[1]["id"].as_i64().unwrap();let last=h[0]["id"].as_i64().unwrap();
  assert!(db.undo_metadata_change(first).is_err());assert_eq!(db.get_track_by_id(id).unwrap().unwrap().artist,"First");
  db.undo_metadata_change(last).unwrap();db.undo_metadata_change(first).unwrap();assert_eq!(db.get_track_by_id(id).unwrap().unwrap().artist,"Artist");assert_eq!(db.metadata_override_origin(id).unwrap(),None);
 }
 #[test] fn invalid_batches_and_conflicting_undo_are_atomic(){
  let d=tempfile::tempdir().unwrap();let path=d.path().join("db");let db=Db::open(path.to_str().unwrap()).unwrap();let a=db.insert_track("/a.flac","A","Artist","Album",1,1,1).unwrap();let b=db.insert_track("/b.flac","B","Artist","Album",1,1,1).unwrap();
  assert!(edit(&path,&[a,99999],MetadataPatch{artist:Some("Wrong".into()),..Default::default()}).is_err());assert_eq!(db.metadata_history().unwrap().as_array().unwrap().len(),0);
  edit(&path,&[a,b],MetadataPatch{artist:Some("Shared".into()),..Default::default()}).unwrap();let batch=db.metadata_history().unwrap()[0]["id"].as_i64().unwrap();
  edit(&path,&[b],MetadataPatch{artist:Some("Later".into()),..Default::default()}).unwrap();assert!(db.undo_metadata_change(batch).is_err());assert_eq!(db.get_track_by_id(a).unwrap().unwrap().artist,"Shared");assert_eq!(db.get_track_by_id(b).unwrap().unwrap().artist,"Later");
 }
 #[test] fn suggestions_paginate_search_and_keep_current(){
  let d=tempfile::tempdir().unwrap();let path=d.path().join("db");let db=Db::open(path.to_str().unwrap()).unwrap();let token=candidate();for n in 0..55 {let id=db.insert_track(&format!("/{n}.flac"),&format!("Song {n}"),"Artist","Album",1,1,1).unwrap();seed(&db,id,&token);}
  let result=workspace(&path,WorkspaceRequest{action:"list".into(),offset:Some(50),..Default::default()}).unwrap();assert_eq!(result["total"],55);assert_eq!(result["items"].as_array().unwrap().len(),5);
  let item=&result["items"][0];workspace(&path,WorkspaceRequest{action:"dismiss".into(),track_id:item["track_id"].as_i64(),revision:item["revision"].as_i64(),token:Some(token),..Default::default()}).unwrap();
  let filtered=workspace(&path,WorkspaceRequest{action:"list".into(),search:Some("Song 54".into()),..Default::default()}).unwrap();assert_eq!(filtered["total"],1);
 }
 #[test] fn undo_artwork_restores_canonical_cache_without_rewriting_images(){
  let d=tempfile::tempdir().unwrap();let path=d.path().join("db");let db=Db::open(path.to_str().unwrap()).unwrap();let id=db.insert_track("/a.flac","A","Artist","Album",1,1,1).unwrap();
  let c=musicos_init::database::Connection::open(&path).unwrap();c.execute_batch("CREATE TABLE desktop_album_artwork_v1(album_id TEXT PRIMARY KEY,path TEXT);INSERT INTO desktop_album_artwork_v1 VALUES('album','old.png')").unwrap();
  db.update_track_metadata_override(&[id],&MetadataUpdate{artwork_path:Some("new.png".into()),..Default::default()}).unwrap();let batch=db.metadata_history().unwrap()[0]["id"].as_i64().unwrap();c.execute("UPDATE desktop_album_artwork_v1 SET path='new.png'",[]).unwrap();db.undo_metadata_change(batch).unwrap();assert_eq!(c.query_row("SELECT path FROM desktop_album_artwork_v1",[],|r|r.get::<_,String>(0)).unwrap(),"old.png");
 }
}
