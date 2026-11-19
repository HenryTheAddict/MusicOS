use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};
use musicos_init::database::{Db, LibrarySource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedRoot {
    pub source_id: i64,
    pub path: PathBuf,
    pub canonical_path: PathBuf,
    pub display_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustedSourceSet {
    pub managed: Option<TrustedRoot>,
    pub references: Vec<TrustedRoot>,
}

impl TrustedSourceSet {
    pub fn roots(&self) -> impl Iterator<Item = &TrustedRoot> {
        self.managed.iter().chain(self.references.iter())
    }

    pub fn source_ids(&self) -> Vec<i64> {
        self.roots().map(|root| root.source_id).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenTrackRecord {
    pub id: i64,
    pub path: PathBuf,
    pub previous_source_id: Option<i64>,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenRootSummary {
    pub root: PathBuf,
    pub count: usize,
    pub mounted: bool,
    pub examples: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceAuditReport {
    pub trusted_tracks: usize,
    pub repaired_tracks: usize,
    pub hidden_tracks: Vec<HiddenTrackRecord>,
    pub grouped_roots: Vec<HiddenRootSummary>,
    pub revision: u64,
    pub trusted_source_ids: Vec<i64>,
    pub playlist_references: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCleanupReport {
    pub removed_records: usize,
    pub removed_playlist_references: usize,
    pub backup_path: PathBuf,
}

pub fn audit_sources(db: &Db, managed_root: &Path) -> Result<SourceAuditReport> {
    let sources = db.library_sources()?;
    let trusted = trusted_sources(&sources, managed_root)?;
    let records = db.track_source_records()?;
    let mut report = SourceAuditReport {
        trusted_source_ids: trusted.source_ids(),
        ..SourceAuditReport::default()
    };
    let mut grouped = BTreeMap::<PathBuf, (usize, Vec<PathBuf>)>::new();
    let mut revision = std::collections::hash_map::DefaultHasher::new();

    for record in records {
        let path = absolute_normalized(Path::new(&record.path));
        if let Some(root) = matching_root(&path, &trusted) {
            report.trusted_tracks += 1;
            if record.source_id != Some(root.source_id) {
                db.assign_track_source(record.id, root.source_id)?;
                report.repaired_tracks += 1;
            }
            continue;
        }

        record.id.hash(&mut revision);
        record.path.hash(&mut revision);
        record.source_id.hash(&mut revision);
        let group_root = display_group_root(&path);
        let entry = grouped.entry(group_root).or_default();
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(path.clone());
        }
        report.hidden_tracks.push(HiddenTrackRecord {
            id: record.id,
            exists: path.exists(),
            path,
            previous_source_id: record.source_id,
        });
    }

    report.revision = revision.finish();
    report.playlist_references = db.playlist_reference_count_for_tracks(
        &report
            .hidden_tracks
            .iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
    )?;
    report.grouped_roots = grouped
        .into_iter()
        .map(|(root, (count, examples))| HiddenRootSummary {
            mounted: root.exists(),
            root,
            count,
            examples,
        })
        .collect();
    report.grouped_roots.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.root.cmp(&right.root))
    });
    Ok(report)
}

fn trusted_sources(sources: &[LibrarySource], managed_root: &Path) -> Result<TrustedSourceSet> {
    let managed_path = absolute_normalized(managed_root);
    let mut set = TrustedSourceSet::default();
    // Disabled reference roots remain trusted ownership boundaries. They are
    // hidden from normal snapshots, but must never be misclassified as junk
    // that Diagnostics may offer to delete.
    for source in sources {
        let path = absolute_normalized(Path::new(&source.path));
        let root = TrustedRoot {
            source_id: source.id,
            canonical_path: path.canonicalize().unwrap_or_else(|_| path.clone()),
            path: path.clone(),
            display_name: source.display_name.clone(),
        };
        if source.kind == "reference" {
            set.references.push(root);
        } else if same_path(&path, &managed_path) {
            set.managed = Some(root);
        }
    }
    anyhow::ensure!(
        set.managed.is_some(),
        "configured managed library source {} is missing",
        managed_root.display()
    );
    set.references.sort_by(|left, right| {
        right
            .path
            .components()
            .count()
            .cmp(&left.path.components().count())
    });
    Ok(set)
}

fn matching_root<'a>(path: &Path, trusted: &'a TrustedSourceSet) -> Option<&'a TrustedRoot> {
    let canonical = path.canonicalize().ok();
    trusted
        .roots()
        .filter(|root| {
            path.starts_with(&root.path)
                || canonical
                    .as_deref()
                    .is_some_and(|value| value.starts_with(&root.canonical_path))
        })
        .max_by_key(|root| root.path.components().count())
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn absolute_normalized(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    #[cfg(windows)]
    let absolute = absolute
        .ancestors()
        .find_map(|ancestor| {
            ancestor.canonicalize().ok().map(|canonical| {
                canonical.join(absolute.strip_prefix(ancestor).unwrap())
            })
        })
        .unwrap_or(absolute);
    let normalized = normalize_components(&absolute);
    #[cfg(target_os = "macos")]
    {
        if let Ok(relative) = normalized.strip_prefix("/var") {
            return Path::new("/private/var").join(relative);
        }
        if let Ok(relative) = normalized.strip_prefix("/tmp") {
            return Path::new("/private/tmp").join(relative);
        }
    }
    normalized
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            #[cfg(windows)]
            Component::Prefix(prefix) => match prefix.kind() {
                std::path::Prefix::VerbatimDisk(drive) => {
                    normalized.push(format!("{}:", drive as char));
                }
                std::path::Prefix::VerbatimUNC(server, share) => {
                    let mut root = std::ffi::OsString::from(r"\\");
                    root.push(server);
                    root.push(r"\");
                    root.push(share);
                    normalized.push(root);
                }
                _ => normalized.push(prefix.as_os_str()),
            },
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn display_group_root(path: &Path) -> PathBuf {
    let components = path.components().collect::<Vec<_>>();
    let keep = if components
        .get(1)
        .is_some_and(|component| component.as_os_str() == "Volumes")
    {
        3
    } else if components
        .get(1)
        .is_some_and(|component| component.as_os_str() == "Users")
    {
        3
    } else {
        2
    };
    components
        .into_iter()
        .take(keep)
        .fold(PathBuf::new(), |mut path, component| {
            path.push(component.as_os_str());
            path
        })
}

pub fn open_and_audit(db_path: &Path, managed_root: &Path) -> Result<SourceAuditReport> {
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    audit_sources(&db, managed_root)
}

pub fn cleanup_hidden_records(
    db_path: &Path,
    managed_root: &Path,
    expected_revision: u64,
) -> Result<SourceCleanupReport> {
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    let audit = audit_sources(&db, managed_root)?;
    anyhow::ensure!(
        audit.revision == expected_revision,
        "the indexed-leftovers set changed; review it again before removing records"
    );
    anyhow::ensure!(
        !audit.hidden_tracks.is_empty(),
        "there are no indexed leftovers to remove"
    );
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_path = db_path.with_extension(format!("sqlite3.pre-cleanup-{timestamp}.bak"));
    db.backup_to(&backup_path)?;
    let ids = audit
        .hidden_tracks
        .iter()
        .map(|track| track.id)
        .collect::<Vec<_>>();
    let removed_records = db.delete_track_records(&ids)?;
    Ok(SourceCleanupReport {
        removed_records,
        removed_playlist_references: audit.playlist_references,
        backup_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn canonical_windows_prefixes_match_missing_descendants() {
        for (canonical, ordinary) in [
            (r"\\?\C:\Music", r"C:\Music"),
            (r"\\?\UNC\server\share\Music", r"\\server\share\Music"),
        ] {
            let root = absolute_normalized(Path::new(canonical));
            let track = absolute_normalized(&Path::new(ordinary).join("missing.mp3"));
            assert!(track.starts_with(&root));
        }
    }

    #[test]
    fn hides_outside_tracks_and_repairs_inside_tracks() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("Music");
        let outside = temp.path().join("Applications");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let db_path = temp.path().join("library.sqlite3");
        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        let managed_source = db
            .add_library_source("managed", managed.to_str().unwrap(), "Library")
            .unwrap();
        let inside_id = db
            .insert_track(
                managed.join("inside.mp3").to_str().unwrap(),
                "Inside",
                "Artist",
                "Album",
                120,
                1,
                1,
            )
            .unwrap();
        let outside_id = db
            .insert_track(
                outside.join("plugin.wav").to_str().unwrap(),
                "Plugin",
                "Vendor",
                "Resources",
                2,
                1,
                1,
            )
            .unwrap();

        let report = audit_sources(&db, &managed).unwrap();
        assert_eq!(report.trusted_tracks, 1);
        assert_eq!(report.repaired_tracks, 1);
        assert_eq!(report.hidden_tracks.len(), 1);
        assert_eq!(report.hidden_tracks[0].id, outside_id);
        assert_eq!(
            db.track_source_records()
                .unwrap()
                .into_iter()
                .find(|track| track.id == inside_id)
                .unwrap()
                .source_id,
            Some(managed_source.id)
        );
    }

    #[test]
    fn path_prefix_collision_is_not_trusted() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("Music");
        let collision = temp.path().join("Music Backup");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::create_dir_all(&collision).unwrap();
        let db = Db::open(temp.path().join("db.sqlite3").to_str().unwrap()).unwrap();
        db.add_library_source("managed", managed.to_str().unwrap(), "Library")
            .unwrap();
        db.insert_track(
            collision.join("wrong.mp3").to_str().unwrap(),
            "Wrong",
            "Artist",
            "Album",
            120,
            1,
            1,
        )
        .unwrap();

        let report = audit_sources(&db, &managed).unwrap();
        assert_eq!(report.trusted_tracks, 0);
        assert_eq!(report.hidden_tracks.len(), 1);
    }

    #[test]
    fn cleanup_backs_up_database_without_deleting_media() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("Music");
        let outside = temp.path().join("Outside");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let media_path = outside.join("keep-me.mp3");
        std::fs::write(&media_path, b"audio").unwrap();
        let db_path = temp.path().join("library.sqlite3");
        let db = Db::open(db_path.to_str().unwrap()).unwrap();
        db.add_library_source("managed", managed.to_str().unwrap(), "Library")
            .unwrap();
        db.insert_track(
            media_path.to_str().unwrap(),
            "Outside",
            "Artist",
            "Album",
            120,
            5,
            1,
        )
        .unwrap();
        let audit = audit_sources(&db, &managed).unwrap();
        drop(db);

        let cleanup = cleanup_hidden_records(&db_path, &managed, audit.revision).unwrap();

        assert_eq!(cleanup.removed_records, 1);
        assert!(cleanup.backup_path.exists());
        assert!(media_path.exists());
        assert!(Db::open(db_path.to_str().unwrap())
            .unwrap()
            .track_source_records()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn disabled_reference_sources_are_not_classified_as_leftovers() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("Managed");
        let reference = temp.path().join("Archive");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::create_dir_all(&reference).unwrap();
        let db = Db::open(temp.path().join("library.sqlite3").to_str().unwrap()).unwrap();
        db.add_library_source("managed", managed.to_str().unwrap(), "Managed")
            .unwrap();
        let source = db
            .add_library_source("reference", reference.to_str().unwrap(), "Archive")
            .unwrap();
        let track = db
            .insert_track(
                reference.join("track.ogg").to_str().unwrap(),
                "Track",
                "Artist",
                "Album",
                120,
                1,
                1,
            )
            .unwrap();
        db.assign_track_source(track, source.id).unwrap();
        db.update_library_source(source.id, None, Some(false))
            .unwrap();

        let report = audit_sources(&db, &managed).unwrap();
        assert!(report.hidden_tracks.is_empty());
        assert!(report.trusted_source_ids.contains(&source.id));
    }
}
