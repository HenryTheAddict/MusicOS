//! Optional, checksum-pinned local artwork depth. Audio/model consent is independent.
use anyhow::{Result,ensure,Context};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::{path::{Path,PathBuf},sync::{Mutex,OnceLock,atomic::{AtomicBool,Ordering}},io::{Read,Write}};
pub const REVISION:&str="cfef6f6f2a70783dedc0bfae40cecbc2052285d3";
const PACKAGE:&str="DepthAnythingV2SmallF32.mlpackage";
pub const FILES:[(&str,u64,&str);3]=[
 ("Data/com.apple.CoreML/model.mlmodel",366829,"dfd7fd461ab2544f4951e757a59d878d6af93b5544541ed534329d84e958f0c9"),
 ("Data/com.apple.CoreML/weights/weight.bin",98821504,"6b02236f9b6c8c4694904f89c0fc84c3b6175cde43f9f918f77948708c36b58d"),
 ("Manifest.json",617,"048f5f5fbc11cff5ee8ab21e3a97830b7e0aacab5404eff0b3ea198dd1a39336")];
#[derive(Clone,Default,Serialize)]
pub struct Status {pub supported:bool,pub ready:bool,pub running:bool,pub downloaded:u64,pub total:u64,pub error:Option<String>,pub revision:&'static str,pub attribution:&'static str}
static STATE:OnceLock<Mutex<Status>>=OnceLock::new();static CANCEL:AtomicBool=AtomicBool::new(false);
static WORKER:Mutex<()>=Mutex::new(());
fn state()->&'static Mutex<Status>{STATE.get_or_init(||Mutex::new(Status{supported:cfg!(target_os="macos"),total:FILES.iter().map(|f|f.1).sum(),revision:REVISION,attribution:"Depth Anything V2 Small F32 · Apple Core ML conversion · Apache 2.0",..Default::default()}))}
pub fn root()->PathBuf{musicos_desktop::storage::desktop_data_dir().join("depth-artwork-v1")}
pub fn verify(directory:&Path)->Result<()> {for (name,size,expected) in FILES {let mut file=std::fs::File::open(directory.join(name))?;ensure!(file.metadata()?.len()==size,"Depth model size check failed");let mut hash=Sha256::new();let mut buffer=[0;65536];loop{let n=file.read(&mut buffer)?;if n==0{break;}hash.update(&buffer[..n]);}ensure!(format!("{:x}",hash.finalize())==expected,"Depth model checksum failed. Retry installation.");}Ok(())}
fn verified(directory:&Path)->Result<()> {
 static VERIFIED:OnceLock<Mutex<Option<(PathBuf,Vec<(u64,std::time::SystemTime)>)>>>=OnceLock::new();
 let identity=FILES.iter().map(|(n,_,_)|{let m=directory.join(n).metadata()?;Ok((m.len(),m.modified()?))}).collect::<Result<Vec<_>>>()?;
 let cache=VERIFIED.get_or_init(Default::default);let mut value=cache.lock().unwrap_or_else(|e|e.into_inner());
 if value.as_ref()!=Some(&(directory.to_owned(),identity.clone())){verify(directory)?;*value=Some((directory.to_owned(),identity));}Ok(())
}
pub fn status()->Status{let mut result=state().lock().unwrap_or_else(|e|e.into_inner()).clone();result.ready=verified(&root().join(PACKAGE)).is_ok();result}
pub fn action(action:&str)->Result<Status>{match action {
 "status"=>{},
 "install"=>{ensure!(cfg!(target_os="macos"),"Depth artwork currently requires macOS");let mut value=state().lock().unwrap_or_else(|e|e.into_inner());ensure!(!value.running,"Depth installation is already running");CANCEL.store(false,Ordering::Release);value.running=true;value.downloaded=0;value.error=None;drop(value);
 std::thread::Builder::new().name("musicos-depth-install".into()).spawn(||{let result=install_at(&root());let mut value=state().lock().unwrap_or_else(|e|e.into_inner());value.running=false;value.error=result.err().map(|e|e.to_string());})?;},
 "cancel"=>{CANCEL.store(true,Ordering::Release);},
 "remove"=>{ensure!(!state().lock().unwrap_or_else(|e|e.into_inner()).running,"Cancel installation before removing the model");let _lock=WORKER.lock().unwrap_or_else(|e|e.into_inner());if root().exists(){std::fs::remove_dir_all(root())?;}state().lock().unwrap_or_else(|e|e.into_inner()).error=None;},
 _=>anyhow::bail!("Unknown depth setup action")};Ok(status())}
