//! Global listening controls. Coefficients are prepared on the controller thread.
use serde::{Deserialize,Serialize};
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(default)]
pub struct Listening {pub bands:u8,pub gains3:[f32;3],pub gains8:[f32;8],pub speed:f32,pub pitch:f32,pub linked:bool,pub reverb:f32}
impl Default for Listening {fn default()->Self{Self{bands:3,gains3:[0.;3],gains8:[0.;8],speed:1.,pitch:0.,linked:true,reverb:0.}}}
impl Listening {
 pub fn normalize(mut self)->Self{self.reverb=if self.reverb.is_finite(){self.reverb.clamp(0.,1.)}else{0.};self.bands=if self.bands==8{8}else{3};for gain in self.gains3.iter_mut().chain(self.gains8.iter_mut()){*gain=if gain.is_finite(){gain.clamp(-12.,12.)}else{0.};}self.speed=if self.speed.is_finite(){self.speed.clamp(0.5,2.)}else{1.};self.pitch=if self.linked{12.*self.speed.log2()}else if self.pitch.is_finite(){self.pitch.clamp(-12.,12.)}else{0.};self}
 pub fn transpose(&self)->f32{2_f32.powf(self.pitch/12.)}
 pub fn coefficients(&self)->Equalizer{let mut result=Equalizer::default();result.reverb=self.reverb;let (freq,gains):(&[f32],&[f32])=if self.bands==8{(&[63.,125.,250.,500.,1000.,2000.,4000.,8000.],&self.gains8)}else{(&[125.,1000.,8000.],&self.gains3)};let mut boost=0_f32;for(i,(&hz,&gain))in freq.iter().zip(gains).enumerate(){result.coefficients[i]=peak(hz,gain,if self.bands==3{0.6}else{1.});boost=boost.max(gain);}result.headroom=10_f32.powf(-boost/20.);result}
}
#[derive(Debug,Clone,Copy)]pub struct Equalizer{pub coefficients:[[f32;5];8],pub headroom:f32,pub reverb:f32}
impl Default for Equalizer{fn default()->Self{Self{coefficients:[[1.,0.,0.,0.,0.];8],headroom:1.,reverb:0.}}}
fn peak(hz:f32,gain:f32,q:f32)->[f32;5]{let a=10_f32.powf(gain/40.);let w=2.*std::f32::consts::PI*hz/48000.;let alpha=w.sin()/(2.*q);let a0=1.+alpha/a;[(1.+alpha*a)/a0,-2.*w.cos()/a0,(1.-alpha*a)/a0,-2.*w.cos()/a0,(1.-alpha/a)/a0]}
#[derive(Clone,Copy,Default)]struct Bank{coeff:Equalizer,z:[[[f32;2];8];2]}
impl Bank{fn process(&mut self,mut x:[f32;2])->[f32;2]{for(c,sample)in x.iter_mut().enumerate(){*sample*=self.coeff.headroom;for(i,b)in self.coeff.coefficients.iter().enumerate(){let z=&mut self.z[c][i];let y=b[0]**sample+z[0];z[0]=b[1]**sample-b[3]*y+z[1];z[1]=b[2]**sample-b[4]*y;*sample=y;}}x}}
#[derive(Default)]pub struct EqProcessor{current:Bank,next:Bank,ramp:usize}
impl EqProcessor{
 pub fn set(&mut self,coeff:Equalizer){if self.ramp>0{self.current=self.next;}self.next=self.current;self.next.coeff=coeff;self.ramp=960;}
 pub fn process(&mut self,x:[f32;2])->[f32;2]{let a=self.current.process(x);if self.ramp==0{return a;}let b=self.next.process(x);let t=1.-self.ramp as f32/960.;self.ramp-=1;if self.ramp==0{self.current=self.next;}[a[0]*(1.-t)+b[0]*t,a[1]*(1.-t)+b[1]*t]}
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn linked_defaults_and_bounds(){let p=Listening{speed:2.,pitch:-12.,..Default::default()}.normalize();assert_eq!(p.pitch,12.);assert_eq!(p.transpose(),2.);let p=Listening{speed:f32::NAN,gains3:[f32::NAN,40.,-40.],..Default::default()}.normalize();assert_eq!(p.gains3,[0.,12.,-12.]);assert_eq!(p.speed,1.);}
 #[test]fn flat_is_transparent_and_boost_is_bounded(){let mut eq=EqProcessor::default();for _ in 0..1000{assert_eq!(eq.process([0.25,-0.5]),[0.25,-0.5]);}eq.set(Listening{gains3:[12.;3],..Default::default()}.coefficients());for i in 0..48000{let x=(i as f32*0.13).sin()*0.5;assert!(eq.process([x,x]).iter().all(|v|v.is_finite()&&v.abs()<2.));}}
 #[test]fn mid_cut_attenuates_mid_tone(){let mut eq=EqProcessor::default();eq.set(Listening{gains3:[0.,-12.,0.],..Default::default()}.coefficients());let mut power=0.;for i in 0..48000{let x=(i as f32*2.*std::f32::consts::PI*1000./48000.).sin();let y=eq.process([x,x])[0];if i>24000{power+=y*y;}}assert!((power/24000.).sqrt()<0.2);}
}

