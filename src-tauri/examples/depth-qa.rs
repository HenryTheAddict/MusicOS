//! Isolated optional-model setup and native inference; never enables app preferences.
fn main()->anyhow::Result<()>{
 let root=std::path::PathBuf::from(std::env::args().nth(1).ok_or_else(||anyhow::anyhow!("Pass isolated model directory"))?);
 let source=std::path::PathBuf::from(std::env::args().nth(2).ok_or_else(||anyhow::anyhow!("Pass artwork path"))?);
 if musicos_tauri::depth::verify(&root.join("DepthAnythingV2SmallF32.mlpackage")).is_err(){anyhow::ensure!(!std::env::args().any(|a|a=="--cache-only"),"Install the optional model in Appearance before running cache QA");musicos_tauri::depth::install_at(&root)?;}
 let start=std::time::Instant::now();let (output,fresh)=musicos_tauri::depth::generate_report(&root,&source)?;let first_ms=start.elapsed().as_millis();
 let map=output.ok_or_else(||anyhow::anyhow!("No useful depth map returned"))?;let before=map.metadata()?.modified()?;
 let start=std::time::Instant::now();let (second,second_fresh)=musicos_tauri::depth::generate_report(&root,&source)?;
 let second_ms=start.elapsed().as_millis();anyhow::ensure!(second.as_ref()==Some(&map)&&!second_fresh&&map.metadata()?.modified()?==before,"Depth cache was not reused");
 let first=musicos_tauri::depth::textures(&source,&map)?;let second=musicos_tauri::depth::textures(&source,&map)?;anyhow::ensure!(first.map==second.map&&first.full_map==second.full_map,"Prepared textures changed across cache reuse");
 println!("{}",serde_json::json!({"source":source,"map":map,"fresh":fresh,"first_ms":first_ms,"second_ms":second_ms,"cache_reused":true,"revision":musicos_tauri::depth::REVISION}));Ok(())
}
