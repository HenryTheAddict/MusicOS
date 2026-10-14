use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{LibraryRepository, LocalLibraryRepository};

#[derive(Debug, Clone, Default)]
pub struct M3uImportResult {
    pub playlist_id: i64,
    pub imported: usize,
    pub missing: Vec<PathBuf>,
}

pub fn export_m3u(
    repository: &LocalLibraryRepository,
    playlist_id: i64,
    path: &Path,
) -> Result<()> {
    let db=musicos_init::database::Db::open(&repository.db_path().to_string_lossy())?;
    let tracks = db.get_playlist_tracks(playlist_id)?;
    let mut body = String::from("#EXTM3U\n");
    for track in tracks {
        body.push_str(&format!(
            "#EXTINF:{},{} - {}\n{}\n",
            track.duration.max(0),
            track.artist,
            track.title,
            track.path
        ));
    }
    fs::write(path, body).with_context(|| format!("writing {}", path.display()))
}

pub fn import_m3u(repository: &LocalLibraryRepository, path: &Path) -> Result<M3uImportResult> {
    let body =
        fs::read_to_string(path).with_context(|| format!("reading playlist {}", path.display()))?;
    let db=musicos_init::database::Db::open(&repository.db_path().to_string_lossy())?;
    let tracks = db.get_tracks()?;
    let playlist_name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported playlist");
    let playlist = repository.create_playlist(playlist_name)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut ids = Vec::new();
    let mut missing = Vec::new();
    for line in body.trim_start_matches('\u{feff}').lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let candidate = if Path::new(line).is_absolute() {
            PathBuf::from(line)
        } else {
            base.join(line)
        };
        let canonical = candidate.canonicalize().unwrap_or(candidate);
        if let Some(track) = tracks.iter().find(|track| {
            Path::new(&track.path)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&track.path))
                == canonical
        }) {
            if !ids.contains(&track.id){ids.push(track.id);}
        } else {
            missing.push(canonical);
        }
    }
    repository.add_tracks(playlist.id, &ids)?;
    Ok(M3uImportResult {
        playlist_id: playlist.id,
        imported: ids.len(),
        missing,
    })
}
