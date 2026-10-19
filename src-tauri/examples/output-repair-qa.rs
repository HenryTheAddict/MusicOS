//! Capture the production output mixer with real decoded audio. Never edits music.
use anyhow::{Context,Result,ensure};
use musicos_desktop::local::{decoded_audio,dj_audio,listening::{Listening,Reverb}};
use rodio::{Source,source::UniformSourceIterator};
use std::{path::Path,time::Duration};
fn wav(path:&Path,frames:&[[f32;2]])->Result<()> {
 let bytes=(frames.len()*8) as u32;let mut data=Vec::with_capacity(bytes as usize+44);data.extend(b"RIFF");data.extend((bytes+36).to_le_bytes());data.extend(b"WAVEfmt ");data.extend(16u32.to_le_bytes());data.extend(3u16.to_le_bytes());data.extend(2u16.to_le_bytes());data.extend(48000u32.to_le_bytes());data.extend(384000u32.to_le_bytes());data.extend(8u16.to_le_bytes());data.extend(32u16.to_le_bytes());data.extend(b"data");data.extend(bytes.to_le_bytes());for sample in frames.iter().flatten(){data.extend(sample.to_le_bytes());}std::fs::write(path,data)?;Ok(())
}
fn main()->Result<()> {
 let args=std::env::args().collect::<Vec<_>>();let path=args.get(1).context("Pass a source audio file and output directory")?;let out=Path::new(args.get(2).context("Pass output directory")?);std::fs::create_dir_all(out)?;
 let mut source=decoded_audio::open(Path::new(path))?;let _=source.try_seek(Duration::from_secs(20));let samples=UniformSourceIterator::new(source.convert_samples::<f32>(),2,48000).take(48000*8*2).collect::<Vec<_>>();let input=samples.chunks_exact(2).map(|p|[p[0],p[1]]).collect::<Vec<_>>();ensure!(!input.is_empty(),"Source produced no audio");
 let depth=args.get(3).map(|v|v.parse::<f32>()).transpose()?.unwrap_or(0.5);ensure!((0.0..=1.0).contains(&depth),"Depth must be 0–1");
 let dry=dj_audio::capture(&input,Listening::default(),96000);let wet=dj_audio::capture(&input,Listening{reverb:0.45,reverb_depth:depth,..Default::default()},96000);let strong=dj_audio::capture(&input,Listening{reverb:1.,reverb_depth:depth,..Default::default()},96000);
 let difference=(wet.iter().flatten().zip(dry.iter().flatten()).map(|(a,b)|(a-b).powi(2)).sum::<f32>()/(wet.len()*2) as f32).sqrt();let tail=wet[input.len()..].iter().flatten().map(|x|x*x).sum::<f32>();ensure!(difference>0.0001&&tail>0.00001,"Output remains dry");ensure!(wet.iter().chain(&strong).flatten().all(|x|x.is_finite()&&x.abs()<=1.),"Unbounded output");
 let mut room=Reverb::default();room.set(1.);room.set_depth(depth);let mut prelimit_peak=0_f32;let mut over_full_scale=0_usize;for frame in input.iter().copied().chain(std::iter::repeat_n([0.;2],96000)){for value in room.process(frame){ensure!(value.is_finite(),"Non-finite room before limiting");prelimit_peak=prelimit_peak.max(value.abs());if value.abs()>1.{over_full_scale+=1;}}}
 wav(&out.join("dry.wav"),&dry)?;wav(&out.join("reverb-45.wav"),&wet)?;wav(&out.join("reverb-100.wav"),&strong)?;let report=serde_json::json!({"production_mixer":true,"depth":depth,"source_frames":input.len(),"wet_difference_rms":difference,"wet_tail_energy":tail,"dry_pcm_exact":dry[..input.len()]==input,"input_peak":input.iter().flatten().map(|x|x.abs()).fold(0_f32,f32::max),"dry_includes_existing_safety_limiter":true,"clipping_bounded":true,"prelimit_peak_at_max_wet":prelimit_peak,"prelimit_samples_over_full_scale":over_full_scale,"audible_acceptance":false});std::fs::write(out.join("report.json"),serde_json::to_vec_pretty(&report)?)?;println!("{report}");Ok(())
}
