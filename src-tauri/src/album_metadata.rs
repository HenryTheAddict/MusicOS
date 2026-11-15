//! Album-scoped comparisons retain each recording's provenance and revision.
use anyhow::{ensure,Context,Result};use serde::{Deserialize};use serde_json::{json,Value};use sha2::{Digest,Sha256};use std::path::Path;use musicos_init::database::{Db,Connection,MetadataUpdate};
#[derive(Default,Deserialize)]pub struct Request{
    pub action:String,pub album_id:Option<String>,pub token:Option<String>,pub rows:Option<Vec<Selection>>,
    pub search:Option<String>,pub write_files:Option<bool>,pub release_id:Option<String>,
    pub image_path:Option<String>,pub image_data:Option<String>,pub cover_id:Option<String>,
    pub positions:Option<Vec<Position>>,
}
#[derive(Deserialize)]pub struct Selection{pub track_id:i64,pub fields:Vec<String>}
#[derive(Deserialize)]pub struct Position{
    track_id:i64,disc_number:Option<i32>,track_number:Option<i32>,
}

const MAX_COVER_BYTES:usize=20*1024*1024;

fn cover_directory(db:&Path)->std::path::PathBuf{
    db.with_file_name("library-assets").join("imported-covers")
}

fn inspect_cover(bytes:&[u8])->Result<(&'static str,[u32;2])>{
    ensure!(!bytes.is_empty()&&bytes.len()<=MAX_COVER_BYTES,"Choose an image smaller than 20 MB");
    let mut reader=image::io::Reader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let suffix=match reader.format(){
        Some(image::ImageFormat::Jpeg)=>"jpg",Some(image::ImageFormat::Png)=>"png",
        Some(image::ImageFormat::WebP)=>"webp",_=>anyhow::bail!("Choose a JPEG, PNG, or WebP image"),
    };
    let mut limits=image::io::Limits::default();
    limits.max_image_width=Some(8192);
    limits.max_image_height=Some(8192);
    limits.max_alloc=Some(128*1024*1024);
    reader.limits(limits);
    let decoded=reader.decode().context("The cover could not be decoded or exceeds the image size limit")?;
    Ok((suffix,[decoded.width(),decoded.height()]))
}

pub fn stage_cover(db:&Path,request:&Request)->Result<Value>{
    use base64::{Engine,engine::general_purpose::STANDARD};
    use std::io::{Read,Write};
    let bytes=match (&request.image_path,&request.image_data){
        (Some(path),None)=>{
            let file=std::fs::File::open(path).context("The image could not be opened")?;
            ensure!(file.metadata()?.is_file(),"Choose an image file");
            let mut bytes=Vec::new();
            file.take((MAX_COVER_BYTES+1) as u64).read_to_end(&mut bytes)?;
            bytes
        }
        (None,Some(data))=>{
            ensure!(data.len()<=MAX_COVER_BYTES.div_ceil(3)*4,"Choose an image smaller than 20 MB");
            STANDARD.decode(data).context("The image could not be read")?
        }
        _=>anyhow::bail!("Choose one image"),
    };
    let (suffix,size)=inspect_cover(&bytes)?;
    let id=format!("{:x}.{suffix}",Sha256::digest(&bytes));
    let directory=cover_directory(db);
    std::fs::create_dir_all(&directory)?;
    let path=directory.join(&id);
    if !path.is_file(){
        let temporary=directory.join(format!(".{}-{:016x}.tmp",std::process::id(),rand::random::<u64>()));
        let result=(||->Result<()>{
            let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary,&path)?;
            Ok(())
        })();
        if result.is_err(){let _=std::fs::remove_file(temporary);}
        result?;
    }
    Ok(json!({"imported_cover":{"id":id,"path":path,"size":size}}))
}

