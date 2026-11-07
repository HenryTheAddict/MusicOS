//! Album-scoped comparisons retain each recording's provenance and revision.
use anyhow::{ensure,Context,Result};use serde::{Deserialize};use serde_json::{json,Value};use sha2::{Digest,Sha256};use std::path::Path;use musicos_init::database::{Db,Connection,MetadataUpdate};
#[derive(Default,Deserialize)]pub struct Request{pub action:String,pub album_id:Option<String>,pub token:Option<String>,pub rows:Option<Vec<Selection>>,pub search:Option<String>,pub write_files:Option<bool>,pub release_id:Option<String>}
#[derive(Deserialize)]pub struct Selection{pub track_id:i64,pub fields:Vec<String>}
pub fn summary(path:&Path,library:&crate::library::Library)->Result<Value>{
 let c=Connection::open(path)?;let statuses=c.prepare("SELECT track_id,state,candidate_json FROM metadata_enrichment_status")?.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
 let mut states=serde_json::Map::new();let mut ids=Vec::new();
 for album in &library.albums{let mut count=0;let mut checked=0;let mut failed=0;let mut matched=0;
  for id in &album.track_ids{if let Some((_,state,candidate))=statuses.iter().find(|(track,_,_)|track==id){checked+=1;if state=="failed"{failed+=1}if state=="review"{if let Some(value)=candidate.as_deref().and_then(|v|serde_json::from_str::<Value>(v).ok()){
   if let Some(track)=library.tracks.iter().find(|t|t.id==*id){for (field,current) in [("title",json!(track.title)),("artist",json!(track.artist)),("album",json!(track.album)),("year",json!(track.year)),("track_number",json!(track.track_number)),("disc_number",json!(track.disc_number))]{if !value[field].is_null()&&value[field]!=current{count+=1;}}
    if value["artwork_path"].as_str().is_some_and(|p|Some(p)!=album.cover.as_deref()){count+=1;}
   }matched+=1;
  }}}}
  let state=if count>0{"changes_available"}else if failed>0{"lookup_failed"}else if checked==0{"not_checked"}else if matched>0{"already_matches"}else{"no_confident_match"};
  if count>0{ids.push(album.id.clone());}states.insert(album.id.clone(),json!({"state":state,"changed_fields":count,"checked":checked,"total":album.track_ids.len()}));
 }Ok(json!({"album_ids":ids,"states":states}))
}

pub fn list(path: &Path, library: &crate::library::Library, search: &str) -> Result<Value> {
    let needle = search.to_lowercase();
    let c = Connection::open(path)?;
    let pending = c.prepare("SELECT track_id FROM metadata_enrichment_status WHERE state='review'")?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
    let db = Db::open(&path.to_string_lossy())?;
    let albums = library.albums.iter()
        .filter(|album| album.track_ids.iter().any(|id| pending.contains(id))
            && format!("{} {}", album.title, album.artist).to_lowercase().contains(&needle))
        .map(|album| comparison(path,&db, &c, library, album))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"albums": albums}))
}

pub fn detail(path: &Path, library: &crate::library::Library, id: &str) -> Result<Value> {
    let album = library.albums.iter().find(|album| album.id == id)
        .context("This album is no longer in the library")?;
    let db = Db::open(&path.to_string_lossy())?;
    let c = Connection::open(path)?;
    comparison(path,&db, &c, library, album)
}

