use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub fn reveal_in_file_manager(path: &Path) -> Result<()> {
    let target = if path.exists() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg("-R").arg(target);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", target.display()));
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(target.parent().unwrap_or(target));
        command
    };

    let status = command
        .status()
        .context("opening the system file manager")?;
    if !status.success() {
        bail!("the system file manager exited with {status}");
    }
    Ok(())
}

pub fn application_path() -> Result<std::path::PathBuf> {
    std::env::current_exe().context("locating the MusicOS application")
}