pub fn install_at(root:&Path)->Result<()> {
 std::fs::create_dir_all(root)?;let staged=root.join("installing.mlpackage");if staged.exists(){std::fs::remove_dir_all(&staged)?;}
 let result=(||->Result<()>{let client=reqwest::blocking::Client::builder().connect_timeout(std::time::Duration::from_secs(15)).timeout(std::time::Duration::from_secs(300)).build()?;
 for (name,size,expected) in FILES {ensure!(!CANCEL.load(Ordering::Acquire),"Installation canceled");let destination=staged.join(name);std::fs::create_dir_all(destination.parent().context("Invalid model path")?)?;
 let url=format!("https://huggingface.co/apple/coreml-depth-anything-v2-small/resolve/{REVISION}/{PACKAGE}/{name}");let mut response=client.get(url).send()?.error_for_status()?;let mut file=std::fs::File::create(destination)?;let mut hash=Sha256::new();let mut count=0;let mut buffer=[0;65536];
 loop{ensure!(!CANCEL.load(Ordering::Acquire),"Installation canceled");let n=response.read(&mut buffer)?;if n==0{break;}count+=n as u64;ensure!(count<=size,"Depth model download exceeded pinned size");file.write_all(&buffer[..n])?;hash.update(&buffer[..n]);state().lock().unwrap_or_else(|e|e.into_inner()).downloaded+=n as u64;}
 ensure!(count==size&&format!("{:x}",hash.finalize())==expected,"Depth model checksum failed. Retry installation.");file.sync_all()?;
 }verify(&staged)?;ensure!(!CANCEL.load(Ordering::Acquire),"Installation canceled");let _lock=WORKER.lock().unwrap_or_else(|e|e.into_inner());let destination=root.join(PACKAGE);if destination.exists(){std::fs::remove_dir_all(&destination)?;}std::fs::rename(&staged,destination)?;Ok(())})();
 if result.is_err(){let _=std::fs::remove_dir_all(staged);}result
}
pub fn source_identity(source:&Path)->Result<String>{
 let metadata=source.metadata()?;ensure!(metadata.len()<=128*1024*1024,"Artwork exceeds the processing limit");
 let mut file=std::fs::File::open(source)?;let mut hash=Sha256::new();let mut buffer=[0;65536];loop{let n=file.read(&mut buffer)?;if n==0{break;}hash.update(&buffer[..n]);}
 let after=source.metadata()?;ensure!(metadata.len()==after.len()&&metadata.modified()?==after.modified()?,"Artwork changed during processing");Ok(format!("{:x}",hash.finalize()))
}
fn valid_map(path:&Path)->bool{image::image_dimensions(path).is_ok_and(|(w,h)|w>0&&h>0&&w<=256&&h<=256)&&image::open(path).is_ok()}
pub fn generate(source:&Path)->Result<Option<PathBuf>>{generate_at(&root(),source)}
pub fn reported(source:&Path)->Result<(Option<PathBuf>,bool)>{generate_report(&root(),source)}
pub fn generate_at(root:&Path,source:&Path)->Result<Option<PathBuf>>{Ok(generate_report(root,source)?.0)}
pub fn generate_report(root:&Path,source:&Path)->Result<(Option<PathBuf>,bool)>{
 let _lock=WORKER.lock().unwrap_or_else(|e|e.into_inner());let package=root.join(PACKAGE);verified(&package)?;
 let key=format!("depth-v3-f32-crop-v3:{REVISION}:{}",source_identity(source)?);
 let id=format!("{:x}",Sha256::digest(key.as_bytes()));let cache=root.join("maps");std::fs::create_dir_all(&cache)?;let output=cache.join(format!("{id}.png"));let negative=cache.join(format!("{id}.none"));
 if output.exists(){if valid_map(&output){return Ok((Some(output),false));}let _=std::fs::remove_file(&output);let _=std::fs::remove_file(output.with_extension("depth-texture.png"));}if negative.exists(){return Ok((None,false));}
 let temp=cache.join(format!("{id}.tmp.png"));
 #[cfg(target_os="macos")]
 musicos_desktop::local::decoded_audio::apple::depth(&package,source,&temp)?;
 #[cfg(not(target_os="macos"))]
 anyhow::bail!("Depth artwork currently requires macOS");
 let map=image::open(&temp)?.thumbnail(256,256).to_luma8();let _=std::fs::remove_file(&temp);
 let count=map.len().max(1) as f64;let mean=map.iter().map(|v|*v as f64).sum::<f64>()/count;let variance=map.iter().map(|v|(*v as f64-mean).powi(2)).sum::<f64>()/count;
 if variance<64. {std::fs::write(negative,b"flat")?;return Ok((None,false));}
 map.save(&temp)?;std::fs::rename(temp,&output)?;Ok((Some(output),true))
}
#[cfg(test)]mod tests{use super::*;#[test]fn pinned_artifacts_reject_incomplete_or_wrong_files(){let dir=tempfile::tempdir().unwrap();assert!(verify(dir.path()).is_err());for(name,size,_)in FILES{let path=dir.path().join(name);std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::File::create(path).unwrap().set_len(size).unwrap();}assert!(verify(dir.path()).is_err());}#[test]fn manifest_is_pinned_and_small(){assert_eq!(FILES.iter().map(|f|f.1).sum::<u64>(),99188950);assert_eq!(REVISION.len(),40);}}

