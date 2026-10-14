//! Run against local cached portraits; never rewrites originals or uploads images.
fn main() -> anyhow::Result<()> {
 for input in std::env::args().skip(1) {
  let source=std::path::Path::new(&input);let started=std::time::Instant::now();
  let output=musicos_tauri::portrait::crop(source)?;
  println!("{}",serde_json::json!({"source":source,"output":output,"face_framed":source!=output,"milliseconds":started.elapsed().as_millis(),"dimensions":image::image_dimensions(&output)?}));
 }
 Ok(())
}
