use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

use crate::database::Db;
use crate::renderer::{Color, CoverArt};
use id3::Tag;
use image::GenericImageView;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;

static COVER_ART_CACHE: OnceLock<Mutex<HashMap<String, Option<CoverArt>>>> = OnceLock::new();

pub fn cached_cover_art(track_path: &str) -> Option<CoverArt> {
    let cache = COVER_ART_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = cache.lock() {
        if let Some(art) = cache.get(track_path) {
            return art.clone();
        }
    }

    let art = extract_cover_art(track_path);
    if let Ok(mut cache) = cache.lock() {
        if cache.len() > 128 {
            cache.clear();
        }
        cache.insert(track_path.to_string(), art.clone());
    }
    art
}

pub fn extract_cover_art(track_path: &str) -> Option<CoverArt> {
    if let Ok(tagged_file) = lofty::read_from_path(track_path) {
        let picture = tagged_file
            .primary_tag()
            .and_then(|tag| tag.pictures().first())
            .or_else(|| {
                tagged_file
                    .first_tag()
                    .and_then(|tag| tag.pictures().first())
            });
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
        if let Some(picture) = tag.pictures().next() {
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
    // album folder first, then its parent, and accept common case variants.
    if let Some(parent) = Path::new(track_path).parent() {
        for dir in parent.ancestors().take(2) {
            if let Some(path) = find_folder_cover_art(dir) {
                if let Some(art) = load_cover_art_file(&path) {
                    return Some(art);
                }
            }
        }
    }

    None
}

pub fn load_cover_art_file(path: &Path) -> Option<CoverArt> {
    image::open(path).ok().map(cover_art_from_image)
}

fn cover_art_from_image(img: image::DynamicImage) -> CoverArt {
    let img = img.resize_to_fill(600, 600, image::imageops::FilterType::Lanczos3);
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
        if NAMES.iter().any(|candidate| lowered == candidate.to_ascii_lowercase()) {
            return Some(path);
        }
    }

    None
}

pub fn index_default_locations(db: &Db) -> Result<usize> {
    let mut roots = vec![
        "/mnt/sdcard".to_string(),
        "/media".to_string(),
        "/roms".to_string(),
        "/boot/music".to_string(),
        "/music".to_string(),
    ];

    if Path::new("./music").exists() {
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

    let mut indexed = 0;
    for root in roots {
        indexed += index_location(db, Path::new(&root))?;
    }
    Ok(indexed)
}

pub fn index_location(db: &Db, root: &Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut indexed = 0;
    let reindex_all = std::env::var("MUSICOS_REINDEX_ALL").as_deref() == Ok("1");
    for entry in WalkDir::new(root).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() || !is_audio(entry.path()) {
            continue;
        }
        let path_str = entry.path().display().to_string();
        let Ok(stats) = file_stats(entry.path()) else {
            continue;
        };
        if !reindex_all {
            if let Ok(true) = db.track_is_fresh(&path_str, stats.file_size, stats.modified_at) {
                continue;
            }
        }
        match index_file(db, entry.path(), stats) {
            Ok(()) => indexed += 1,
            Err(err) => log::warn!("failed to index file {}: {err}", entry.path().display()),
        }
    }
    Ok(indexed)
}

fn index_file(db: &Db, path: &Path, stats: FileStats) -> Result<()> {
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

    db.insert_track_with_number(
        &path_str,
        &title,
        &artist,
        &album,
        track_number,
        duration,
        stats.file_size,
        stats.modified_at,
    )?;
    Ok(())
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

fn track_number_from_stem(stem: &str) -> Option<i32> {
    let trimmed = stem.trim();
    let digits: String = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() || digits.len() > 3 {
        return None;
    }

    let rest = trimmed[digits.len()..].trim_start();
    if rest.is_empty() || !matches!(rest.chars().next(), Some('.') | Some('-') | Some('_')) {
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
        "music" | "media.localized" | "automatically add to music" | "unknown artist"
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
        Some("mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let root = std::env::temp_dir().join(format!(
            "musicos-cover-art-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let image_path = root.join("Cover.PNG");
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]));
        image.save(&image_path).unwrap();

        let art = extract_cover_art(root.join("01 - Demo.mp3").to_str().unwrap()).unwrap();
        assert_eq!(art.width, 600);
        assert_eq!(art.height, 600);
        assert_eq!(art.pixels[0], Color::rgba(20, 40, 60, 255));

        let _ = std::fs::remove_dir_all(root);
    }
}
