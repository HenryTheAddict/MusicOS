//! Desktop-only, local DSP estimates. These are not Sparklist embeddings.
use anyhow::{ensure, Result};
use rodio::Source;
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime},
};
const VERSION: &str = "dsp-7-exact-duration-beat-octave-11025-4096-512";
const RATE: usize = 11025;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Features {
    pub track_id: i64,
    pub bpm: Option<f32>,
    pub bpm_confidence: f32,
    pub key: Option<String>,
    pub key_index: Option<usize>,
    pub key_confidence: f32,
    #[serde(default)]
    pub structure: Option<crate::structure::Structure>,
}
#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub running: bool,
    pub paused: bool,
    pub processed: usize,
    pub total: usize,
    pub features: Vec<Features>,
    pub errors: Vec<String>,
}
#[derive(Default, Clone)]
pub struct Analysis {
    pub status: Arc<Mutex<Status>>,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    priority: Arc<Mutex<Vec<i64>>>,
    requested: Arc<Mutex<Vec<(i64,String,i64)>>>,
    seed_priority: Arc<Mutex<Vec<i64>>>,
    model_setup: Arc<Mutex<Option<(PathBuf,PathBuf,bool)>>>,
    revisions:Arc<Mutex<std::collections::HashMap<i64,usize>>>,
}
fn identity(path: &Path) -> Result<String> {
    let m = std::fs::metadata(path)?;
    Ok(format!(
        "{}:{}:{}",
        path.display(),
        m.len(),
        m.modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos()
    ))
}
fn cache(db: &Path) -> Result<musicos_init::database::Connection> {
    let c = musicos_init::database::Connection::open_background(db)?;
    c.busy_timeout(Duration::from_secs(5))?;
    musicos_init::database_access::initialize_schema(db,"audio-analysis-v1",||{c.execute_batch("CREATE TABLE IF NOT EXISTS desktop_dsp_v1(track_id INTEGER PRIMARY KEY,identity TEXT NOT NULL,version TEXT NOT NULL,result TEXT NOT NULL)")?;Ok(())})?;
    Ok(c)
}
pub fn cached(db:&Path,id:i64,path:&str)->Option<Features>{cached_mode(db,id,path,false)}
/// Old valid results remain usable while the worker replaces them incrementally.
pub fn cached_current(db:&Path,id:i64,path:&str)->Option<Features>{cached_mode(db,id,path,true)}
fn cached_mode(db: &Path, id: i64, path: &str,current:bool) -> Option<Features> {
    let signature = identity(Path::new(path)).ok()?;
    cache(db)
        .ok()?
        .query_row(
            "SELECT result FROM desktop_dsp_v1 WHERE track_id=? AND identity=? AND (?=0 OR version=?)",
            rusqlite::params![id, signature, current, VERSION],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}
pub fn cached_many(db:&Path,tracks:&[(i64,&str)])->Vec<Option<Features>>{cached_many_mode(db,tracks,false)}
/// Heart motion needs tempo, not each song's full beat/structure payload.
pub fn cached_rhythms(db:&Path,tracks:&[(i64,&str)])->Result<Vec<(i64,f32,Option<f32>)>> {
    let connection=cache(db)?;
    let mut statement=connection.prepare("SELECT track_id,identity,json_extract(result,'$.bpm'),CASE WHEN json_extract(result,'$.structure.beat_confidence')>=0.5 THEN json_extract(result,'$.structure.beats[0]') END FROM desktop_dsp_v1 WHERE json_valid(result)")?;
    let rows=statement.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<f32>>(2)?,r.get::<_,Option<f32>>(3)?)))?;
    let paths=tracks.iter().copied().collect::<std::collections::HashMap<_,_>>();
    let mut tempos=Vec::new();
    for row in rows {
        let (id,signature,bpm,offset)=row?;
        if let Some(bpm)=bpm.filter(|b|b.is_finite()&&(20.0..=400.0).contains(b)) {
            if paths.get(&id).is_some_and(|path|identity(Path::new(path)).is_ok_and(|current|current==signature)) {tempos.push((id,bpm,offset.filter(|v|v.is_finite()&&*v>=0.)));}
        }
    }
    Ok(tempos)
}
#[cfg(test)]
pub fn cached_tempos(db:&Path,tracks:&[(i64,&str)])->Result<Vec<(i64,f32)>>{Ok(cached_rhythms(db,tracks)?.into_iter().map(|(id,bpm,_)|(id,bpm)).collect())}
pub fn cached_many_current(db:&Path,tracks:&[(i64,&str)])->Vec<Option<Features>>{cached_many_mode(db,tracks,true)}
fn cached_many_mode(db: &Path, tracks: &[(i64, &str)],current:bool) -> Vec<Option<Features>> {
    let lookup = (|| -> Result<Vec<Option<Features>>> {
        let conn = cache(db)?;
        let mut query = conn.prepare(
            "SELECT result FROM desktop_dsp_v1 WHERE track_id=? AND identity=? AND (?=0 OR version=?)",
        )?;
        Ok(tracks.iter().map(|(id, path)| {
            let signature = identity(Path::new(path)).ok()?;
            let row = query.query_row(rusqlite::params![id, signature, current, VERSION], |row| row.get::<_, String>(0)).ok()?;
            serde_json::from_str(&row).ok()
        }).collect())
    })();
    lookup.unwrap_or_else(|_| vec![None; tracks.len()])
}

