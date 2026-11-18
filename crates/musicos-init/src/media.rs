use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

use crate::database::{Db, MetadataUpdate, Track};
use crate::renderer::{Color, CoverArt};
use id3::frame::PictureType as Id3PictureType;
use id3::Tag;
use image::GenericImageView;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{Picture, PictureType};
use lofty::tag::{Accessor, Tag as LoftyTag};
use sha2::{Digest, Sha256};

struct CoverEntry{art:Option<CoverArt>,identity:Option<(u64,std::time::SystemTime)>,used:std::time::Instant,bytes:usize}
#[derive(Default)]struct CoverCache{entries:HashMap<String,CoverEntry>,bytes:usize}
static COVER_ART_CACHE: OnceLock<Mutex<CoverCache>> = OnceLock::new();
static LAST_SCAN_ROOTS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
const COVER_ART_CACHE_VERSION_DIR: &str = "v3-originals";

pub fn write_metadata_batch(paths: &[String], update: &MetadataUpdate) -> Result<()> {
    write_metadata_batch_with(paths, update, || Ok(()))
}

pub fn write_metadata_batch_with<T>(
    paths: &[String],
    update: &MetadataUpdate,
    commit: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let mut backups = Vec::new();
    for (index, value) in paths.iter().enumerate() {
        let path = Path::new(value);
        anyhow::ensure!(
            path.is_file(),
            "metadata target is missing: {}",
            path.display()
        );
        let backup = path.with_extension(format!(
            "{}.musicos-metadata-{}-{index}.bak",
            path.extension()
                .and_then(|value| value.to_str())
                .unwrap_or("audio"),
            std::process::id()
        ));
        std::fs::copy(path, &backup)?;
        backups.push((path.to_path_buf(), backup));
    }

    let result = paths
        .iter()
        .try_for_each(|value| write_metadata(Path::new(value), update))
        .and_then(|()| commit());
    if let Err(err) = result {
        for (path, backup) in &backups {
            let _ = std::fs::copy(backup, path);
        }
        for (_, backup) in backups {
            let _ = std::fs::remove_file(backup);
        }
        return Err(err);
    }
    for (_, backup) in backups {
        let _ = std::fs::remove_file(backup);
    }
    result
}

fn write_metadata(path: &Path, update: &MetadataUpdate) -> Result<()> {
    let mut tagged = lofty::read_from_path(path)?;
    if tagged.primary_tag().is_none() {
        tagged.insert_tag(LoftyTag::new(tagged.primary_tag_type()));
    }
    let tag = tagged
        .primary_tag_mut()
        .ok_or_else(|| anyhow::anyhow!("{} does not support writable metadata", path.display()))?;
    if let Some(value) = update.title.as_ref() {
        tag.set_title(value.trim().to_string());
    }
    if let Some(value) = update.artist.as_ref() {
        tag.set_artist(value.trim().to_string());
    }
    if let Some(value) = update.album.as_ref() {
        tag.set_album(value.trim().to_string());
    }
    if let Some(value) = update.genre.as_ref() {
        tag.set_genre(value.trim().to_string());
    }
    if let Some(value) = update.year {
        tag.set_year(value.max(0) as u32);
    }
    if let Some(value) = update.track_number {
        tag.set_track(value.max(0) as u32);
    }
    if let Some(value) = update.disc_number {
        tag.set_disk(value.max(0) as u32);
    }
    if let Some(value) = update
        .artwork_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let mut artwork = std::fs::File::open(value)?;
        let mut picture = Picture::from_reader(&mut artwork)?;
        picture.set_pic_type(PictureType::CoverFront);
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(picture);
    }
    tagged.save_to_path(path, WriteOptions::default())?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub indexed: usize,
    pub processed: usize,
    pub discovered: usize,
    pub current_root: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScanMode {
    #[default]
    Changed,
    Full,
}

#[derive(Debug, Clone)]
pub struct ScanRequest {
    pub managed_root: PathBuf,
    pub reference_source_ids: Option<Vec<i64>>,
    pub mode: ScanMode,
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub indexed: usize,
    pub processed: usize,
    pub discovered: usize,
    pub cancelled: bool,
}

