//! Versioned, deterministic abstract artwork parameters derived from cached audio.
use crate::spark_engine::Candidate;
use serde::{Deserialize,Serialize};
use std::collections::BTreeMap;
use base64::Engine;
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]pub struct Art{pub version:u8,pub hue:f32,pub energy:f32,pub shape:u8,pub seed:u32,pub variant:u32,pub tempo:Option<f32>}
impl Art{
 pub fn choice(&self)->String{format!("vibe/{}",base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).unwrap()))}
 pub fn parse(choice:&str)->anyhow::Result<Self>{anyhow::ensure!(choice.len()<1024,"Artwork choice is too large");let bytes=base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(choice.strip_prefix("vibe/").ok_or_else(||anyhow::anyhow!("Invalid generated cover"))?)?;let value:Self=serde_json::from_slice(&bytes)?;anyhow::ensure!(matches!(value.version,1|2)&&value.hue.is_finite()&&(0. ..360.).contains(&value.hue)&&value.energy.is_finite()&&(0. ..=1.).contains(&value.energy)&&value.shape<=2&&value.tempo.is_none_or(|t|t.is_finite()&&(20. ..=300.).contains(&t)),"Invalid generated cover parameters");Ok(value)}
}
pub fn generate(candidates:&[Candidate],ids:&[i64],moods:&BTreeMap<String,Vec<f32>>,variant:u32,energy:Option<f32>,tempo:Option<f32>)->Art{
 let mut embedding=[0_f32;512];let mut seed=2166136261_u32;for c in candidates.iter().filter(|c|ids.contains(&c.id)){for(a,b)in embedding.iter_mut().zip(&c.embedding){*a+=b;}for b in c.id.to_le_bytes(){seed=(seed^b as u32).wrapping_mul(16777619);}}
 let norm=embedding.iter().map(|v|v*v).sum::<f32>().sqrt().max(1e-8);for v in &mut embedding{*v/=norm;}
 let winner=moods.iter().max_by(|a,b|{let dot=|m:&Vec<f32>|embedding.iter().zip(m).map(|(a,b)|a*b).sum::<f32>();dot(a.1).total_cmp(&dot(b.1))}).map(|(name,_)|name.as_str());
 let hue=match winner{Some("Energetic")=>15.,Some("Chilled")=>195.,Some("Melancholic")=>245.,Some("Bright")=>55.,Some("Focused")=>155.,_=>285.};
 let energy=energy.unwrap_or(if winner==Some("Energetic"){0.8}else if winner==Some("Chilled"){0.3}else{0.5}).clamp(0.,1.);
 let detail=embedding.iter().enumerate().map(|(i,v)|v*(i as f32*0.37).sin()).sum::<f32>();
 Art{version:2,hue:(hue+detail*12.+variant as f32*17.).rem_euclid(360.),energy,shape:if energy>0.65{2}else if energy<0.4{0}else{1},seed,variant,tempo}
}
#[cfg(test)]mod tests{use super::*;#[test]fn deterministic_audio_artwork_and_validated_roundtrip(){let mut v=vec![0.;512];v[1]=1.;let c=Candidate{id:1,artist:"A".into(),album:"B".into(),favorite:false,embedding:v};let a=generate(&[c.clone()],&[1],&BTreeMap::new(),0,Some(0.8),Some(120.));assert_eq!(a,Art::parse(&a.choice()).unwrap());assert_eq!(a,generate(&[c.clone()],&[1],&BTreeMap::new(),0,Some(0.8),Some(120.)));assert_ne!(a,generate(&[c],&[1],&BTreeMap::new(),1,Some(0.8),Some(120.)));assert!(Art::parse("vibe/invalid").is_err());}}