#[derive(Clone,Copy,Debug,Default)]
pub struct Progress {pub running:bool,pub paused:bool,pub processed:usize,pub total:usize}
impl Analysis {
    pub fn progress(&self)->Progress {let s=self.status.lock().unwrap_or_else(|e|e.into_inner());Progress{running:s.running,paused:s.paused,processed:s.processed,total:s.total}}
    pub fn pair_revision(&self,ids:&[i64])->usize {let revisions=self.revisions.lock().unwrap_or_else(|e|e.into_inner());ids.iter().map(|id|revisions.get(id).copied().unwrap_or(0)).sum()}
    pub fn request_seeds(&self,tracks:Vec<(i64,String,i64)>,model:Option<(PathBuf,PathBuf,bool)>)->bool{
        let status=self.status.lock().unwrap_or_else(|e|e.into_inner());if !status.running{return false;}
        *self.seed_priority.lock().unwrap_or_else(|e|e.into_inner())=tracks.iter().map(|t|t.0).collect();
        self.requested.lock().unwrap_or_else(|e|e.into_inner()).extend(tracks);
        if model.is_some(){*self.model_setup.lock().unwrap_or_else(|e|e.into_inner())=model;}true
    }
    pub fn prioritize_seed(&self,ids:Vec<i64>){*self.seed_priority.lock().unwrap_or_else(|e|e.into_inner())=ids;}
    pub fn prioritize(&self, ids: Vec<i64>) {
        *self.priority.lock().unwrap_or_else(|e| e.into_inner()) = ids;
    }
    pub fn snapshot(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn auto_allowed(&self) -> bool {
        !self.cancel.load(Ordering::Relaxed) && !self.paused.load(Ordering::Relaxed)
    }
    pub fn control(&self, action: &str) -> Result<()> {
        match action {
            "cancel" => self.cancel.store(true, Ordering::Relaxed),
            "pause" => self.paused.store(true, Ordering::Relaxed),
            "resume" => self.paused.store(false, Ordering::Relaxed),
            _ => anyhow::bail!("Unknown analysis action"),
        };
        self.status.lock().unwrap_or_else(|e| e.into_inner()).paused =
            self.paused.load(Ordering::Relaxed);
        Ok(())
    }
    pub fn start(&self, db: PathBuf, tracks: Vec<(i64, String, i64)>) -> Result<()> {
        self.start_with_model(db, tracks, None)
    }
    pub fn start_with_model(
        &self,
        db: PathBuf,
        tracks: Vec<(i64, String, i64)>,
        model_setup: Option<(PathBuf, PathBuf, bool)>,
    ) -> Result<()> {
        self.start_options(db,tracks,model_setup,false,false)
    }
    pub fn rescan(&self,db:PathBuf,tracks:Vec<(i64,String,i64)>,model:Option<(PathBuf,PathBuf,bool)>,force:bool)->Result<()> {
        self.start_options(db,tracks,model,true,force)
    }
    fn start_options(&self,db:PathBuf,tracks:Vec<(i64,String,i64)>,model_setup:Option<(PathBuf,PathBuf,bool)>,refresh_metadata:bool,force:bool)->Result<()> {
        {
            let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
            ensure!(
                !s.running,
                "Analysis is already running. Pause or cancel it first."
            );
            *s = Status {
                running: true,
                total: tracks.len(),
                ..Default::default()
            };
        }
        self.cancel.store(false, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
        *self.model_setup.lock().unwrap_or_else(|e|e.into_inner())=model_setup.clone();
        let worker = self.clone();
        std::thread::Builder::new().name("musicos-audio-analysis".into()).spawn(move||{
   let mut model=model_setup.and_then(|(dir,runtime,evaluation)|match crate::spark_engine::Model::load(&dir,&runtime,evaluation){Ok(model)=>Some(model),Err(error)=>{worker.status.lock().unwrap_or_else(|e|e.into_inner()).errors.push(format!("Audio model: {error:#}"));None}});
   let mut remaining=tracks;
   loop {
    let pending=worker.requested.lock().unwrap_or_else(|e|e.into_inner()).drain(..).collect::<Vec<_>>();
    for track in pending{if !remaining.iter().any(|t|t.0==track.0){remaining.push(track);worker.status.lock().unwrap_or_else(|e|e.into_inner()).total+=1;}}
    if remaining.is_empty(){let mut status=worker.status.lock().unwrap_or_else(|e|e.into_inner());if !worker.requested.lock().unwrap_or_else(|e|e.into_inner()).is_empty(){continue;}status.running=false;status.paused=false;return;}
    if model.is_none(){let setup=worker.model_setup.lock().unwrap_or_else(|e|e.into_inner()).take();if let Some((dir,runtime,beta))=setup{match crate::spark_engine::Model::load(&dir,&runtime,beta){Ok(value)=>model=Some(value),Err(e)=>worker.status.lock().unwrap_or_else(|e|e.into_inner()).errors.push(format!("Audio model: {e:#}"))}}}
    let mut priority=worker.seed_priority.lock().unwrap_or_else(|e|e.into_inner()).clone();
    priority.extend(worker.priority.lock().unwrap_or_else(|e|e.into_inner()).iter().copied());
    let index=remaining.iter().enumerate().min_by_key(|(_,t)|priority.iter().position(|id|*id==t.0).unwrap_or(usize::MAX)).map(|(i,_)|i).unwrap();
    let (id,path,mut duration)=remaining.remove(index);
    worker.seed_priority.lock().unwrap_or_else(|e|e.into_inner()).retain(|seed|*seed!=id);
    if !worker.checkpoint(){break;}
    if refresh_metadata {
     let probe=(||->Result<()>{let library=musicos_init::database::Db::open(&db.to_string_lossy())?;let track=library.get_track_by_id(id)?.ok_or_else(||anyhow::anyhow!("Song no longer exists"))?;
      let metadata=crate::desktop_metadata::refresh(&db,&musicos_desktop::local::library::track_from_init(track))?;if let Some(seconds)=metadata.duration{duration=seconds as i64;}Ok(())})();
     if let Err(error)=probe {let mut status=worker.status.lock().unwrap_or_else(|e|e.into_inner());if status.errors.len()<20{status.errors.push(format!("Specifications {}: {error:#}",Path::new(&path).file_name().unwrap_or_default().to_string_lossy()));}}
    }
    let mut refreshed=false;let result=(||->Result<Features>{if !force {if let Some(f)=cached_current(&db,id,&path){return Ok(f)}}refreshed=true;let signature=identity(Path::new(&path))?;let mut excerpts=Vec::new();let length=duration.max(0) as f32;let window=18_f32.min(length.max(1.));let starts=if length>window*2. {vec![0.,(length-window)*0.5,(length-window)*0.85]}else{vec![0.]};
     for start in starts {ensure!(worker.checkpoint(),"Analysis cancelled");let mut decoder=musicos_desktop::local::decoded_audio::open(Path::new(&path))?;if start>0. {decoder.try_seek(Duration::from_secs_f32(start)).map_err(|e|anyhow::anyhow!("This decoder cannot seek to representative excerpts: {e}"))?;}let source_rate=decoder.sample_rate() as usize;let channels=decoder.channels() as usize;ensure!(source_rate>0&&channels>0,"Invalid audio format");let uniform=crate::structure::MonoResampler::new(decoder,RATE as u32);let mut mono=Vec::with_capacity(RATE*18);for (i,sample) in uniform.take((window*RATE as f32) as usize).enumerate(){if i%RATE==0 {ensure!(worker.checkpoint(),"Analysis cancelled");}mono.push(sample);}excerpts.push(mono);}
     ensure!(identity(Path::new(&path))?==signature,"File changed during analysis; run analysis again");let mut f=extract(id,&excerpts);let structure=crate::structure::scan(Path::new(&path),f.bpm,f.bpm_confidence,||worker.checkpoint())?;if structure.beat_confidence>=0.5 {f.bpm=structure.tempo;f.bpm_confidence=structure.beat_confidence;}f.structure=Some(structure);ensure!(identity(Path::new(&path))?==signature,"File changed during analysis; run analysis again");cache(&db)?.execute("INSERT INTO desktop_dsp_v1 VALUES(?,?,?,?) ON CONFLICT(track_id) DO UPDATE SET identity=excluded.identity,version=excluded.version,result=excluded.result",rusqlite::params![id,signature,VERSION,serde_json::to_string(&f)?])?;Ok(f)})();
    if worker.cancel.load(Ordering::Relaxed){break;}
    if result.is_ok() {if let Some(model)=model.as_mut(){if let Err(error)=crate::embeddings::analyze(&db,id,Path::new(&path),duration as f32,model,||worker.checkpoint()){if !worker.cancel.load(Ordering::Relaxed){let mut status=worker.status.lock().unwrap_or_else(|e|e.into_inner());if status.errors.len()<20 {status.errors.push(format!("Audio embedding {}: {error:#}",Path::new(&path).file_name().unwrap_or_default().to_string_lossy()));}}}}}
    if worker.cancel.load(Ordering::Relaxed){break;}
    if refreshed&&result.is_ok(){let mut revisions=worker.revisions.lock().unwrap_or_else(|e|e.into_inner());*revisions.entry(id).or_default()+=1;}
    let mut s=worker.status.lock().unwrap_or_else(|e|e.into_inner());match result {Ok(f)=>s.features.push(f),Err(e)=>{if s.errors.len()<20{s.errors.push(format!("{}: {e:#}",Path::new(&path).file_name().unwrap_or_default().to_string_lossy()));}}}s.processed+=1;
   }
   let mut s=worker.status.lock().unwrap_or_else(|e|e.into_inner());s.running=false;s.paused=false;
  })?;
        Ok(())
    }
    fn checkpoint(&self) -> bool {
        while self.paused.load(Ordering::Relaxed) && !self.cancel.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(80));
        }
        !self.cancel.load(Ordering::Relaxed)
    }
}
pub fn extract(id:i64,excerpts:&[Vec<f32>])->Features{extract_mode(id,excerpts,false)}
pub fn extract_multiband(id:i64,excerpts:&[Vec<f32>])->Features{extract_mode(id,excerpts,true)}
fn extract_mode(id: i64, excerpts: &[Vec<f32>],multiband:bool) -> Features {
    let n = 4096;
    let hop = 512;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
    let mut chroma = [0_f32; 12];
    let mut tempos = Vec::new();
    for mono in excerpts {
        let mut onset = Vec::new();
        let mut whitening=crate::onsets::Multiband::default();
        let mut previous = vec![0_f32; n / 2];
        let mut buf = vec![Complex::new(0., 0.); n];
        for frame in mono.windows(n).step_by(hop) {
            for (i, &s) in frame.iter().enumerate() {
                buf[i] = Complex::new(
                    s * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()),
                    0.,
                );
            }
            fft.process(&mut buf);
            let mut flux = [0.;3];
            for k in 1..n / 2 {
                let frequency = k as f32 * RATE as f32 / n as f32;
                let magnitude = buf[k].norm();
                if frequency < 4000. {
                    flux[if frequency<200.{0}else if frequency<2000.{1}else{2}] += if multiband{((1.+magnitude).ln() - (1.+previous[k]).ln()).max(0.)}else{(magnitude-previous[k]).max(0.)};
                }
                previous[k] = magnitude;
                if (65.0..2000.0).contains(&frequency)
                    && magnitude > buf[k - 1].norm()
                    && magnitude >= buf[k + 1].norm()
                {
                    let midi = 69. + 12. * (frequency / 440.).log2();
                    let pitch = midi.round();
                    if (midi - pitch).abs() < 0.35 {
                        chroma[(pitch as i32).rem_euclid(12) as usize] += magnitude;
                    }
                }
            }
            onset.push(if multiband{whitening.combine(flux)}else{flux.iter().sum()});
        }
        let rate = RATE as f32 / hop as f32;
        if onset.len() > 50 {
            let mean = onset.iter().sum::<f32>() / onset.len() as f32;
            let onset = onset.iter().map(|v| (v - mean).max(0.)).collect::<Vec<_>>();
            let energy = onset.iter().map(|v| v * v).sum::<f32>();
            if energy > 1e-8 {
                let mut best = (0., 0.);
                for lag in ((rate * 60. / 180.).floor() as usize).max(1)
                    ..=(rate * 60. / 65.).ceil() as usize
                {
                    let corr = onset
                        .iter()
                        .skip(lag)
                        .zip(&onset)
                        .map(|(a, b)| a * b)
                        .sum::<f32>()
                        / energy;
                    let bpm = 60. * rate / lag as f32;
                    if corr > best.1 {
                        best = (bpm, corr);
                    }
                }
                if best.1 > 0.12 {
                    tempos.push(best);
                }
            }
        }
    }
    tempos.sort_by(|a, b| a.0.total_cmp(&b.0));
    let tempo = tempos.get(tempos.len() / 2).copied();
    let major = [
        6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
    ];
    let minor = [
        6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
    ];
    let mut scores = Vec::new();
    for (mode, profile) in [major, minor].iter().enumerate() {
        for root in 0..12 {
            let rotated = std::array::from_fn::<_, 12, _>(|i| chroma[(root + i) % 12]);
            scores.push((root + 12 * mode, correlation(&rotated, profile)));
        }
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    let best = scores[0];
    let confidence = ((best.1 - scores[1].1) * 4.).clamp(0., 1.);
    let key = if best.1 > 0.45 && confidence > 0.08 {
        Some(best.0)
    } else {
        None
    };
    let names = [
        "C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B",
    ];
    Features {
        track_id: id,
        bpm: tempo.map(|t| (t.0 * 10.).round() / 10.),
        bpm_confidence: tempo.map(|t| t.1.clamp(0., 1.)).unwrap_or(0.),
        key: key.map(|k| {
            format!(
                "{} {}",
                names[k % 12],
                if k < 12 { "major" } else { "minor" }
            )
        }),
        key_index: key,
        key_confidence: confidence,
        structure: None,
    }
}
fn correlation(a: &[f32; 12], b: &[f32; 12]) -> f32 {
    let ma = a.iter().sum::<f32>() / 12.;
    let mb = b.iter().sum::<f32>() / 12.;
    let numerator = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - ma) * (y - mb))
        .sum::<f32>();
    let denominator = (a.iter().map(|x| (x - ma).powi(2)).sum::<f32>()
        * b.iter().map(|y| (y - mb).powi(2)).sum::<f32>())
    .sqrt();
    if denominator > 1e-8 {
        numerator / denominator
    } else {
        0.
    }
}
pub fn compatibility(a: Option<&Features>, b: Option<&Features>) -> f32 {
    let (Some(a), Some(b)) = (a, b) else {
        return 2.;
    };
    let tempo = match (a.bpm, b.bpm) {
        (Some(a), Some(b)) => {
            [b / 2., b, b * 2.]
                .iter()
                .map(|v| ((a / v).ln()).abs())
                .fold(f32::INFINITY, f32::min)
                * 6.
        }
        _ => 1.,
    };
    let key = match (a.key_index, b.key_index) {
        (Some(a), Some(b)) => {
            let same = a == b;
            let relative = a < 12 && b == ((a + 9) % 12 + 12) || b < 12 && a == ((b + 9) % 12 + 12);
            let fifth =
                a / 12 == b / 12 && ((a % 12 + 7) % 12 == b % 12 || (b % 12 + 7) % 12 == a % 12);
            if same || relative {
                0.
            } else if fifth {
                0.2
            } else {
                0.7
            }
        }
        _ => 0.5,
    };
    tempo + key
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_stays_unknown() {
        let f = extract(1, &[vec![0.; RATE * 5]]);
        assert!(f.bpm.is_none());
        assert!(f.key.is_none());
    }
    #[test]
    fn estimates_click_tempo() {
        let mut wave = vec![0.; RATE * 18];
        for beat in 0..36 {
            for k in 0..150 {
                wave[beat * RATE / 2 + k] = (k as f32 * 0.9).sin() * (1. - k as f32 / 150.);
            }
        }
        let f = extract(1, &[wave]);
        assert!((f.bpm.unwrap() - 120.).abs() < 5., "{f:?}");
    }
    #[test]
    fn estimates_tonal_key() {
        let mut wave = vec![0.; RATE * 12];
        let notes = [
            (60., 6.35),
            (62., 3.48),
            (64., 4.38),
            (65., 4.09),
            (67., 5.19),
            (69., 3.66),
            (71., 2.88),
        ];
        for (midi, weight) in notes {
            let frequency = 440_f32 * 2_f32.powf((midi - 69.) / 12.);
            for (i, v) in wave.iter_mut().enumerate() {
                *v += (std::f32::consts::TAU * frequency * i as f32 / RATE as f32).sin() * weight
                    / 40.;
            }
        }
        let f = extract(1, &[wave]);
        assert_eq!(f.key.as_deref(), Some("C major"), "{f:?}");
    }
    #[test]
    fn cancelled_worker_stops_without_persisting_fake_features() {
        let dir = tempfile::tempdir().unwrap();
        let worker = Analysis::default();
        let tracks = (0..1000)
            .map(|id| {
                (
                    id,
                    dir.path()
                        .join("missing.wav")
                        .to_string_lossy()
                        .into_owned(),
                    30,
                )
            })
            .collect();
        worker.start(dir.path().join("cache.db"), tracks).unwrap();
        worker.control("pause").unwrap();
        worker.control("cancel").unwrap();
        for _ in 0..100 {
            if !worker.snapshot().running {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!worker.snapshot().running);
        assert!(worker.snapshot().features.is_empty());
    }
    #[test]fn old_valid_cache_survives_failed_refresh_but_is_not_current(){
        let dir=tempfile::tempdir().unwrap();let db=dir.path().join("cache.db");let file=dir.path().join("corrupt.wav");std::fs::write(&file,b"not audio").unwrap();let f=extract(7,&[]);
        cache(&db).unwrap().execute("INSERT INTO desktop_dsp_v1 VALUES(?,?,?,?)",rusqlite::params![7,identity(&file).unwrap(),"previous-version",serde_json::to_string(&f).unwrap()]).unwrap();
        assert!(cached(&db,7,file.to_str().unwrap()).is_some());assert!(cached_current(&db,7,file.to_str().unwrap()).is_none());
        let inputs=[(7,file.to_str().unwrap())];assert!(cached_many(&db,&inputs)[0].is_some());assert!(cached_many_current(&db,&inputs)[0].is_none());
        let worker=Analysis::default();worker.start(db.clone(),vec![(7,file.to_string_lossy().into_owned(),30)]).unwrap();
        let end=std::time::Instant::now()+Duration::from_secs(3);while worker.snapshot().running{assert!(std::time::Instant::now()<end);std::thread::sleep(Duration::from_millis(10));}
        assert!(!worker.snapshot().errors.is_empty());assert!(cached(&db,7,file.to_str().unwrap()).is_some());assert!(cached_current(&db,7,file.to_str().unwrap()).is_none());
    }
    #[test]
    fn changed_file_invalidates_cache() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("cache.db");
        let file = dir.path().join("x.wav");
        std::fs::write(&file, b"one").unwrap();
        let f = extract(7, &[]);
        cache(&db)
            .unwrap()
            .execute(
                "INSERT INTO desktop_dsp_v1 VALUES(?,?,?,?)",
                rusqlite::params![
                    7,
                    identity(&file).unwrap(),
                    VERSION,
                    serde_json::to_string(&f).unwrap()
                ],
            )
            .unwrap();
        assert!(cached(&db, 7, file.to_str().unwrap()).is_some());
        std::fs::write(&file, b"changed").unwrap();
        assert!(cached(&db, 7, file.to_str().unwrap()).is_none());
    }
    #[test]
    fn harmonic_and_tempo_compatibility() {
        let mut a = extract(1, &[]);
        a.bpm = Some(120.);
        a.key_index = Some(0);
        let mut b = a.clone();
        b.bpm = Some(60.);
        b.key_index = Some(21);
        assert_eq!(compatibility(Some(&a), Some(&b)), 0.);
        b.bpm = Some(89.);
        b.key_index = Some(6);
        assert!(compatibility(Some(&a), Some(&b)) > 1.);
    }
}

