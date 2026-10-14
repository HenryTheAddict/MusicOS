use musicos_desktop::models::{LibrarySnapshot, Track};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;
fn canonical(value: &str) -> String {
    value
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
pub fn artist_name_key(value:&str)->String {canonical(value).replace(['’','‘','ʼ'],"'").replace(['‐','‑','–','—'],"-")}
fn artist_identity(value:&str)->String {crate::providers::cached_artist_info(value).and_then(|p|p.artist_mbid).map(|id|format!("mbid:{id}")).unwrap_or_else(||artist_name_key(value))}
// Typographic apostrophes do not identify a different album edition. Keep the
// original spelling in presentation and add the previous grouping IDs as aliases.
fn album_canonical(value: &str) -> String {
    canonical(value).replace(['’', '‘', 'ʼ'], "'")
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub artist_id: String,
    #[serde(default)]
    pub legacy_ids: Vec<String>,
    #[serde(default)]
    pub release_type: Option<String>,
    #[serde(default)]
    pub release_type_source: Option<String>,
    pub title: String,
    pub artist: String,
    pub genre: String,
    pub year: i32,
    pub cover: Option<String>,
    pub track_ids: Vec<i64>,
    pub duration: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    #[serde(default)]
    pub legacy_ids:Vec<String>,
    #[serde(default)]
    pub credited_names:Vec<String>,
    pub name: String,
    pub portrait: Option<String>,
    pub album_ids: Vec<String>,
    pub track_ids: Vec<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: String,
    pub name: String,
    pub track_ids: Vec<i64>,
    pub album_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Library {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub genres: Vec<Genre>,
    pub sources: Vec<musicos_desktop::models::LibrarySource>,
}
pub fn stable_id(kind: &str, fields: &[&str]) -> String {
    let mut h = Sha256::new();
    for field in fields {
        h.update((field.len() as u64).to_le_bytes());
        h.update(field.as_bytes());
    }
    format!("local/{kind}/{:x}", h.finalize())
}
pub fn split_artist_credits(value: &str) -> Vec<&str> {
    let mut names = Vec::new();
    for credit in value.split([';', '&']) {
        let mut rest = credit;
        loop {
            let lower = rest.to_ascii_lowercase();
            let boundary = [" featuring ", " feat. ", " feat ", " ft. ", " ft "]
                .into_iter()
                .filter_map(|marker| lower.find(marker).map(|i| (i, marker.len())))
                .min_by_key(|(i, _)| *i);
            let (name, next) = match boundary {
                Some((i, len)) => (&rest[..i], Some(&rest[i + len..])),
                None => (rest, None),
            };
            let name = name.trim();
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
            match next {
                Some(next) => rest = next,
                None => break,
            }
        }
    }
    if names.is_empty() {
        names.push("Unknown artist");
    }
    names
}
pub fn group(snapshot: LibrarySnapshot) -> Library {
    let mut albums = BTreeMap::<String, Album>::new();
    let mut artists = BTreeMap::<String, Artist>::new();
    let mut genres = BTreeMap::<String, Genre>::new();
    let mut identities=std::collections::HashMap::<String,String>::new();
    let mut releases = BTreeMap::<(String, String), std::collections::BTreeSet<String>>::new();
    for t in &snapshot.tracks {
        if let Some(release) = t.release_id.as_ref() {
            let credits = split_artist_credits(&t.artist);
            let primary = t.album_artist.as_deref().unwrap_or(credits[0]);
            releases
                .entry((canonical(primary), album_canonical(&t.album)))
                .or_default()
                .insert(release.clone());
        }
    }
    for t in &snapshot.tracks {
        let artist_name = if t.artist.trim().is_empty() {
            "Unknown artist"
        } else {
            t.artist.trim()
        };
        let album_title = if t.album.trim().is_empty() {
            "Unknown album"
        } else {
            t.album.trim()
        };
        let mut credits = split_artist_credits(artist_name);
        let primary = t
            .album_artist
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(str::trim)
            .unwrap_or(credits[0]);
        if !credits.contains(&primary) {
            credits.insert(0, primary);
        }
        let artist_id = stable_id("artist", &[primary]);
        let album_key = album_canonical(album_title);
        let primary_key = canonical(primary);
        let editions = releases.get(&(primary_key.clone(), album_key.clone()));
        let edition = t
            .release_id
            .as_deref()
            .or_else(|| {
                editions
                    .filter(|ids| ids.len() == 1)
                    .and_then(|ids| ids.first().map(String::as_str))
            })
            .unwrap_or("");
        let album_id = stable_id("album", &[&primary_key, &album_key, edition]);
        let previous_id = stable_id("album", &[artist_name, album_title]);
        let album = albums.entry(album_id.clone()).or_insert_with(|| Album {
            id: album_id.clone(),
            artist_id: artist_id.clone(),
            legacy_ids: vec![],
            release_type: None,
            release_type_source: None,
            title: album_title.into(),
            artist: primary.into(),
            genre: t.genre.clone(),
            year: t.year,
            cover: None,
            track_ids: vec![],
            duration: 0,
        });
        for alias in [
            stable_id("album", &[&primary_key, &canonical(album_title), edition]),
            stable_id("album", &[&primary_key, &canonical(album_title), ""]),
            stable_id("album", &[&primary_key, &album_key, ""]),
            stable_id("album", &[&canonical(credits[0]), &album_key, ""]),
            previous_id,
            stable_id("album", &[credits[0], album_title]),
            stable_id("album", &[primary, album_title]),
        ] {
            if alias != album_id && !album.legacy_ids.contains(&alias) {
                album.legacy_ids.push(alias);
            }
        }
        album.track_ids.push(t.id);
        album.duration += i64::from(t.duration.max(0));
        if album.cover.is_none() {
            album.cover = t
                .cover_art_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
        }
        for name in credits {
            let id = stable_id("artist", &[name]);
            let key=identities.entry(artist_name_key(name)).or_insert_with(||artist_identity(name)).clone();
            let artist = artists.entry(key).or_insert_with(|| Artist {
                id:id.clone(),legacy_ids:vec![],credited_names:vec![name.into()],
                name: name.into(),
                portrait: None,
                album_ids: vec![],
                track_ids: vec![],
            });
            if artist.id!=id&&!artist.legacy_ids.contains(&id){artist.legacy_ids.push(id);}
            if !artist.credited_names.iter().any(|n|n==name){artist.credited_names.push(name.into());}
            if !artist.album_ids.contains(&album_id) {
                artist.album_ids.push(album_id.clone());
            }
            if !artist.track_ids.contains(&t.id){artist.track_ids.push(t.id);}
        }
        let name = if t.genre.trim().is_empty() {
            "Unknown genre"
        } else {
            t.genre.trim()
        };
        let genre_id = stable_id("genre", &[name]);
        let genre = genres.entry(genre_id.clone()).or_insert_with(|| Genre {
            id: genre_id,
            name: name.into(),
            track_ids: vec![],
            album_ids: vec![],
        });
        genre.track_ids.push(t.id);
        if !genre.album_ids.contains(&album_id) {
            genre.album_ids.push(album_id)
        }
    }
    let routes=artists.values().flat_map(|a|std::iter::once((&a.id,&a.id)).chain(a.legacy_ids.iter().map(|id|(id,&a.id)))).collect::<std::collections::HashMap<_,_>>();
    for album in albums.values_mut(){if let Some(id)=routes.get(&album.artist_id){album.artist_id=(*id).clone();}}
    let order: std::collections::HashMap<_, _> = snapshot
        .tracks
        .iter()
        .map(|t| (t.id, (t.disc_number.max(1), if t.track_number>0{t.track_number}else{i32::MAX}, t.title.clone())))
        .collect();
    for album in albums.values_mut() {
        album.track_ids.sort_by_key(|id| order.get(id).cloned());
    }
    let mut albums: Vec<_> = albums.into_values().collect();
    albums.sort_by_key(|a| a.title.to_lowercase());
    let mut artists: Vec<_> = artists.into_values().collect();
    artists.sort_by_key(|a| a.name.to_lowercase());
    let album_lookup: std::collections::HashMap<_, _> =
        albums.iter().map(|a| (a.id.as_str(), a)).collect();
    for artist in &mut artists {
        artist.album_ids.sort_by_key(|id| {
            album_lookup
                .get(id.as_str())
                .map(|a| a.title.to_lowercase())
        });
        artist.track_ids = artist
            .album_ids
            .iter()
            .filter_map(|id| album_lookup.get(id.as_str()))
            .flat_map(|a| a.track_ids.iter().copied())
            .filter(|id| artist.track_ids.contains(id))
            .collect();
    }
    let mut genres: Vec<_> = genres.into_values().collect();
    genres.sort_by_key(|g| g.name.to_lowercase());
    Library {
        tracks: snapshot.tracks,
        albums,
        artists,
        genres,
        sources: snapshot.sources,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn equivalent_artist_spellings_preserve_tracks_and_routes(){
        let library=group(LibrarySnapshot{tracks:vec![Track{id:1,artist:"Eazy-E".into(),album:"Album".into(),..Default::default()},Track{id:2,artist:" eazy‐e ".into(),album:"Album".into(),..Default::default()}],..Default::default()});
        assert_eq!(library.artists.len(),1);let artist=&library.artists[0];assert_eq!(artist.track_ids.len(),2);assert_eq!(library.tracks.len(),2);assert!(artist.legacy_ids.contains(&stable_id("artist",&["eazy‐e"])));assert!(library.albums.iter().all(|a|a.artist_id==artist.id));
    }
    #[test]
    fn collaborators_have_individual_identities_and_sequences() {
        assert_eq!(
            split_artist_credits(" Kevin Abstract; Dominic Fike & A ; A "),
            vec!["Kevin Abstract", "Dominic Fike", "A"]
        );
        assert_eq!(
            split_artist_credits("Beyoncé feat. Guest & Third"),
            vec!["Beyoncé", "Guest", "Third"]
        );
        let l = group(LibrarySnapshot {
            tracks: vec![
                Track {
                    id: 1,
                    artist: "Kevin Abstract; Dominic Fike".into(),
                    album: "Shared".into(),
                    ..Default::default()
                },
                Track {
                    id: 2,
                    artist: "Dominic Fike".into(),
                    album: "Solo".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        assert_eq!(l.artists.len(), 2);
        assert_eq!(
            l.artists
                .iter()
                .find(|a| a.name == "Kevin Abstract")
                .unwrap()
                .track_ids,
            vec![1]
        );
        assert_eq!(
            l.artists
                .iter()
                .find(|a| a.name == "Dominic Fike")
                .unwrap()
                .track_ids
                .len(),
            2
        );
        assert_eq!(
            l.albums
                .iter()
                .find(|a| a.title == "Shared")
                .unwrap()
                .artist_id,
            stable_id("artist", &["Kevin Abstract"])
        );
    }
    #[test]
    fn guest_credits_do_not_split_an_album_or_erase_artist_membership() {
        let l = group(LibrarySnapshot {
            tracks: vec![
                Track {
                    id: 1,
                    artist: "Tyler, The Creator".into(),
                    album: "CHROMAKOPIA".into(),
                    ..Default::default()
                },
                Track {
                    id: 2,
                    artist: "Tyler, The Creator & Doechii".into(),
                    album: "CHROMAKOPIA".into(),
                    ..Default::default()
                },
                Track {
                    id: 3,
                    artist: "Another artist".into(),
                    album: "CHROMAKOPIA".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        assert_eq!(l.albums.len(), 2);
        let album = l
            .albums
            .iter()
            .find(|a| a.artist == "Tyler, The Creator")
            .unwrap();
        assert_eq!(album.track_ids.len(), 2);
        assert_eq!(
            album.id,
            stable_id("album", &["tyler, the creator", "chromakopia", ""])
        );
        assert!(album.legacy_ids.contains(&stable_id(
            "album",
            &["Tyler, The Creator & Doechii", "CHROMAKOPIA"]
        )));
        assert_eq!(
            l.artists
                .iter()
                .find(|a| a.name == "Doechii")
                .unwrap()
                .track_ids,
            vec![2]
        );
    }
    #[test]
    fn casing_guests_and_album_artist_merge_but_editions_do_not() {
        let track =
            |id, title: &str, artist: &str, album_artist: Option<&str>, release: Option<&str>| {
                Track {
                    id,
                    album: title.into(),
                    artist: artist.into(),
                    album_artist: album_artist.map(str::to_owned),
                    release_id: release.map(str::to_owned),
                    ..Default::default()
                }
            };
        let l = group(LibrarySnapshot {
            tracks: vec![
                track(
                    1,
                    "To Pimp a Butterfly",
                    "Kendrick Lamar",
                    None,
                    Some("release-a"),
                ),
                track(
                    2,
                    "To Pimp A Butterfly",
                    "Kendrick Lamar & Rapsody",
                    None,
                    None,
                ),
                track(
                    3,
                    "To Pimp a Butterfly (Deluxe)",
                    "Kendrick Lamar",
                    None,
                    None,
                ),
                track(
                    4,
                    "To Pimp a Butterfly",
                    "Guest first",
                    Some("Kendrick Lamar"),
                    Some("release-a"),
                ),
                track(
                    5,
                    "To Pimp a Butterfly",
                    "Kendrick Lamar",
                    None,
                    Some("release-b"),
                ),
            ],
            ..Default::default()
        });
        // Unknown release cannot be assigned across two known distinct editions.
        assert_eq!(l.albums.len(), 4);
        let a = l.albums.iter().find(|a| a.track_ids.contains(&1)).unwrap();
        assert!(a.track_ids.contains(&4));
        assert!(!a.track_ids.contains(&5));
        assert!(l
            .artists
            .iter()
            .find(|a| a.name == "Rapsody")
            .unwrap()
            .track_ids
            .contains(&2));
        let l = group(LibrarySnapshot {
            tracks: vec![
                track(1, "GNX", "Kendrick Lamar", None, Some("release-a")),
                track(2, "gnx", "Kendrick Lamar & SZA", None, None),
            ],
            ..Default::default()
        });
        assert_eq!(l.albums.len(), 1);
        assert_eq!(l.albums[0].track_ids.len(), 2);
    }
    #[test]
    fn identities_and_counts_are_stable() {
        let a = Track {
            id: 1,
            title: "One".into(),
            artist: "A".into(),
            album: "Same".into(),
            duration: 20,
            track_number: 2,
            ..Default::default()
        };
        let b = Track {
            id: 2,
            artist: "B".into(),
            ..a.clone()
        };
        let c = Track {
            id: 3,
            track_number: 1,
            ..a.clone()
        };
        let l = group(LibrarySnapshot {
            tracks: vec![a, b, c],
            ..Default::default()
        });
        assert_eq!(l.albums.len(), 2);
        let a = l.albums.iter().find(|a| a.artist == "A").unwrap();
        assert_eq!(a.track_ids, vec![3, 1]);
        assert_eq!(a.duration, 40);
        assert_eq!(
            l.artists.iter().find(|a| a.name == "A").unwrap().track_ids,
            vec![3, 1]
        );
        assert_eq!(a.id, stable_id("album", &["a", "same", ""]));
        assert!(a.legacy_ids.contains(&stable_id("album", &["A", "Same"])));
        assert_ne!(
            stable_id("album", &["a", "bc"]),
            stable_id("album", &["ab", "c"])
        );
    }
}

// Decode only when file identity changes. Repeated library refreshes must not
// re-decode every cover; the bounded cache never changes a user's source images.
fn valid_cover(path: &std::path::Path) -> bool {
    use std::sync::{Mutex, OnceLock};
    type Identity = (u64, Option<std::time::SystemTime>);
    static CACHE: OnceLock<Mutex<BTreeMap<std::path::PathBuf, (Identity, bool)>>> = OnceLock::new();
    let Ok(metadata) = path.metadata() else { return false; };
    if !metadata.is_file() { return false; }
    let identity = (metadata.len(), metadata.modified().ok());
    let cache = CACHE.get_or_init(Default::default);
    if let Some((previous, valid)) = cache.lock().unwrap().get(path) {
        if *previous == identity { return *valid; }
    }
    let valid = (|| {
        let mut reader = image::io::Reader::open(path)?.with_guessed_format()?;
        let mut limits = image::io::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        Ok::<_, anyhow::Error>(reader.decode().is_ok())
    })().unwrap_or(false);
    let mut cache = cache.lock().unwrap();
    if cache.len() >= 4096 { cache.clear(); }
    cache.insert(path.to_owned(), (identity, valid));
    valid
}

pub fn canonical_artwork(database: &std::path::Path, library: &mut Library) -> anyhow::Result<()> {
    let db = rusqlite::Connection::open(database)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS desktop_album_artwork_v1(album_id TEXT PRIMARY KEY,path TEXT NOT NULL)")?;
    for album in &mut library.albums {
        // Honor a valid explicit replacement first. A failed replacement retains
        // the saved image. Alias lookup preserves artwork across grouping fixes.
        let custom: Option<String> = db.query_row("SELECT artwork_path FROM track_metadata_overrides WHERE origin='user' AND artwork_path IS NOT NULL AND track_id IN (SELECT value FROM json_each(?)) ORDER BY updated_at DESC LIMIT 1", [serde_json::to_string(&album.track_ids)?], |r| r.get(0)).ok().filter(|p: &String| valid_cover(p.as_ref()));
        let saved = std::iter::once(&album.id).chain(album.legacy_ids.iter()).find_map(|id| {
            db.query_row("SELECT path FROM desktop_album_artwork_v1 WHERE album_id=?", [id], |r| r.get::<_, String>(0)).ok().filter(|p| valid_cover(p.as_ref()))
        });
        album.cover = custom.or(saved).or_else(|| album.track_ids.iter()
            .filter_map(|id| library.tracks.iter().find(|t| t.id == *id)?.cover_art_path.as_ref())
            .find(|p| valid_cover(p)).map(|p| p.to_string_lossy().into_owned()));
        if let Some(path) = &album.cover {
            db.execute("INSERT INTO desktop_album_artwork_v1(album_id,path) VALUES(?,?) ON CONFLICT(album_id) DO UPDATE SET path=excluded.path", rusqlite::params![album.id, path])?;
        }
        for track in library.tracks.iter_mut().filter(|t| album.track_ids.contains(&t.id)) {
            track.cover_art_path = album.cover.as_ref().map(std::path::PathBuf::from);
        }
    }
    Ok(())
}

#[cfg(test)] mod artwork_consistency_tests {
 use super::*;
 #[test] fn canonical_artwork_survives_reordering_and_missing_song_images(){
  let dir=tempfile::tempdir().unwrap();let first=dir.path().join("first.png");let other=dir.path().join("other.png");image::RgbImage::new(2,2).save(&first).unwrap();image::RgbImage::new(3,3).save(&other).unwrap();
  let db=dir.path().join("library.sqlite");
  let tracks=vec![Track{id:1,artist:"A".into(),album:"Album".into(),cover_art_path:Some(first.clone()),..Default::default()},Track{id:2,artist:"A".into(),album:"Album".into(),cover_art_path:Some(other.clone()),..Default::default()},Track{id:3,artist:"A".into(),album:"Album".into(),..Default::default()}];
  let make=|tracks|group(LibrarySnapshot{tracks,..Default::default()});
  let mut library=make(tracks.clone());canonical_artwork(&db,&mut library).unwrap();assert!(library.tracks.iter().all(|t|t.cover_art_path.as_ref()==Some(&first)));
  let mut reversed=tracks;reversed.reverse();let mut reopened=make(reversed);canonical_artwork(&db,&mut reopened).unwrap();assert_eq!(reopened.albums[0].cover,Some(first.to_string_lossy().into_owned()));
  std::fs::remove_file(first).unwrap();canonical_artwork(&db,&mut reopened).unwrap();assert!(reopened.albums[0].cover.is_none());
 }
}

#[cfg(test)] mod polish_tests {
 use super::*;
 #[test] fn apostrophe_variants_merge_without_merging_editions_or_verified_releases() {
  let track=Track{id:1,artist:"M83".into(),album:"Hurry Up, We’re Dreaming".into(),..Default::default()};
  let old=stable_id("album", &["m83", "hurry up, we’re dreaming", ""]);
  let library=group(LibrarySnapshot{tracks:vec![track.clone(),Track{id:2,album:"Hurry Up, We're Dreaming".into(),artist:"M83 & Guest".into(),..track.clone()},Track{id:3,album:"Hurry Up, We're Dreaming (Deluxe)".into(),..track.clone()}],..Default::default()});
  assert_eq!(library.albums.len(),2);
  let merged=library.albums.iter().find(|a|a.track_ids.len()==2).unwrap();assert!(merged.legacy_ids.contains(&old));
  let editions=group(LibrarySnapshot{tracks:vec![Track{release_id:Some("release-a".into()),..track.clone()},Track{id:2,release_id:Some("release-b".into()),..track}],..Default::default()});assert_eq!(editions.albums.len(),2);
 }
 #[test] fn corrupt_replacement_retains_saved_art_and_changed_files_are_revalidated() {
  let dir=tempfile::tempdir().unwrap();let good=dir.path().join("good.png");let bad=dir.path().join("bad.png");image::RgbImage::new(2,2).save(&good).unwrap();std::fs::write(&bad,b"broken").unwrap();
  assert!(!valid_cover(&bad));image::RgbImage::new(3,3).save(&bad).unwrap();assert!(valid_cover(&bad));std::fs::write(&bad,b"broken again").unwrap();assert!(!valid_cover(&bad));
  let path=dir.path().join("db.sqlite");let mut library=group(LibrarySnapshot{tracks:vec![Track{id:1,artist:"Artist".into(),album:"Album".into(),cover_art_path:Some(good.clone()),..Default::default()}],..Default::default()});canonical_artwork(&path,&mut library).unwrap();
  let db=rusqlite::Connection::open(&path).unwrap();db.execute_batch("CREATE TABLE track_metadata_overrides(track_id INTEGER, origin TEXT, artwork_path TEXT, updated_at INTEGER)").unwrap();db.execute("INSERT INTO track_metadata_overrides VALUES(1,'user',?,1)",[bad.to_string_lossy().as_ref()]).unwrap();canonical_artwork(&path,&mut library).unwrap();assert_eq!(library.albums[0].cover,Some(good.to_string_lossy().into_owned()));
 }
}

pub fn recover_filename_numbers(tracks:&mut [Track],protected:&std::collections::HashSet<i64>){
 for track in tracks {if track.track_number<=0&&!protected.contains(&track.id){if let Some(stem)=std::path::Path::new(&track.path).file_stem().and_then(|s|s.to_str()){if let Some(number)=musicos_init::media::track_number_from_stem(stem){track.track_number=number;}}}}
}
#[cfg(test)]mod filename_sequence_tests{use super::*;
 #[test]fn igor_space_prefix_recovers_last_track_without_overriding_user_numbers(){
  let mut tracks=vec![Track{id:1,title:"IGOR'S THEME".into(),artist:"Tyler".into(),album:"IGOR".into(),track_number:1,..Default::default()},Track{id:12,title:"ARE WE STILL FRIENDS?".into(),artist:"Tyler".into(),album:"IGOR".into(),path:"/music/12 ARE WE STILL FRIENDS-.flac".into(),..Default::default()},Track{id:99,title:"Unknown number".into(),artist:"Tyler".into(),album:"IGOR".into(),..Default::default()}];
  recover_filename_numbers(&mut tracks,&Default::default());assert_eq!(tracks[1].track_number,12);
  let library=group(LibrarySnapshot{tracks:tracks.clone(),..Default::default()});assert_eq!(library.albums[0].track_ids,vec![1,12,99]);assert_eq!(library.tracks.len(),3);
  tracks[1].track_number=0;recover_filename_numbers(&mut tracks,&[12].into_iter().collect());assert_eq!(tracks[1].track_number,0);
 }
}
