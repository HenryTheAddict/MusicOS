use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::database::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbStatus {
    pub roots_checked: Vec<String>,
    pub media_files_indexed: usize,
    pub staged_update: Option<String>,
    pub staged_update_kind: Option<String>,
    pub installed_update: Option<String>,
    pub message: String,
}

impl Default for UsbStatus {
    fn default() -> Self {
        Self {
            roots_checked: Vec::new(),
            media_files_indexed: 0,
            staged_update: None,
            staged_update_kind: None,
            installed_update: None,
            message: "USB sync has not run yet.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateKind {
    Elf,
    Squashfs,
}

impl UpdateKind {
    fn label(self) -> &'static str {
        match self {
            Self::Elf => "binary",
            Self::Squashfs => "system image",
        }
    }

    fn staged_name(self) -> &'static str {
        match self {
            Self::Elf => "musicos-init.next",
            Self::Squashfs => "musicos-knulli.next",
        }
    }

    fn backup_name(self) -> &'static str {
        match self {
            Self::Elf => "musicos-init.previous",
            Self::Squashfs => "musicos-knulli.previous",
        }
    }
}

#[derive(Debug, Clone)]
struct StagedUpdate {
    path: PathBuf,
    kind: UpdateKind,
}

pub fn sync_from_usb(db: &Db) -> Result<UsbStatus> {
    let roots = discover_usb_roots();
    let mut media_files_indexed = 0;
    let mut staged_update: Option<StagedUpdate> = None;

    for root in &roots {
        media_files_indexed += crate::media::index_location(db, root)?;
        if staged_update.is_none() {
            staged_update = stage_update_from_root(root)?;
        }
    }

    let message = if roots.is_empty() {
        "No mounted USB storage found.".to_string()
    } else if let Some(update) = staged_update.as_ref() {
        format!(
            "USB media indexed and {} update staged.",
            update.kind.label()
        )
    } else if media_files_indexed > 0 {
        "USB media indexed.".to_string()
    } else {
        "USB storage found, but no media/update bundle was detected.".to_string()
    };

    Ok(UsbStatus {
        roots_checked: roots
            .iter()
            .map(|root| root.display().to_string())
            .collect(),
        media_files_indexed,
        staged_update: staged_update
            .as_ref()
            .map(|update| update.path.display().to_string()),
        staged_update_kind: staged_update.map(|update| update.kind.label().to_string()),
        installed_update: None,
        message,
    })
}

pub fn get_update_from_usb() -> Result<UsbStatus> {
    let roots = discover_usb_roots();
    let mut staged_update: Option<StagedUpdate> = None;
    let mut installed_update = None;

    for root in &roots {
        if staged_update.is_none() {
            staged_update = stage_update_from_root(root)?;
        }
    }

    if let Some(update) = staged_update.as_ref() {
        installed_update = Some(install_staged_update(&update.path)?);
    }

    let message = if roots.is_empty() {
        "No mounted USB storage found.".to_string()
    } else if let Some(path) = installed_update.as_ref() {
        format!(
            "Update installed to {}. Restart to boot it.",
            path.display()
        )
    } else if staged_update.is_some() {
        "Update staged. Install target was not available.".to_string()
    } else {
        "USB storage found, but no MusicOS update was detected.".to_string()
    };

    Ok(UsbStatus {
        roots_checked: roots
            .iter()
            .map(|root| root.display().to_string())
            .collect(),
        media_files_indexed: 0,
        staged_update: staged_update
            .as_ref()
            .map(|update| update.path.display().to_string()),
        staged_update_kind: staged_update.map(|update| update.kind.label().to_string()),
        installed_update: installed_update.map(|path| path.display().to_string()),
        message,
    })
}

pub fn install_pending_update() -> Result<Option<PathBuf>> {
    let stage_dir = writable_stage_dir()?;
    for kind in [UpdateKind::Squashfs, UpdateKind::Elf] {
        let staged = stage_dir.join(kind.staged_name());
        if staged.is_file() {
            return install_staged_update(&staged).map(Some);
        }
    }
    Ok(None)
}

fn discover_usb_roots() -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    mount_removable_candidates(&mut roots);
    if let Ok(value) = std::env::var("MUSICOS_USB_ROOTS") {
        for item in value.split(':').filter(|item| !item.trim().is_empty()) {
            let path = PathBuf::from(item);
            if path.exists() {
                roots.insert(path.clone());
                collect_children(&path, &mut roots, 2);
            }
        }
        return roots.into_iter().collect();
    }

    for candidate in ["/mnt/usb", "/media", "/run/media", "/Volumes"] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            roots.insert(path.clone());
            collect_children(&path, &mut roots, 2);
        }
    }

    roots.into_iter().collect()
}

