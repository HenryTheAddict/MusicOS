//! Local face-aware portrait framing. Original photographs are never modified.
use anyhow::Result;
use std::path::{Path,PathBuf};
#[derive(Clone,Copy,Debug)]
struct Face {x:f64,y:f64,w:f64,h:f64,confidence:f64}
fn framing(width:u32,height:u32,faces:&[Face])->Option<(u32,u32,u32)> {
 let face=faces.iter().filter(|f|f.confidence>=0.65&&f.w>0.&&f.h>0.).max_by(|a,b|(a.w*a.h*a.confidence).total_cmp(&(b.w*b.h*b.confidence)))?;
 // Keep similarly prominent faces together rather than picking an arbitrary person.
 let peers:Vec<_>=faces.iter().filter(|f|f.confidence>=0.65&&f.w*f.h>=face.w*face.h*0.6).collect();
 let (left,top,right,bottom)=peers.iter().fold((1f64,1f64,0f64,0f64),|(l,t,r,b),f|(l.min(f.x),t.min(f.y),r.max(f.x+f.w),b.max(f.y+f.h)));
 let fw=(right-left)*width as f64;let fh=(bottom-top)*height as f64;
 let side=(fw*3.2).max(fh*3.0).min(width.min(height) as f64).max(1.).round() as u32;
 let cx=(left+right)*0.5*width as f64;let cy=(top+bottom)*0.5*height as f64;
 let x=(cx-side as f64*0.5).clamp(0.,(width-side) as f64).round() as u32;
 let y=(cy-side as f64*0.48).clamp(0.,(height-side) as f64).round() as u32;
 if x as f64>left*width as f64||y as f64>top*height as f64||((x+side) as f64)<right*width as f64||((y+side) as f64)<bottom*height as f64{return None;}
 Some((x,y,side))
}
#[cfg(target_os="macos")]
fn detect(image:&image::DynamicImage)->Result<Vec<Face>> {
 use objc2::{AnyThread,rc::autoreleasepool};
 use objc2_foundation::{NSData,NSDictionary,NSArray};
 use objc2_vision::{VNDetectFaceLandmarksRequest,VNImageRequestHandler,VNRequest};
 let mut bytes=std::io::Cursor::new(Vec::new());image.thumbnail(2048,2048).write_to(&mut bytes,image::ImageOutputFormat::Png)?;
 autoreleasepool(|_|unsafe {
  let request=VNDetectFaceLandmarksRequest::new();
  let data=NSData::with_bytes(bytes.get_ref());
  let handler=VNImageRequestHandler::initWithData_options(VNImageRequestHandler::alloc(),&data,&NSDictionary::new());
  let requests=NSArray::from_slice(&[&*request as &VNRequest]);
  handler.performRequests_error(&requests).map_err(|e|anyhow::anyhow!("Portrait detection failed: {e}"))?;
  Ok(request.results().map(|faces|faces.iter().map(|face|{let r=face.boundingBox();Face{x:r.origin.x,y:1.-r.origin.y-r.size.height,w:r.size.width,h:r.size.height,confidence:face.confidence() as f64}}).collect()).unwrap_or_default())
 })
}
#[cfg(not(target_os="macos"))]
fn detect(_image:&image::DynamicImage)->Result<Vec<Face>> {Ok(vec![])}
pub fn crop(source:&Path)->Result<PathBuf> {
 use sha2::{Digest,Sha256};use std::sync::{Mutex,OnceLock};
 static WORKER:OnceLock<Mutex<()>>=OnceLock::new();
 let _worker=WORKER.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());
 let key=format!("portrait-v4-sdr-landmarks-headroom:{}",crate::depth::source_identity(source)?);
 let cache=musicos_desktop::storage::desktop_data_dir().join("portrait-framing-v3");std::fs::create_dir_all(&cache)?;
 let id=format!("{:x}",Sha256::digest(key.as_bytes()));let output=cache.join(format!("{id}.png"));let checked=cache.join(format!("{id}.json"));
 if output.is_file(){if image::open(&output).is_ok(){return Ok(output);}let _=std::fs::remove_file(&output);}
 if checked.is_file(){return Ok(source.to_owned());}
 #[cfg(not(target_os="macos"))]
 let image={let mut reader=image::io::Reader::open(source)?.with_guessed_format()?;let mut limits=image::io::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(128*1024*1024);reader.limits(limits);reader.decode()?};
 #[cfg(target_os="macos")]
 let image={let(w,h)=image::image_dimensions(source)?;anyhow::ensure!(w<=8192&&h<=8192,"Portrait exceeds the decoding limit");
 let oriented=cache.join(format!("{id}.oriented.png"));crate::artwork_renditions::display_sdr_png(source,&oriented,2048)?;
 let result=image::open(&oriented);let _=std::fs::remove_file(&oriented);result?};
 let faces=detect(&image)?;
 if let Some((x,y,side))=framing(image.width(),image.height(),&faces){
  let temp=cache.join(format!("{id}.tmp.png"));image.crop_imm(x,y,side,side).thumbnail(640,640).save(&temp)?;std::fs::rename(temp,&output)?;return Ok(output);
 }
 musicos_desktop::storage::atomic_write_json(&checked,&serde_json::json!({"face_detected":false}))?;
 Ok(source.to_owned())
}
#[cfg(test)]mod tests{
 use super::*;
 #[cfg(target_os="macos")]
 #[test]fn orientation_is_applied_before_framing(){
  let dir=tempfile::tempdir().unwrap();let source=dir.path().join("rotated.jpg");let output=dir.path().join("upright.png");
  let image=image::RgbImage::new(80,120);let mut encoded=Vec::new();image::codecs::jpeg::JpegEncoder::new(&mut encoded).encode_image(&image).unwrap();
  // TIFF little endian, one EXIF orientation=6 (90 degrees clockwise).
  let exif=b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
  let mut jpeg=vec![0xff,0xd8,0xff,0xe1];jpeg.extend_from_slice(&((exif.len()+2)as u16).to_be_bytes());jpeg.extend_from_slice(exif);jpeg.extend_from_slice(&encoded[2..]);std::fs::write(&source,jpeg).unwrap();
  crate::artwork_renditions::display_sdr_png(&source,&output,2048).unwrap();assert_eq!(image::image_dimensions(output).unwrap(),(120,80));
 }
 #[test]fn uncertain_or_missing_faces_keep_original(){assert!(framing(400,800,&[]).is_none());assert!(framing(400,800,&[Face{x:0.1,y:0.1,w:0.2,h:0.1,confidence:0.3}]).is_none());}
 #[test]fn framing_centers_face_and_stays_inside_image(){let face=Face{x:0.6,y:0.2,w:0.15,h:0.12,confidence:0.95};let(x,y,s)=framing(800,1200,&[face]).unwrap();assert!(x+s<=800&&y+s<=1200);assert!(((face.x+face.w/2.)*800.-(x as f64+s as f64/2.)).abs()<1.);assert!(((face.y+face.h/2.)*1200.-(y as f64+s as f64*0.48)).abs()<1.);}
 #[test]fn comparable_faces_remain_inside_crop(){let faces=[Face{x:0.1,y:0.3,w:0.1,h:0.15,confidence:0.9},Face{x:0.65,y:0.3,w:0.1,h:0.15,confidence:0.9}];let(x,y,s)=framing(1000,1000,&faces).unwrap();assert!(x<=100&&x+s>=750&&y<=300&&y+s>=450);}
}
