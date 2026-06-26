use anyhow::Result;
use log::{Level, LevelFilter, Metadata, Record};
use std::io::Write;

struct MusicLogger;

impl log::Log for MusicLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}] {}", record.level(), record.args());
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/boot/musicos-runtime.log")
            {
                let _ = writeln!(file, "[{}] {}", record.level(), record.args());
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: MusicLogger = MusicLogger;

pub fn install_logger() {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));
}

pub fn prepare_pid1_environment() -> Result<()> {
    if std::process::id() != 1 {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        use nix::mount::{mount, MsFlags};
        use nix::sys::stat::{makedev, mknod, Mode, SFlag};
        use std::path::Path;

        std::fs::create_dir_all("/proc")?;
        std::fs::create_dir_all("/sys")?;
        std::fs::create_dir_all("/dev")?;
        std::fs::create_dir_all("/run")?;
        std::fs::create_dir_all("/var/lib/musicos")?;

        let _ = mount(
            Some("proc"),
            "/proc",
            Some("proc"),
            MsFlags::empty(),
            None::<&str>,
        );
        let _ = mount(
            Some("sysfs"),
            "/sys",
            Some("sysfs"),
            MsFlags::empty(),
            None::<&str>,
        );
        let _ = mount(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None::<&str>,
        );
        let _ = mount(
            Some("tmpfs"),
            "/run",
            Some("tmpfs"),
            MsFlags::empty(),
            None::<&str>,
        );

        if !Path::new("/dev/null").exists() {
            let _ = mknod(
                "/dev/null",
                SFlag::S_IFCHR,
                Mode::from_bits_truncate(0o666),
                makedev(1, 3),
            );
        }
    }
    Ok(())
}
