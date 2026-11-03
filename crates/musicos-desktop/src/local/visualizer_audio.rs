//! Native-only visualizer telemetry from the already mixed audible output.
//! This module is never called by the audio callback and has no serialization.
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
#[derive(Clone)]
pub struct Features {pub waveform:[[f32;512];2],pub spectrum:[[f32;512];2],pub bands:[f64;3],pub attenuated:[f64;3],pub sequence:u64}
impl Default for Features{fn default()->Self{Self{waveform:[[0.;512];2],spectrum:[[0.;512];2],bands:[1.;3],attenuated:[1.;3],sequence:0}}}
#[derive(Default)]
pub struct Tap {active:AtomicBool,latest:Mutex<Arc<Features>>}
impl Tap{
 pub fn set_active(&self,active:bool){self.active.store(active,Ordering::Release);}
 pub fn active(&self)->bool{self.active.load(Ordering::Acquire)}
 pub fn latest(&self)->Arc<Features>{self.latest.lock().unwrap_or_else(|e|e.into_inner()).clone()}
 fn publish(&self,features:Features){*self.latest.lock().unwrap_or_else(|e|e.into_inner())=Arc::new(features);}
}
pub struct Analyzer{samples:[[f32;2];1024],at:usize,count:usize,sequence:u64,avg:[f64;3],long:[f64;3],pub transforms:u64}
impl Default for Analyzer{fn default()->Self{Self{samples:[[0.;2];1024],at:0,count:0,sequence:0,avg:[1.;3],long:[1.;3],transforms:0}}}
impl Analyzer{
 pub fn push(&mut self,frame:[f32;2]){self.samples[self.at]=frame.map(|s|if s.is_finite(){s.clamp(-1.,1.)}else{0.});self.at=(self.at+1)%1024;self.count=(self.count+1).min(1024);}
 pub fn reset_history(&mut self){self.count=0;}
 pub fn publish(&mut self,tap:&Tap,dt:f64){
  if !tap.active()||self.count<1024{return;}
  let mut raw=[[0_f32;1024];3];
  for i in 0..1024{let v=self.samples[(self.at+i)%1024];for c in 0..2{raw[c][i]=(v[c]*128.).floor().clamp(-128.,127.);}raw[2][i]=((v[0]+v[1])*64.).floor().clamp(-128.,127.);}
  let mut frame=Features::default();
  for c in 0..2{for j in 0..512{let i=j*2;frame.waveform[c][j]=((raw[c][i]+raw[c][i.saturating_sub(1)])*0.5).trunc()/128.;}frame.spectrum[c]=spectrum(&raw[c]);}
  let mono=spectrum(&raw[2]);self.transforms+=3;self.sequence+=1;frame.sequence=self.sequence;
  let bin=48000_f64/1024.;let boundary=|hz:f64|((hz/bin).round()as isize-1).clamp(0,511)as usize;
  let starts=[boundary(20.),boundary(320.),boundary(2800.)];let stops=[boundary(320.),boundary(2800.),boundary(11025.)];
  let fps=(1./dt.max(1./144.)).clamp(15.,144.);
  for i in 0..3{let immediate=mono[starts[i]..stops[i]].iter().map(|v|*v as f64).sum::<f64>();let rate=if immediate>self.avg[i]{0.2_f64}else{0.5_f64}.powf(30./fps);self.avg[i]=self.avg[i]*rate+immediate*(1.-rate);let long=if self.sequence<50{0.9_f64}else{0.992_f64}.powf(30./fps);self.long[i]=self.long[i]*long+immediate*(1.-long);if self.long[i]>=0.001{frame.bands[i]=immediate/self.long[i];frame.attenuated[i]=self.avg[i]/self.long[i];}}
  tap.publish(frame);
 }
}
/// Radix-two FFT, with MilkDrop's frequency equalization. Fixed stack storage;
/// no allocation or FFT planning is performed on the output thread.
fn spectrum(input:&[f32;1024])->[f32;512]{
 let mut re=[0_f32;1024];let mut im=[0_f32;1024];for i in 0..1024{re[i]=input[(i as u16).reverse_bits()as usize>>6];}
 let mut size=2;while size<=1024{let theta=-std::f32::consts::TAU/size as f32;let(wpr,wpi)=(theta.cos(),theta.sin());let(mut wr,mut wi)=(1.,0.);for m in 0..size/2{for i in (m..1024).step_by(size){let j=i+size/2;let tr=wr*re[j]-wi*im[j];let ti=wr*im[j]+wi*re[j];re[j]=re[i]-tr;im[j]=im[i]-ti;re[i]+=tr;im[i]+=ti;}let old=wr;wr=old*wpr-wi*wpi;wi=wi*wpr+old*wpi;}size*=2;}
 std::array::from_fn(|i|-0.02*((512-i)as f32/512.).ln()*re[i].hypot(im[i]))
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn silence_and_inactive_work_are_bounded(){let tap=Tap::default();let mut a=Analyzer::default();for _ in 0..1024{a.push([0.;2]);}a.publish(&tap,1./60.);assert_eq!(a.transforms,0);tap.set_active(true);a.publish(&tap,1./60.);assert_eq!(a.transforms,3);assert!(tap.latest().spectrum.iter().flatten().all(|v|*v==0.));tap.set_active(false);a.publish(&tap,1./60.);assert_eq!(a.transforms,3);}
 #[test]fn fft_peak_and_channels_follow_the_audible_mixture(){let tap=Tap::default();tap.set_active(true);let mut a=Analyzer::default();for i in 0..1024{a.push([(i as f32*std::f32::consts::TAU*32./1024.).sin()*0.5,0.]);}a.publish(&tap,1./60.);let f=tap.latest();let peak=f.spectrum[0].iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1)).unwrap().0;assert_eq!(peak,32);assert!(f.spectrum[1].iter().all(|v|*v==0.));assert!(f.bands.iter().all(|v|v.is_finite()));}
}