/// Scans a desktop library using explicit paths. This deliberately avoids
/// `MUSICOS_UPLOAD_DIR`, allowing the GPUI application to own its library
/// without a companion process or process-global path mutation.
pub fn scan_library_with_progress<F>(
    db_path: &Path,
    request: &ScanRequest,
    cancel: &AtomicBool,
    mut progress: F,
) -> Result<ScanResult>
where
    F: FnMut(ScanProgress),
{
    std::fs::create_dir_all(&request.managed_root)?;
    let db = Db::open_background(
        db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("library database path is not valid UTF-8"))?,
    )?;
    let managed_path = request.managed_root.display().to_string();
    let managed_name = request
        .managed_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("MusicOS Library");
    let managed = db.add_library_source("managed", &managed_path, managed_name)?;
    let requested_references = request.reference_source_ids.as_ref();
    let mut sources = db
        .library_sources()?
        .into_iter()
        .filter(|source| {
            source.enabled
                && (source.id == managed.id
                    || (source.kind == "reference"
                        && requested_references
                            .map(|ids| ids.contains(&source.id))
                            .unwrap_or(true)))
        })
        .collect::<Vec<_>>();
    sources.sort_by_key(|source| (source.id != managed.id, source.id));

    let mut result = ScanResult::default();
    let mut claimed_paths = std::collections::HashSet::new();
    for source in sources {
        if cancel.load(Ordering::Relaxed) {
            result.cancelled = true;
            break;
        }
        let root = Path::new(&source.path);
        if !root.is_dir() {
            db.mark_library_source(source.id, false, Some("Folder is unavailable"))?;
            continue;
        }
        let (files, scan) = audio_files_under(root);
        let files = files
            .into_iter()
            .filter(|path| claimed_paths.insert(path.clone()))
            .collect::<Vec<_>>();
        result.discovered += files.len();
        progress(ScanProgress {
            indexed: result.indexed,
            processed: result.processed,
            discovered: result.discovered,
            current_root: Some(source.path.clone()),
        });

        let indexed_before = result.indexed;
        let processed_before = result.processed;
        let local_processed = std::cell::Cell::new(0usize);
        let discovered = result.discovered;
        result.indexed += index_files(
            &db,
            &files,
            request.mode == ScanMode::Full,
            Some(source.id),
            Some(cancel),
            |local_indexed, processed| {
                local_processed.set(processed);
                progress(ScanProgress {
                    indexed: indexed_before + local_indexed,
                    processed: processed_before + processed,
                    discovered,
                    current_root: Some(source.path.clone()),
                });
            },
        )?;
        result.processed += local_processed.get();
        if cancel.load(Ordering::Relaxed) {
            result.cancelled = true;
            break;
        }
        let existing_paths = files
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        if scan.available(root) {
            db.remove_missing_tracks_for_source(source.id, &existing_paths)?;
            db.mark_library_source(source.id, true, None)?;
        } else {
            db.mark_library_source(
                source.id,
                false,
                Some("Storage changed or traversal failed during scan"),
            )?;
        }
    }
    progress(ScanProgress {
        indexed: result.indexed,
        processed: result.processed,
        discovered: result.discovered,
        current_root: None,
    });
    Ok(result)
}

pub fn cached_cover_art(track_path: &str) -> Option<CoverArt> {
    let cache=COVER_ART_CACHE.get_or_init(Default::default);let identity=std::fs::metadata(track_path).ok().and_then(|m|Some((m.len(),m.modified().ok()?)));
    if let Ok(mut cache)=cache.lock(){if let Some(entry)=cache.entries.get_mut(track_path){if entry.identity==identity{entry.used=std::time::Instant::now();return entry.art.clone();}}}
    let art=extract_cover_art(track_path);let bytes=art.as_ref().map_or(0,|a|a.pixels.len()*std::mem::size_of::<Color>())+256+track_path.len();const BUDGET:usize=64*1024*1024;
    if bytes<=BUDGET{if let Ok(mut cache)=cache.lock(){if let Some(old)=cache.entries.remove(track_path){cache.bytes-=old.bytes;}while cache.bytes+bytes>BUDGET{let Some(key)=cache.entries.iter().min_by_key(|(_,entry)|entry.used).map(|(key,_)|key.clone())else{break};if let Some(old)=cache.entries.remove(&key){cache.bytes-=old.bytes;}}cache.entries.insert(track_path.into(),CoverEntry{art:art.clone(),identity,used:std::time::Instant::now(),bytes});cache.bytes+=bytes;}}
    art
}

pub fn cached_cover_art_for_track(db: &Db, track: &Track) -> Option<CoverArt> {
    if track.cover_art_source_modified_at == Some(track.modified_at) {
        if let Some(path) = track.cover_art_path.as_deref() {
            let path = Path::new(path);
            let versioned_local_cache =
                path.starts_with(db.cover_art_dir().join(COVER_ART_CACHE_VERSION_DIR));
            let downloaded_cache = path.starts_with(db.cover_art_dir().join("musicbrainz"));
            if versioned_local_cache || downloaded_cache {
                if let Some(art) = load_cover_art_file(path) {
                    return Some(art);
                }
            }
        } else {
            return None;
        }
    }

    let art = cached_cover_art(&track.path);
    if let Some(ref art) = art {
        let _ = persist_cover_art(db, track.id, art, track.modified_at);
    } else {
        let _ = db.set_track_cover_art(track.id, None, track.modified_at);
    }
    art
}

pub fn cover_art_memory_cache_len() -> usize {
    COVER_ART_CACHE
        .get()
        .and_then(|cache| cache.lock().ok().map(|cache| cache.entries.len()))
        .unwrap_or(0)
}

pub fn clear_cover_art_memory_cache() {
    if let Some(cache) = COVER_ART_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.entries.clear();cache.bytes=0;
        }
    }
}

pub fn last_scan_roots() -> Vec<String> {
    LAST_SCAN_ROOTS
        .get()
        .and_then(|roots| roots.lock().ok().map(|roots| roots.clone()))
        .unwrap_or_default()
}

pub fn extract_cover_art(track_path: &str) -> Option<CoverArt> {
    if let Ok(tagged_file) = lofty::read_from_path(track_path) {
        let picture = tagged_file
            .primary_tag()
            .and_then(preferred_lofty_picture)
            .or_else(|| tagged_file.first_tag().and_then(preferred_lofty_picture));
        if let Some(picture) = picture {
            match image::load_from_memory(picture.data()) {
                Ok(img) => return Some(cover_art_from_image(img)),
                Err(err) => {
                    log::warn!(
                        "failed to load embedded cover art for {}: {}",
                        track_path,
                        err
                    )
                }
            }
        }
    }

    // some mp3s only expose art through the old id3 path
    if let Ok(tag) = Tag::read_from_path(track_path) {
        let picture = tag
            .pictures()
            .find(|picture| picture.picture_type == Id3PictureType::CoverFront)
            .or_else(|| {
                tag.pictures()
                    .find(|picture| picture.picture_type == Id3PictureType::Other)
            })
            .or_else(|| tag.pictures().next());
        if let Some(picture) = picture {
            match image::load_from_memory(&picture.data) {
                Ok(img) => return Some(cover_art_from_image(img)),
                Err(err) => log::warn!(
                    "Failed to load cover art from memory for {}: {}",
                    track_path,
                    err
                ),
            }
        }
    }

    // Folder art still shows up a lot, especially from older rips. Search the
    // album folder and only walk up once when the track is inside a clearly
    // named disc subfolder. Blindly searching the parent used to pick artist
    // portraits and unrelated covers from the artist directory.
    if let Some(parent) = Path::new(track_path).parent() {
        let mut directories = vec![parent];
        if is_disc_directory(parent) {
            if let Some(album_directory) = parent.parent() {
                directories.push(album_directory);
            }
        }
        for dir in directories {
            if let Some(path) = find_folder_cover_art(dir) {
                if let Some(art) = load_cover_art_file(&path) {
                    return Some(art);
                }
            }
        }
    }

    None
}

