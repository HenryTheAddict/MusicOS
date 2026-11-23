//! Explicitly reviewed, same-volume moves. No file tags, memberships or track IDs change.
use anyhow::{ensure, Context, Result};
use rusqlite::{params};
use musicos_init::database::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};
#[derive(Clone, Serialize, Deserialize)]
pub struct Move {
    pub id: i64,
    pub from: String,
    pub to: String,
    pub hash: String,
    pub revision: i64,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Preview {
    pub token: String,
    pub moves: Vec<Move>,
    pub skipped: usize,
    pub undo_available: usize,
    #[serde(default)]
    pub has_pending_recovery: bool,
    #[serde(default)]pub issues:Vec<PreflightIssue>,
}
#[derive(Clone,Serialize,Deserialize)]pub struct PreflightIssue{pub path:String,pub message:String}
fn connection(path: &Path) -> Result<Connection> {
    let db = Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    musicos_init::database_access::initialize_schema(path,"file-organization-v1",||{db.execute_batch("CREATE TABLE IF NOT EXISTS desktop_organization_plan(token TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS desktop_organization_history(id INTEGER PRIMARY KEY,track_id INTEGER NOT NULL,old_path TEXT NOT NULL,new_path TEXT NOT NULL,hash TEXT NOT NULL,state TEXT NOT NULL)")?;Ok(())})?;
    Ok(db)
}
fn hash(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn segment(value: &str, fallback: &str) -> String {
    musicos_desktop::library_import::sanitize_segment(value, fallback)
        .chars()
        .take(100)
        .collect()
}
fn undo_count(db: &Connection) -> Result<usize> {
    Ok(db.query_row(
        "SELECT count(*) FROM desktop_organization_history WHERE state IN ('moved','cleanup','undoing','undo_cleanup')",
        [],
        |r| r.get::<_, i64>(0),
    )? as usize)
}
pub fn preview(db_path:&Path,library:&crate::library::Library)->Result<Preview>{preview_with_progress(db_path,library,&std::sync::atomic::AtomicBool::new(false),|_,_|{})}
pub fn preview_with_progress(db_path: &Path, library: &crate::library::Library,cancel:&std::sync::atomic::AtomicBool,mut progress:impl FnMut(usize,usize)) -> Result<Preview> {
    let db = connection(db_path)?;
    let mut result = Preview {
        undo_available: undo_count(&db)?,
        has_pending_recovery: has_pending_recovery(&db)?,
        ..Default::default()
    };
    let mut reserved = std::collections::HashSet::new();
    for (index,track) in library.tracks.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed){break;}progress(index,library.tracks.len());
        let Some(source) = library
            .sources
            .iter()
            .filter(|s| s.enabled && s.available && s.kind != "managed")
            .filter(|s| Path::new(&track.path).starts_with(&s.path))
            .max_by_key(|s| s.path.components().count())
        else {
            result.skipped += 1;
            continue;
        };
        let from = Path::new(&track.path);
        if !from.is_file() || std::fs::symlink_metadata(from)?.file_type().is_symlink() {
            result.skipped += 1;
            continue;
        }
        let root = std::fs::canonicalize(&source.path)?;
        let original = std::fs::canonicalize(from)?;
        if !original.starts_with(&root) {
            result.skipped += 1;
            continue;
        }
        let artist = track
            .album_artist
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::library::split_artist_credits(&track.artist)[0]);
        let parent = root
            .join(segment(artist, "Unknown Artist"))
            .join(segment(&track.album, "Unknown Album"));
        let filename = original.file_name().context("Song has no filename")?;
        let mut to = parent.join(filename);
        if original == to {
            continue;
        }
        // Collisions get stable IDs, never replacement or content-based deduplication.
        if to.exists() || reserved.contains(&to) {
            let stem = original.file_stem().unwrap().to_string_lossy();
            let ext = original
                .extension()
                .map(|s| format!(".{}", s.to_string_lossy()))
                .unwrap_or_default();
            to = parent.join(format!("{stem} ({}){ext}", track.id));
        }
        if to.exists() || !reserved.insert(to.clone()) {
            result.skipped += 1;
            continue;
        }
        let preflight=(||->Result<()>{ensure!(!std::fs::metadata(original.parent().unwrap())?.permissions().readonly(),"Folder is read-only. Choose it again to grant access.");let mut parent=to.parent().context("Missing destination")?;while !parent.exists(){parent=parent.parent().context("No writable parent folder")?;}ensure!(!parent.metadata()?.permissions().readonly(),"Destination folder is read-only");let probe=parent.join(format!(".musicos-preflight-{}-{}",std::process::id(),track.id));std::fs::hard_link(&original,&probe).context("Cannot organize here. Choose this folder again to grant access, or use a volume that supports safe moves.")?;std::fs::remove_file(probe)?;Ok(())})();if let Err(error)=preflight{result.issues.push(PreflightIssue{path:track.path.clone(),message:error.to_string()});continue;}
        let revision = db.query_row(
            "SELECT revision FROM tracks WHERE id=? AND deleted_at IS NULL",
            [track.id],
            |r| r.get(0),
        )?;
        result.moves.push(Move {
            id: track.id,
            from: track.path.clone(),
            to: to.to_string_lossy().into_owned(),
            hash: hash(from)?,
            revision,
        });
    }
    result.token = format!("{:x}", Sha256::digest(serde_json::to_vec(&result.moves)?));
    let tx = db.unchecked_transaction()?;
    tx.execute("DELETE FROM desktop_organization_plan", [])?;
    tx.execute(
        "INSERT INTO desktop_organization_plan VALUES(?,?)",
        params![result.token, serde_json::to_string(&result)?],
    )?;
    tx.commit()?;
    Ok(result)
}
// Checking canonical ancestry before creating/linking prevents a changed symlink
// directory from sending files outside the reviewed source root.
fn safe_parent(to: &Path, root: &Path) -> Result<()> {
    let relative = to.strip_prefix(root).ok().map(Path::to_path_buf);
    let root = std::fs::canonicalize(root)?;
    let to_owned = if let Some(parent) = to.parent().filter(|p| p.is_dir()) {
        std::fs::canonicalize(parent)?.join(to.file_name().context("Missing filename")?)
    } else {
        relative
            .map(|p| root.join(p))
            .unwrap_or_else(|| to.to_path_buf())
    };
    let to = to_owned.as_path();
    ensure!(
        to.starts_with(&root),
        "Destination is outside its music folder"
    );
    let mut current = root.clone();
    for part in to
        .parent()
        .context("Missing destination folder")?
        .strip_prefix(&root)?
        .components()
    {
        ensure!(
            matches!(part, std::path::Component::Normal(_)),
            "Invalid destination"
        );
        current.push(part);
        if !current.exists() {
            std::fs::create_dir(&current)?;
        }
        ensure!(
            !std::fs::symlink_metadata(&current)?
                .file_type()
                .is_symlink(),
            "A destination folder became a symbolic link"
        );
        ensure!(
            std::fs::canonicalize(&current)?.starts_with(&root),
            "Destination left its music folder"
        );
    }
    Ok(())
}
pub fn apply(db_path:&Path,token:&str)->Result<usize>{apply_with_progress(db_path,token,&std::sync::atomic::AtomicBool::new(false),|_,_|{})}
pub fn apply_with_progress(db_path: &Path, token: &str,cancel:&std::sync::atomic::AtomicBool,mut progress:impl FnMut(usize,usize)) -> Result<usize> {
    let db = connection(db_path)?;
    let plan: Preview = serde_json::from_str(
        &db.query_row(
            "SELECT value FROM desktop_organization_plan WHERE token=?",
            [token],
            |r| r.get::<_, String>(0),
        )
        .context("Preview expired. Preview again.")?,
    )?;
    // Validate the entire review before making the first change.
    for item in &plan.moves {
        let (path,revision,enabled):(String,i64,bool)=db.query_row("SELECT t.path,t.revision,s.enabled FROM tracks t JOIN library_sources s ON s.id=t.source_id WHERE t.id=? AND t.deleted_at IS NULL",[item.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        ensure!(
            path == item.from && revision == item.revision && enabled,
            "Library changed. Preview again."
        );
        ensure!(
            hash(Path::new(&item.from))? == item.hash && !Path::new(&item.to).exists(),
            "A file changed or destination is occupied. Preview again."
        );
    }
    let mut completed = 0;let total=plan.moves.len();
    for item in plan.moves {
        if cancel.load(std::sync::atomic::Ordering::Relaxed){break;}
        let root: String = db.query_row(
            "SELECT s.path FROM library_sources s JOIN tracks t ON t.source_id=s.id WHERE t.id=?",
            [item.id],
            |r| r.get(0),
        )?;
        safe_parent(Path::new(&item.to), Path::new(&root))?;
        // create-new hard link is atomic and cannot overwrite another file. No bytes
        // are transcoded; source removal happens only after the DB path is durable.
        db.execute("INSERT INTO desktop_organization_history(track_id,old_path,new_path,hash,state) VALUES(?,?,?,?,'planned')",params![item.id,item.from,item.to,item.hash])?;let journal_id=db.last_insert_rowid();
        if let Err(error)=std::fs::hard_link(&item.from,&item.to){db.execute("UPDATE desktop_organization_history SET state='aborted' WHERE id=?",[journal_id])?;return Err(anyhow::anyhow!("Cannot move {}: {error}. Original retained; choose its folder again if access is needed.",item.from));}
        let write = (|| -> Result<i64> {
            let tx = db.unchecked_transaction()?;
            ensure!(tx.execute("UPDATE tracks SET path=?,revision=revision+1 WHERE id=? AND path=? AND revision=?",params![item.to,item.id,item.from,item.revision])?==1,"Song changed. Preview again.");
            tx.execute("UPDATE desktop_organization_history SET state='cleanup' WHERE id=?",[journal_id])?;
            let id = journal_id;
            tx.commit()?;
            Ok(id)
        })();
        let id = match write {
            Ok(id) => id,
            Err(e) => {
                let _ = std::fs::remove_file(&item.to);
                db.execute("UPDATE desktop_organization_history SET state='aborted' WHERE id=?",[journal_id])?;
                return Err(e);
            }
        };
        if std::fs::remove_file(&item.from).is_ok() {
            db.execute(
                "UPDATE desktop_organization_history SET state='moved' WHERE id=?",
                [id],
            )?;
        }
        completed += 1;progress(completed,total);
    }
    db.execute("DELETE FROM desktop_organization_plan", [])?;
    Ok(completed)
}
/// Recover only journaled links whose byte identity and database path agree.
pub fn recover(db_path:&Path)->Result<()> {let db=connection(db_path)?;let rows=db.prepare("SELECT id,track_id,old_path,new_path,hash,state FROM desktop_organization_history WHERE state IN ('planned','cleanup','undoing','undo_cleanup')")?.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;for(id,track,old,new,digest,state)in rows{let current:String=db.query_row("SELECT path FROM tracks WHERE id=?",[track],|r|r.get(0))?;if state=="undo_cleanup"{ensure!(current==old&&hash(Path::new(&old))?==digest,"Restored file changed; cleanup needs review");if Path::new(&new).exists(){ensure!(hash(Path::new(&new))?==digest,"Cleanup location changed; files retained");std::fs::remove_file(&new)?;}db.execute("UPDATE desktop_organization_history SET state='undone' WHERE id=?",[id])?;}else if state=="undoing"{ensure!(hash(Path::new(&new))?==digest,"A moved song changed; Undo needs review");if !Path::new(&old).exists(){std::fs::hard_link(&new,&old)?;}ensure!(hash(Path::new(&old))?==digest,"The original location is occupied; both files retained");let tx=db.unchecked_transaction()?;tx.execute("UPDATE tracks SET path=?,revision=revision+1 WHERE id=? AND path IN (?,?)",params![old,track,old,new])?;tx.execute("UPDATE desktop_organization_history SET state='undo_cleanup' WHERE id=?",[id])?;tx.commit()?;std::fs::remove_file(&new)?;db.execute("UPDATE desktop_organization_history SET state='undone' WHERE id=?",[id])?;}else if state=="planned"&&current==old {if Path::new(&new).exists(){ensure!(hash(Path::new(&new))?==digest&&hash(Path::new(&old))?==digest,"An interrupted move needs review; both files were retained");std::fs::remove_file(&new)?;}db.execute("UPDATE desktop_organization_history SET state='aborted' WHERE id=?",[id])?;}else if current==new&&hash(Path::new(&new))?==digest {if Path::new(&old).exists(){ensure!(hash(Path::new(&old))?==digest,"The old location changed; both files were retained");std::fs::remove_file(&old)?;}db.execute("UPDATE desktop_organization_history SET state='moved' WHERE id=?",[id])?;}}Ok(())}
fn has_pending_recovery(db: &Connection) -> Result<bool> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM desktop_organization_history WHERE state IN ('planned','cleanup','undoing','undo_cleanup'))", [], |row| row.get(0))?)
}
pub fn pending(db_path: &Path) -> Result<bool> {
    has_pending_recovery(&connection(db_path)?)
}
pub fn undo(db_path: &Path) -> Result<usize> {
    let db = connection(db_path)?;
    let mut query=db.prepare("SELECT id,track_id,old_path,new_path,hash FROM desktop_organization_history WHERE state IN ('moved','cleanup','undoing','undo_cleanup') ORDER BY id DESC")?;
    let rows = query
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(query);
    let mut count = 0;
    for (id, track, old, new, digest) in rows {
        let current: String =
            db.query_row("SELECT path FROM tracks WHERE id=?", [track], |r| r.get(0))?;
        ensure!(
            current == new && hash(Path::new(&new))? == digest,
            "A moved song changed. Undo stopped without replacing it."
        );
        let root: String = db.query_row(
            "SELECT s.path FROM library_sources s JOIN tracks t ON t.source_id=s.id WHERE t.id=?",
            [track],
            |r| r.get(0),
        )?;
        safe_parent(Path::new(&old), Path::new(&root))?;
        if Path::new(&old).exists(){ensure!(hash(Path::new(&old))?==digest,"Original location is occupied. Undo stopped.");}
        db.execute("UPDATE desktop_organization_history SET state='undoing' WHERE id=?",[id])?;
        if Path::new(&old).exists() {
            ensure!(
                hash(Path::new(&old))? == digest,
                "Original location is occupied. Undo stopped."
            );
        } else {
            std::fs::create_dir_all(Path::new(&old).parent().unwrap())?;
            std::fs::hard_link(&new, &old)?;
        }
        let tx = db.unchecked_transaction()?;
        tx.execute(
            "UPDATE tracks SET path=?,revision=revision+1 WHERE id=?",
            params![old, track],
        )?;
        tx.execute(
            "UPDATE desktop_organization_history SET state='undo_cleanup' WHERE id=?",
            [id],
        )?;
        tx.commit()?;
        // Both locations contain verified identical bytes; removing our new link
        // leaves the restored original. Failures leave a harmless extra copy.
        std::fs::remove_file(&new)?;db.execute("UPDATE desktop_organization_history SET state='undone' WHERE id=?",[id])?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn fixture() -> (tempfile::TempDir, PathBuf, crate::library::Library) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("library.db");
        let root = temp.path().join("Loose music");
        std::fs::create_dir(&root).unwrap();
        let file = root.join("Song.flac");
        std::fs::write(&file, b"original audio bytes").unwrap();
        let db = musicos_init::database::Db::open(path.to_str().unwrap()).unwrap();
        let source = db
            .add_library_source("reference", root.to_str().unwrap(), "Loose music")
            .unwrap();
        db.insert_track(
            file.to_str().unwrap(),
            "Song",
            "Artist",
            "Release",
            30,
            20,
            1,
        )
        .unwrap();
        let track = db.get_tracks().unwrap().remove(0);
        db.assign_track_source(track.id, source.id).unwrap();
        let playlist = db.create_playlist("Keep").unwrap();
        db.add_tracks_to_playlist(playlist, &[track.id]).unwrap();
        db.toggle_favorite(track.id).unwrap();
        let library = crate::library::Library {
            protected_source_ids:Vec::new(),
            tracks: vec![musicos_desktop::local::library::track_from_init(
                db.get_track_by_id(track.id).unwrap().unwrap(),
            )],
            albums: vec![],
            artists: vec![],
            genres: vec![],
            sources: vec![musicos_desktop::models::LibrarySource {
                id: source.id,
                path: root,
                display_name: "Loose music".into(),
                kind: "referenced".into(),
                enabled: true,
                available: true,
                last_scan_at: None,
                last_error: None,
                song_count: 1,
            }],
        };
        (temp, path, library)
    }
    #[test]
    fn canceled_organization_stops_between_files_and_interrupted_links_recover() {
        let (_temp, path, library) = fixture();
        let plan = preview(&path, &library).unwrap();
        assert!(!plan.has_pending_recovery);
        assert_eq!(
            apply_with_progress(
                &path,
                &plan.token,
                &std::sync::atomic::AtomicBool::new(true),
                |_, _| {}
            )
            .unwrap(),
            0
        );
        assert!(Path::new(&plan.moves[0].from).exists());
        let plan = preview(&path, &library).unwrap();
        let item = &plan.moves[0];
        safe_parent(Path::new(&item.to), &library.sources[0].path).unwrap();
        let db = connection(&path).unwrap();
        db.execute(
            "INSERT INTO desktop_organization_history(track_id,old_path,new_path,hash,state) VALUES(?,?,?,?,'planned')",
            params![item.id, item.from, item.to, item.hash],
        ).unwrap();
        std::fs::hard_link(&item.from, &item.to).unwrap();
        assert!(preview(&path, &library).unwrap().has_pending_recovery);
        recover(&path).unwrap();
        assert!(Path::new(&item.from).exists());
        assert!(!Path::new(&item.to).exists());
        assert!(!preview(&path, &library).unwrap().has_pending_recovery);
    }
    #[test]
    fn reviewed_move_and_undo_preserve_bytes_ids_favorites_and_membership() {
        let (_temp, path, library) = fixture();
        let plan = preview(&path, &library).unwrap();
        assert_eq!(plan.moves.len(), 1);
        assert!(Path::new(&plan.moves[0].from).is_file());
        assert!(!Path::new(&plan.moves[0].to).exists());
        assert_eq!(apply(&path, &plan.token).unwrap(), 1);
        assert!(!Path::new(&plan.moves[0].from).exists());
        assert_eq!(
            std::fs::read(&plan.moves[0].to).unwrap(),
            b"original audio bytes"
        );
        let db = musicos_init::database::Db::open(path.to_str().unwrap()).unwrap();
        let track = db.get_track_by_id(library.tracks[0].id).unwrap().unwrap();
        assert!(track.favorited);
        assert_eq!(
            db.playlist_reference_count_for_tracks(&[track.id]).unwrap(),
            1
        );
        assert_eq!(undo(&path).unwrap(), 1);
        assert!(Path::new(&plan.moves[0].from).is_file());
        assert!(!Path::new(&plan.moves[0].to).exists());
    }
    #[test]
    fn collisions_and_stale_reviews_never_overwrite() {
        let (_temp, path, library) = fixture();
        let target = library.sources[0].path.join("Artist/Release/Song.flac");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"unrelated existing file").unwrap();
        let plan = preview(&path, &library).unwrap();
        assert_ne!(plan.moves[0].to, target.to_string_lossy());
        std::fs::write(&plan.moves[0].from, b"changed after preview").unwrap();
        assert!(apply(&path, &plan.token).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"unrelated existing file");
        assert!(Path::new(&plan.moves[0].from).exists());
    }
    #[test]
    fn undo_refuses_changed_recordings() {
        let (_temp, path, library) = fixture();
        let plan = preview(&path, &library).unwrap();
        apply(&path, &plan.token).unwrap();
        std::fs::write(&plan.moves[0].to, b"new bytes").unwrap();
        assert!(undo(&path).is_err());
        assert_eq!(std::fs::read(&plan.moves[0].to).unwrap(), b"new bytes");
    }
    #[cfg(unix)]
    #[test]
    fn changed_destination_symlink_is_rejected() {
        let (temp, path, library) = fixture();
        let plan = preview(&path, &library).unwrap();
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, library.sources[0].path.join("Artist")).unwrap();
        assert!(apply(&path, &plan.token).is_err());
        assert!(Path::new(&plan.moves[0].from).exists());
        assert!(!outside.join("Release/Song.flac").exists());
    }
}
