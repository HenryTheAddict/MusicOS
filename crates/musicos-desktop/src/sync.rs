use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use reqwest::blocking::Body;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use walkdir::WalkDir;

use crate::api_client::ApiClient;
use crate::library_import::sha256_file;
use crate::models::{SyncProgress, TranscodeOptions};
use crate::transcode::{prepare_sync_file, validate_transcode_environment};

pub fn resolve_sync_files(root: &Path, requested: Option<&[PathBuf]>) -> Result<Vec<PathBuf>> {
    let canonical_root = fs::canonicalize(root)
        .with_context(|| format!("resolving sync root {}", root.display()))?;
    let mut files = Vec::new();
    if let Some(requested) = requested {
        for path in requested {
            let candidate = fs::canonicalize(path)
                .with_context(|| format!("resolving sync selection {}", path.display()))?;
            anyhow::ensure!(
                candidate.starts_with(&canonical_root),
                "sync selection is outside the music folder: {}",
                path.display()
            );
            anyhow::ensure!(
                candidate.is_file(),
                "sync selection is not a file: {}",
                path.display()
            );
            files.push(candidate);
        }
    } else {
        for entry in WalkDir::new(&canonical_root).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() && is_audio_file(entry.path()) {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

pub fn sync_files(
    root: &Path,
    requested: Option<&[PathBuf]>,
    target: &ApiClient,
    transcode: &TranscodeOptions,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(&SyncProgress),
) -> Result<SyncProgress> {
    validate_transcode_environment(transcode)?;
    let canonical_root = fs::canonicalize(root)?;
    let files = resolve_sync_files(&canonical_root, requested)?;
    let mut state = SyncProgress {
        running: true,
        found: files.len(),
        started_at: Some(timestamp()),
        ..Default::default()
    };
    on_progress(&state);

    for source in files {
        if cancel.load(Ordering::Relaxed) {
            state.cancel_requested = true;
            break;
        }
        let relative = source
            .strip_prefix(&canonical_root)
            .context("sync source escaped canonical root")?;
        state.current = relative.to_string_lossy().replace('\\', "/");
        on_progress(&state);

        let result = (|| -> Result<bool> {
            let prepared = prepare_sync_file(&source, relative, transcode)?;
            let source_size = fs::metadata(&source)?.len();
            let output_metadata = fs::metadata(&prepared.path)?;
            let output_size = output_metadata.len();
            let hash = sha256_file(&prepared.path)?;
            let modified = output_metadata
                .modified()
                .unwrap_or(SystemTime::now())
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            state.source_bytes += source_size;
            state.output_bytes += output_size;
            if prepared.temporary_dir.is_some() {
                state.transcoded += 1;
            }
            let mut last_error = None;
            for attempt in 0..3 {
                match upload_file(
                    target,
                    &prepared.path,
                    &prepared.relative_path,
                    &hash,
                    output_size,
                    modified,
                ) {
                    Ok(skipped) => return Ok(skipped),
                    Err(error) => {
                        last_error = Some(error);
                        if attempt < 2 && !cancel.load(Ordering::Relaxed) {
                            std::thread::sleep(std::time::Duration::from_millis(
                                150 * (attempt + 1),
                            ));
                        }
                    }
                }
            }
            Err(last_error.expect("an upload attempt always records an error"))
        })();

        match result {
            Ok(skipped) if skipped => state.skipped += 1,
            Ok(_) => state.uploaded += 1,
            Err(error) => {
                state.failed += 1;
                state.failed_files.push(source.clone());
                state
                    .errors
                    .push(format!("{}: {error:#}", relative.display()));
            }
        }
        on_progress(&state);
    }

    state.current.clear();
    if !state.cancel_requested {
        if let Err(error) = target.control("scan_library") {
            state.failed += 1;
            state.errors.push(format!("scan: {error:#}"));
        }
    }
    state.running = false;
    state.finished_at = Some(timestamp());
    on_progress(&state);
    Ok(state)
}

fn upload_file(
    target: &ApiClient,
    file: &Path,
    relative: &Path,
    sha256: &str,
    file_size: u64,
    modified_at: u128,
) -> Result<bool> {
    let mut headers = HeaderMap::new();
    insert_header(&mut headers, "x-filename", file_name(relative))?;
    insert_header(
        &mut headers,
        "x-relative-path",
        &relative.to_string_lossy().replace('\\', "/"),
    )?;
    insert_header(&mut headers, "x-sha256", sha256)?;
    insert_header(&mut headers, "x-file-size", &file_size.to_string())?;
    insert_header(&mut headers, "x-modified-at", &modified_at.to_string())?;
    insert_header(&mut headers, "x-trigger-scan", "0")?;
    let response = target
        .raw_client()
        .post(target.url("/api/upload"))
        .headers(headers)
        .body(Body::new(File::open(file)?))
        .send()
        .context("uploading music file")?;
    let status = response.status();
    let body = response.text().unwrap_or_default();
    anyhow::ensure!(
        status.is_success(),
        "upload returned HTTP {}{}",
        status.as_u16(),
        if body.is_empty() {
            String::new()
        } else {
            format!(": {body}")
        }
    );
    let value: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    Ok(value.get("status").and_then(|value| value.as_str()) == Some("skipped"))
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) -> Result<()> {
    headers.insert(
        HeaderName::from_static(name),
        HeaderValue::from_str(value).with_context(|| format!("invalid {name} header"))?,
    );
    Ok(())
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("track.audio")
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "aac" | "flac" | "m4a" | "mp3" | "ogg" | "oga" | "wav"
            )
        })
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_requested_file_outside_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let outside = temp.path().join("outside.mp3");
        fs::write(&outside, b"outside").unwrap();
        assert!(resolve_sync_files(&root, Some(&[outside])).is_err());
    }

    #[test]
    fn recursively_collects_supported_audio_only() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("Artist/Album");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("Track.flac"), b"audio").unwrap();
        fs::write(nested.join("notes.txt"), b"notes").unwrap();
        let files = resolve_sync_files(temp.path(), None).unwrap();
        assert_eq!(files.len(), 1);
    }
}