fn preferred_lofty_picture(tag: &LoftyTag) -> Option<&Picture> {
    tag.get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.get_picture_type(PictureType::Other))
        .or_else(|| tag.pictures().first())
}

fn is_disc_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let compact = name
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    ["cd", "disc", "disk"].iter().any(|prefix| {
        compact.strip_prefix(prefix).is_some_and(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
        })
    })
}

pub fn load_cover_art_file(path: &Path) -> Option<CoverArt> {
    image::open(path).ok().map(cover_art_from_image)
}

fn cover_art_from_image(img: image::DynamicImage) -> CoverArt {
    let (w, h) = img.dimensions();
    let pixels = img.to_rgba8().into_raw();
    let mut colors = Vec::with_capacity((w * h) as usize);
    for chunk in pixels.chunks_exact(4) {
        colors.push(Color::rgba(chunk[0], chunk[1], chunk[2], chunk[3]));
    }
    CoverArt {
        width: w,
        height: h,
        pixels: colors,
    }
}

fn persist_cover_art(
    db: &Db,
    track_id: i64,
    art: &CoverArt,
    source_modified_at: i64,
) -> Result<()> {
    let dir = db.cover_art_dir().join(COVER_ART_CACHE_VERSION_DIR);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{track_id}.png"));
    let mut rgba = Vec::with_capacity(art.pixels.len() * 4);
    for color in &art.pixels {
        rgba.push(color.r);
        rgba.push(color.g);
        rgba.push(color.b);
        rgba.push(color.a);
    }
    let image = image::RgbaImage::from_raw(art.width, art.height, rgba)
        .ok_or_else(|| anyhow::anyhow!("invalid cover-art buffer"))?;
    let temp = dir.join(format!("{track_id}.png.tmp"));
    image.save_with_format(&temp, image::ImageFormat::Png)?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::rename(&temp, &path)?;
    db.set_track_cover_art(track_id, path.to_str(), source_modified_at)?;
    Ok(())
}

fn find_folder_cover_art(dir: &Path) -> Option<PathBuf> {
    const NAMES: &[&str] = &[
        "cover.jpg",
        "cover.jpeg",
        "cover.png",
        "folder.jpg",
        "folder.jpeg",
        "folder.png",
        "album.jpg",
        "album.jpeg",
        "album.png",
        "front.jpg",
        "front.jpeg",
        "front.png",
        "front cover.jpg",
        "front cover.jpeg",
        "front cover.png",
        "artwork.jpg",
        "artwork.jpeg",
        "artwork.png",
        "AlbumArtSmall.jpg",
        "AlbumArt.jpg",
    ];

    for name in NAMES {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let lowered = file_name.to_ascii_lowercase();
        if NAMES
            .iter()
            .any(|candidate| lowered == candidate.to_ascii_lowercase())
        {
            return Some(path);
        }
    }

    None
}

pub fn index_default_locations(db: &Db) -> Result<usize> {
    index_default_locations_with_progress(db, |_| {})
}

pub fn index_default_locations_with_progress<F>(db: &Db, progress: F) -> Result<usize>
where
    F: FnMut(ScanProgress),
{
    index_default_locations_with_mode(db, false, progress)
}

pub fn reindex_default_locations_with_progress<F>(db: &Db, progress: F) -> Result<usize>
where
    F: FnMut(ScanProgress),
{
    index_default_locations_with_mode(db, true, progress)
}

pub fn index_library_source_with_progress<F>(
    db: &Db,
    source_id: i64,
    reindex_all: bool,
    mut progress: F,
) -> Result<usize>
where
    F: FnMut(ScanProgress),
{
    let source = db
        .library_source(source_id)?
        .ok_or_else(|| anyhow::anyhow!("library source {source_id} was not found"))?;
    let root = Path::new(&source.path);
    if !root.is_dir() {
        db.mark_library_source(source.id, false, Some("Folder is unavailable"))?;
        anyhow::bail!("{} is unavailable", source.display_name);
    }
    let (files, scan) = audio_files_under(root);
    progress(ScanProgress {
        indexed: 0,
        processed: 0,
        discovered: files.len(),
        current_root: Some(source.path.clone()),
    });
    let indexed = index_files(
        db,
        &files,
        reindex_all,
        Some(source.id),
        None,
        |indexed, processed| {
            progress(ScanProgress {
                indexed,
                processed,
                discovered: files.len(),
                current_root: Some(source.path.clone()),
            });
        },
    )?;
    if scan.available(root) {
        db.remove_missing_tracks_for_source(
            source.id,
            &files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        )?;
        db.mark_library_source(source.id, true, None)?;
    } else {
        db.mark_library_source(
            source.id,
            false,
            Some("Storage changed or traversal failed during scan"),
        )?;
    }
    progress(ScanProgress {
        indexed,
        processed: files.len(),
        discovered: files.len(),
        current_root: None,
    });
    Ok(indexed)
}

