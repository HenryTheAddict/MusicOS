//! Desktop decoding shared by playback and analysis. No output device is opened here.
use anyhow::{Context, Result};
use rodio::{Source, source::SeekError};
use std::{path::Path, time::Duration};
pub enum DecodedAudio {
    Rodio(rodio::source::SamplesConverter<rodio::Decoder<std::io::BufReader<std::fs::File>>, f32>),
    #[cfg(target_os="macos")]
    Apple(apple::Audio),
}
pub fn open(path: &Path) -> Result<DecodedAudio> {
    #[cfg(target_os="macos")]
    if path.extension().is_some_and(|v|v.eq_ignore_ascii_case("m4a")) {
        return apple::Audio::open(path).map(DecodedAudio::Apple);
    }
    Ok(DecodedAudio::Rodio(rodio::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?))
        .context("This audio file is corrupt or uses an unsupported codec")?.convert_samples()))
}
macro_rules! dispatch {($self:ident,$method:ident $(,$arg:expr)*)=>{match $self {Self::Rodio(s)=>s.$method($($arg),*),#[cfg(target_os="macos")]Self::Apple(s)=>s.$method($($arg),*)}};}
impl Iterator for DecodedAudio {type Item=f32;fn next(&mut self)->Option<f32>{dispatch!(self,next)}}
impl Source for DecodedAudio {
    fn current_frame_len(&self)->Option<usize>{dispatch!(self,current_frame_len)}
    fn channels(&self)->u16{dispatch!(self,channels)}
    fn sample_rate(&self)->u32{dispatch!(self,sample_rate)}
    fn total_duration(&self)->Option<Duration>{dispatch!(self,total_duration)}
    fn try_seek(&mut self,pos:Duration)->std::result::Result<(),SeekError>{dispatch!(self,try_seek,pos)}
}
#[cfg(target_os="macos")]
pub mod apple {
    use super::*;
    use std::{ffi::{CString,CStr,c_void,c_char},ptr::NonNull};
    extern "C" {
        fn mos_webview_native_refresh(pointer:*mut c_void)->i32;
        fn mos_audio_open(path:*const c_char,rate:*mut f64,duration:*mut f64,error:*mut c_char,size:usize)->*mut c_void;
        fn mos_audio_read(handle:*mut c_void,samples:*mut f32,capacity:usize,error:*mut c_char,size:usize)->i32;
        fn mos_audio_seek(handle:*mut c_void,seconds:f64,error:*mut c_char,size:usize)->i32;
        fn mos_audio_close(handle:*mut c_void);
        fn mos_audio_probe(path:*const c_char,rate:*mut f64,duration:*mut f64,codec:*mut u32,channels:*mut u32,bits:*mut u32,bitrate:*mut f64)->i32;
        fn mos_oriented_png(source:*const c_char,destination:*const c_char)->i32;
        fn mos_artwork_png(source:*const c_char,destination:*const c_char,maximum:u32)->i32;
        fn mos_depth(package:*const c_char,source:*const c_char,destination:*const c_char,error:*mut c_char,size:usize)->i32;
    }
    /// Requires a live WKWebView pointer on the AppKit main thread.
    pub unsafe fn prefer_native_refresh(pointer:*mut c_void)->bool{mos_webview_native_refresh(pointer)==1}
    fn path(value:&Path)->Result<CString>{Ok(CString::new(value.as_os_str().as_encoded_bytes())?)}
    fn failure(buffer:&[c_char])->anyhow::Error{anyhow::anyhow!(unsafe{CStr::from_ptr(buffer.as_ptr())}.to_string_lossy().into_owned())}
    #[derive(Debug,Clone,serde::Serialize)]
    pub struct Probe{pub codec:String,pub sample_rate:u32,pub channels:u16,pub bit_depth:Option<u8>,pub bitrate:Option<u32>,pub lossless:bool,pub duration:f64}
    pub fn probe(source:&Path)->Result<Probe>{let(mut rate,mut duration,mut codec,mut channels,mut bits,mut bitrate)=(0.,0.,0,0,0,0.);
        anyhow::ensure!(unsafe{mos_audio_probe(path(source)?.as_ptr(),&mut rate,&mut duration,&mut codec,&mut channels,&mut bits,&mut bitrate)}==0,"Cannot read this file's audio stream");
        let code=codec.to_be_bytes();let name=match &code{b"alac"=>"ALAC",b"aac "|b"aach"|b"aacp"=>"AAC",b"ec-3"=>"E-AC-3",b"ac-3"=>"AC-3",_=>"Unavailable"};
        Ok(Probe{codec:name.into(),sample_rate:rate as u32,channels:channels as u16,bit_depth:if bits>0{Some(bits as u8)}else{None},bitrate:if bitrate>0.{Some((bitrate/1000.).round() as u32)}else{None},lossless:name=="ALAC",duration})}
    pub struct Audio {handle:NonNull<c_void>,buffer:Vec<f32>,index:usize,len:usize,rate:u32,duration:Option<Duration>,error:Option<String>}
    // Each reader is owned by one decoder worker; it is never accessed concurrently.
    unsafe impl Send for Audio {}
    impl Audio {
        pub fn open(source:&Path)->Result<Self>{let mut error=[0;1024];let(mut rate,mut duration)=(0.,0.);
            let handle=unsafe{mos_audio_open(path(source)?.as_ptr(),&mut rate,&mut duration,error.as_mut_ptr(),error.len())};
            let handle=NonNull::new(handle).ok_or_else(||failure(&error))?;
            let mut audio=Self{handle,buffer:vec![0.;262144],index:0,len:0,rate:rate as u32,duration:if duration.is_finite()&&duration>=0.{Some(Duration::from_secs_f64(duration))}else{None},error:None};
            anyhow::ensure!(audio.next().is_some(),"{}",audio.error.as_deref().unwrap_or("This audio stream is empty or unreadable"));audio.index=0;Ok(audio)}
    }
    impl Drop for Audio{fn drop(&mut self){unsafe{mos_audio_close(self.handle.as_ptr());}}}
    impl Iterator for Audio{type Item=f32;fn next(&mut self)->Option<f32>{
        if self.index==self.len {let mut error=[0;1024];let count=unsafe{mos_audio_read(self.handle.as_ptr(),self.buffer.as_mut_ptr(),self.buffer.len(),error.as_mut_ptr(),error.len())};
            if count<=0{if count<0{self.error=Some(failure(&error).to_string());log::warn!("Desktop M4A decode: {}",self.error.as_deref().unwrap_or_default());}return None;}self.index=0;self.len=count as usize;}
        let value=self.buffer[self.index];self.index+=1;Some(if value.is_finite(){value}else{0.})
    }}
    impl Source for Audio{
        fn current_frame_len(&self)->Option<usize>{None}fn channels(&self)->u16{2}fn sample_rate(&self)->u32{self.rate}fn total_duration(&self)->Option<Duration>{self.duration}
        fn try_seek(&mut self,pos:Duration)->std::result::Result<(),SeekError>{let mut error=[0;1024];if unsafe{mos_audio_seek(self.handle.as_ptr(),pos.as_secs_f64(),error.as_mut_ptr(),error.len())}<0{return Err(SeekError::Other(Box::new(std::io::Error::other(failure(&error).to_string()))));}self.index=0;self.len=0;self.error=None;Ok(())}
    }
    pub fn oriented_png(source:&Path,destination:&Path)->Result<()>{anyhow::ensure!(unsafe{mos_oriented_png(path(source)?.as_ptr(),path(destination)?.as_ptr())}==0,"Cannot orient portrait");Ok(())}
    /// ImageIO prepares a bounded, orientation-correct display image.
    pub fn artwork_png(source:&Path,destination:&Path,maximum:u32)->Result<()>{anyhow::ensure!(unsafe{mos_artwork_png(path(source)?.as_ptr(),path(destination)?.as_ptr(),maximum)}==0,"Cannot prepare artwork");Ok(())}
    /// Caller serializes inference and validates the package before this call.
    pub fn depth(package:&Path,source:&Path,destination:&Path)->Result<()>{let mut error=[0;1024];anyhow::ensure!(unsafe{mos_depth(path(package)?.as_ptr(),path(source)?.as_ptr(),path(destination)?.as_ptr(),error.as_mut_ptr(),error.len())}==0,"{}",failure(&error));Ok(())}
}