#[cfg(test)]
#[test]
fn full_rescan_replaces_dsp_only_after_success_without_rewriting_audio(){
 let dir=tempfile::tempdir().unwrap();let file=dir.path().join("song.flac");let database=dir.path().join("library.sqlite");
 let generated=std::process::Command::new("ffmpeg").args(["-loglevel","error","-f","lavfi","-i","sine=frequency=440:duration=1","-y"]).arg(&file).status().unwrap();assert!(generated.success());
 let bytes=std::fs::read(&file).unwrap();let db=musicos_init::database::Db::open(database.to_str().unwrap()).unwrap();let id=db.insert_track(file.to_str().unwrap(),"Preserved title","Artist","Album",1,bytes.len() as i64,1).unwrap();drop(db);
 let original=Features{track_id:id,bpm:Some(123.),bpm_confidence:1.,key:None,key_index:None,key_confidence:0.,structure:None};
 cache(&database).unwrap().execute("INSERT INTO desktop_dsp_v1 VALUES(?,?,?,?)",rusqlite::params![id,identity(&file).unwrap(),VERSION,serde_json::to_string(&original).unwrap()]).unwrap();
 let worker=Analysis::default();let tracks=vec![(id,file.to_string_lossy().into_owned(),1)];
 let wait=||{let until=std::time::Instant::now()+Duration::from_secs(15);while worker.progress().running&&std::time::Instant::now()<until{std::thread::sleep(Duration::from_millis(20));}assert!(!worker.progress().running);assert!(worker.snapshot().errors.is_empty(),"{:?}",worker.snapshot().errors);};
 worker.rescan(database.clone(),tracks.clone(),None,false).unwrap();wait();assert_eq!(worker.pair_revision(&[id]),0);assert_eq!(cached(&database,id,file.to_str().unwrap()).unwrap().bpm,Some(123.));
 worker.rescan(database.clone(),tracks,None,true).unwrap();wait();assert_eq!(worker.pair_revision(&[id]),1);assert_ne!(cached(&database,id,file.to_str().unwrap()).unwrap().bpm,Some(123.));
 assert_eq!(std::fs::read(file).unwrap(),bytes);assert_eq!(musicos_init::database::Db::open(database.to_str().unwrap()).unwrap().get_track_by_id(id).unwrap().unwrap().title,"Preserved title");
}

#[cfg(test)]mod tempo_display_tests {
 use super::*;
 #[test]fn cached_tempos_require_valid_file_identity_and_bounded_bpm(){
  let root=tempfile::tempdir().unwrap();let db=root.path().join("library.sqlite3");let file=root.path().join("song.wav");std::fs::write(&file,b"fixture").unwrap();let signature=identity(&file).unwrap();let conn=cache(&db).unwrap();
  for (id,bpm,signature) in [(1,123.4,signature.as_str()),(2,0.,signature.as_str()),(3,999.,signature.as_str()),(4,120.,"stale")] {conn.execute("INSERT INTO desktop_dsp_v1 VALUES(?,?,?,?)",rusqlite::params![id,signature,"previous-valid-model",serde_json::json!({"bpm":bpm,"structure":{"beats":[0.1,0.6,1.1]}}).to_string()]).unwrap();}
  let path=file.to_str().unwrap();let tempos=cached_tempos(&db,&[(1,path),(2,path),(3,path),(4,path)]).unwrap();assert_eq!(tempos,vec![(1,123.4)]);
 }
}