fn index_default_locations_with_mode<F>(
    db: &Db,
    reindex_all: bool,
    mut progress: F,
) -> Result<usize>
where
    F: FnMut(ScanProgress),
{
    let sources = scan_sources(db)?;
    let roots = sources
        .iter()
        .map(|source| source.path.clone())
        .collect::<Vec<_>>();

    if let Ok(mut last) = LAST_SCAN_ROOTS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
    {
        *last = roots.clone();
    }

    let mut indexed = 0;
    let mut processed = 0;
    let mut discovered = 0;
    let mut claimed_paths = std::collections::HashSet::new();
    for source in sources {
        let root = source.path;
        let root_path = Path::new(&root);
        if !root_path.exists() {
            db.mark_library_source(source.id, false, Some("Folder is unavailable"))?;
            continue;
        }

        let (files, scan) = audio_files_under(root_path);
        let files = files
            .into_iter()
            .filter(|path| claimed_paths.insert(path.clone()))
            .collect::<Vec<_>>();
        discovered += files.len();
        progress(ScanProgress {
            indexed,
            processed,
            discovered,
            current_root: Some(root.clone()),
        });

        indexed += index_files(
            db,
            &files,
            reindex_all,
            Some(source.id),
            None,
            |local_indexed, local_processed| {
                progress(ScanProgress {
                    indexed: indexed + local_indexed,
                    processed: processed + local_processed,
                    discovered,
                    current_root: Some(root.clone()),
                });
            },
        )?;
        let existing_paths = files
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        if scan.available(root_path) {
            db.remove_missing_tracks_for_source(source.id, &existing_paths)?;
            db.mark_library_source(source.id, true, None)?;
        } else {
            db.mark_library_source(
                source.id,
                false,
                Some("Storage changed or traversal failed during scan"),
            )?;
        }
        processed += files.len();
    }

    progress(ScanProgress {
        indexed,
        processed,
        discovered,
        current_root: None,
    });
    Ok(indexed)
}

fn default_scan_roots() -> Vec<String> {
    let mut roots = vec![
        "/mnt/sdcard".to_string(),
        "/media".to_string(),
        "/roms".to_string(),
        "/boot/music".to_string(),
        "/music".to_string(),
    ];

    let upload_dir = std::env::var("MUSICOS_UPLOAD_DIR").unwrap_or_else(|_| "./music".to_string());
    if Path::new(&upload_dir).exists() {
        roots.push(upload_dir);
    } else if Path::new("./music").exists() {
        roots.push("./music".to_string());
    }

    if std::env::var("MUSICOS_SCAN_HOME_MUSIC").as_deref() == Ok("1") {
        if let Ok(home) = std::env::var("HOME") {
            let mac_music = format!("{}/Music", home);
            if Path::new(&mac_music).exists() {
                roots.push(mac_music);
            }
        }
    }
    let mut unique = Vec::new();
    for root in roots {
        let path = Path::new(&root);
        if !path.exists() {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !unique
            .iter()
            .any(|existing: &PathBuf| *existing == canonical)
        {
            unique.push(canonical);
        }
    }
    unique
        .into_iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn scan_sources(db: &Db) -> Result<Vec<crate::database::LibrarySource>> {
    for root in default_scan_roots() {
        let name = Path::new(&root)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("MusicOS Library");
        let _ = db.add_library_source("managed", &root, name)?;
    }
    let mut sources = db
        .library_sources()?
        .into_iter()
        .filter(|source| source.enabled)
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| {
        let left_priority = i32::from(left.kind == "reference");
        let right_priority = i32::from(right.kind == "reference");
        right_priority
            .cmp(&left_priority)
            .then_with(|| right.path.len().cmp(&left.path.len()))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(sources)
}

pub fn index_location(db: &Db, root: &Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let (files, scan) = audio_files_under(root);
    anyhow::ensure!(
        scan.available(root),
        "storage unavailable during USB scan: {}",
        root.display()
    );
    index_files(db, &files, false, None, None, |_, _| {})
}

struct StorageScan {
    identity: Option<String>,
    complete: bool,
}

fn storage_identity(root: &Path) -> Option<String> {
    let metadata = std::fs::metadata(root).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mut identity = format!("{}:{}", metadata.dev(), metadata.ino());
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            let file = std::fs::File::open(root).ok()?;
            let info =
                std::fs::read_to_string(format!("/proc/self/fdinfo/{}", file.as_raw_fd())).ok()?;
            identity.push_str(info.lines().find(|line| line.starts_with("mnt_id:"))?);
        }
        Some(identity)
    }
    #[cfg(not(unix))]
    {
        Some(format!(
            "{:?}:{:?}",
            root.canonicalize().ok()?,
            metadata.created().ok()
        ))
    }
}

impl StorageScan {
    fn available(&self, root: &Path) -> bool {
        let available =
            self.complete && self.identity.is_some() && self.identity == storage_identity(root);
        if !available {
            log::warn!(
                "storage changed or traversal failed; preserving library rows for {}",
                root.display()
            );
        }
        available
    }
}

fn audio_files_under(root: &Path) -> (Vec<PathBuf>, StorageScan) {
    let mut files = Vec::new();
    let mut scan = StorageScan {
        identity: storage_identity(root),
        complete: true,
    };
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !ignored_library_entry(entry.path(), entry.depth()))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                log::warn!("storage traversal failed: {error}");
                scan.complete = false;
                continue;
            }
        };
        if entry.file_type().is_file() && is_audio(entry.path()) {
            files.push(entry.into_path());
        }
    }
    files.sort_unstable();
    (files, scan)
}

