//! Private, invalidated preparation cache. Never contains pairing credentials.
use super::*;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyFormat { #[default] Original, Vorbis256, Vorbis320 }
impl CopyFormat {
    pub fn bitrate(self) -> Option<&'static str> { match self { Self::Original=>None, Self::Vorbis256=>Some("256k"), Self::Vorbis320=>Some("320k") } }
}
#[derive(Clone)]
struct Encoder { path: PathBuf, codec: &'static str, oggenc:Option<PathBuf> }
fn encoder() -> Option<Encoder> {
    static ENCODER: OnceLock<Option<Encoder>> = OnceLock::new();
    ENCODER.get_or_init(|| {
        for path in [PathBuf::from("/opt/homebrew/bin/ffmpeg"),PathBuf::from("/usr/local/bin/ffmpeg"),PathBuf::from("ffmpeg")] {
            let Ok(result)=Command::new(&path).args(["-hide_banner","-encoders"]).stdin(Stdio::null()).stderr(Stdio::null()).output() else { continue };
            if !result.status.success() { continue; }
            let list=String::from_utf8_lossy(&result.stdout);
            if list.lines().any(|line|line.split_whitespace().nth(1)==Some("libvorbis")) {return Some(Encoder{path,codec:"libvorbis",oggenc:None});}
            for oggenc in [PathBuf::from("/opt/homebrew/bin/oggenc"),PathBuf::from("/usr/local/bin/oggenc"),PathBuf::from("oggenc")] {
                if Command::new(&oggenc).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s|s.success()) {return Some(Encoder{path,codec:"oggenc",oggenc:Some(oggenc)});}
            }
        }
        None
    }).clone()
}
pub fn vorbis_available() -> bool { encoder().is_some() }
fn fingerprint(path: &FsPath) -> Result<String> {
    let m=fs::metadata(path)?;ensure!(m.is_file(),"Only regular files can be copied");
    let mut key=format!("{}:{}:{:?}",fs::canonicalize(path)?.display(),m.len(),m.modified()?);
    #[cfg(unix)] { use std::os::unix::fs::MetadataExt; key.push_str(&format!(":{}:{}:{}:{}",m.dev(),m.ino(),m.ctime(),m.ctime_nsec())); }
    Ok(digest(key.as_bytes()))
}
#[derive(Serialize,Deserialize)]
struct Entry { stamp:String, file:File }
#[derive(Default,Serialize,Deserialize)]
pub struct Cache { entries:BTreeMap<String,Entry>, #[serde(skip)] folder:PathBuf }
impl Cache {
    pub fn at(folder:PathBuf)->Self { let mut cache:Self=fs::read(folder.join("preparation-v2.json")).ok().and_then(|v|serde_json::from_slice(&v).ok()).unwrap_or_default();cache.folder=folder;cache }
    pub fn persist(&self)->Result<()> { save(&self.folder.join("preparation-v2.json"),self) }
    fn cached(&self,key:&str)->Option<File> { self.entries.get(key).filter(|e|fingerprint(&e.file.path).ok().as_ref()==Some(&e.stamp)).map(|e|e.file.clone()) }
    fn register(&mut self,key:String,path:&FsPath,mime:&str,current:&dyn Fn()->bool)->Result<File> {
        if let Some(file)=self.cached(&key) { return Ok(file); }
        ensure!(current(),"Sharing preparation was cancelled");
        let before=fingerprint(path)?;
        let path=fs::canonicalize(path)?;let m=fs::metadata(&path)?;let mut input=fs::File::open(&path)?;let mut hash=Sha256::new();let mut bytes=[0;262144];
        loop { ensure!(current(),"Sharing preparation was cancelled");let n=input.read(&mut bytes)?;if n==0 {break;}hash.update(&bytes[..n]); }
        ensure!(before==fingerprint(&path)?,"A file changed while preparing copies. Try again.");
        let file=File{path,size:m.len(),modified:m.modified()?,sha256:hex(&hash.finalize()),mime:mime.into()};
        self.entries.insert(key,Entry{stamp:before,file:file.clone()});Ok(file)
    }
    pub fn audio(&mut self,path:&FsPath,mime:&str,format:CopyFormat,current:&dyn Fn()->bool)->Result<File> {
        let stamp=fingerprint(path)?;
        if format==CopyFormat::Original { return self.register(format!("original/{stamp}"),path,mime,current); }
        let encoder=encoder().context("Vorbis copies need FFmpeg with libvorbis, or FFmpeg plus Xiph oggenc. Choose Original or install these encoders.")?;
        let key=format!("vorbis-v1/{stamp}/{format:?}/{}",encoder.codec);
        if let Some(file)=self.cached(&key) { return Ok(file); }
        let folder=self.folder.join("audio");fs::create_dir_all(&folder)?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt;fs::set_permissions(&folder,fs::Permissions::from_mode(0o700))?; }
        let target=folder.join(format!("{}.ogg",digest(key.as_bytes())));
        // A completed cache entry is trusted only through its recorded stamp. Rebuild orphaned outputs.
        let temp=folder.join(format!("{}.partial.ogg",random()));
        struct Partial(PathBuf);impl Drop for Partial {fn drop(&mut self){let _=fs::remove_file(&self.0);}}
        let _partial=Partial(temp.clone());
        struct Process(std::process::Child);impl Drop for Process {fn drop(&mut self){let _=self.0.kill();let _=self.0.wait();}}
        let mut command=Command::new(&encoder.path);command.args(["-nostdin","-v","error","-y","-i"]).arg(path).args(["-map","0:a:0","-vn","-map_metadata","-1","-ar","48000","-ac","2","-threads","1"]).stdin(Stdio::null()).stderr(Stdio::null());
        let mut decoder=None;
        let mut child=if let Some(oggenc)=&encoder.oggenc {
            command.args(["-c:a","pcm_s16le","-f","wav","-"]).stdout(Stdio::piped());
            let mut decode=Process(command.spawn().context("Could not decode the phone copy")?);
            let pipe=decode.0.stdout.take().context("Missing audio pipe")?;
            let child=Command::new(oggenc).args(["--quiet","--bitrate",format.bitrate().unwrap().trim_end_matches('k'),"--output"]).arg(&temp).arg("-").stdin(Stdio::from(pipe)).stdout(Stdio::null()).stderr(Stdio::null()).spawn().context("Could not start the Vorbis encoder")?;
            decoder=Some(decode);Process(child)
        } else {
            command.args(["-c:a","libvorbis","-b:a",format.bitrate().unwrap()]).arg(&temp).stdout(Stdio::null());
            Process(command.spawn().context("Could not start the Vorbis encoder")?)
        };
        let started=Instant::now();
        loop { ensure!(current()&&started.elapsed()<Duration::from_secs(900),"Vorbis preparation stopped; your originals are unchanged");
            if let Some(status)=child.0.try_wait()? {ensure!(status.success(),"Could not make a Vorbis copy; your originals are unchanged. Try Original for this selection.");break;}
            std::thread::sleep(Duration::from_millis(50));
        }
        if let Some(mut decode)=decoder {ensure!(decode.0.wait()?.success(),"Could not decode this song. Try Original for this selection.");}
        ensure!(current()&&stamp==fingerprint(path)?,"The source changed or preparation was cancelled. Try again.");
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt;fs::set_permissions(&temp,fs::Permissions::from_mode(0o600))?; }
        fs::rename(&temp,&target)?;self.register(key,&target,"audio/ogg",current)
    }
    pub fn artwork(&mut self,path:&FsPath,current:&dyn Fn()->bool)->Result<File> {
        let stamp=fingerprint(path)?;let key=format!("artwork-jpeg-v2/{stamp}");
        if let Some(file)=self.cached(&key) {return Ok(file);}
        ensure!(current(),"Sharing preparation was cancelled");
        let image=image::open(path)?.thumbnail(768,768).to_rgb8();let mut bytes=Vec::new();image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,85).encode_image(&image)?;
        ensure!(stamp==fingerprint(path)?,"Artwork changed while preparing copies");
        let folder=self.folder.join("artwork");fs::create_dir_all(&folder)?;let target=folder.join(format!("{}.jpg",digest(&bytes)));
        if fs::read(&target).ok().is_none_or(|data|digest(&data)!=digest(&bytes)) { fs::write(&target,bytes)?; }
        self.register(key,&target,"image/jpeg",current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn warm_hash_cache_invalidates_changed_sources_and_cancellation() {
        let root=tempfile::tempdir().unwrap();let source=root.path().join("song.wav");fs::write(&source,b"first audio").unwrap();
        let folder=root.path().join("cache");let mut cache=Cache::at(folder.clone());let original=cache.audio(&source,"audio/wav",CopyFormat::Original,&||true).unwrap();cache.persist().unwrap();
        let mut warm=Cache::at(folder);assert_eq!(warm.audio(&source,"audio/wav",CopyFormat::Original,&||true).unwrap().sha256,original.sha256);
        fs::write(&source,b"other audio").unwrap();let changed=warm.audio(&source,"audio/wav",CopyFormat::Original,&||true).unwrap();assert_ne!(changed.sha256,original.sha256);
        fs::write(&source,b"third audio").unwrap();assert!(warm.audio(&source,"audio/wav",CopyFormat::Original,&||false).is_err());
        assert!(serde_json::from_str::<CopyFormat>("\"vorbis128\"").is_err());
    }
    #[test]
    fn vorbis_variants_are_decodable_correctly_labeled_and_reused_without_touching_originals() {
        let encoder=encoder().expect("Vorbis fixture requires the installed FFmpeg");let root=tempfile::tempdir().unwrap();let source=root.path().join("source.wav");
        assert!(Command::new(&encoder.path).args(["-v","error","-f","lavfi","-i","sine=frequency=440:duration=1","-ac","2"]).arg(&source).status().unwrap().success());
        let original=fs::read(&source).unwrap();let mut cache=Cache::at(root.path().join("cache"));let mut hashes=Vec::new();
        for (format,rate) in [(CopyFormat::Vorbis256,256000),(CopyFormat::Vorbis320,320000)] {
            let copy=cache.audio(&source,"audio/wav",format,&||true).unwrap();assert_eq!(copy.mime,"audio/ogg");let bytes=fs::read(&copy.path).unwrap();assert_eq!(&bytes[..4],b"OggS");let marker=bytes.windows(6).position(|p|p==b"vorbis").unwrap();let nominal=i32::from_le_bytes(bytes[marker+19..marker+23].try_into().unwrap());assert_eq!(nominal,rate);
            assert!(Command::new(&encoder.path).args(["-v","error","-i"]).arg(&copy.path).args(["-f","null","-"]).status().unwrap().success());
            let modified=fs::metadata(&copy.path).unwrap().modified().unwrap();let again=cache.audio(&source,"audio/wav",format,&||true).unwrap();assert_eq!(again.sha256,copy.sha256);assert_eq!(fs::metadata(again.path).unwrap().modified().unwrap(),modified);hashes.push(copy.sha256);
        }
        assert_ne!(hashes[0],hashes[1]);assert_eq!(fs::read(source).unwrap(),original);assert!(fs::read_dir(root.path().join("cache/audio")).unwrap().all(|p|!p.unwrap().path().to_string_lossy().contains("partial")));
    }
}
#[cfg(test)]
mod pacing {
    use super::*;
    #[test]
    fn reports_cold_and_reused_preparation_for_64_mib() {
        let root=tempfile::tempdir().unwrap();let bytes=vec![42u8;8*1024*1024];let files=(0..8).map(|i|{let p=root.path().join(format!("{i}.wav"));fs::write(&p,&bytes).unwrap();p}).collect::<Vec<_>>();let folder=root.path().join("cache");let mut cache=Cache::at(folder.clone());let start=Instant::now();for file in &files {cache.audio(file,"audio/wav",CopyFormat::Original,&||true).unwrap();}cache.persist().unwrap();let cold=start.elapsed();let mut warm=Cache::at(folder);let start=Instant::now();for file in &files {warm.audio(file,"audio/wav",CopyFormat::Original,&||true).unwrap();}let reused=start.elapsed();println!("PREPARATION_PACING {{\"bytes\":67108864,\"cold_ms\":{},\"reused_ms\":{}}}",cold.as_secs_f64()*1000.,reused.as_secs_f64()*1000.);
    }
}
