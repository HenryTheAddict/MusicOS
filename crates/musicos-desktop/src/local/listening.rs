//! Global listening controls. Coefficients are prepared on the controller thread.
use serde::{Deserialize,Serialize};
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(default)]
pub struct Listening {pub bands:u8,pub gains3:[f32;3],pub gains8:[f32;8],pub speed:f32,pub pitch:f32,pub linked:bool,pub reverb:f32,pub reverb_depth:f32}
impl Default for Listening {fn default()->Self{Self{bands:3,gains3:[0.;3],gains8:[0.;8],speed:1.,pitch:0.,linked:true,reverb:0.,reverb_depth:0.5}}}
impl Listening {
 pub fn normalize(mut self)->Self{self.reverb_depth=if self.reverb_depth.is_finite(){self.reverb_depth.clamp(0.,1.)}else{0.5};self.reverb=if self.reverb.is_finite(){self.reverb.clamp(0.,1.)}else{0.};self.bands=if self.bands==8{8}else{3};for gain in self.gains3.iter_mut().chain(self.gains8.iter_mut()){*gain=if gain.is_finite(){gain.clamp(-12.,12.)}else{0.};}self.speed=if self.speed.is_finite(){self.speed.clamp(0.5,2.)}else{1.};self.pitch=if self.linked{12.*self.speed.log2()}else if self.pitch.is_finite(){self.pitch.clamp(-12.,12.)}else{0.};self}
 pub fn transpose(&self)->f32{2_f32.powf(self.pitch/12.)}
 pub fn coefficients(&self)->Equalizer{let mut result=Equalizer::default();result.reverb=self.reverb;result.reverb_depth=self.reverb_depth;let (freq,gains):(&[f32],&[f32])=if self.bands==8{(&[63.,125.,250.,500.,1000.,2000.,4000.,8000.],&self.gains8)}else{(&[125.,1000.,8000.],&self.gains3)};let mut boost=0_f32;for(i,(&hz,&gain))in freq.iter().zip(gains).enumerate(){result.coefficients[i]=peak(hz,gain,if self.bands==3{0.6}else{1.});boost=boost.max(gain);}result.headroom=10_f32.powf(-boost/20.);result}
}
#[derive(Debug,Clone,Copy)]pub struct Equalizer{pub coefficients:[[f32;5];8],pub headroom:f32,pub reverb:f32,pub reverb_depth:f32}
impl Default for Equalizer{fn default()->Self{Self{coefficients:[[1.,0.,0.,0.,0.];8],headroom:1.,reverb:0.,reverb_depth:0.5}}}
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

/// Fixed all-pass delays decorrelate only the wet mid into a restrained side signal.
/// Equal-and-opposite side contributions preserve the original mono sum.
struct StereoSpread {a:[f32;211],b:[f32;337],at:[usize;2]}
impl Default for StereoSpread {fn default()->Self{Self{a:[0.;211],b:[0.;337],at:[0;2]}}}
impl StereoSpread {
 fn clear(&mut self){self.a.fill(0.);self.b.fill(0.);self.at=[0;2];}
 fn process(&mut self,wet:[f32;2])->[f32;2]{
  let mid=(wet[0]+wet[1])*0.5;
  let a=self.a[self.at[0]]-mid*0.5;self.a[self.at[0]]=mid+a*0.5;
  let b=self.b[self.at[1]]-mid*0.5;self.b[self.at[1]]=mid+b*0.5;
  self.at[0]=(self.at[0]+1)%self.a.len();self.at[1]=(self.at[1]+1)%self.b.len();
  let spread=(a-b)*0.12;[wet[0]+spread,wet[1]-spread]
 }
}