fn index_files<F>(
    db: &Db,
    files: &[PathBuf],
    reindex_all: bool,
    source_id: Option<i64>,
    cancel: Option<&AtomicBool>,
    mut progress: F,
) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    use rayon::prelude::*;

    let mut indexed = 0;
    let reindex_all = reindex_all || std::env::var("MUSICOS_REINDEX_ALL").as_deref() == Ok("1");
    let imported_hashes = managed_import_hashes(files);

    let total = files.len();
    let mut processed = 0usize;
    let mut last_pct = -1i32;
    let mut report = |indexed: usize, processed: usize| {
        let pct = if total > 0 {
            (processed * 100) / total
        } else {
            100
        } as i32;
        if pct != last_pct {
            log::info!("[INDEXING] {}%", pct);
            last_pct = pct;
        }
        progress(indexed, processed);
    };

    let mut pending: Vec<(PathBuf, FileStats, Option<String>)> = Vec::new();
    for path in files {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed)) {
            break;
        }
        let path_str = path.display().to_string();
        let Ok(stats) = file_stats(path) else {
            processed += 1;
            report(indexed, processed);
            continue;
        };
        if !reindex_all {
            if let Ok(true) = db.track_is_fresh(&path_str, stats.file_size, stats.modified_at) {
                if let Some(source_id) = source_id {
                    if let Err(error) = db.assign_track_source_by_path(&path_str, source_id) {
                        log::warn!(
                            "failed to repair source membership for {}: {error}",
                            path.display()
                        );
                    }
                }
                processed += 1;
                report(indexed, processed);
                continue;
            }
        }
        pending.push((path.clone(), stats, imported_hashes.get(path).cloned()));
    }

    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .max(1);
    let chunk_size = (workers * 4).max(1);
    'outer: for chunk in pending.chunks(chunk_size) {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed)) {
            break;
        }
        let prepared: Vec<Result<PreparedTrack>> = chunk
            .par_iter()
            .map(|(path, stats, known_hash)| prepare_index(path, *stats, known_hash.as_deref()))
            .collect();
        for (entry, result) in chunk.iter().zip(prepared) {
            match result {
                Ok(prepared) => match commit_index(db, &prepared, source_id) {
                    Ok(()) => indexed += 1,
                    Err(err) => log::warn!("failed to index file {}: {err}", entry.0.display()),
                },
                Err(err) => log::warn!("failed to index file {}: {err}", entry.0.display()),
            }
            processed += 1;
            report(indexed, processed);
            if cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed)) {
                break 'outer;
            }
        }
    }
    log::info!("[INDEXING] 100%");

    Ok(indexed)
}

struct PreparedTrack {
    path_str: String,
    title: String,
    artist: String,
    album: String,
    track_number: i32,
    disc_number: Option<i32>,
    genre: Option<String>,
    year: Option<i32>,
    duration: i32,
    file_size: i64,
    modified_at: i64,
    content_hash: String,
}

fn prepare_index(path: &Path, stats: FileStats, known_hash: Option<&str>) -> Result<PreparedTrack> {
    let path_str = path.display().to_string();
    let fallback = metadata_from_path(path);
    let scraped = scrape_embedded_metadata(path).unwrap_or_default();

    let track_number = scraped.track_number.or(fallback.track_number).unwrap_or(0);
    let title = prefer(scraped.title, Some(fallback.title), "Untitled");
    let artist = if is_bad_artist(scraped.artist.as_deref()) {
        fallback
            .artist
            .unwrap_or_else(|| "Unknown Artist".to_string())
    } else {
        prefer(scraped.artist, fallback.artist, "Unknown Artist")
    };
    let album = prefer(scraped.album, fallback.album, "Unknown Album");
    let duration = if scraped.duration > 0 {
        scraped.duration
    } else {
        estimate_duration(stats.file_size)
    };
    let content_hash = match known_hash {
        Some(hash) => hash.to_string(),
        None => sha256_path(path)?,
    };
    Ok(PreparedTrack {
        path_str,
        title,
        artist,
        album,
        track_number,
        disc_number: scraped.disc_number,
        genre: scraped.genre,
        year: scraped.year,
        duration,
        file_size: stats.file_size,
        modified_at: stats.modified_at,
        content_hash,
    })
}

fn commit_index(db: &Db, prepared: &PreparedTrack, source_id: Option<i64>) -> Result<()> {
    let id = db.insert_track_with_number(
        &prepared.path_str,
        &prepared.title,
        &prepared.artist,
        &prepared.album,
        prepared.track_number,
        prepared.duration,
        prepared.file_size,
        prepared.modified_at,
    )?;
    if let Some(source_id) = source_id {
        db.assign_track_source(id, source_id)?;
    }
    db.update_track_metadata(
        &[id],
        &MetadataUpdate {
            disc_number: prepared.disc_number,
            genre: prepared.genre.clone(),
            year: prepared.year,
            ..MetadataUpdate::default()
        },
    )?;
    db.set_track_content_hash(id, &prepared.content_hash)?;
    Ok(())
}

fn managed_import_hashes(files: &[PathBuf]) -> HashMap<PathBuf, String> {
    let env_root = std::env::var_os("MUSICOS_UPLOAD_DIR").map(PathBuf::from);
    let discovered_root = files.first().and_then(|path| {
        path.ancestors()
            .find(|ancestor| ancestor.join(".musicos-import-index.json").is_file())
            .map(Path::to_path_buf)
    });
    let Some(root) = discovered_root.or(env_root) else {
        return HashMap::new();
    };
    let Ok(file) = std::fs::File::open(root.join(".musicos-import-index.json")) else {
        return HashMap::new();
    };
    let Ok(index) = serde_json::from_reader::<_, HashMap<String, String>>(file) else {
        return HashMap::new();
    };
    index
        .into_iter()
        .map(|(hash, relative)| (root.join(relative), hash))
        .collect()
}

