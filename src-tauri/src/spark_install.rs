//! Optional, atomic installation of the single reviewed beta artifact. The
//! directory is selected by the user; model contents never choose runtime code.
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::{Read, Write}, path::PathBuf, sync::{atomic::{AtomicBool, Ordering}, Mutex, OnceLock}};
#[derive(Clone, Default, Serialize)]
pub struct Status {pub running:bool,pub copied:u64,pub total:u64,pub error:Option<String>}
static STATUS:OnceLock<Mutex<Status>>=OnceLock::new();
static CANCEL:AtomicBool=AtomicBool::new(false);
pub fn status()->Status {STATUS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner()).clone()}
fn update(f:impl FnOnce(&mut Status)){f(&mut STATUS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner()));}
pub fn cancel(){CANCEL.store(true,Ordering::Release);}
pub fn start(source:PathBuf,destination:PathBuf)->Result<()> {
 let candidate:crate::spark_engine::Manifest=serde_json::from_str(include_str!("../resources/spark-candidate.json"))?;
 {let mut state=STATUS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());ensure!(!state.running,"Model installation is already running");*state=Status{running:true,total:candidate.files.iter().map(|f|f.bytes).sum(),..Default::default()};}
 CANCEL.store(false,Ordering::Release);
 std::thread::spawn(move||{
  let staging=destination.with_extension("installing");
  let result=(||->Result<()>{
   ensure!(source!=destination&&source!=staging,"Choose the staged model folder");
   ensure!(!destination.exists(),"Remove the installed model before replacing it");
   if staging.exists(){fs::remove_dir_all(&staging)?;}fs::create_dir_all(&staging)?;
   for artifact in &candidate.files {
    let mut input=fs::File::open(source.join(&artifact.name)).with_context(||format!("Missing {} in selected folder",artifact.name))?;
    ensure!(input.metadata()?.len()==artifact.bytes,"Model size check failed");
    let mut output=fs::File::create(staging.join(&artifact.name))?;let mut hash=Sha256::new();let mut buffer=[0_u8;65536];
    loop {ensure!(!CANCEL.load(Ordering::Acquire),"Model installation cancelled");let n=input.read(&mut buffer)?;if n==0{break;}output.write_all(&buffer[..n])?;hash.update(&buffer[..n]);update(|s|s.copied+=n as u64);}
    output.sync_all()?;ensure!(format!("{:x}",hash.finalize())==artifact.sha256,"Model checksum failed: {}",artifact.name);
   }
   ensure!(!CANCEL.load(Ordering::Acquire),"Model installation cancelled");
   fs::write(staging.join("manifest.json"),serde_json::to_vec_pretty(&candidate)?)?;
   fs::File::open(staging.join("manifest.json"))?.sync_all()?;
   fs::rename(&staging,&destination)?;Ok(())
  })();
  if result.is_err(){let _=fs::remove_dir_all(&staging);}
  update(|s|{s.running=false;s.error=result.err().map(|e|format!("{e:#}"));});
 });Ok(())
}
pub fn remove(destination:&std::path::Path)->Result<()>{ensure!(!status().running,"Cancel installation before removing the model");if destination.exists(){fs::remove_dir_all(destination)?;}update(|s|*s=Status::default());Ok(())}
#[cfg(test)]mod tests{
 use super::*;
 fn wait(){let start=std::time::Instant::now();while status().running{assert!(start.elapsed()<std::time::Duration::from_secs(60));std::thread::sleep(std::time::Duration::from_millis(10));}}
 #[test]fn installation_failure_cancellation_and_retry_do_not_leave_partial_models(){
  let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source");let destination=temp.path().join("model");fs::create_dir(&source).unwrap();
  start(source.clone(),destination.clone()).unwrap();wait();assert!(status().error.is_some());assert!(!destination.exists());assert!(!destination.with_extension("installing").exists());
  let pinned:crate::spark_engine::Manifest=serde_json::from_str(include_str!("../resources/spark-candidate.json")).unwrap();
  fs::File::create(source.join("audio.onnx")).unwrap().set_len(pinned.files[0].bytes).unwrap();
  start(source.clone(),destination.clone()).unwrap();cancel();wait();assert!(status().error.unwrap().contains("cancelled"));assert!(!destination.exists());
  start(source,destination.clone()).unwrap();wait();assert!(status().error.unwrap().contains("checksum"));assert!(!destination.exists());remove(&destination).unwrap();assert!(status().error.is_none());
 }
}