/// Unity-energy all-pass diffusion, allocated off the callback.
struct Diffusion {lines:Vec<Vec<[f32;2]>>,at:[usize;4]}
impl Default for Diffusion{fn default()->Self{Self{lines:[241,379,631,887].into_iter().map(|n|vec![[0.;2];n]).collect(),at:[0;4]}}}
impl Diffusion{
 fn clear(&mut self){for line in &mut self.lines{line.fill([0.;2]);}self.at=[0;4];}
 fn process(&mut self,mut input:[f32;2])->[f32;2]{for i in 0..4{for channel in 0..2{let output=self.lines[i][self.at[i]][channel]-input[channel]*0.55;self.lines[i][self.at[i]][channel]=input[channel]+output*0.55;input[channel]=output;}self.at[i]=(self.at[i]+1)%self.lines[i].len();}input}
}
/// A damped feedback delay network, allocated before output starts. Orthogonal
/// feedback scatters each reflection into every delay rather than repeating four
/// isolated combs. This builds a room tail underneath sustained music.
/// Wet amount is smoothed per sample; zero is an exact dry bypass.
pub struct Reverb {lines:Vec<Vec<[f32;2]>>,at:[usize;4],damp:[[f32;2];4],amount:f32,target:f32,tail_level:f32,feedback:f32,target_feedback:f32,spread:StereoSpread,diffusion:Diffusion,low_cut:[f32;2]}
impl Default for Reverb {fn default()->Self {Self{lines:[1423,1789,2137,2557].into_iter().map(|n|vec![[0.;2];n]).collect(),at:[0;4],damp:[[0.;2];4],amount:0.,target:0.,tail_level:0.,feedback:0.85,target_feedback:0.85,spread:StereoSpread::default(),diffusion:Diffusion::default(),low_cut:[0.;2]}}}
impl Reverb {
 pub fn clear(&mut self){for line in &mut self.lines{line.fill([0.;2]);}self.damp=[[0.;2];4];self.tail_level=0.;self.spread.clear();self.diffusion.clear();self.low_cut=[0.;2];}
 pub fn active(&self)->bool{self.amount>0.&&self.tail_level>0.0001}
 pub fn set(&mut self,amount:f32){self.target=if amount.is_finite(){amount.clamp(0.,1.)}else{0.};}
 /// Depth changes decay without resizing buffers. The feedback matrix has unit
 /// norm, so damping and feedback below one make the network strictly lossy.
 pub fn set_depth(&mut self,depth:f32){let d=if depth.is_finite(){depth.clamp(0.,1.)}else{0.5};self.target_feedback=if d<=0.5{0.55+d*0.6}else{0.85+(d-0.5)*0.18};}
 pub fn process(&mut self,input:[f32;2])->[f32;2]{
  let input=input.map(|v|if v.is_finite(){v}else{0.});
  self.feedback+=(self.target_feedback-self.feedback)*0.0005;
  self.amount+=(self.target-self.amount)*0.0015;
  if self.target==0.&&self.amount<0.00001 {self.amount=0.;}
  // Normalize wet energy rather than DC gain: the excitation is high-passed.
  // Linear (1-feedback) compensation made deep rooms effectively disappear.
  let excitation=0.4*((1.-self.feedback*self.feedback)/(1.-0.85_f32.powi(2))).sqrt().min(1.);
  // Remove DC and bass accumulation from the wet excitation only. This avoids
  // low-frequency comb reinforcement without attenuating the dry recording.
  let mut wet_input=input;for c in 0..2{self.low_cut[c]+=(input[c]-self.low_cut[c])*0.015586;wet_input[c]-=self.low_cut[c];}
  let diffused=self.diffusion.process(wet_input);
  // Read and damp all lines before writing feedback. A normalized Hadamard
  // matrix conserves feedback energy while multiplying reflection density.
  let delayed: [[f32;2];4]=std::array::from_fn(|i|self.lines[i][self.at[i]]);
  for i in 0..4 {for c in 0..2 {self.damp[i][c]+=(delayed[i][c]-self.damp[i][c])*0.45;}}
  let mut wet=[0.;2];
  for c in 0..2 {
   let [a,b,d,e]=self.damp.map(|line|line[c]);
   let scattered=[a+b+d+e,a-b+d-e,a+b-d-e,a-b-d+e].map(|v|v*0.5);
   // Alternating injection polarity avoids reinforcing the common DC mode.
   let polarity=[1.,-1.,1.,-1.];
   for i in 0..4 {self.lines[i][self.at[i]][c]=diffused[c]*excitation*polarity[i]+scattered[i]*self.feedback;wet[c]+=delayed[i][c]*polarity[i]*0.5;}
  }
  for i in 0..4 {self.at[i]=(self.at[i]+1)%self.lines[i].len();}
  let wet=self.spread.process(wet);
  self.tail_level=(self.tail_level*0.9995).max(input[0].abs()).max(input[1].abs()).max(wet[0].abs()).max(wet[1].abs());
  if self.amount==0. {return input;}
  // An ambience send: retain the recording at unity and add the room.
  // The previous crossfade/normalizer reduced the dry source by 3 dB at
  // the default amount and 7.5 dB at maximum, masking the tail as a volume cut.
  [input[0]+wet[0]*self.amount,input[1]+wet[1]*self.amount]
 }
}
#[cfg(test)]mod reverb_tests {use super::*;
 #[test]fn enabling_room_preserves_the_direct_recording(){
  for amount in [0.25,0.45,1.] {let mut r=Reverb::default();r.set(amount);for _ in 0..48000{r.process([0.;2]);}for i in 0..1000{let input=[(i as f32*0.071).sin()*0.25,(i as f32*0.11).sin()*0.2];assert_eq!(r.process(input),input,"The direct arrival must not be ducked at {amount}");}}
 }

 #[test]fn deep_room_retains_broadband_energy_without_limiter() {
  fn energy(depth:f32)->f32 {let mut room=Reverb::default();room.set(0.45);room.set_depth(depth);for _ in 0..48000{room.process([0.;2]);}let mut seed=7_u32;let mut energy=0.;for i in 0..240000 {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);let x=(seed as f32/u32::MAX as f32-0.5)*0.4;let y=room.process([x,x]);assert!(y.iter().all(|v|v.is_finite()&&v.abs()<0.9));if i>48000{energy+=(y[0]-x).powi(2)+(y[1]-x).powi(2);}}energy}
  let ordinary=energy(0.5);let deep=energy(1.);assert!(deep>ordinary*0.2,"Depth must extend the room without erasing it: {ordinary} -> {deep}");assert!(deep<ordinary*2.,"Deep room gain must stay bounded: {ordinary} -> {deep}");
 }

 #[test]fn sustained_and_rapid_parameters_stay_stable_before_limiting(){
  for depth in [0.,0.5,1.] {let mut r=Reverb::default();r.set(1.);r.set_depth(depth);for _ in 0..48000{r.process([0.;2]);}for _ in 0..192000 {let y=r.process([0.75;2]);assert!(y.iter().all(|v|v.is_finite()&&v.abs()<0.9),"Runaway wet gain: {depth} {y:?}");}}
  let mut r=Reverb::default();for i in 0..192000 {if i%113==0{r.set(if i%2==0{1.}else{0.});r.set_depth((i%100)as f32/100.);}let y=r.process([(i as f32*0.071).sin()*0.35;2]);assert!(y.iter().all(|v|v.is_finite()&&v.abs()<1.));}r.set(f32::NAN);r.set_depth(f32::INFINITY);for _ in 0..1000{assert!(r.process([f32::NAN,0.]).iter().all(|v|v.is_finite()));}
 }

 #[test]fn dry_is_exact_and_tail_decays(){let mut r=Reverb::default();assert_eq!(r.process([0.4,-0.3]),[0.4,-0.3]);r.set(1.);for _ in 0..10000{r.process([0.;2]);}r.process([1.,0.]);let mut tail=0_f32;for _ in 0..20000{let v=r.process([0.;2]);assert!(v.iter().all(|x|x.is_finite()&&x.abs()<1.));tail=tail.max(v[0].abs()+v[1].abs());}assert!(tail>0.001);for _ in 0..480000 {r.process([0.;2]);}assert!(r.process([0.;2]).iter().all(|x|x.abs()<0.00001));r.set(0.);for _ in 0..10000 {r.process([0.;2]);}assert_eq!(r.process([0.4,-0.3]),[0.4,-0.3]);}
 #[test]fn modest_amount_has_audible_tail_energy(){let mut r=Reverb::default();r.set(0.25);for _ in 0..10000{r.process([0.;2]);}r.process([1.,1.]);let mut energy=0.;for _ in 0..24000{let y=r.process([0.;2]);energy+=y[0]*y[0]+y[1]*y[1];}assert!(energy>0.002,"Wet signal is too weak: {energy}");}
 #[test]fn default_room_has_dense_late_reflections(){
  let mut room=Reverb::default();room.set(0.45);for _ in 0..48000{room.process([0.;2]);}
  room.process([0.5;2]);let mut occupied=0;let mut late_energy=0.;
  for frame in 1..48000 {let wet=room.process([0.;2]);if frame>=9600 {late_energy+=wet.iter().map(|s|s*s).sum::<f32>();if wet.iter().any(|s|s.abs()>0.00001){occupied+=1;}}}
  assert!(occupied as f32/38400.>0.85,"Room tail remains sparse: {occupied}");
  assert!(late_energy>0.0005,"Default room dies before a perceptible late tail: {late_energy}");
 }
 #[test]fn depth_extends_tail_without_changing_dry_bypass(){
  fn late_energy(depth:f32)->f32 {let mut r=Reverb::default();r.set(0.6);r.set_depth(depth);for _ in 0..20000{r.process([0.;2]);}r.process([0.5,0.5]);let mut energy=0.;for i in 0..96000{let y=r.process([0.;2]);assert!(y.iter().all(|v|v.is_finite()&&v.abs()<1.));if i>24000{energy+=y[0]*y[0]+y[1]*y[1];}}energy}
  let shallow=late_energy(0.);let deep=late_energy(1.);assert!(deep>shallow*10.,"Depth must change rendered decay: {shallow} -> {deep}");
  let mut r=Reverb::default();for i in 0..48000 {if i%97==0{r.set_depth(if i%2==0{1.}else{0.});}assert_eq!(r.process([0.2,-0.3]),[0.2,-0.3]);}
  assert_eq!(Listening{reverb_depth:f32::NAN,..Default::default()}.normalize().reverb_depth,0.5);
  assert_eq!(Listening{reverb_depth:2.,..Default::default()}.normalize().reverb_depth,1.);
 }
 #[test]fn wet_spread_preserves_mono_and_stays_subtle(){
  let mut spread=StereoSpread::default();let mut mid_energy=0.;let mut side_energy=0.;
  for i in 0..96000 {let mid=(i as f32*0.071).sin()*0.1+(i as f32*0.113).sin()*0.1;let dry_side=(i as f32*0.019).sin()*0.02;let wet=[mid+dry_side,mid-dry_side];let y=spread.process(wet);assert!(((y[0]+y[1])*0.5-mid).abs()<0.000001);let added=(y[0]-y[1])*0.5-dry_side;mid_energy+=mid*mid;side_energy+=added*added;assert!(y.iter().all(|v|v.is_finite()&&v.abs()<0.4));}
  assert!(side_energy>mid_energy*0.001);assert!(side_energy<mid_energy*0.06,"Spread is too strong: {}",side_energy/mid_energy);
  spread.clear();for _ in 0..1000 {assert_eq!(spread.process([0.;2]),[0.;2]);}
 }
 #[test]fn mono_music_gets_stereo_tail_and_seek_clears_spread(){
  let mut r=Reverb::default();r.set(0.45);for _ in 0..20000{r.process([0.;2]);}r.process([0.5,0.5]);let mut side_energy=0.;for _ in 0..48000{let y=r.process([0.;2]);side_energy+=(y[0]-y[1]).powi(2);}assert!(side_energy>0.00001,"Mono source still has a mono reverb tail");r.clear();for _ in 0..10000{assert_eq!(r.process([0.;2]),[0.;2]);}
 }
 #[test]fn old_preferences_default_to_dry(){let p:Listening=serde_json::from_str(r#"{"speed":1.2}"#).unwrap();assert_eq!(p.reverb,0.);assert_eq!(p.reverb_depth,0.5);assert_eq!(Listening{reverb:f32::NAN,..Default::default()}.normalize().reverb,0.);}
}
