use std::path::PathBuf;

use anyhow::{Context, Result};
use musicos_init::database::Db;

pub fn ensure_artwork(db_path: PathBuf, track_id: i64) -> Result<Option<PathBuf>> {
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    let Some(track) = db.get_track_by_id(track_id)? else {
        return Ok(None);
    };
    let _ = musicos_init::media::cached_cover_art_for_track(&db, &track);
    Ok(db
        .get_track_by_id(track_id)?
        .and_then(|track| track.cover_art_path)
        .map(PathBuf::from))
}
