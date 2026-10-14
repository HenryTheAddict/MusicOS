//! Bounded three-band onset whitening: sustained treble must not drown out bass attacks.
#[derive(Default)]pub struct Multiband{mean:[f32;3]}
impl Multiband{pub fn combine(&mut self,flux:[f32;3])->f32{let mut result=0.;for(i,x)in flux.into_iter().enumerate(){let x=if x.is_finite(){x.max(0.)}else{0.};self.mean[i]=self.mean[i]*0.98+x*0.02;if self.mean[i]>0.00001{result+=(x/(self.mean[i]+0.00001)).min(4.);}}result/3.}}
#[cfg(test)]mod tests{use super::*;#[test]fn silent_or_nonfinite_input_does_not_invent_onsets(){let mut m=Multiband::default();assert_eq!(m.combine([0.;3]),0.);assert_eq!(m.combine([f32::NAN;3]),0.);}
#[test]fn weak_bass_attack_survives_sustained_loud_treble(){let mut m=Multiband::default();for _ in 0..200{m.combine([0.001,0.01,10.]);}let quiet=m.combine([0.001,0.01,10.]);let attack=m.combine([0.02,0.01,10.]);assert!(attack>quiet*1.5);}}
