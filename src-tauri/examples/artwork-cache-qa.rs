//! Disposable-source measurements of the production display cache.
use anyhow::{Context,Result};
use sha2::{Digest,Sha256};
fn main()->Result<()> {
 let source=std::env::args_os().nth(1).context("Pass a photo fixture")?;
 let directory=tempfile::tempdir()?;
 let path=directory.path().join("original.png");std::fs::copy(&source,&path)?;
 let original_hash=format!("{:x}",Sha256::digest(std::fs::read(&path)?));
 let mut samples=Vec::new();
 for size in [640,1200,128,1200,640,128] {
  let start=std::time::Instant::now();let cached=musicos_tauri::artwork_renditions::rendition(&path,size)?;
  let milliseconds=start.elapsed().as_secs_f64()*1000.;let (width,height)=image::image_dimensions(&cached)?;
  samples.push(serde_json::json!({"requested":size,"width":width,"height":height,"bytes":cached.metadata()?.len(),"format":cached.extension().unwrap().to_string_lossy(),"ms":milliseconds,"path":cached}));
 }
 let start=std::time::Instant::now();let palette=musicos_tauri::providers::palette(&path)?;let palette_ms=start.elapsed().as_secs_f64()*1000.;
 let unchanged=original_hash==format!("{:x}",Sha256::digest(std::fs::read(&path)?));
 println!("{}",serde_json::json!({"source_bytes":path.metadata()?.len(),"source_dimensions":image::image_dimensions(&path)?,"original_unchanged":unchanged,"samples":samples,"palette":palette,"palette_ms":palette_ms}));
 Ok(())
}