fn sha256_path(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    // GPUI's background executor uses deliberately small worker stacks. Keep
    // the hashing buffer on the heap so scanning a previously unseen file
    // cannot overflow one of those worker stacks.
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Copy)]
struct FileStats {
    file_size: i64,
    modified_at: i64,
}

#[derive(Debug, Default)]
struct EmbeddedMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    track_number: Option<i32>,
    disc_number: Option<i32>,
    genre: Option<String>,
    year: Option<i32>,
    duration: i32,
}

#[derive(Debug, PartialEq, Eq)]
struct PathMetadata {
    title: String,
    artist: Option<String>,
    album: Option<String>,
    track_number: Option<i32>,
}

fn file_stats(path: &Path) -> Result<FileStats> {
    let metadata = path.metadata()?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    Ok(FileStats {
        file_size: metadata.len() as i64,
        modified_at,
    })
}

/// Read-only destination hints for managed copies; files need no existing hierarchy.
pub fn organization_tags(path:&Path)->(Option<String>,Option<String>){let Ok(file)=lofty::read_from_path(path)else{return(None,None)};let Some(tag)=file.primary_tag().or_else(||file.first_tag())else{return(None,None)};let artist=clean_tag(tag.get_string(&lofty::tag::ItemKey::AlbumArtist)).or_else(||clean_tag(tag.artist().as_deref()));(artist,clean_tag(tag.album().as_deref()))}

fn scrape_embedded_metadata(path: &Path) -> Option<EmbeddedMetadata> {
    let tagged_file = lofty::read_from_path(path).ok()?;
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let mut metadata = EmbeddedMetadata {
        duration: tagged_file.properties().duration().as_secs() as i32,
        ..EmbeddedMetadata::default()
    };

    if let Some(tag) = tag {
        metadata.title = clean_title_tag(tag.title().as_deref());
        metadata.artist = clean_tag(tag.artist().as_deref());
        metadata.album = clean_tag(tag.album().as_deref());
        metadata.track_number = tag
            .track()
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value > 0);
        metadata.disc_number = tag
            .disk()
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value > 0);
        metadata.genre = clean_tag(tag.genre().as_deref());
        metadata.year = tag
            .year()
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value > 0);
    }

    Some(metadata)
}

fn metadata_from_path(path: &Path) -> PathMetadata {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled");
    let track_number = track_number_from_stem(stem);
    let mut title = clean_title_from_stem(stem);
    let mut artist_from_name = None;

    if let Some((left, right)) = split_artist_title(stem) {
        let left = trim_track_prefix(left);
        if !is_track_number(left) {
            artist_from_name = Some(normalize_name(left));
        }
        title = clean_title_from_stem(right);
    }

    let album = parent_name(path, 1).filter(|value| !is_noise_folder(value));
    let artist = parent_name(path, 2)
        .filter(|value| !is_noise_folder(value))
        .or(artist_from_name);

    PathMetadata {
        title,
        artist,
        album,
        track_number,
    }
}

fn parent_name(path: &Path, levels: usize) -> Option<String> {
    let mut current: PathBuf = path.to_path_buf();
    for _ in 0..levels {
        current = current.parent()?.to_path_buf();
    }
    current
        .file_name()
        .and_then(|value| value.to_str())
        .map(normalize_name)
        .filter(|value| !value.is_empty())
}

fn split_artist_title(stem: &str) -> Option<(&str, &str)> {
    for delimiter in [" - ", " – ", " — "] {
        if let Some((left, right)) = stem.split_once(delimiter) {
            if !right.trim().is_empty() {
                return Some((left.trim(), right.trim()));
            }
        }
    }
    None
}

fn clean_title_from_stem(stem: &str) -> String {
    let trimmed = trim_track_prefix(stem);
    let without_suffix = trimmed
        .strip_suffix(" copy")
        .or_else(|| trimmed.strip_suffix(" 1"))
        .unwrap_or(trimmed);
    normalize_name(without_suffix)
}

fn trim_track_prefix(value: &str) -> &str {
    let trimmed = value.trim();
    let mut chars = trimmed.char_indices().peekable();
    let mut saw_digit = false;
    let mut last_digit_end = 0;

    while let Some((idx, ch)) = chars.peek().copied() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            last_digit_end = idx + ch.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    if !saw_digit {
        return trimmed;
    }

    let rest = trimmed[last_digit_end..].trim_start();
    let rest = rest
        .strip_prefix('.')
        .or_else(|| rest.strip_prefix('-'))
        .or_else(|| rest.strip_prefix('_'))
        .unwrap_or(rest)
        .trim_start();

    if rest.is_empty() {
        trimmed
    } else {
        rest
    }
}

pub fn track_number_from_stem(stem: &str) -> Option<i32> {
    let trimmed = stem.trim();
    let digits: String = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() || digits.len() > 3 {
        return None;
    }

    let suffix = &trimmed[digits.len()..];
    let spaced_prefix=digits.len()>=2 && suffix.starts_with(char::is_whitespace);
    let rest = suffix.trim_start();
    if rest.is_empty() || (!spaced_prefix && !matches!(rest.chars().next(), Some('.') | Some('-') | Some('_'))) {
        return None;
    }

    digits.parse::<i32>().ok().filter(|value| *value > 0)
}