fn comparison(path:&Path,db: &Db, c: &Connection, library: &crate::library::Library, album: &crate::library::Album) -> Result<Value> {
    use rusqlite::OptionalExtension;
    let mut query = c.prepare("SELECT state, candidate_json FROM metadata_enrichment_status WHERE track_id=?")?;
    let mut rows = Vec::with_capacity(album.track_ids.len());
    for id in &album.track_ids {
        let mut current = db.metadata_values(*id)?;
        if current["album_artist"].is_null() {
            current["album_artist"] = library.tracks.iter().find(|track| track.id == *id)
                .and_then(|track| track.album_artist.clone()).into();
        }
        let (status, cached): (String, Option<String>) = query
            .query_row([id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?
            .unwrap_or_else(|| ("unchecked".into(), None));
        let candidate_token = cached.filter(|_| status == "review");
        let candidate = candidate_token.as_deref().map(serde_json::from_str::<Value>).transpose()?;
        let artwork = candidate.as_ref().and_then(|value| value["artwork_path"].as_str());
        let art_identity = artwork.and_then(|path| std::fs::metadata(path).ok())
            .map(|metadata| format!("{}:{:?}", metadata.len(), metadata.modified().ok()));
        let art_resolution = artwork.and_then(|path| image::image_dimensions(path).ok())
            .map(|(width, height)| [width, height]);
        let mut fields=Vec::new();if let Some(value)=&candidate{for field in ["title","artist","album","album_artist","year","track_number","disc_number"]{if !value[field].is_null()&&value[field]!=current[field]{fields.push(field)}}}
        let art_comparison=match (album.cover.as_deref(),artwork){(Some(old),Some(new))=>art_comparison(old,new),(_,Some(_))=>"new",_=>"none"};
        if artwork.is_some()&&art_comparison!="identical"{fields.push("artwork");}
        rows.push(json!({"changed_fields":fields,"art_comparison":art_comparison,"track_id": id, "current": current, "candidate": candidate,
            "candidate_token": candidate_token, "status": status, "art_identity": art_identity, "art_resolution": art_resolution}));
    }
    let token = format!("{:x}", Sha256::digest(serde_json::to_vec(&json!([album.id, rows]))?));
    let resolution = album.cover.as_deref().and_then(|path| image::image_dimensions(path).ok())
        .map(|(width, height)| [width, height]);
    let release_choices=album.track_ids.first().map(|id|musicos_desktop::local::enrichment::saved_release_choices(path,*id)).transpose()?.unwrap_or(json!({"choices":[]}));
    let changed=rows.iter().map(|r|r["changed_fields"].as_array().map_or(0,Vec::len)).sum::<usize>();
    let status=if changed>0{"changes_available"}else if rows.iter().any(|r|r["status"]=="failed"){"lookup_failed"}else if rows.iter().all(|r|r["status"]=="unchecked"){"not_checked"}else if rows.iter().any(|r|!r["candidate"].is_null()){"already_matches"}else{"no_confident_match"};
    Ok(json!({"status":status,"changed_fields":changed,"release_choices":release_choices,"album_id": album.id, "title": album.title, "artist": album.artist,
        "resolution": resolution, "cover": album.cover, "rows": rows, "token": token}))
}

fn art_comparison(old:&str,new:&str)->&'static str{
 if old==new{return "identical"}
 let loaded=(||->Option<bool>{let a=image::open(old).ok()?.thumbnail_exact(32,32).to_rgb8();let b=image::open(new).ok()?.thumbnail_exact(32,32).to_rgb8();Some(a.as_raw().iter().zip(b.as_raw()).map(|(a,b)|(*a as i32-*b as i32).unsigned_abs() as u64).sum::<u64>()<32*32*3*2)})();
 if loaded==Some(true){return "identical"}
 match (image::image_dimensions(old),image::image_dimensions(new)){(Ok((a,b)),Ok((c,d)))if ((a as f32/b as f32)-(c as f32/d as f32)).abs()>0.03=>"different_crop",_=>"different"}
}

pub fn apply(path:&Path,library:&crate::library::Library,request:Request)->Result<Value>{let id=request.album_id.context("Choose an album")?;let album=detail(path,library,&id)?;ensure!(album["token"].as_str()==request.token.as_deref(),"This album changed. Refresh the comparison.");let chosen=request.rows.unwrap_or_default();ensure!(!chosen.is_empty(),"Choose changes to apply");let mut changes=std::collections::BTreeMap::<i64,MetadataUpdate>::new();let mut art=None;
 for selection in &chosen{let row=album["rows"].as_array().unwrap().iter().find(|r|r["track_id"]==selection.track_id).context("Song left this album")?;let candidate=&row["candidate"];ensure!(!candidate.is_null(),"No match was suggested for this song");let patch=changes.entry(selection.track_id).or_default();for field in &selection.fields{match field.as_str(){"title"=>patch.title=candidate["title"].as_str().map(str::to_owned),"artist"=>patch.artist=candidate["artist"].as_str().map(str::to_owned),"album"=>patch.album=candidate["album"].as_str().map(str::to_owned),"album_artist"=>patch.album_artist=candidate["album_artist"].as_str().map(str::to_owned),"year"=>patch.year=candidate["year"].as_i64().map(|n|n as i32),"track_number"=>patch.track_number=candidate["track_number"].as_i64().map(|n|n as i32),"disc_number"=>patch.disc_number=candidate["disc_number"].as_i64().map(|n|n as i32),"artwork"=>{let path=candidate["artwork_path"].as_str().context("No artwork was suggested")?;let dimensions=image::image_dimensions(path)?;ensure!(dimensions.0 as u64*dimensions.1 as u64<=36_000_000,"Suggested image is too large");image::io::Reader::open(path)?.with_guessed_format()?.decode()?;art=Some(path.to_owned());},_=>anyhow::bail!("Unsupported review field")}}}
 if let Some(art)=art{for row in album["rows"].as_array().unwrap(){changes.entry(row["track_id"].as_i64().unwrap()).or_default().artwork_path=Some(art.clone());}}
 let reviews=album["rows"].as_array().unwrap().iter().filter(|r|changes.contains_key(&r["track_id"].as_i64().unwrap())).map(|r|(r["track_id"].as_i64().unwrap(),r["current"]["revision"].as_i64().unwrap(),r["candidate_token"].as_str().unwrap_or("").to_owned())).collect::<Vec<_>>();let changes=changes.into_iter().collect::<Vec<_>>();
 let (count,history_id)=crate::file_tags::apply(path,&changes,&reviews,request.write_files.unwrap_or(false))?;Ok(json!({"count":count,"history_id":history_id}))}
#[cfg(test)]mod tests{use super::*;
 #[test]fn selected_fields_and_album_art_are_one_stale_checked_undoable_change(){let temp=tempfile::tempdir().unwrap();let path=temp.path().join("db");let db=Db::open(&path.to_string_lossy()).unwrap();let a=db.insert_track("/a.flac","A","Artist","Record",1,2,1).unwrap();let b=db.insert_track("/b.flac","B","Artist","Record",2,2,1).unwrap();let art=temp.path().join("new.png");image::RgbaImage::new(30,30).save(&art).unwrap();let token=serde_json::to_string(&json!({"title":"Corrected","artist":"Wrong artist","album":"Record","confidence":0.9,"artwork_path":art,"track_number":1})).unwrap();let c=Connection::open(&path).unwrap();c.execute("INSERT INTO metadata_enrichment_status(track_id,state,candidate_json,checked_at) VALUES(?,'review',?,1)",rusqlite::params![a,token]).unwrap();let library=crate::library::group(musicos_desktop::models::LibrarySnapshot{tracks:db.get_tracks().unwrap().into_iter().map(musicos_desktop::local::library::track_from_init).collect(),..Default::default()});let album=list(&path,&library,"").unwrap()["albums"][0].clone();let request=||Request{action:"apply".into(),album_id:album["album_id"].as_str().map(str::to_owned),token:album["token"].as_str().map(str::to_owned),rows:Some(vec![Selection{track_id:a,fields:vec!["title".into(),"artwork".into()]}]),..Default::default()};let result=apply(&path,&library,request()).unwrap();assert_eq!(result["count"],2);assert_eq!(db.get_track_by_id(a).unwrap().unwrap().artist,"Artist");assert_eq!(db.get_track_by_id(a).unwrap().unwrap().title,"Corrected");assert_eq!(db.get_track_by_id(b).unwrap().unwrap().cover_art_path.as_deref(),Some(art.to_str().unwrap()));assert!(apply(&path,&library,request()).is_err());db.undo_metadata_change(result["history_id"].as_i64().unwrap()).unwrap();assert_eq!(db.get_track_by_id(a).unwrap().unwrap().title,"A");assert!(db.get_track_by_id(b).unwrap().unwrap().cover_art_path.is_none());}
 #[test]
 fn cover_only_preserves_album_identity_and_tags_in_library_and_original_files(){
  use lofty::{file::TaggedFileExt,tag::Accessor};
  for write_files in [false,true]{
   let temp=tempfile::tempdir().unwrap();let path=temp.path().join("db");let db=Db::open(&path.to_string_lossy()).unwrap();
   let art=temp.path().join("new.png");image::RgbaImage::new(30,30).save(&art).unwrap();
   let mut ids=Vec::new();let mut originals=Vec::new();
   for n in 1..=2{
    let audio=temp.path().join(format!("{n}.flac"));
    assert!(std::process::Command::new("/opt/homebrew/bin/ffmpeg").args(["-v","error","-f","lavfi","-i","sine=frequency=440:duration=0.1","-metadata","title=Original","-metadata","artist=Artist","-metadata","album=Record","-metadata","album_artist=Artist","-metadata","date=2000","-metadata","track=1","-metadata","disc=1","-metadata","genre=Rock"]).arg(&audio).status().unwrap().success());
    let id=db.insert_track(&audio.to_string_lossy(),"Original","Artist","Record",n,2,1).unwrap();ids.push(id);originals.push(std::fs::read(audio).unwrap());
   }
   let outside=db.insert_track("/other.flac","Other song","Artist","Other album",1,2,1).unwrap();let outside_before=db.metadata_values(outside).unwrap();
   let group=||crate::library::group(musicos_desktop::models::LibrarySnapshot{tracks:db.get_tracks().unwrap().into_iter().map(musicos_desktop::local::library::track_from_init).collect(),..Default::default()});
   let library=group();let album=list(&path,&library,"").unwrap();assert_eq!(album["albums"].as_array().unwrap().len(),0);
   let token=serde_json::to_string(&json!({"title":"Wrong title","artist":"Wrong artist","album":"Different edition","album_artist":"Wrong album artist","year":2025,"disc_number":9,"track_number":99,"artwork_path":art})).unwrap();
   let c=Connection::open(&path).unwrap();for id in &ids{c.execute("INSERT INTO metadata_enrichment_status(track_id,state,candidate_json,checked_at) VALUES(?,'review',?,1)",rusqlite::params![id,token]).unwrap();}
   let album=list(&path,&library,"").unwrap()["albums"][0].clone();let before=ids.iter().map(|id|db.metadata_values(*id).unwrap()).collect::<Vec<_>>();
   let request=||Request{action:"apply".into(),album_id:album["album_id"].as_str().map(str::to_owned),token:album["token"].as_str().map(str::to_owned),rows:Some(vec![Selection{track_id:ids[0],fields:vec!["artwork".into()]}]),write_files:Some(write_files),..Default::default()};
   let result=apply(&path,&library,request()).unwrap();assert_eq!(result["count"],2);assert!(apply(&path,&library,request()).is_err());
   let after=group();assert_eq!(library.albums.iter().map(|a|(&a.id,&a.track_ids)).collect::<Vec<_>>(),after.albums.iter().map(|a|(&a.id,&a.track_ids)).collect::<Vec<_>>());
   for (index,id) in ids.iter().enumerate(){
    let track=db.metadata_values(*id).unwrap();for key in ["title","artist","album","album_artist","year","genre","disc_number","track_number"]{assert_eq!(before[index][key],track[key],"{key} changed in a cover-only update");}
    assert_eq!(track["cover_art_path"],art.to_str().unwrap());
    let audio=temp.path().join(format!("{}.flac",index+1));let tagged=lofty::read_from_path(&audio).unwrap();let tag=tagged.primary_tag().unwrap();
    assert_eq!(tag.title().as_deref(),Some("Original"));assert_eq!(tag.artist().as_deref(),Some("Artist"));assert_eq!(tag.album().as_deref(),Some("Record"));assert_eq!(tag.year(),Some(2000));assert_eq!(tag.track(),Some(1));assert_eq!(tag.disk(),Some(1));assert_eq!(tag.genre().as_deref(),Some("Rock"));
    if write_files{assert_eq!(tag.pictures().len(),1);}else{assert_eq!(std::fs::read(audio).unwrap(),originals[index]);}
   }
   assert_eq!(db.metadata_values(outside).unwrap(),outside_before);
   crate::file_tags::undo(&path,result["history_id"].as_i64().unwrap()).unwrap();
   for (index,id) in ids.iter().enumerate(){assert!(db.get_track_by_id(*id).unwrap().unwrap().cover_art_path.is_none());assert_eq!(std::fs::read(temp.path().join(format!("{}.flac",index+1))).unwrap(),originals[index]);}
  }
 }

}