#[cfg(target_os = "linux")]
fn mount_removable_candidates(out: &mut BTreeSet<PathBuf>) {
    use nix::mount::{mount, MsFlags};

    let Ok(entries) = std::fs::read_dir("/dev") else {
        return;
    };

    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !is_usb_block_partition(&name) {
            continue;
        }

        let source = entry.path();
        let mountpoint = PathBuf::from("/mnt/usb").join(&name);
        if std::fs::create_dir_all(&mountpoint).is_err() {
            continue;
        }

        for fs_type in ["vfat", "exfat", "ext4", "ntfs3"] {
            if mount(
                Some(source.as_path()),
                mountpoint.as_path(),
                Some(fs_type),
                MsFlags::MS_RDONLY | MsFlags::MS_NOATIME,
                None::<&str>,
            )
            .is_ok()
            {
                out.insert(mountpoint.clone());
                break;
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn mount_removable_candidates(_out: &mut BTreeSet<PathBuf>) {}

#[cfg(target_os = "linux")]
fn is_usb_block_partition(name: &str) -> bool {
    let bytes = name.as_bytes();
    (bytes.len() == 3 && bytes[0] == b's' && bytes[1] == b'd' && bytes[2].is_ascii_lowercase())
        || (bytes.len() == 4
            && bytes[0] == b's'
            && bytes[1] == b'd'
            && bytes[2].is_ascii_lowercase()
            && bytes[3].is_ascii_digit())
}

fn collect_children(path: &Path, out: &mut BTreeSet<PathBuf>, depth: usize) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            out.insert(child.clone());
            collect_children(&child, out, depth - 1);
        }
    }
}

fn stage_update_from_root(root: &Path) -> Result<Option<StagedUpdate>> {
    for relative in [
        "musicos-update/musicos-knulli-root.squashfs",
        "MusicOS/update/musicos-knulli-root.squashfs",
        "musicos-update/musicos-knulli-ui.squashfs",
        "MusicOS/update/musicos-knulli-ui.squashfs",
        "musicos-update/knulli",
        "MusicOS/update/knulli",
        "knulli",
        "musicos-update/musicos-init",
        "MusicOS/update/musicos-init",
        "musicos-init",
    ] {
        let source = root.join(relative);
        if source.is_file() {
            return stage_update(&source).map(Some);
        }
    }
    Ok(None)
}

fn stage_update(source: &Path) -> Result<StagedUpdate> {
    let kind = detect_update_kind(source)?;
    let stage_dir = writable_stage_dir()?;
    let dest = stage_dir.join(kind.staged_name());
    let temp = dest.with_extension("tmp");
    copy_update_file(source, &temp, kind)
        .with_context(|| format!("copying update from {}", source.display()))?;
    std::fs::rename(&temp, &dest)
        .with_context(|| format!("staging update to {}", dest.display()))?;
    sync_parent(&dest);
    std::fs::write(
        stage_dir.join("README.txt"),
        format!(
            "MusicOS {} update staged from USB. The updater will install it to the configured target.\n",
            kind.label()
        ),
    )?;
    Ok(StagedUpdate { path: dest, kind })
}

fn install_staged_update(staged: &Path) -> Result<PathBuf> {
    let kind = detect_update_kind(staged)?;
    let target = update_target(kind);
    let parent = target
        .parent()
        .with_context(|| format!("update target has no parent: {}", target.display()))?;
    std::fs::create_dir_all(parent)?;

    let stage_dir = writable_stage_dir()?;
    let backup = stage_dir.join(kind.backup_name());
    if target.is_file() {
        let _ = std::fs::copy(&target, &backup);
    }

    let temp = parent.join(format!(
        ".{}.installing",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("musicos-update")
    ));
    copy_update_file(staged, &temp, kind)?;
    std::fs::rename(&temp, &target)
        .with_context(|| format!("installing update to {}", target.display()))?;
    sync_parent(&target);

    let _ = std::fs::remove_file(staged);
    std::fs::write(
        stage_dir.join("installed.txt"),
        format!(
            "MusicOS {} update installed to {}. Restart the device to run it.\n",
            kind.label(),
            target.display()
        ),
    )?;
    Ok(target)
}

fn detect_update_kind(path: &Path) -> Result<UpdateKind> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic)
        .with_context(|| format!("reading {}", path.display()))?;
    match magic {
        [0x7f, b'E', b'L', b'F'] => Ok(UpdateKind::Elf),
        [b'h', b's', b'q', b's'] => Ok(UpdateKind::Squashfs),
        _ => anyhow::bail!(
            "{} is not a supported MusicOS update (expected ELF or SquashFS)",
            path.display()
        ),
    }
}

fn copy_update_file(source: &Path, dest: &Path, _kind: UpdateKind) -> Result<()> {
    let mut input = File::open(source)?;
    let mut output = File::create(dest)?;
    std::io::copy(&mut input, &mut output)?;
    output.flush()?;
    output.sync_all()?;
    #[cfg(unix)]
    {
        if _kind == UpdateKind::Elf {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(dest, perms)?;
        }
    }
    Ok(())
}

fn update_target(kind: UpdateKind) -> PathBuf {
    match kind {
        UpdateKind::Elf => std::env::var("MUSICOS_UPDATE_TARGET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let musicos_init = PathBuf::from("/sbin/musicos-init");
                if musicos_init.exists() {
                    musicos_init
                } else {
                    PathBuf::from("/sbin/init")
                }
            }),
        UpdateKind::Squashfs => std::env::var("MUSICOS_UPDATE_IMAGE_TARGET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/boot/knulli")),
    }
}

fn writable_stage_dir() -> Result<PathBuf> {
    let candidates = [
        std::env::var("MUSICOS_UPDATE_STAGING")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/musicos/update")),
        PathBuf::from("./out/staged-update"),
    ];

    for candidate in candidates {
        if std::fs::create_dir_all(&candidate).is_ok() {
            return Ok(candidate);
        }
    }

    anyhow::bail!("no writable update staging directory");
}

fn sync_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
}
