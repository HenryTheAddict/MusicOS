//! Read-only native decode/seek checks against every M4A in the supplied library.
use anyhow::{Result,ensure};use rodio::Source;use std::{path::Path,time::Duration};
fn check(path:&Path)->Result<serde_json::Value>{
 let mut source=musicos_desktop::local::decoded_audio::open(path)?;let duration=source.total_duration().ok_or_else(||anyhow::anyhow!("Missing duration"))?.as_secs_f64();
 for seconds in [0.,duration*0.5,(duration-1.).max(0.)] {source.try_seek(Duration::from_secs_f64(seconds)).map_err(|e|anyhow::anyhow!(e.to_string()))?;let mut count=0;for sample in source.by_ref().take(4096){ensure!(sample.is_finite(),"Invalid PCM");count+=1;}ensure!(count>0,"No decoded audio at {seconds}s");}
 #[cfg(not(target_os="macos"))]let quality=serde_json::json!({"verified":false});
 #[cfg(target_os="macos")]let quality=musicos_desktop::local::decoded_audio::apple::probe(path)?;
 Ok(serde_json::json!({"file":path.file_name().map(|p|p.to_string_lossy()),"duration":duration,"quality":quality,"decode_seek":"pass"}))
}
fn main()->Result<()>{let db=std::env::args().nth(1).ok_or_else(||anyhow::anyhow!("Pass library database"))?;
 let db=rusqlite::Connection::open_with_flags(db,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
 let mut query=db.prepare("SELECT path FROM tracks WHERE lower(path) LIKE '%.m4a'")?;let paths=query.query_map([],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
 let mut failed=0;for path in &paths{match check(Path::new(path)){Ok(v)=>println!("{v}"),Err(e)=>{failed+=1;println!("{}",serde_json::json!({"file":Path::new(path).file_name().map(|p|p.to_string_lossy()),"error":e.to_string()}));}}}
 // Real ALAC fixture, independent of the user's files.
 let temporary=tempfile::tempdir()?;let fixture=temporary.path().join("alac.m4a");let status=std::process::Command::new("ffmpeg").args(["-v","error","-f","lavfi","-i","sine=frequency=440:duration=3","-c:a","alac"]).arg(&fixture).status()?;ensure!(status.success(),"Cannot generate ALAC fixture");println!("{}",check(&fixture)?);
 println!("{}",serde_json::json!({"existing_files":paths.len(),"failed":failed,"alac_fixture":"pass"}));ensure!(failed==0,"{failed} existing M4As failed");Ok(())}
