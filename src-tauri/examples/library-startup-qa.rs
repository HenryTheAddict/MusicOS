//! Read-only profiling of the real library through an isolated SQLite snapshot.
use anyhow::Result;
use musicos_desktop::{local::{LocalLibraryRepository,LibraryRepository},storage};
use std::{path::Path, process::Command, time::Instant};

fn samples(db: &Path, media: &Path) -> Result<serde_json::Value> {
    let repo=LocalLibraryRepository::new(db,media);
    let mut samples=Vec::new();
    for run in 0..7 {
        let start=Instant::now();
        let snapshot=repo.snapshot(&Default::default())?;
        let snapshot_ms=start.elapsed().as_secs_f64()*1000.;
        let start=Instant::now();
        let mut library=musicos_tauri::library::group(snapshot);
        let grouping_ms=start.elapsed().as_secs_f64()*1000.;
        let start=Instant::now();
        musicos_tauri::library::canonical_artwork(db,&mut library)?;
        let artwork_ms=start.elapsed().as_secs_f64()*1000.;
        let start=Instant::now();
        let payload=serde_json::to_vec(&library)?;
        samples.push(serde_json::json!({"run":run,"tracks":library.tracks.len(),"albums":library.albums.len(),"snapshot_ms":snapshot_ms,"grouping_ms":grouping_ms,"artwork_ms":artwork_ms,"serialize_ms":start.elapsed().as_secs_f64()*1000.,"payload_bytes":payload.len()}));
    }
    Ok(serde_json::json!(samples))
}

fn main()->Result<()> {
    let config=storage::load_config()?;
    if let Some(db)=std::env::args_os().nth(1) {
        println!("{}",samples(Path::new(&db),&config.media_dir)?);
        return Ok(());
    }
    let temp=tempfile::tempdir()?;
    let db=temp.path().join("profile.db");
    let source=rusqlite::Connection::open(&config.db_path)?;
    source.execute("VACUUM INTO ?",[db.to_string_lossy().as_ref()])?;
    drop(source);
    // Populate the cache first, regardless of the user's cache state.
    rusqlite::Connection::open(&db)?.execute_batch("DROP TABLE IF EXISTS desktop_artwork_validation_v1")?;
    let mut processes=Vec::new();
    for phase in ["first_process","relaunch"] {
        let output=Command::new(std::env::current_exe()?).arg(&db).output()?;
        anyhow::ensure!(output.status.success(),"Profiler failed: {}",String::from_utf8_lossy(&output.stderr));
        processes.push(serde_json::json!({"phase":phase,"samples":serde_json::from_slice::<serde_json::Value>(&output.stdout)?}));
    }
    println!("{}",serde_json::to_string_pretty(&processes)?);
    Ok(())
}
