//! Desktop-only metadata cache; file tags and embedded database tables are untouched.
use anyhow::{Context, Result};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use musicos_desktop::models::{AudioQuality, Track};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip)] pub needs_refresh:bool,
    #[serde(default)]
    pub probe_error: Option<String>,
    #[serde(default)] pub duration:Option<i32>,
    pub album_artist: Option<String>,
    #[serde(default)]
    pub release_type: Option<String>,
    #[serde(default)] pub standalone_title:Option<String>,
    #[serde(default)] pub release_track_total:Option<u32>,
    #[serde(default)] pub release_disc_total:Option<u32>,
    #[serde(default)]
    pub artist_credits: Vec<String>,
    pub release_id: Option<String>,
    pub release_group_id: Option<String>,
    pub quality: Option<AudioQuality>,
}
fn identity(path: &Path) -> Result<String> {
    let m = std::fs::metadata(path)?;
    Ok(format!(
        "v7-release-totals:{}:{}:{}",
        path.display(),
        m.len(),
        m.modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ))
}
fn db(path: &Path) -> Result<musicos_init::database::Connection> {
    let c = musicos_init::database::Connection::open_background(path)?;
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    musicos_init::database_access::initialize_schema(path,"file-metadata-v1",||{c.execute_batch("CREATE TABLE IF NOT EXISTS desktop_file_metadata_v1(track_id INTEGER PRIMARY KEY,identity TEXT NOT NULL,value TEXT NOT NULL)")?;Ok(())})?;
    Ok(c)
}
/// Older recording lookups could replace a tagged studio album with a live release.
/// Keep the previous automatic value for diagnostics; user edits are never repaired.
pub fn repair_automatic_albums(path: &Path) -> Result<usize> {
    let mut connection = db(path)?;
    let transaction = connection.transaction()?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS desktop_album_override_repairs(
        track_id INTEGER PRIMARY KEY, album TEXT, provider TEXT, confidence REAL,
        repaired_at INTEGER NOT NULL DEFAULT(unixepoch()))")?;
    transaction.execute_batch("INSERT OR IGNORE INTO desktop_album_override_repairs(track_id,album,provider,confidence)
        SELECT o.track_id,o.album,o.provider,o.confidence FROM track_metadata_overrides o
        JOIN tracks t ON t.id=o.track_id WHERE o.origin='automatic'
        AND trim(coalesce(t.album,'')) NOT IN ('','Unknown Album','Unknown album')
        AND lower(trim(o.album))<>lower(trim(t.album));
        UPDATE metadata_enrichment_status SET state='review',release_mbid=NULL,release_group_mbid=NULL
        WHERE track_id IN (SELECT o.track_id FROM track_metadata_overrides o JOIN desktop_album_override_repairs r ON o.track_id=r.track_id WHERE o.origin='automatic' AND o.album IS NOT NULL);")?;
    let count = transaction.execute("UPDATE track_metadata_overrides SET album=NULL
        WHERE origin='automatic' AND album IS NOT NULL
        AND track_id IN (SELECT track_id FROM desktop_album_override_repairs)", [])?;
    transaction.commit()?;
    Ok(count)
}
pub fn cached(path: &Path, track: &Track) -> Option<Metadata> {
    let id = identity(Path::new(&track.path)).ok()?;
    let result = db(path)
        .ok()?
        .query_row(
            "SELECT value FROM desktop_file_metadata_v1 WHERE track_id=? AND identity=?",
            rusqlite::params![track.id, id],
            |r| r.get::<_, String>(0),
        )
        .ok()?;
    serde_json::from_str(&result).ok()
}
pub fn cached_many(path: &Path, tracks: &[Track]) -> std::collections::HashMap<i64, Metadata> {
    let mut values = std::collections::HashMap::new();
    let Ok(db) = db(path) else {
        return values;
    };
    // User-authored artist overrides take precedence over read-only file credits.
    let protected: std::collections::HashSet<i64> = db.prepare("SELECT track_id FROM track_metadata_overrides WHERE origin='user' AND artist IS NOT NULL")
        .ok().and_then(|mut query| query.query_map([], |r| r.get(0)).ok().map(|rows| rows.filter_map(Result::ok).collect())).unwrap_or_default();
    let Ok(mut query) =
        db.prepare("SELECT identity,value FROM desktop_file_metadata_v1 WHERE track_id=?")
    else {
        return values;
    };
    for track in tracks {
        let Ok(id) = identity(Path::new(&track.path)) else {
            continue;
        };
        if let Ok((signature, value)) = query.query_row([track.id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            if signature == id || ["v4-native-m4a:","v5-release-tags:","v6-release-totals:"].iter().any(|prefix|signature.strip_prefix(prefix).is_some_and(|old|id.strip_prefix("v7-release-totals:")==Some(old))) {
                if let Ok(mut value) = serde_json::from_str::<Metadata>(&value) {
                    value.needs_refresh=signature!=id;
                    if protected.contains(&track.id) { value.artist_credits.clear(); }
                    values.insert(track.id, value);
                }
            }
        }
    }
    values
}
pub fn probe(path: &Path) -> Result<Metadata> {
    let result=probe_tags(path);
    #[cfg(target_os="macos")]
    if path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case("m4a")) {
        let native=musicos_desktop::local::decoded_audio::apple::probe(path)?;
        let mut value=result.unwrap_or_default();
        if native.duration.is_finite()&&native.duration>0. {value.duration=Some(native.duration.round().min(i32::MAX as f64) as i32);}
        value.quality=Some(AudioQuality{format:native.codec,lossless:native.lossless,sample_rate:Some(native.sample_rate),bit_depth:native.bit_depth.or_else(||value.quality.as_ref().and_then(|q|q.bit_depth)),bitrate:native.bitrate,channels:Some(native.channels as u8)});
        return Ok(value);
    }
    result
}
fn probe_tags(path: &Path) -> Result<Metadata> {
    let file = lofty::read_from_path(path).context("Unable to read this audio file")?;
    let properties = file.properties();
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let get = |key: ItemKey| {
        tag.and_then(|t| t.get_string(&key))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let mut format = format!("{:?}", file.file_type());
    let mut lossless = matches!(
        file.file_type(),
        lofty::file::FileType::Flac
            | lofty::file::FileType::Wav
            | lofty::file::FileType::Aiff
            | lofty::file::FileType::Ape
            | lofty::file::FileType::WavPack
    );
    if file.file_type() == lofty::file::FileType::Mp4 {
        let mut reader = std::fs::File::open(path)?;
        let mp4 = lofty::mp4::Mp4File::read_from(&mut reader, lofty::config::ParseOptions::new())?;
        format = format!("{:?}", mp4.properties().codec());
        lossless = *mp4.properties().codec() == lofty::mp4::Mp4Codec::ALAC;
    }
    Ok(Metadata {
        needs_refresh:false,
        probe_error: None,
        duration:Some(properties.duration().as_secs().min(i32::MAX as u64) as i32),
        album_artist: get(ItemKey::AlbumArtist),
        release_type: tag.and_then(tag_release_kind).or_else(||get(ItemKey::AlbumTitle).and_then(|title|explicit_title_kind(&title))),
        standalone_title: if get(ItemKey::AlbumTitle).is_none() {get(ItemKey::TrackTitle).or_else(||path.file_stem().map(|s|s.to_string_lossy().into_owned()))}else{None},
        release_track_total:tag.and_then(|tag|lofty::tag::Accessor::track_total(tag)),
        release_disc_total:tag.and_then(|tag|lofty::tag::Accessor::disk_total(tag)),
        artist_credits: tag.map(|tag| {
            let mut credits = Vec::new();
            for value in tag.get_strings(&ItemKey::TrackArtist) {
                for name in crate::library::split_artist_credits(value) {
                    if name != "Unknown artist" && !credits.iter().any(|old: &String| old.eq_ignore_ascii_case(name)) { credits.push(name.to_owned()); }
                }
            }
            credits
        }).unwrap_or_default(),
        release_id: get(ItemKey::MusicBrainzReleaseId),
        release_group_id: get(ItemKey::MusicBrainzReleaseGroupId),
        quality: Some(AudioQuality {
            format,
            lossless,
            sample_rate: properties.sample_rate(),
            bit_depth: properties.bit_depth(),
            bitrate: properties.audio_bitrate(),
            channels: properties.channels(),
        }),
    })
}
fn tag_release_kind(tag:&lofty::tag::Tag)->Option<String>{tag.items().find_map(|item|{let ItemKey::Unknown(key)=item.key()else{return None};let key=key.to_ascii_lowercase().replace([' ', '_','-'], "");let key=key.rsplit(':').next().unwrap_or(&key);if ["releasetype","albumtype","musicbrainzalbumtype","musicbrainzreleasetype"].contains(&key){item.value().text().and_then(release_kind)}else{None}})}
fn release_kind(value:&str)->Option<String>{
    // Secondary release flags (for example Single;Live) do not hide the primary type.
    value.split([';', '/', ',', '\\']).find_map(|part|match part.trim().to_ascii_lowercase().as_str(){"single"=>Some("single".into()),"ep"=>Some("ep".into()),"album"=>Some("album".into()),_=>None})
}
fn explicit_title_kind(title:&str)->Option<String>{let lower=title.trim().to_ascii_lowercase();["single","ep"].into_iter().find(|kind|[format!(" - {kind}"),format!(" ({kind})"),format!(" [{kind}]")].iter().any(|suffix|lower.ends_with(suffix))).map(str::to_owned)}
pub fn ensure(path: &Path, track: &Track) -> Result<Metadata> {
    if let Some(m) = cached(path, track) {
        return Ok(m);
    }
    refresh(path,track)
}
/// Replace a successful probe atomically; a failed refresh keeps valid cached data.
pub fn refresh(path:&Path,track:&Track)->Result<Metadata>{
    let signature = identity(Path::new(&track.path))?;
    let result = probe(Path::new(&track.path));
    if result.is_err() && cached(path,track).is_some_and(|m|m.probe_error.is_none()){return result;}
    let value = result.as_ref().cloned().unwrap_or_else(|e| Metadata {
        probe_error: Some(format!("{e:#}")),
        ..Default::default()
    });
    anyhow::ensure!(
        signature == identity(Path::new(&track.path))?,
        "File changed during metadata reading"
    );
    db(path)?.execute("INSERT INTO desktop_file_metadata_v1 VALUES(?,?,?) ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity,value=excluded.value",rusqlite::params![track.id,signature,serde_json::to_string(&value)?])?;
    result.map(|_| value)
}
pub fn apply(track: &mut Track, value: Metadata) {
    if let Some(duration)=value.duration.filter(|d|*d>0){track.duration=duration;}
    if !value.artist_credits.is_empty() { track.artist = value.artist_credits.join("; "); }
    track.album_artist = value.album_artist;
    track.release_id = value.release_id;
    track.release_group_id = value.release_group_id;
    track.audio_quality = value.quality;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn loose_single_tags_and_mp4_keys_are_recognized(){
        use lofty::tag::{Tag,TagType,TagItem,ItemValue};
        for key in ["RELEASETYPE","MusicBrainz Album Type","MUSICBRAINZ_ALBUMTYPE","musicbrainz_releasetype","----:com.apple.iTunes:MusicBrainz Album Type"] {
            let mut tag=Tag::new(TagType::Id3v2);tag.insert_unchecked(TagItem::new(ItemKey::Unknown(key.into()),ItemValue::Text("Single;Live".into())));assert_eq!(tag_release_kind(&tag),Some("single".into()),"{key}");
        }
    }
    #[test]fn loose_tagged_single_without_album_has_its_own_release_title(){
        use lofty::tag::{Tag,TagType,TagItem,ItemValue,TagExt};
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("loose.wav");let data=[0_u8;400];let mut wav=Vec::new();wav.extend(b"RIFF");wav.extend((36_u32+data.len()as u32).to_le_bytes());wav.extend(b"WAVEfmt ");wav.extend(16_u32.to_le_bytes());wav.extend(1_u16.to_le_bytes());wav.extend(1_u16.to_le_bytes());wav.extend(48000_u32.to_le_bytes());wav.extend(96000_u32.to_le_bytes());wav.extend(2_u16.to_le_bytes());wav.extend(16_u16.to_le_bytes());wav.extend(b"data");wav.extend((data.len()as u32).to_le_bytes());wav.extend(data);std::fs::write(&path,wav).unwrap();
        let mut tag=Tag::new(TagType::Id3v2);tag.insert_text(ItemKey::TrackTitle,"Loose song".into());tag.insert_unchecked(TagItem::new(ItemKey::Unknown("MusicBrainz Album Type".into()),ItemValue::Text("single".into())));tag.save_to_path(&path,lofty::config::WriteOptions::default()).unwrap();let metadata=probe(&path).unwrap();assert_eq!(metadata.release_type.as_deref(),Some("single"));assert_eq!(metadata.standalone_title.as_deref(),Some("Loose song"));
        let database=dir.path().join("cache.db");musicos_init::database::Db::open(database.to_str().unwrap()).unwrap();let track=Track{id:1,path:path.to_string_lossy().into_owned(),..Default::default()};let mut old=metadata;old.album_artist=Some("Cached album artist".into());let signature=identity(&path).unwrap().replace("v7-release-totals:","v4-native-m4a:");db(&database).unwrap().execute("INSERT INTO desktop_file_metadata_v1 VALUES(1,?,?)",rusqlite::params![signature,serde_json::to_string(&old).unwrap()]).unwrap();assert!(cached(&database,&track).is_none());let values=cached_many(&database,&[track.clone()]);assert!(values[&1].needs_refresh);assert_eq!(values[&1].album_artist.as_deref(),Some("Cached album artist"));std::fs::write(&path,b"file changed").unwrap();assert!(cached_many(&database,&[track]).is_empty());

    }
    #[test]fn explicit_release_types_do_not_depend_on_folders(){assert_eq!(release_kind("Single;Live"),Some("single".into()));assert_eq!(explicit_title_kind("Song - Single"),Some("single".into()));assert_eq!(explicit_title_kind("EPIC"),None);assert_eq!(explicit_title_kind("One song"),None);assert_eq!(release_kind("compilation"),None);}
    #[test]
    fn automatic_album_repair_keeps_manual_editions_and_guest_credits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");
        musicos_init::database::Db::open(path.to_str().unwrap()).unwrap();
        let connection = db(&path).unwrap();
        // Only the columns needed by the repair are significant in this isolated fixture.
        connection.execute_batch("DROP TABLE tracks; CREATE TABLE tracks(id INTEGER PRIMARY KEY,album TEXT);
            INSERT INTO tracks VALUES(1,'CHROMAKOPIA'),(2,'Discovery');
            INSERT INTO track_metadata_overrides(track_id,album,artist,origin,updated_at)
            VALUES(1,'CHROMAKOPIA (Live at Camp Flog Gnaw)','Tyler, The Creator & Doechii','automatic',0),
                  (2,'Discovery (Deluxe)','Daft Punk','user',0);").unwrap();
        assert_eq!(repair_automatic_albums(&path).unwrap(), 1);
        assert_eq!(repair_automatic_albums(&path).unwrap(), 0);
        let value: (Option<String>,String) = connection.query_row("SELECT album,artist FROM track_metadata_overrides WHERE track_id=1",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(value,(None,"Tyler, The Creator & Doechii".into()));
        assert_eq!(connection.query_row("SELECT album FROM track_metadata_overrides WHERE track_id=2",[],|r|r.get::<_,String>(0)).unwrap(),"Discovery (Deluxe)");
        assert_eq!(connection.query_row("SELECT album FROM desktop_album_override_repairs WHERE track_id=1",[],|r|r.get::<_,String>(0)).unwrap(),"CHROMAKOPIA (Live at Camp Flog Gnaw)");
    }
    #[test]
    fn failed_probe_is_cached_until_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.flac");
        std::fs::write(&path, b"broken").unwrap();
        let db = dir.path().join("cache.db");
        let track = Track {
            id: 1,
            path: path.to_string_lossy().into_owned(),
            ..Default::default()
        };
        assert!(ensure(&db, &track).is_err());
        assert!(cached(&db, &track).unwrap().probe_error.is_some());
        assert!(cached_many(&db, &[track.clone()]).contains_key(&1));
        std::fs::write(path, b"changed broken file").unwrap();
        assert!(cached(&db, &track).is_none());
    }
    #[test]
    fn cached_credits_respect_manual_artist_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("track.flac");
        std::fs::write(&file, b"identity fixture").unwrap();
        let database = dir.path().join("cache.db");
        let c = db(&database).unwrap();
        c.execute_batch("CREATE TABLE track_metadata_overrides(track_id INTEGER,origin TEXT,artist TEXT)").unwrap();
        c.execute("INSERT INTO track_metadata_overrides VALUES(1,'user','My chosen artist')", []).unwrap();
        let value = Metadata { artist_credits: vec!["File artist".into(),"Guest".into()], ..Default::default() };
        let signature = identity(&file).unwrap();
        for id in [1,2] { c.execute("INSERT INTO desktop_file_metadata_v1 VALUES(?,?,?)",rusqlite::params![id,signature,serde_json::to_string(&value).unwrap()]).unwrap(); }
        let mut manual = Track { id:1,path:file.to_string_lossy().into_owned(),artist:"My chosen artist".into(),..Default::default() };
        let mut automatic = Track { id:2,..manual.clone() };
        let cached = cached_many(&database,&[manual.clone(),automatic.clone()]);
        apply(&mut manual,cached[&1].clone());apply(&mut automatic,cached[&2].clone());
        assert_eq!(manual.artist,"My chosen artist");assert_eq!(automatic.artist,"File artist; Guest");
    }
    #[test]
    fn codec_probe_distinguishes_alac_from_aac_and_preserves_tags() {
        let dir = tempfile::tempdir().unwrap();
        for (extension, codec, lossless) in [
            ("flac", "flac", true),
            ("m4a", "alac", true),
            ("m4a", "aac", false),
            ("mp3", "libmp3lame", false),
        ] {
            let path = dir.path().join(format!("{codec}.{extension}"));
            let before = std::process::Command::new("ffmpeg")
                .args([
                    "-loglevel",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440:duration=0.2",
                    "-c:a",
                    codec,
                    "-metadata",
                    "album_artist=Main Artist",
                    "-metadata",
                    "artist=Main Artist;Guest One;Guest Two",
                    "-y",
                ])
                .arg(&path)
                .status();
            if before.is_err() {
                eprintln!("ffmpeg unavailable: codec fixture test not run");
                return;
            }
            assert!(before.unwrap().success());
            let bytes = std::fs::read(&path).unwrap();
            let meta = probe(&path).unwrap();
            assert_eq!(meta.quality.unwrap().lossless, lossless, "{codec}");
            assert_eq!(meta.album_artist.as_deref(), Some("Main Artist"));
            assert_eq!(meta.artist_credits, ["Main Artist", "Guest One", "Guest Two"]);
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
    }
}

/// Embedded artwork belongs to a recording, even when album enrichment supplied
/// the same fallback cover for every track. Cache misses and bytes by file identity.
pub fn song_artwork(database: &Path, track_id: i64, file: &Path) -> Result<Option<std::path::PathBuf>> {
    use sha2::{Digest, Sha256};
    use lofty::picture::PictureType;
    if !file.is_file() { return Ok(None); }
    let identity = identity(file)?;
    let connection = db(database)?;
    connection.execute_batch("CREATE TABLE IF NOT EXISTS desktop_song_artwork_v1(track_id INTEGER PRIMARY KEY, identity TEXT NOT NULL, path TEXT)")?;
    let cached = connection.query_row("SELECT path FROM desktop_song_artwork_v1 WHERE track_id=? AND identity=?",rusqlite::params![track_id,identity],|r|r.get::<_,Option<String>>(0));
    if let Ok(path) = cached { if path.as_ref().is_none_or(|p|Path::new(p).is_file()) { return Ok(path.map(Into::into)); } }
    let picture = lofty::read_from_path(file).ok().and_then(|tagged| {
        tagged.tags().iter().flat_map(|tag|tag.pictures()).find(|p|p.pic_type()==PictureType::CoverFront)
            .or_else(||tagged.tags().iter().flat_map(|tag|tag.pictures()).next()).map(|p|p.data().to_vec())
    });
    let path = picture.and_then(|bytes|image::load_from_memory(&bytes).ok()).map(|image| -> Result<std::path::PathBuf> {
        let directory = database.parent().unwrap_or(Path::new(".")).join("song-artwork-v1");
        std::fs::create_dir_all(&directory)?;
        let name = format!("{:x}",Sha256::digest(identity.as_bytes()));
        let target = directory.join(format!("{name}.png"));
        let temporary = directory.join(format!("{name}.tmp"));
        image.save_with_format(&temporary,image::ImageFormat::Png)?;
        std::fs::rename(temporary,&target)?;
        Ok(target)
    }).transpose()?;
    connection.execute("INSERT INTO desktop_song_artwork_v1(track_id,identity,path) VALUES(?,?,?) ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity,path=excluded.path",rusqlite::params![track_id,identity,path.as_ref().map(|p|p.to_string_lossy().into_owned())])?;
    Ok(path)
}

#[cfg(test)]
mod artwork_tests {
    use super::*;
    use lofty::{config::WriteOptions,picture::{Picture,PictureType},tag::Tag};
    #[test]
    fn songs_in_one_album_keep_their_own_embedded_covers_and_invalidate_changes() {
        let dir=tempfile::tempdir().unwrap();
        let database=dir.path().join("library.db");
        let mut covers=Vec::new();
        for (id,color) in [(1,[255u8,0,0]),(2,[0u8,0,255])] {
            let audio=dir.path().join(format!("{id}.flac"));
            let status=std::process::Command::new("ffmpeg").args(["-loglevel","error","-f","lavfi","-i","sine=frequency=440:duration=0.2","-metadata","album=Same Album","-y"]).arg(&audio).status().expect("ffmpeg fixture dependency");
            assert!(status.success());
            let picture=dir.path().join(format!("{id}.png"));
            image::RgbImage::from_pixel(8,8,image::Rgb(color)).save(&picture).unwrap();
            let mut tagged=lofty::read_from_path(&audio).unwrap();
            if tagged.primary_tag().is_none() { tagged.insert_tag(Tag::new(tagged.primary_tag_type())); }
            let mut file=std::fs::File::open(&picture).unwrap();
            let mut cover=Picture::from_reader(&mut file).unwrap();cover.set_pic_type(PictureType::CoverFront);
            tagged.primary_tag_mut().unwrap().push_picture(cover);
            tagged.save_to_path(&audio,WriteOptions::default()).unwrap();
            let before=std::fs::read(&audio).unwrap();
            let result=song_artwork(&database,id,&audio).unwrap().unwrap();
            assert_eq!(image::open(&result).unwrap().to_rgb8().get_pixel(0,0).0,color);
            assert_eq!(song_artwork(&database,id,&audio).unwrap(),Some(result.clone()));
            assert_eq!(std::fs::read(&audio).unwrap(),before);
            covers.push(result);
            if id==2 {
                tagged.primary_tag_mut().unwrap().remove_picture_type(PictureType::CoverFront);
                tagged.save_to_path(&audio,WriteOptions::default()).unwrap();
                assert_eq!(song_artwork(&database,id,&audio).unwrap(),None);
                assert_eq!(song_artwork(&database,id,&audio).unwrap(),None);
            }
        }
        assert_ne!(covers[0],covers[1]);
    }
}

#[cfg(test)]
#[test]
fn failed_refresh_keeps_previous_successful_cache_and_source(){
 let dir=tempfile::tempdir().unwrap();let database=dir.path().join("db.sqlite");let audio=dir.path().join("corrupt.flac");std::fs::write(&audio,b"unreadable fixture").unwrap();
 let track:Track=serde_json::from_value(serde_json::json!({"id":1,"path":audio,"title":"Song","artist":"Artist","album":"Album","track_number":1,"duration":10})).unwrap();
 let value=Metadata{album_artist:Some("Artist".into()),quality:Some(AudioQuality{format:"FLAC".into(),lossless:true,..Default::default()}),..Default::default()};
 let json=serde_json::to_string(&value).unwrap();db(&database).unwrap().execute("INSERT INTO desktop_file_metadata_v1 VALUES(?,?,?)",rusqlite::params![1,identity(&audio).unwrap(),json]).unwrap();
 assert!(refresh(&database,&track).is_err());assert_eq!(cached(&database,&track).unwrap().quality.unwrap().format,"FLAC");assert_eq!(std::fs::read(audio).unwrap(),b"unreadable fixture");
}