fn normalize_name(value: &str) -> String {
    value
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn clean_tag(value: Option<&str>) -> Option<String> {
    let value = normalize_name(value?);
    if value.is_empty() || value.eq_ignore_ascii_case("unknown") {
        None
    } else {
        Some(value)
    }
}

fn clean_title_tag(value: Option<&str>) -> Option<String> {
    let value = normalize_name(value?);
    if value.is_empty() {
        return None;
    }
    if let Some((left, right)) = split_artist_title(&value) {
        if is_track_number(left) {
            return Some(clean_title_from_stem(right));
        }
    }
    Some(value)
}

fn prefer(primary: Option<String>, fallback: Option<String>, default: &str) -> String {
    primary
        .filter(|value| !value.trim().is_empty())
        .or_else(|| fallback.filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| default.to_string())
}

fn is_bad_artist(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let normalized = value.trim();
    normalized.is_empty()
        || normalized.eq_ignore_ascii_case("unknown artist")
        || normalized.eq_ignore_ascii_case("unknown")
        || is_track_number(normalized)
}

fn is_track_number(value: &str) -> bool {
    let cleaned = value.trim().trim_start_matches('#');
    !cleaned.is_empty() && cleaned.len() <= 3 && cleaned.chars().all(|ch| ch.is_ascii_digit())
}

fn is_noise_folder(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "music"
            | "audio"
            | "downloads"
            | "tracks"
            | "offline music"
            | "media.localized"
            | "automatically add to music"
            | "unknown artist"
    )
}

fn estimate_duration(file_size: i64) -> i32 {
    ((file_size as f64 / 16000.0) as i32).clamp(60, 600)
}

fn is_audio(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("mp3" | "flac" | "ogg" | "oga" | "wav" | "m4a" | "aac")
    )
}

fn ignored_library_entry(path: &Path, depth: usize) -> bool {
    if depth == 0 {
        return false;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if name.starts_with('.')
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "$recycle.bin" | "system volume information" | "node_modules" | "target" | "__macosx"
        )
    {
        return true;
    }
    if path.is_dir() {
        return matches!(
            path.extension()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("app" | "bundle" | "framework" | "plugin" | "component" | "pkg")
        );
    }
    false
}