#[derive(Clone,serde::Deserialize,serde::Serialize)]
pub struct FrameMetrics{pub width:u32,pub height:u32,pub frames:u32,pub interface_percent:u16,pub raf_median_ms:f64,pub raf_p95_ms:f64,pub cpu_draw_p95_ms:f64,pub over_120hz_budget:f64}
pub fn record_metrics(sample:FrameMetrics)->Result<()>{ensure!(sample.frames<=240&&sample.width<=32768&&sample.height<=32768&&[sample.raf_median_ms,sample.raf_p95_ms,sample.cpu_draw_p95_ms,sample.over_120hz_budget].iter().all(|v|v.is_finite()&&*v>=0.),"Invalid artwork performance sample");
 static SAMPLES:OnceLock<Mutex<Vec<FrameMetrics>>>=OnceLock::new();let mut samples=SAMPLES.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());if samples.len()==32{samples.remove(0);}samples.push(sample);musicos_desktop::storage::atomic_write_json(&musicos_desktop::storage::desktop_data_dir().join("artwork-performance.json"),&*samples)?;Ok(())}

#[derive(Serialize)]pub struct Artwork{pub photo:String,pub map:String,pub full_map:String,pub identity:String,pub fresh:bool}
pub fn textures(source:&Path,map:&Path)->Result<Artwork>{
 let _lock=WORKER.lock().unwrap_or_else(|e|e.into_inner());
 use base64::Engine;
 // Preserve the original ICC/color profile. Browser decoding must match the
 // ordinary cover; a separate Rust RGB conversion caused brightness flashes.
 let depth=map.with_extension("depth-texture.png");
 if !valid_map(&depth){
  let upright=map.with_extension("upright.png");
  #[cfg(target_os="macos")]
  musicos_desktop::local::decoded_audio::apple::oriented_png(source,&upright)?;
  #[cfg(not(target_os="macos"))]
  std::fs::copy(source,&upright)?;
  let dimensions=image::image_dimensions(&upright);let _=std::fs::remove_file(upright);let(w,h)=dimensions?;let side=w.min(h);
  let image=image::open(map)?;let(dw,dh)=(image.width(),image.height());let cw=(dw as f64*side as f64/w as f64).round().max(1.)as u32;let ch=(dh as f64*side as f64/h as f64).round().max(1.)as u32;
  let temporary=depth.with_extension("tmp.png");image.crop_imm((dw-cw)/2,(dh-ch)/2,cw,ch).resize_exact(256,256,image::imageops::FilterType::Triangle).save(&temporary)?;std::fs::rename(temporary,&depth)?;
 }
 let bytes=std::fs::read(source)?;ensure!(bytes.len()<16*1024*1024,"Artwork is too large for depth rendering");
 let mime=image::guess_format(&bytes)?.to_mime_type();
 let photo=format!("data:{mime};base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes));
 let encode=|path:&Path|->Result<String>{let bytes=std::fs::read(path)?;ensure!(bytes.len()<2*1024*1024,"Prepared artwork texture is too large");Ok(format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes)))};
 Ok(Artwork{photo,map:encode(&depth)?,full_map:encode(map)?,identity:map.file_stem().unwrap_or_default().to_string_lossy().into_owned(),fresh:false})
}

#[cfg(test)]
#[test]
fn texture_cache_preserves_source_bytes_and_reuses_depth_crop(){
 use base64::Engine;
 let dir=tempfile::tempdir().unwrap();let source=dir.path().join("source.png");let map=dir.path().join("map.png");
 image::RgbImage::from_pixel(40,60,image::Rgb([180,70,30])).save(&source).unwrap();
 image::GrayImage::from_pixel(32,32,image::Luma([120])).save(&map).unwrap();
 let first=textures(&source,&map).unwrap();let cached=map.with_extension("depth-texture.png");let modified=cached.metadata().unwrap().modified().unwrap();
 let second=textures(&source,&map).unwrap();assert_eq!(first.photo,second.photo);assert_eq!(first.map,second.map);assert_eq!(modified,cached.metadata().unwrap().modified().unwrap());
 let encoded=first.photo.split_once(',').unwrap().1;assert_eq!(base64::engine::general_purpose::STANDARD.decode(encoded).unwrap(),std::fs::read(source).unwrap());
}

#[cfg(test)]
#[test]
fn content_cache_identity_survives_moves_and_rejects_corruption(){
 let dir=tempfile::tempdir().unwrap();let a=dir.path().join("a.png");let b=dir.path().join("b.png");
 image::GrayImage::from_pixel(16,16,image::Luma([40])).save(&a).unwrap();std::fs::copy(&a,&b).unwrap();
 assert_eq!(source_identity(&a).unwrap(),source_identity(&b).unwrap());assert!(valid_map(&a));
 std::fs::write(&b,b"broken image").unwrap();assert_ne!(source_identity(&a).unwrap(),source_identity(&b).unwrap());assert!(!valid_map(&b));
}
