use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::models::ImportProgress;
use crate::storage::{atomic_write_json, read_json};

const INDEX_FILE: &str = ".musicos-import-index.json";
const AUDIO_EXTENSIONS: &[&str] = &["aac", "flac", "m4a", "mp3", "ogg", "oga", "wav"];

pub fn collect_audio_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut output = Vec::new();
    for input in inputs {
        let absolute = fs::canonicalize(input)
            .with_context(|| format!("resolving import path {}", input.display()))?;
        if absolute.is_file() {
            if is_audio_file(&absolute) {
                output.push(absolute);
            }
            continue;
        }
        for entry in WalkDir::new(&absolute).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() && is_audio_file(entry.path()) {
                output.push(entry.into_path());
            }
        }
    }
    output.sort();
    output.dedup();
    Ok(output)
}

pub fn import_managed_library(
    inputs: &[PathBuf],
    media_dir: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(&ImportProgress),
) -> Result<ImportProgress> {
    fs::create_dir_all(media_dir)?;
    let files = collect_audio_files(inputs)?;
    let index_path = media_dir.join(INDEX_FILE);
    let mut index: BTreeMap<String, String> = if index_path.exists() {
        read_json(&index_path).unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let mut state = ImportProgress {
        running: true,
        found: files.len(),
        ..Default::default()
    };
    on_progress(&state);

    for source in files {
        if cancel.load(Ordering::Relaxed) {
            state.cancel_requested = true;
            break;
        }
        state.current = source.display().to_string();
        match import_one(&source, media_dir, &mut index) {
            Ok(ImportOutcome::Imported) => state.imported += 1,
            Ok(ImportOutcome::Duplicate) => state.duplicates += 1,
            Err(error) => {
                state.failed += 1;
                state
                    .errors
                    .push(format!("{}: {error:#}", source.display()));
            }
        }
        state.processed += 1;
        on_progress(&state);
    }

    atomic_write_json(&index_path, &index)?;
    state.running = false;
    state.current.clear();
    on_progress(&state);
    Ok(state)
}

enum ImportOutcome {
    Imported,
    Duplicate,
}

fn import_one(
    source: &Path,
    media_dir: &Path,
    index: &mut BTreeMap<String, String>,
) -> Result<ImportOutcome> {
    let hash = sha256_file(source)?;
    if let Some(relative) = index.get(&hash) {
        if media_dir.join(relative).is_file() {
            return Ok(ImportOutcome::Duplicate);
        }
    }

    let preferred = destination_for(source, media_dir);
    if let Some(parent) = preferred.parent() {
        fs::create_dir_all(parent)?;
    }
    let destination = unique_destination(&preferred, &hash);
    let temporary = destination.with_extension(format!(
        "{}.importing-{}",
        destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("audio"),
        std::process::id()
    ));

    copy_verified(source, &temporary, &hash)?;
    fs::rename(&temporary, &destination)?;
    let relative = destination
        .strip_prefix(media_dir)
        .unwrap_or(&destination)
        .to_string_lossy()
        .to_string();
    index.insert(hash, relative);
    Ok(ImportOutcome::Imported)
}

fn copy_verified(source: &Path, destination: &Path, expected_hash: &str) -> Result<()> {
    let result = (|| -> Result<()> {
        let mut input = File::open(source)?;
        let mut output = File::create(destination)?;
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            output.write_all(&buffer[..count])?;
        }
        output.sync_all()?;
        anyhow::ensure!(
            sha256_file(destination)? == expected_hash,
            "copied file checksum did not match the source"
        );
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result
}

pub fn destination_for(source: &Path, media_dir: &Path) -> PathBuf {
    let album = source
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .map(|value| sanitize_segment(value, "Unknown Album"))
        .unwrap_or_else(|| "Unknown Album".to_string());
    let artist = source
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .map(|value| sanitize_segment(value, "Unknown Artist"))
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let title = source
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| sanitize_segment(value, "Untitled"))
        .unwrap_or_else(|| "Untitled".to_string());
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("audio")
        .to_ascii_lowercase();
    media_dir
        .join(artist)
        .join(album)
        .join(format!("{title}.{extension}"))
}

pub fn sanitize_segment(value: &str, fallback: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        {
            cleaned.push('_');
        } else {
            cleaned.push(character);
        }
    }
    let cleaned = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', ' '])
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn unique_destination(preferred: &Path, hash: &str) -> PathBuf {
    if !preferred.exists() {
        return preferred.to_path_buf();
    }
    let stem = preferred
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled");
    let extension = preferred
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("audio");
    preferred.with_file_name(format!("{stem}-{}.{}", &hash[..8], extension))
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| AUDIO_EXTENSIONS.contains(&value.to_ascii_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_platform_reserved_filename_characters() {
        assert_eq!(
            sanitize_segment("  Dawn: Part / One?  ", "Untitled"),
            "Dawn_ Part _ One_"
        );
    }

    #[test]
    fn imports_once_and_detects_duplicate_by_hash() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Artist").join("Album");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("Song.mp3"), b"fixture audio").unwrap();
        let media = temp.path().join("managed");
        let cancel = AtomicBool::new(false);
        let first = import_managed_library(&[source.clone()], &media, &cancel, |_| {}).unwrap();
        let second = import_managed_library(&[source], &media, &cancel, |_| {}).unwrap();
        assert_eq!(first.imported, 1);
        assert_eq!(second.duplicates, 1);
    }
}