#[cfg(test)]
#[test]
fn unsupported_opus_files_are_not_indexed() {
    assert!(!is_audio(Path::new("album/track.opus")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn explicit_scan_request_uses_its_paths_and_reports_cancellation() {
        let root =
            std::env::temp_dir().join(format!("musicos-explicit-scan-{}", std::process::id()));
        let media_dir = root.join("managed");
        let db_path = root.join("library.sqlite3");
        std::fs::create_dir_all(&media_dir).unwrap();
        std::fs::write(
            media_dir.join("Local Artist - Explicit Path.mp3"),
            b"minimal fixture",
        )
        .unwrap();

        let cancel = AtomicBool::new(false);
        let mut observed = Vec::new();
        let result = scan_library_with_progress(
            &db_path,
            &ScanRequest {
                managed_root: media_dir.clone(),
                reference_source_ids: Some(Vec::new()),
                mode: ScanMode::Changed,
            },
            &cancel,
            |progress| observed.push(progress),
        )
        .unwrap();
        assert_eq!(result.discovered, 1);
        assert_eq!(result.processed, 1);
        assert!(!observed.is_empty());
        assert_eq!(
            Db::open(db_path.to_str().unwrap())
                .unwrap()
                .library_counts()
                .unwrap()
                .tracks,
            1
        );

        cancel.store(true, Ordering::Relaxed);
        let cancelled = scan_library_with_progress(
            &db_path,
            &ScanRequest {
                managed_root: media_dir,
                reference_source_ids: Some(Vec::new()),
                mode: ScanMode::Full,
            },
            &cancel,
            |_| {},
        )
        .unwrap();
        assert!(cancelled.cancelled);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn apple_music_paths_use_folders_before_track_numbers() {
        let path = Path::new(
            "/Users/henry/Music/Music/Media.localized/Music/TAS 1000/A Message for Marta/02 - (I've Been) Delayed.mp3",
        );

        assert_eq!(
            metadata_from_path(path),
            PathMetadata {
                title: "(I've Been) Delayed".to_string(),
                artist: Some("TAS 1000".to_string()),
                album: Some("A Message for Marta".to_string()),
                track_number: Some(2),
            }
        );
    }

    #[test]
    fn filename_artist_title_is_used_when_folders_are_not_helpful() {
        let path = Path::new("/music/Loose Songs/Bjork - Hyperballad.flac");

        assert_eq!(
            metadata_from_path(path),
            PathMetadata {
                title: "Hyperballad".to_string(),
                artist: Some("Bjork".to_string()),
                album: Some("Loose Songs".to_string()),
                track_number: None,
            }
        );
    }

    #[test]
    fn numeric_artists_are_treated_as_bad_metadata() {
        assert!(is_bad_artist(Some("02")));
        assert!(is_bad_artist(Some("001")));
        assert!(!is_bad_artist(Some("TAS 1000")));
    }

    #[test]
    fn tag_titles_drop_numeric_dash_prefixes_only() {
        assert_eq!(
            clean_title_tag(Some("02 - (I've Been) Delayed")),
            Some("(I've Been) Delayed".to_string())
        );
        assert_eq!(
            clean_title_tag(Some("99 Problems")),
            Some("99 Problems".to_string())
        );
    }

    #[test]
    fn folder_cover_art_lookup_is_case_insensitive() {
        let root = std::env::temp_dir().join(format!("musicos-cover-art-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let image_path = root.join("Cover.PNG");
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]));
        image.save(&image_path).unwrap();

        let art = extract_cover_art(root.join("01 - Demo.mp3").to_str().unwrap()).unwrap();
        assert_eq!(art.width, 2);
        assert_eq!(art.height, 2);
        assert_eq!(art.pixels[0], Color::rgba(20, 40, 60, 255));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn album_art_does_not_leak_from_artist_folder() {
        let root =
            std::env::temp_dir().join(format!("musicos-cover-art-boundary-{}", std::process::id()));
        let artist = root.join("Artist");
        let album = artist.join("Album");
        std::fs::create_dir_all(&album).unwrap();
        let artist_image = image::RgbaImage::from_pixel(2, 2, image::Rgba([200, 10, 20, 255]));
        artist_image.save(artist.join("cover.png")).unwrap();

        assert!(extract_cover_art(album.join("01 Song.mp3").to_str().unwrap()).is_none());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn disc_subfolders_can_use_their_album_folder_art() {
        let root =
            std::env::temp_dir().join(format!("musicos-cover-art-disc-{}", std::process::id()));
        let album = root.join("Album");
        let disc = album.join("Disc 2");
        std::fs::create_dir_all(&disc).unwrap();
        let album_image = image::RgbaImage::from_pixel(2, 2, image::Rgba([30, 90, 180, 255]));
        album_image.save(album.join("folder.png")).unwrap();

        let art = extract_cover_art(disc.join("01 Song.flac").to_str().unwrap()).unwrap();
        assert_eq!(art.pixels[0], Color::rgba(30, 90, 180, 255));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cached_cover_art_is_persisted_and_reused() {
        let root =
            std::env::temp_dir().join(format!("musicos-cover-art-cache-{}", std::process::id()));
        let music_dir = root.join("music");
        let db_path = root.join("media.sqlite3");
        std::fs::create_dir_all(&music_dir).unwrap();
        let image_path = music_dir.join("cover.png");
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([80, 20, 120, 255]));
        image.save(&image_path).unwrap();

        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let track_path = music_dir.join("01 - Cache Me.mp3");
        let track_id = db
            .insert_track_with_number(
                track_path.to_str().unwrap(),
                "Cache Me",
                "MusicOS",
                "Persistence",
                1,
                120,
                1024,
                55,
            )
            .unwrap();
        let track = db.get_track_by_id(track_id).unwrap().unwrap();
        let art = cached_cover_art_for_track(&db, &track).unwrap();
        assert_eq!(art.width, 2);
        assert_eq!(art.pixels[0], Color::rgba(80, 20, 120, 255));

        let refreshed = db.get_track_by_id(track_id).unwrap().unwrap();
        let cached_path = refreshed.cover_art_path.as_ref().unwrap();
        assert!(Path::new(cached_path).exists());
        assert_eq!(
            Path::new(cached_path)
                .parent()
                .and_then(Path::file_name)
                .and_then(|value| value.to_str()),
            Some(COVER_ART_CACHE_VERSION_DIR)
        );
        assert_eq!(refreshed.cover_art_source_modified_at, Some(55));
        assert_eq!(db.cover_art_cache_count().unwrap(), 1);

        let reused = cached_cover_art_for_track(&db, &refreshed).unwrap();
        assert_eq!(reused.pixels[0], Color::rgba(80, 20, 120, 255));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scanner_supports_ogg_audio_and_skips_application_bundles() {
        let root = std::env::temp_dir().join(format!("musicos-scan-filter-{}", std::process::id()));
        let album = root.join("Artist").join("Album");
        let application = root
            .join("Mounted Tool.app")
            .join("Contents")
            .join("Resources");
        std::fs::create_dir_all(&album).unwrap();
        std::fs::create_dir_all(&application).unwrap();
        std::fs::write(album.join("01 Song.ogg"), b"fixture").unwrap();
        std::fs::write(album.join("02 Song.oga"), b"fixture").unwrap();
        std::fs::write(application.join("alert.wav"), b"fixture").unwrap();

        let (files, _) = audio_files_under(&root);
        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|path| !path.to_string_lossy().contains(".app")));

        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)] mod filename_order_tests {use super::*;#[test]fn padded_track_prefix_can_use_whitespace(){assert_eq!(track_number_from_stem("12 ARE WE STILL FRIENDS-"),Some(12));assert_eq!(track_number_from_stem("01. IGOR'S THEME"),Some(1));assert_eq!(track_number_from_stem("2019 - IGOR"),None);assert_eq!(track_number_from_stem("7 rings"),None);assert_eq!(track_number_from_stem("12"),None);}}

#[cfg(test)]mod organization_tag_tests{
 use super::*;
 #[test]fn managed_copy_hints_use_album_artist_and_album_without_existing_folders(){
  use lofty::tag::{TagExt,ItemKey};
  let dir=std::env::temp_dir().join(format!("musicos-copy-tags-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();let path=dir.join("loose.wav");let data=[0_u8;400];let mut wav=Vec::new();wav.extend(b"RIFF");wav.extend((36_u32+data.len()as u32).to_le_bytes());wav.extend(b"WAVEfmt ");wav.extend(16_u32.to_le_bytes());wav.extend(1_u16.to_le_bytes());wav.extend(1_u16.to_le_bytes());wav.extend(48000_u32.to_le_bytes());wav.extend(96000_u32.to_le_bytes());wav.extend(2_u16.to_le_bytes());wav.extend(16_u16.to_le_bytes());wav.extend(b"data");wav.extend((data.len()as u32).to_le_bytes());wav.extend(data);std::fs::write(&path,wav).unwrap();let mut tag=LoftyTag::new(lofty::tag::TagType::Id3v2);tag.set_artist("Artist A + Guest".into());tag.insert_text(ItemKey::AlbumArtist,"Artist A".into());tag.set_album("Tagged album".into());tag.save_to_path(&path,WriteOptions::default()).unwrap();let before=std::fs::read(&path).unwrap();assert_eq!(organization_tags(&path),(Some("Artist A".into()),Some("Tagged album".into())));assert_eq!(std::fs::read(&path).unwrap(),before);std::fs::remove_dir_all(&dir).unwrap();
 }
}
