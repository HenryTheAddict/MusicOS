use anyhow::{ensure, Context, Result};
use musicos_desktop::{local::{LibraryRepository, LocalLibraryRepository, smart_playlists::SmartPlaylistDef}, storage::{atomic_write_json}};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spark { pub id: String, pub definition: SmartPlaylistDef }
#[derive(Serialize)]
pub struct Collection { pub updated_at:i64, pub id: String, pub name: String, pub track_ids: Vec<i64>, pub spark: Option<SmartPlaylistDef>, pub kind: String, pub description: String, pub artwork: Option<String>, pub artwork_choice: Option<String> }
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request { pub action: String, pub id: Option<String>, pub name: Option<String>, pub track_ids: Option<Vec<i64>>, pub definition: Option<SmartPlaylistDef>, pub description: Option<String>, pub artwork_choice: Option<String> }
fn presentation(repo:&LocalLibraryRepository)->Result<rusqlite::Connection>{
    let connection=rusqlite::Connection::open(repo.db_path())?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS desktop_playlist_details_v1 (playlist_id INTEGER PRIMARY KEY REFERENCES playlists(id) ON DELETE CASCADE, description TEXT NOT NULL DEFAULT '', artwork TEXT);")?;
    connection.execute_batch("CREATE TABLE IF NOT EXISTS desktop_collection_art_v1(collection_id TEXT PRIMARY KEY, choice TEXT NOT NULL)")?;
    connection.execute_batch("CREATE TABLE IF NOT EXISTS desktop_collection_modified_v1(id TEXT PRIMARY KEY,updated_at INTEGER NOT NULL)")?;
    Ok(connection)
}
const COVERS:&[&str]=&["Apart","Baltimore","Beach","Biggest","Bird","Cave","Clouds","Crowd","Flamingo","Flock","Hills","Hungry","Lake","Lama","Luck","Monk","Overcast","Pier","Pup","Rocks","SaltyRoads","Sand","Sea","State","Stop","Towers","Vacate","butterfly","poletower","skate","sky","tritower"];
fn validate_cover(choice:&str)->Result<()> {
    ensure!(choice=="collage"||choice=="custom"||COVERS.iter().any(|name|choice==format!("playlist/{name}"))||(1..=9).any(|n|choice==format!("auto/{n}")),"Choose an available collection cover");Ok(())
}
fn cover_choice(repo:&LocalLibraryRepository,id:&str)->Result<Option<String>> {
    use rusqlite::OptionalExtension;
    Ok(presentation(repo)?.query_row("SELECT choice FROM desktop_collection_art_v1 WHERE collection_id=?",[id],|r|r.get(0)).optional()?)
}
fn save_cover(repo:&LocalLibraryRepository,id:&str,choice:&str)->Result<()> {
    validate_cover(choice)?;
    presentation(repo)?.execute("INSERT INTO desktop_collection_art_v1 VALUES (?,?) ON CONFLICT(collection_id) DO UPDATE SET choice=excluded.choice",rusqlite::params![id,choice])?;Ok(())
}
fn spark_path(db: &Path) -> std::path::PathBuf { db.with_extension("sparklists.json") }
fn read_sparks(db: &Path) -> Result<Vec<Spark>> {
    let path=spark_path(db);
    if !path.exists() { return Ok(vec![]); }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
pub fn list(repo: &LocalLibraryRepository) -> Result<Vec<Collection>> {
    let mut items=Vec::new();
    let connection=presentation(repo)?;
    let db=musicos_init::database::Db::open(&repo.db_path().to_string_lossy())?;
    for p in repo.playlists()? {
        use rusqlite::OptionalExtension;
        let (description,artwork)=connection.query_row("SELECT description,artwork FROM desktop_playlist_details_v1 WHERE playlist_id=?1",[p.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?))).optional()?.unwrap_or_default();
        items.push(Collection {updated_at:0,id:format!("playlist/{}",p.id),name:p.name,track_ids:db.get_playlist_tracks(p.id)?.iter().map(|t|t.id).collect(),spark:None,kind:"manual".into(),description,artwork,artwork_choice:cover_choice(repo,&format!("playlist/{}",p.id))?});
    }
    let tracks=repo.snapshot(&Default::default())?.tracks;
    for s in read_sparks(repo.db_path())? { items.push(Collection { updated_at:0,id:s.id.clone(), name:s.definition.name.clone(), track_ids:s.definition.evaluate(&tracks).iter().map(|t|t.id).collect(), spark:Some(s.definition),kind:"rule".into(),description:String::new(),artwork:None,artwork_choice:cover_choice(repo,&s.id)? }); }
    for item in crate::audio_collections::list(repo.db_path())? {items.push(Collection{updated_at:item.updated_at,id:item.id,name:item.name,track_ids:item.track_ids,spark:None,kind:"audio".into(),description:item.description,artwork:None,artwork_choice:Some(item.artwork_choice)});}
    for item in &mut items {
        use rusqlite::OptionalExtension;
        let saved=connection.query_row("SELECT updated_at FROM desktop_collection_modified_v1 WHERE id=?",[&item.id],|r|r.get::<_,i64>(0)).optional()?;
        let created=item.id.split('/').next_back().and_then(|v|v.parse::<i128>().ok()).filter(|v|*v>1_000_000_000_000).map(|v|(v/1_000_000_000)as i64).unwrap_or(0);
        let manual=item.id.strip_prefix("playlist/").and_then(|id|connection.query_row("SELECT updated_at FROM playlists WHERE id=?",[id],|r|r.get::<_,i64>(0)).ok()).unwrap_or(0);
        item.updated_at=item.updated_at.max(saved.unwrap_or(manual.max(created)));
    }
    Ok(items)
}
fn valid_name(name: &str) -> Result<&str> { let name=name.trim(); ensure!(!name.is_empty() && name.chars().count()<=120,"Use a name between 1 and 120 characters"); Ok(name) }
pub fn file_action(repo:&LocalLibraryRepository,action:&str,id:Option<&str>,path:&Path)->Result<serde_json::Value>{
    if action=="import" {
        ensure!(path.extension().and_then(|v|v.to_str()).is_some_and(|v|v.eq_ignore_ascii_case("m3u")||v.eq_ignore_ascii_case("m3u8")),"Choose an M3U or M3U8 playlist");
        ensure!(std::fs::metadata(path)?.len()<=10*1024*1024,"Playlist file is too large");
        let result=musicos_desktop::local::playlists::import_m3u(repo,path)?;
        return Ok(serde_json::json!({"items":list(repo)?,"missing":result.missing,"imported":result.imported}));
    }
    let id=id.and_then(|s|s.strip_prefix("playlist/")).and_then(|s|s.parse::<i64>().ok()).ok_or_else(||anyhow::anyhow!("Choose a manual playlist"))?;
    ensure!(repo.playlists()?.iter().any(|p|p.id==id),"Playlist no longer exists");
    match action {
        "export"=>musicos_desktop::local::playlists::export_m3u(repo,id,path)?,
        "artwork"=>{
            use sha2::{Digest,Sha256};
            ensure!(std::fs::metadata(path)?.len()<=20*1024*1024,"Choose artwork smaller than 20 MB");
            let mut reader=image::io::Reader::open(path)?.with_guessed_format()?;
            let mut limits=image::io::Limits::default();limits.max_alloc=Some(96*1024*1024);limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);reader.limits(limits);
            let image=reader.decode()?.thumbnail(1200,1200);
            let dir=repo.db_path().parent().unwrap_or(Path::new(".")).join("playlist-artwork");std::fs::create_dir_all(&dir)?;
            let mut bytes=std::io::Cursor::new(Vec::new());image.write_to(&mut bytes,image::ImageOutputFormat::Png)?;
            let target=dir.join(format!("{:x}.png",Sha256::digest(bytes.get_ref())));
            if !target.exists(){let pending=target.with_extension(format!("{}.pending",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()));std::fs::write(&pending,bytes.get_ref())?;std::fs::rename(pending,&target)?;}
            presentation(repo)?.execute("INSERT INTO desktop_playlist_details_v1 (playlist_id,artwork) VALUES (?1,?2) ON CONFLICT(playlist_id) DO UPDATE SET artwork=excluded.artwork",rusqlite::params![id,target.to_string_lossy()])?;
        },
        _=>anyhow::bail!("Unknown playlist file action"),
    }
    if action=="artwork" {save_cover(repo,&format!("playlist/{id}"),"custom")?;}
    Ok(serde_json::json!({"items":list(repo)?,"missing":[],"imported":0}))
}
pub fn change(repo: &LocalLibraryRepository, request: Request) -> Result<Vec<Collection>> {
    let current=list(repo)?;
    if request.id.as_deref().is_some_and(|id|id.starts_with("audio/")) {
        ensure!(matches!(request.action.as_str(),"rename"|"describe"|"cover"|"delete"),"Refresh this audio Sparklist rather than editing its generated membership");
        crate::audio_collections::edit(repo.db_path(),request.id.as_deref().unwrap(),request.name.as_deref(),request.description.as_deref(),request.artwork_choice.as_deref(),request.action=="delete")?;
    } else if request.action=="cover" {
        let id=request.id.as_deref().unwrap_or("");ensure!(current.iter().any(|c|c.id==id),"Collection no longer exists");
        save_cover(repo,id,request.artwork_choice.as_deref().context("Choose a cover")?)?;
    } else if request.action=="create_spark" || request.action=="update_spark" || (request.action=="delete" && request.id.as_deref().unwrap_or("").starts_with("spark/")) {
        let mut sparks=read_sparks(repo.db_path())?;
        if request.action=="delete" { ensure!(sparks.iter().any(|s|Some(&s.id)==request.id.as_ref()),"Sparklist no longer exists"); sparks.retain(|s|Some(&s.id)!=request.id.as_ref()); }
        else {
            let mut definition=request.definition.ok_or_else(||anyhow::anyhow!("Choose Sparklist rules"))?;
            definition.name=valid_name(&definition.name)?.to_owned();
            ensure!(!definition.rules.is_empty() && definition.rules.len()<=20,"Add between 1 and 20 rules");
            ensure!(definition.limit.is_none_or(|n|n>0 && n<=100000),"Invalid song limit");
            use musicos_desktop::local::smart_playlists::{SmartRuleField as F, SmartRuleOp as O};
            for rule in &definition.rules {
                ensure!(rule.value.chars().count()<=512,"Rule value too long");
                match rule.field {
                    F::Favorited=>ensure!(rule.op==O::IsTrue,"Use is_true for favorites"),
                    F::Genre|F::Artist|F::Album=>ensure!(matches!(rule.op,O::Contains|O::Equals)&&!rule.value.trim().is_empty(),"Choose a text match and enter a value"),
                    _=>ensure!(matches!(rule.op,O::Equals|O::GreaterThan|O::LessThan)&&rule.value.trim().parse::<i64>().is_ok(),"Choose a numeric comparison and enter a number"),
                }
            }
            if request.action=="update_spark" { let spark=sparks.iter_mut().find(|s|Some(&s.id)==request.id.as_ref()).ok_or_else(||anyhow::anyhow!("Sparklist no longer exists"))?; spark.definition=definition; }
            else { let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos(); sparks.push(Spark {id:format!("spark/{stamp}"),definition}); }
        }
        atomic_write_json(&spark_path(repo.db_path()),&sparks)?;
    } else if request.action=="create" {
        if let Some(choice)=&request.artwork_choice {validate_cover(choice)?;}
        let playlist=repo.create_playlist(valid_name(request.name.as_deref().unwrap_or(""))?)?;
        let choice=request.artwork_choice.unwrap_or_else(||format!("playlist/{}",COVERS[(playlist.id as usize)%COVERS.len()]));
        if let Err(error)=save_cover(repo,&format!("playlist/{}",playlist.id),&choice) {repo.delete_playlist(playlist.id)?;return Err(error);}
    } else {
        let collection=current.iter().find(|c|Some(&c.id)==request.id.as_ref()).ok_or_else(||anyhow::anyhow!("Playlist no longer exists"))?;
        ensure!(collection.spark.is_none(),"Edit Sparklist rules instead of individual songs");
        let id=collection.id.strip_prefix("playlist/").unwrap_or("").parse::<i64>()?;
        match request.action.as_str() {
            "rename"=>{repo.rename_playlist(id,valid_name(request.name.as_deref().unwrap_or(""))?)?;},
            "describe"=>{let value=request.description.unwrap_or_default();ensure!(value.chars().count()<=2000,"Use a description under 2,000 characters");presentation(repo)?.execute("INSERT INTO desktop_playlist_details_v1 (playlist_id,description) VALUES (?1,?2) ON CONFLICT(playlist_id) DO UPDATE SET description=excluded.description",rusqlite::params![id,value])?;},
            "delete"=>{repo.delete_playlist(id)?;presentation(repo)?.execute("DELETE FROM desktop_collection_art_v1 WHERE collection_id=?",[&collection.id])?;},
            "add"|"remove"|"reorder"=>{
                let ids=request.track_ids.unwrap_or_default();
                ensure!(ids.len()<=10000,"Select up to 10,000 songs");
                let db=musicos_init::database::Db::open(&repo.db_path().to_string_lossy())?;
                ensure!(ids.iter().all(|id|db.get_track_by_id(*id).ok().flatten().is_some()),"A selected song no longer exists");
                match request.action.as_str() { "add"=>repo.add_tracks(id,&ids)?,"remove"=>repo.remove_tracks(id,&ids)?,_=>{ensure!(ids.len()==collection.track_ids.len() && collection.track_ids.iter().all(|id|ids.iter().filter(|other|*other==id).count()==1),"Reorder must include every song once");repo.reorder_tracks(id,&ids)?;} }
            },
            _=>anyhow::bail!("Unknown playlist action"),
        }
    }
    let next=list(repo)?;
    for item in &next {if current.iter().find(|old|old.id==item.id).is_none_or(|old|serde_json::to_value(old).ok()!=serde_json::to_value(item).ok()) {
        presentation(repo)?.execute("INSERT INTO desktop_collection_modified_v1 VALUES (?,unixepoch()) ON CONFLICT(id) DO UPDATE SET updated_at=excluded.updated_at",[&item.id])?;
    }}
    list(repo)
}
#[cfg(test)] mod tests {
    use super::*;

    #[test] fn built_in_covers_are_persisted_without_overwriting_custom_artwork() {
        let dir=tempfile::tempdir().unwrap();let repo=LocalLibraryRepository::new(dir.path().join("library.sqlite"),dir.path().join("media"));repo.ensure_ready().unwrap();
        let items=change(&repo,Request{action:"create".into(),name:Some("Cover test".into()),artwork_choice:Some("playlist/Lake".into()),..Default::default()}).unwrap();let id=items[0].id.clone();
        assert_eq!(list(&repo).unwrap()[0].artwork_choice.as_deref(),Some("playlist/Lake"));
        let bad=change(&repo,Request{action:"cover".into(),id:Some(id.clone()),artwork_choice:Some("playlist/not-real".into()),..Default::default()});assert!(bad.is_err());assert_eq!(list(&repo).unwrap()[0].artwork_choice.as_deref(),Some("playlist/Lake"));
        let art=dir.path().join("art.png");image::RgbImage::from_pixel(4,4,image::Rgb([10,20,30])).save(&art).unwrap();file_action(&repo,"artwork",Some(&id),&art).unwrap();
        let custom=list(&repo).unwrap()[0].artwork.clone();assert_eq!(list(&repo).unwrap()[0].artwork_choice.as_deref(),Some("custom"));
        change(&repo,Request{action:"cover".into(),id:Some(id.clone()),artwork_choice:Some("collage".into()),..Default::default()}).unwrap();assert_eq!(list(&repo).unwrap()[0].artwork,custom);
        change(&repo,Request{action:"delete".into(),id:Some(id.clone()),..Default::default()}).unwrap();assert!(cover_choice(&repo,&id).unwrap().is_none());
    }
    #[test] fn names_are_trimmed_and_required() { assert_eq!(valid_name("  Night drive  ").unwrap(),"Night drive"); assert!(valid_name(" ").is_err()); }
    #[test] fn playlists_and_sparks_survive_reload() {
        let dir=tempfile::tempdir().unwrap(); let repo=LocalLibraryRepository::new(dir.path().join("library.sqlite"),dir.path().join("media")); repo.ensure_ready().unwrap();
        change(&repo,Request{action:"create".into(),id:None,name:Some(" Test ".into()),track_ids:None,definition:None,..Default::default()}).unwrap();
        let definition=musicos_desktop::local::smart_playlists::default_smart_playlists().remove(0);
        change(&repo,Request{action:"create_spark".into(),id:None,name:None,track_ids:None,definition:Some(definition),..Default::default()}).unwrap();
        let items=list(&repo).unwrap(); assert_eq!(items.len(),2); assert_eq!(items[0].name,"Test"); assert!(items[1].spark.is_some());
        let db=musicos_init::database::Db::open(repo.db_path().to_str().unwrap()).unwrap();
        let a_path=repo.media_dir().join("a.flac"); let b_path=repo.media_dir().join("b.flac"); std::fs::write(&a_path,b"fixture").unwrap(); std::fs::write(&b_path,b"fixture").unwrap();
        let a=db.insert_track(a_path.to_str().unwrap(),"One","Artist","Album",1,1,20).unwrap();
        let b=db.insert_track(b_path.to_str().unwrap(),"Two","Artist","Album",2,1,30).unwrap();
        let source=db.library_sources().unwrap().into_iter().find(|s|s.kind=="managed").unwrap();db.assign_track_source(a,source.id).unwrap();db.assign_track_source(b,source.id).unwrap();
        let id=items[0].id.clone();
        let request=|action:&str,ids:Vec<i64>|Request{action:action.into(),id:Some(id.clone()),name:None,track_ids:Some(ids),definition:None,..Default::default()};
        change(&repo,request("add",vec![a,b,a])).unwrap();
        assert_eq!(list(&repo).unwrap()[0].track_ids,vec![a,b]);
        change(&repo,request("reorder",vec![b,a])).unwrap();
        assert_eq!(list(&repo).unwrap()[0].track_ids,vec![b,a]);
        assert!(change(&repo,request("add",vec![a,999999])).is_err());
        repo.update_source(source.id,false).unwrap();
        change(&repo,request("reorder",vec![a,b])).unwrap();
        assert_eq!(list(&repo).unwrap()[0].track_ids,vec![a,b]);
        change(&repo,Request{action:"describe".into(),id:Some(id.clone()),description:Some("Saved after restarting".into()),..Default::default()}).unwrap();
        let image=dir.path().join("art.png");image::RgbImage::from_pixel(8,8,image::Rgb([120,50,80])).save(&image).unwrap();
        file_action(&repo,"artwork",Some(&id),&image).unwrap();
        std::fs::remove_file(image).unwrap();
        let reloaded=list(&repo).unwrap();assert_eq!(reloaded[0].description,"Saved after restarting");assert!(Path::new(reloaded[0].artwork.as_ref().unwrap()).is_file());
        let export=dir.path().join("preserved.m3u8");file_action(&repo,"export",Some(&id),&export).unwrap();
        let body=std::fs::read_to_string(&export).unwrap();assert!(body.contains(a_path.to_str().unwrap())&&body.contains(b_path.to_str().unwrap()));
        std::fs::write(&export,format!("{body}{}\n{}\n",a_path.display(),dir.path().join("missing.flac").display())).unwrap();
        let imported=file_action(&repo,"import",None,&export).unwrap();assert_eq!(imported["imported"],2);assert_eq!(imported["missing"].as_array().unwrap().len(),1);
        change(&repo,request("remove",vec![a])).unwrap();
        assert_eq!(list(&repo).unwrap()[0].track_ids,vec![b]);
        assert!(db.toggle_favorite(b).unwrap());
        let reopened=musicos_init::database::Db::open(repo.db_path().to_str().unwrap()).unwrap();
        assert!(reopened.get_track_by_id(b).unwrap().unwrap().favorited);
    }
}