pub fn apply_manual(db:&Path,library:&crate::library::Library,request:Request)->Result<Value>{
    use std::collections::BTreeMap;
    let album=detail(db,library,request.album_id.as_deref().context("Choose an album")?)?;
    ensure!(album["token"].as_str()==request.token.as_deref(),"This album changed. Review it again before saving.");
    let rows=album["rows"].as_array().context("Album songs unavailable")?;
    let mut changes=BTreeMap::new();
    match request.action.as_str(){
        "apply_cover"=>{
            let id=request.cover_id.as_deref().context("Choose an image")?;
            let (hash,suffix)=id.split_once('.').context("Invalid image reference")?;
            ensure!(hash.len()==64&&hash.bytes().all(|b|b.is_ascii_hexdigit())&&matches!(suffix,"jpg"|"png"|"webp"),"Invalid image reference");
            let path=cover_directory(db).join(id);
            ensure!(path.metadata()?.len()<=MAX_COVER_BYTES as u64,"The image exceeds the size limit");
            let bytes=std::fs::read(&path)?;
            ensure!(format!("{:x}",Sha256::digest(&bytes))==hash,"The image changed. Choose it again.");
            let path=path.to_string_lossy().into_owned();
            for row in rows {
                changes.insert(row["track_id"].as_i64().unwrap(),MetadataUpdate{artwork_path:Some(path.clone()),..Default::default()});
            }
        }
        "apply_positions"=>{
            for position in request.positions.context("Enter track positions")?{
                let row=rows.iter().find(|row|row["track_id"]==position.track_id).context("A song left this release")?;
                let mut patch=MetadataUpdate::default();
                for (field,value,target) in [
                    ("disc_number",position.disc_number,&mut patch.disc_number),
                    ("track_number",position.track_number,&mut patch.track_number),
                ]{
                    if let Some(value)=value{
                        ensure!((0..=9999).contains(&value),"Disc and track numbers must be between 0 and 9999");
                        if row["current"][field].as_i64().unwrap_or(0)!=i64::from(value){*target=Some(value);}
                    }
                }
                ensure!(patch.disc_number.is_some()||patch.track_number.is_some(),"A reviewed position has not changed");
                ensure!(changes.insert(position.track_id,patch).is_none(),"A song was listed twice");
            }
        }
        _=>anyhow::bail!("Unknown album editing action"),
    }
    ensure!(!changes.is_empty(),"Choose changes to save");
    let reviews=rows.iter().map(|row|(row["track_id"].as_i64().unwrap(),row["current"]["revision"].as_i64().unwrap(),String::new())).collect::<Vec<_>>();
    let changes=changes.into_iter().collect::<Vec<_>>();
    let (count,history_id)=crate::file_tags::apply(db,&changes,&reviews,request.write_files.unwrap_or(false))?;
    Ok(json!({"count":count,"history_id":history_id}))
}

#[cfg(test)]
mod manual_tests{
    use super::*;
    use musicos_desktop::{local::library::track_from_init,models::LibrarySnapshot};

