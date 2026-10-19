//! Full-track, low-resolution structural estimates. Region labels deliberately
//! describe evidence, not asserted verse/chorus semantics.
use anyhow::{ensure,Result};

use rustfft::{FftPlanner,num_complex::Complex};
use serde::{Deserialize,Serialize};
use std::path::Path;
pub const VERSION:&str="structure-4-measured-phrases-11025-2048-256";
const RATE:usize=11025;
const HOP:usize=256;
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
pub struct Boundary {pub seconds:f32,pub confidence:f32}
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
pub struct Region {pub start:f32,pub end:f32,pub repeated:bool,pub confidence:f32}
#[derive(Clone,Debug,Default,Serialize,Deserialize)]
pub struct Structure {
 pub version:String,pub duration:f32,pub energy:f32,pub energy_step:f32,
 pub envelope:Vec<f32>,pub beats:Vec<f32>,pub beat_confidence:f32,
 #[serde(default)] pub tempo:Option<f32>,
 pub phrases:Vec<Boundary>,pub changes:Vec<Boundary>,pub regions:Vec<Region>,
 pub silence:Vec<Region>,
}
/// Bounded streaming decoder: retains spectral summaries, never a full PCM file.
/// Called exclusively from the resumable background analysis worker.
pub fn scan(path:&Path,bpm:Option<f32>,confidence:f32,checkpoint:impl FnMut()->bool)->Result<Structure>{scan_mode(path,bpm,confidence,checkpoint,false)}
pub fn scan_multiband(path:&Path,bpm:Option<f32>,confidence:f32,checkpoint:impl FnMut()->bool)->Result<Structure>{scan_mode(path,bpm,confidence,checkpoint,true)}
fn scan_mode(path:&Path,bpm:Option<f32>,tempo_confidence:f32,mut checkpoint:impl FnMut()->bool,multiband:bool)->Result<Structure>{
 let decoder=musicos_desktop::local::decoded_audio::open(path)?;
 let mut source=rodio::source::UniformSourceIterator::<_,f32>::new(decoder,1,RATE as u32);
 let fft=FftPlanner::<f32>::new().plan_fft_forward(2048);
 let window=(0..2048).map(|i|0.5-0.5*(std::f32::consts::TAU*i as f32/2048.).cos()).collect::<Vec<_>>();
 let mut ring=[0_f32;2048];let mut pos=0;let mut samples=0;let mut buffer=vec![Complex::new(0.,0.);2048];
 let mut whitening=crate::onsets::Multiband::default();
 let mut previous=[0_f32;1024];let mut flux=Vec::new();let mut energy=Vec::new();let mut timbres=Vec::new();
 loop {
  if samples%(RATE*2)<HOP {ensure!(checkpoint(),"Analysis cancelled");}
  let mut read=0;for _ in 0..HOP {let Some(s)=source.next()else{break};ring[pos]=if s.is_finite(){s}else{0.};pos=(pos+1)%2048;samples+=1;read+=1;}
  if read==0{break;}
  ensure!(samples<=RATE*60*60*4,"Track exceeds the four-hour analysis limit");
  let rms=(ring.iter().map(|v|v*v).sum::<f32>()/2048.).sqrt();energy.push(rms);
  for i in 0..2048 {buffer[i]=Complex::new(ring[(pos+i)%2048]*window[i],0.);}
  fft.process(&mut buffer);let mut onset=[0.;3];let mut bands=[0_f32;8];
  for k in 1..1024 {let magnitude=buffer[k].norm();let log=(1.+magnitude).ln();let hz=k as f32*RATE as f32/2048.;if multiband{if hz<5000.{onset[if hz<200.{0}else if hz<2000.{1}else{2}]+=(log-previous[k]).max(0.);}}else{onset[0]+=(log-previous[k]).max(0.);}previous[k]=log;let band=((k as f32).log2() as usize).saturating_sub(2).min(7);bands[band]+=magnitude;}
  let norm=bands.iter().map(|v|v*v).sum::<f32>().sqrt().max(1e-8);bands.iter_mut().for_each(|v|*v/=norm);timbres.push(bands);flux.push(if multiband{whitening.combine(onset)}else{onset[0]});
 }
 ensure!(checkpoint(),"Analysis cancelled");
 let mut result=summarize(samples as f32/RATE as f32,&energy,&flux,&timbres,bpm,tempo_confidence);if multiband{result.version="structure-candidate-multiband-v1".into();}Ok(result)
}
fn summarize(duration:f32,energy:&[f32],flux:&[f32],timbres:&[[f32;8]],bpm:Option<f32>,_tempo_confidence:f32)->Structure {
 let step=HOP as f32/RATE as f32;let peak=energy.iter().copied().fold(0_f32,f32::max);
 let normalized=energy.iter().map(|e|if peak>1e-6{e/peak}else{0.}).collect::<Vec<_>>();
 let mut out=Structure{version:VERSION.into(),duration,energy:if energy.is_empty(){0.}else{energy.iter().sum::<f32>()/energy.len() as f32},energy_step:step,envelope:normalized,..Default::default()};
 let mut silence_start=None;
 for (i,&e) in energy.iter().enumerate(){if e<0.001 {silence_start.get_or_insert(i);}else if let Some(start)=silence_start.take(){if i-start>= (0.5/step) as usize {out.silence.push(Region{start:start as f32*step,end:i as f32*step,confidence:1.,repeated:false});}}}
 if let Some(start)=silence_start {if duration-start as f32*step>=0.5 {out.silence.push(Region{start:start as f32*step,end:duration,confidence:1.,repeated:false});}}
 let evidence=crate::beat::track(flux,step,bpm);
 out.beat_confidence=evidence.confidence;
 out.tempo=evidence.bpm;
 // The analysis window is centered 768 samples before its nominal hop time.
 out.beats=evidence.beats.into_iter().map(|t|(t-(1024.-HOP as f32)/RATE as f32).max(0.)).collect();

 // Compare two-second windows. Changes are relative timbre/energy evidence.
 let span=(2./step) as usize;let mut boundaries=vec![0.];let mut last=0.;
 if energy.len()>span*2 {for i in (span..energy.len()-span).step_by((0.5/step) as usize){let before=energy[i-span..i].iter().sum::<f32>()/span as f32;let after=energy[i..i+span].iter().sum::<f32>()/span as f32;let timbre=timbres[i-span].iter().zip(timbres[i+span-1]).map(|(a,b)|(a-b).powi(2)).sum::<f32>().sqrt();let contrast=(before-after).abs()/peak.max(1e-6)+timbre*0.4;let seconds=i as f32*step;if contrast>0.3&&seconds-last>4. {out.changes.push(Boundary{seconds,confidence:contrast.min(1.)});boundaries.push(seconds);last=seconds;}}}
 boundaries.push(duration);
 // Compare time-varying fingerprints and duration, not just average timbre.
 // Similar instrumentation alone is not evidence of a repeated section.
 let fingerprints=boundaries.windows(2).map(|pair|{
  let start=(pair[0]/step) as usize;let end=((pair[1]/step) as usize).min(timbres.len());let mut bins=[[0_f32;9];8];
  for (bin,values) in bins.iter_mut().enumerate(){let a=start+(end-start)*bin/8;let b=start+(end-start)*(bin+1)/8;if a<b {for i in a..b {for k in 0..8 {values[k]+=timbres[i][k]/(b-a) as f32;}values[8]+=out.envelope[i]/(b-a) as f32;}}}bins
 }).collect::<Vec<_>>();
 for (i,pair) in boundaries.windows(2).enumerate(){let length=pair[1]-pair[0];let profile=&fingerprints[i];let range=profile.iter().map(|p|p[8]).fold(0_f32,f32::max)-profile.iter().map(|p|p[8]).fold(1_f32,f32::min);
  let repeated=length>6.&&range>0.08&&boundaries.windows(2).enumerate().any(|(j,other)|{let other_length=other[1]-other[0];j!=i&&(length/other_length-1.).abs()<0.1&&profile.iter().flatten().zip(fingerprints[j].iter().flatten()).map(|(a,b)|(a-b).powi(2)).sum::<f32>()/72.<0.0025});
  out.regions.push(Region{start:pair[0],end:pair[1],repeated,confidence:if repeated{0.6}else{0.35}});
 }
 out.phrases=phrase_candidates(&out.beats,&out.changes,out.beat_confidence);

 out
}
/// Infer a recurring boundary phase only when several measured section changes
/// support it. A regular beat train by itself does not establish phrase phase.
pub fn phrase_candidates(beats:&[f32],changes:&[Boundary],confidence:f32)->Vec<Boundary> {
 if confidence<0.5||beats.len()<32{return Vec::new();}
 let matched=changes.iter().filter_map(|change|beats.iter().enumerate().min_by(|(_,a),(_,b)|(*a-change.seconds).abs().total_cmp(&(*b-change.seconds).abs())).filter(|(_,beat)|(**beat-change.seconds).abs()<0.3).map(|(index,_)|(index,change.confidence))).collect::<Vec<_>>();
 if matched.len()<4{return Vec::new();}
 let mut winner=None;
 for meter in [32,16,8,4] {let mut votes=vec![0_f32;meter];for &(index,weight) in &matched{votes[index%meter]+=weight;}
  let total=votes.iter().sum::<f32>();if total<=0.{continue;}
  for phase in 0..meter {let support=(votes[phase]+0.25*(votes[(phase+1)%meter]+votes[(phase+meter-1)%meter]))/total;
   let distinct=matched.iter().filter(|(index,_)|index%meter==phase).map(|(index,_)|index/meter).collect::<std::collections::BTreeSet<_>>().len();
   if distinct>=3&&support>=0.6&&winner.as_ref().is_none_or(|(_,_,old):&(usize,usize,f32)|support>*old){winner=Some((meter,phase,support));}
  }
 }
 let Some((meter,phase,support))=winner else{return Vec::new()};
 beats.iter().enumerate().filter(|(i,_)|i%meter==phase).map(|(_,beat)|Boundary{seconds:*beat,confidence:confidence.min(support)}).collect()
}
#[cfg(test)]mod tests {use super::*;
 #[test]fn phrase_phase_requires_structural_evidence(){let beats=(0..128).map(|i|i as f32*0.5).collect::<Vec<_>>();assert!(phrase_candidates(&beats,&[],1.).is_empty());let changes=[16,32,48,64,80].map(|i|Boundary{seconds:beats[i],confidence:0.6});let phrases=phrase_candidates(&beats,&changes,0.8);assert!(phrases.len()>=8);assert!(phrases.iter().all(|p|(p.seconds/8.).fract()==0.&&p.confidence>=0.5));}
 #[test]fn silence_never_invents_beats_or_repetition(){let s=summarize(30.,&vec![0.;646],&vec![0.;646],&vec![[0.;8];646],Some(120.),1.);assert!(s.beats.is_empty()&&s.phrases.is_empty());assert_eq!(s.silence.len(),1);assert!(s.regions.iter().all(|r|!r.repeated));}
 #[test]fn irregular_low_confidence_never_forces_beats(){let s=summarize(30.,&vec![0.1;646],&vec![1.;646],&vec![[0.2;8];646],Some(120.),0.1);assert_eq!(s.beat_confidence,0.);assert!(s.beats.is_empty());}
}
