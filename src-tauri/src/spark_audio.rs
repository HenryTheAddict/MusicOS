//! Deterministic CLAP preprocessing. Matches the pinned music checkpoint's
//! non-fusion Slaney mel frontend; no Python is used by the desktop runtime.
use rustfft::{FftPlanner,num_complex::Complex};
pub const SAMPLE_RATE:u32=48000;
pub const SAMPLES:usize=480000;
pub const FRAMES:usize=1001;
pub const BINS:usize=64;
pub const VERSION:&str="clap-music-slaney-48k-1024-480-repeatpad-v1";
fn hz_to_mel(hz:f64)->f64 {if hz<1000. {hz/(200./3.)}else{15.+(hz/1000.).ln()/(6.4_f64.ln()/27.)}}
fn mel_to_hz(mel:f64)->f64 {if mel<15. {mel*(200./3.)}else{1000.*((mel-15.)*(6.4_f64.ln()/27.)).exp()}}
pub fn mel(audio:&[f32])->Vec<f32>{
 let mut waveform=vec![0_f64;SAMPLES];
 if !audio.is_empty(){let len=audio.len().min(SAMPLES);let repeats=SAMPLES/len;for n in 0..repeats {for i in 0..len {waveform[n*len+i]=audio[i] as f64;}}}
 let low=hz_to_mel(50.);let high=hz_to_mel(14000.);
 let frequencies=(0..BINS+2).map(|i|mel_to_hz(low+(high-low)*i as f64/(BINS+1) as f64)).collect::<Vec<_>>();
 let filters=(0..BINS).map(|m|(0..513).map(|k|{let hz=k as f64*SAMPLE_RATE as f64/1024.;let lower=(hz-frequencies[m])/(frequencies[m+1]-frequencies[m]);let upper=(frequencies[m+2]-hz)/(frequencies[m+2]-frequencies[m+1]);lower.min(upper).max(0.)*2./(frequencies[m+2]-frequencies[m])}).collect::<Vec<_>>()).collect::<Vec<_>>();
 let fft=FftPlanner::<f64>::new().plan_fft_forward(1024);let window=(0..1024).map(|i|0.5-0.5*(std::f64::consts::TAU*i as f64/1024.).cos()).collect::<Vec<_>>();
 let mut buffer=vec![Complex::new(0.,0.);1024];let mut power=vec![0.;513];let mut output=vec![0_f32;FRAMES*BINS];
 for frame in 0..FRAMES {for i in 0..1024 {let raw=frame as isize*480+i as isize-512;let index=if raw<0{-raw as usize}else if raw>=SAMPLES as isize{(2*SAMPLES-2) as isize as usize-raw as usize}else{raw as usize};buffer[i]=Complex::new(waveform[index]*window[i],0.);}
 fft.process(&mut buffer);for k in 0..513 {power[k]=buffer[k].norm_sqr();}
 for m in 0..BINS {let energy=power.iter().zip(&filters[m]).map(|(p,w)|p*w).sum::<f64>().max(1e-10);output[frame*BINS+m]=(10.*energy.log10()) as f32;}}
 output
}
#[cfg(test)]mod tests {use super::*;
 #[test]fn silence_has_finite_floor(){let out=mel(&[]);assert_eq!(out.len(),FRAMES*BINS);assert!(out.iter().all(|v|*v == -100.));}
 #[test]fn preprocessing_matches_checkpoint_reference(){let audio=(0..SAMPLES).map(|i|((i as f64*std::f64::consts::TAU*440./48000.).sin()*0.2) as f32).collect::<Vec<_>>();let expected=include_bytes!("../tests/fixtures/clap-sine-mel.bin").chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();let actual=mel(&audio);assert_eq!(actual.len(),expected.len());let error=actual.iter().zip(expected).map(|(a,b)|(a-b).abs()).fold(0_f32,f32::max);assert!(error<0.02,"mel error {error}");}
}
