use ort::{session::Session,value::Tensor};
fn main()->Result<(),Box<dyn std::error::Error>>{
 let args=std::env::args().collect::<Vec<_>>();let mut session=Session::builder()?.with_intra_threads(2)?.commit_from_file(&args[1])?;
 let bytes=std::fs::read(&args[2])?;let samples=bytes.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
 let expected=std::fs::read(&args[3])?.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
 for _ in 0..3 {let input=Tensor::from_array(([1,1,1001,64],samples.clone().into_boxed_slice()))?;let started=std::time::Instant::now();let output=session.run(ort::inputs![input])?;let (_,embedding)=output[0].try_extract_tensor::<f32>()?;let error=embedding.iter().zip(&expected).map(|(a,b)|(a-b).abs()).fold(0_f32,f32::max);let dot=embedding.iter().zip(&expected).map(|(a,b)|(*a as f64)*(*b as f64)).sum::<f64>();let norm=|v:&[f32]|v.iter().map(|x|(*x as f64).powi(2)).sum::<f64>().sqrt();let cosine=dot/(norm(embedding)*norm(&expected));println!("seconds={:.6} dimensions={} max_abs_error={:.9} cosine={:.9}",started.elapsed().as_secs_f32(),embedding.len(),error,cosine);assert!(embedding.len()==512 && error<0.001 && cosine>0.99999,"Embedding parity failed");}
 Ok(())
}