/// Four damped, decorrelated stereo delay lines, allocated before output starts.
/// Wet amount is smoothed per sample; zero is an exact dry bypass.
pub struct Reverb {lines:Vec<Vec<[f32;2]>>,at:[usize;4],damp:[[f32;2];4],amount:f32,target:f32}
impl Default for Reverb {fn default()->Self {Self{lines:[1423,1789,2137,2557].into_iter().map(|n|vec![[0.;2];n]).collect(),at:[0;4],damp:[[0.;2];4],amount:0.,target:0.}}}
impl Reverb {
 pub fn clear(&mut self){for line in &mut self.lines{line.fill([0.;2]);}self.damp=[[0.;2];4];}
 pub fn set(&mut self,amount:f32){self.target=amount.clamp(0.,1.);}
 pub fn process(&mut self,input:[f32;2])->[f32;2]{
  self.amount+=(self.target-self.amount)*0.0015;
  if self.target==0.&&self.amount<0.00001 {self.amount=0.;}
  let mut wet=[0.;2];
  for i in 0..4 {let delayed=self.lines[i][self.at[i]];for c in 0..2 {self.damp[i][c]+=(delayed[c]-self.damp[i][c])*0.32;self.lines[i][self.at[i]][c]=input[c]*0.35+self.damp[i][1-c]*0.78;wet[c]+=delayed[c]*0.25;}self.at[i]=(self.at[i]+1)%self.lines[i].len();}
  if self.amount==0. {return input;}
  [(input[0]*(1.-self.amount*0.45)+wet[0]*self.amount*0.8)/(1.+self.amount*0.3),(input[1]*(1.-self.amount*0.45)+wet[1]*self.amount*0.8)/(1.+self.amount*0.3)]
 }
}
#[cfg(test)]mod reverb_tests {use super::*;
 #[test]fn dry_is_exact_and_tail_decays(){let mut r=Reverb::default();assert_eq!(r.process([0.4,-0.3]),[0.4,-0.3]);r.set(1.);for _ in 0..10000{r.process([0.;2]);}r.process([1.,0.]);let mut tail=0_f32;for _ in 0..20000{let v=r.process([0.;2]);assert!(v.iter().all(|x|x.is_finite()&&x.abs()<1.));tail=tail.max(v[0].abs()+v[1].abs());}assert!(tail>0.001);for _ in 0..480000 {r.process([0.;2]);}assert!(r.process([0.;2]).iter().all(|x|x.abs()<0.00001));r.set(0.);for _ in 0..10000 {r.process([0.;2]);}assert_eq!(r.process([0.4,-0.3]),[0.4,-0.3]);}
 #[test]fn old_preferences_default_to_dry(){let p:Listening=serde_json::from_str(r#"{"speed":1.2}"#).unwrap();assert_eq!(p.reverb,0.);assert_eq!(Listening{reverb:f32::NAN,..Default::default()}.normalize().reverb,0.);}
}