    #[test]
    fn manual_cover_and_track_order_are_revision_checked_and_keep_editions_separate(){
        let temporary=tempfile::tempdir().unwrap();
        let path=temporary.path().join("library.sqlite3");
        let database=Db::open(path.to_str().unwrap()).unwrap();
        let mut originals=Vec::new();
        for (index,title,release,disc) in [
            (0,"Knock Yourself Out XD","SMILE! :D",1),
            (1,"Cheerleader (Nitepunk Remix)","SMILE! :D plus+",1),
            (2,"Easier to Love You (Vertile Remix)","SMILE! :D plus+",1),
            (3,"Another remix","SMILE! :D plus+",2),
        ]{
            let audio=temporary.path().join(format!("{index}.flac"));
            let bytes=format!("original {index}").into_bytes();
            std::fs::write(&audio,&bytes).unwrap();
            originals.push((audio.clone(),bytes));
            let id=database.insert_track_with_number(audio.to_str().unwrap(),title,"Porter Robinson",release,1,200,12,1).unwrap();
            Connection::open(&path).unwrap().execute("UPDATE tracks SET disc_number=? WHERE id=?",rusqlite::params![disc,id]).unwrap();
        }
        let library=||crate::library::group(LibrarySnapshot{tracks:database.get_tracks().unwrap().into_iter().map(track_from_init).collect(),..Default::default()});
        let before=library();
        let original=before.albums.iter().find(|album|album.title=="SMILE! :D").unwrap();
        let remix=before.albums.iter().find(|album|album.title=="SMILE! :D plus+").unwrap();
        let source=temporary.path().join("source.png");
        image::RgbImage::new(40,24).save(&source).unwrap();
        let prepared=stage_cover(&path,&Request{image_path:Some(source.to_string_lossy().into_owned()),..Default::default()}).unwrap();
        assert_eq!(prepared["imported_cover"]["size"],json!([40,24]));
        std::fs::remove_file(&source).unwrap();
        std::fs::write(&source,b"invalid").unwrap();
        assert!(stage_cover(&path,&Request{image_path:Some(source.to_string_lossy().into_owned()),..Default::default()}).is_err());
        let oversized=std::fs::File::create(source).unwrap();
        oversized.set_len((MAX_COVER_BYTES+1) as u64).unwrap();
        assert!(inspect_cover(&vec![0;MAX_COVER_BYTES+1]).is_err());
        let cover=prepared["imported_cover"]["id"].as_str().unwrap();
        let revision=detail(&path,&before,&original.id).unwrap()["token"].as_str().unwrap().to_owned();
        let request=||Request{action:"apply_cover".into(),album_id:Some(original.id.clone()),token:Some(revision.clone()),cover_id:Some(cover.into()),..Default::default()};
        let applied=apply_manual(&path,&before,request()).unwrap();
        assert!(apply_manual(&path,&before,request()).is_err());
        let mut reopened=library();
        crate::library::canonical_artwork(&path,&mut reopened).unwrap();
        assert!(reopened.albums.iter().find(|album|album.id==original.id).unwrap().cover.as_ref().is_some_and(|path|Path::new(path).is_file()));
        assert!(reopened.albums.iter().find(|album|album.id==remix.id).unwrap().cover.is_none());
        database.undo_metadata_change(applied["history_id"].as_i64().unwrap()).unwrap();
        let mut reopened=library();
        crate::library::canonical_artwork(&path,&mut reopened).unwrap();
        assert!(reopened.albums.iter().find(|album|album.id==original.id).unwrap().cover.is_none());
        let current=detail(&path,&reopened,&remix.id).unwrap();
        let rows=current["rows"].as_array().unwrap();
        let pos=rows.iter().find(|row|row["current"]["title"]=="Easier to Love You (Vertile Remix)").unwrap()["track_id"].as_i64().unwrap();
        let edited=apply_manual(&path,&reopened,Request{action:"apply_positions".into(),album_id:Some(remix.id.clone()),token:current["token"].as_str().map(str::to_owned),
            positions:Some(vec![Position{track_id:pos,disc_number:Some(1),track_number:Some(2)}]),..Default::default()}).unwrap();
        assert_eq!(edited["count"],1);
        assert_eq!(database.metadata_values(pos).unwrap()["track_number"],2);
        for (audio,bytes) in originals{assert_eq!(std::fs::read(audio).unwrap(),bytes);}
    }
}
pub fn summary(path:&Path,library:&crate::library::Library)->Result<Value>{
    use std::collections::{HashMap,HashSet};
    let c = Connection::open(path)?;
    let statuses = c.prepare("SELECT track_id,state,candidate_json FROM metadata_enrichment_status")?
        .query_map([],|r|Ok((r.get::<_,i64>(0)?,(r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?))))?
        .collect::<rusqlite::Result<HashMap<_,_>>>()?;
    let tracks = library.tracks.iter().map(|track|(track.id,track)).collect::<HashMap<_,_>>();
    let choices_exist = c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='desktop_release_choices_v1')",[],|r|r.get::<_,bool>(0))?;
    let choices = if choices_exist {
        c.prepare("SELECT track_id FROM desktop_release_choices_v1 WHERE selected IS NULL AND choices <> '[]'")?
            .query_map([],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<HashSet<_>>>()?
    } else { HashSet::new() };
    let mut states = serde_json::Map::new();
    let mut ids = Vec::new();
    for album in &library.albums {
        let (mut changed,mut checked,mut failed,mut matched) = (0,0,0,0);
        let mut different_cover = false;
        for id in &album.track_ids {
            let Some((state,candidate)) = statuses.get(id) else { continue };
            checked += 1;
            failed += usize::from(state=="failed");
            if state=="applied" { matched += 1; }
            if state!="review" { continue; }
            let Some(value) = candidate.as_deref().and_then(|v|serde_json::from_str::<Value>(v).ok()) else { continue };
            matched += 1;
            if let Some(track) = tracks.get(id) {
                for (field,current) in [("title",json!(track.title)),("artist",json!(track.artist)),("album",json!(track.album)),
                    ("album_artist",json!(track.album_artist)),("year",json!(track.year)),("track_number",json!(track.track_number)),("disc_number",json!(track.disc_number))] {
                    if !value[field].is_null() && value[field]!=current { changed += 1; }
                }
            }
            if let Some(proposed) = value["artwork_path"].as_str().filter(|p|Path::new(p).is_file()) {
                different_cover |= Some(proposed)!=album.cover.as_deref();
            }
        }
        let needs_choice = album.track_ids.iter().any(|id|choices.contains(id));
        let state = if changed>0 || different_cover {"changes_available"} else if needs_choice {"choose_release"}
            else if failed>0 {"lookup_failed"} else if checked==0 {"not_checked"}
            else if matched==album.track_ids.len() {"already_matches"}
            else if matched>0 {"partially_matched"} else {"no_confident_match"};
        if changed>0 || different_cover || needs_choice { ids.push(album.id.clone()); }
        states.insert(album.id.clone(),json!({"state":state,"changed_fields":changed,"cover_review":different_cover,"checked":checked,"matched":matched,"total":album.track_ids.len()}));
    }
    Ok(json!({"album_ids":ids,"states":states}))
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
    let mut query = c.prepare("SELECT state, candidate_json, error FROM metadata_enrichment_status WHERE track_id=?")?;
    let mut rows = Vec::with_capacity(album.track_ids.len());
    let mut cover_comparisons = std::collections::HashMap::new();
    for id in &album.track_ids {
        let mut current = db.metadata_values(*id)?;
        if current["album_artist"].is_null() {
            current["album_artist"] = library.tracks.iter().find(|track| track.id == *id)
                .and_then(|track| track.album_artist.clone()).into();
        }
        let (status, cached, error): (String, Option<String>, Option<String>) = query
            .query_row([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?
            .unwrap_or_else(|| ("unchecked".into(), None, None));
        let candidate_token = cached.filter(|_| status == "review");
        let candidate = candidate_token.as_deref().map(serde_json::from_str::<Value>).transpose()?;
        let artwork = candidate.as_ref().and_then(|value| value["artwork_path"].as_str()).filter(|path|Path::new(path).is_file());
        let art_identity = artwork.and_then(|path| std::fs::metadata(path).ok())
            .map(|metadata| format!("{}:{:?}", metadata.len(), metadata.modified().ok()));
        let art_resolution = artwork.and_then(|path| image::image_dimensions(path).ok())
            .map(|(width, height)| [width, height]);
        let mut fields=Vec::new();if let Some(value)=&candidate{for field in ["title","artist","album","album_artist","year","track_number","disc_number"]{if !value[field].is_null()&&value[field]!=current[field]{fields.push(field)}}}
        let art_comparison = match (album.cover.as_deref(), artwork) {
            (Some(old), Some(new)) => *cover_comparisons
                .entry(new.to_owned())
                .or_insert_with(|| art_comparison(old, new)),
            (_, Some(_)) => "new",
            _ => "none",
        };
        if artwork.is_some()&&art_comparison!="identical"{fields.push("artwork");}
        rows.push(json!({"changed_fields":fields,"art_comparison":art_comparison,"track_id": id, "current": current, "candidate": candidate,
            "candidate_token": candidate_token, "status": status, "lookup_error": error, "art_identity": art_identity, "art_available": artwork.is_some(), "art_resolution": art_resolution}));
    }
    let token = format!("{:x}", Sha256::digest(serde_json::to_vec(&json!([album.id, album.cover, rows]))?));
    let resolution = album.cover.as_deref().and_then(|path| image::image_dimensions(path).ok())
        .map(|(width, height)| [width, height]);
    let release_choices=album.track_ids.first().map(|id|musicos_desktop::local::enrichment::saved_release_choices(path,*id)).transpose()?.unwrap_or(json!({"choices":[]}));
    let cover_change=rows.iter().any(|r| r["changed_fields"].as_array().is_some_and(|f|f.iter().any(|f|f=="artwork")));
    let changed=usize::from(cover_change)+rows.iter().flat_map(|r|r["changed_fields"].as_array().into_iter().flatten()).filter(|f|*f!="artwork").count();
    let matched=rows.iter().filter(|r|!r["candidate"].is_null() || r["status"]=="applied").count();
    let needs_choice=release_choices["selected"].is_null() && release_choices["choices"].as_array().is_some_and(|choices|!choices.is_empty());
    let status=if changed>0 {"changes_available"} else if needs_choice {"choose_release"}
        else if rows.iter().any(|r|r["status"]=="failed") {"lookup_failed"}
        else if rows.iter().all(|r|r["status"]=="unchecked") {"not_checked"}
        else if matched==rows.len() {"already_matches"} else if matched>0 {"partially_matched"} else {"no_confident_match"};
    Ok(json!({"status":status,"matched_tracks":matched,"changed_fields":changed,"release_choices":release_choices,"album_id": album.id, "title": album.title, "artist": album.artist,
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
