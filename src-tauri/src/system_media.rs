//! Public MediaPlayer integration, confined to the desktop main thread.
use musicos_desktop::local::{LocalPlaybackClient, LocalPlaybackCommand, LocalPlaybackSnapshot};
use objc2::{rc::Retained, runtime::AnyObject, AnyThread};
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use objc2_media_player::*;
use std::{cell::RefCell, ptr::NonNull};
struct Adapter {targets:Vec<(Retained<MPRemoteCommand>,Retained<AnyObject>)>,art_key:Option<std::path::PathBuf>,art:Option<Retained<MPMediaItemArtwork>>}
thread_local! {static ADAPTER:RefCell<Option<Adapter>>=const{RefCell::new(None)};}
pub fn register(client:LocalPlaybackClient){ADAPTER.with(|slot|{if slot.borrow().is_some(){return;}unsafe{
 let center=MPRemoteCommandCenter::sharedCommandCenter();let mut targets=Vec::new();
 for (command,action) in [(center.playCommand(),0),(center.pauseCommand(),1),(center.togglePlayPauseCommand(),2),(center.previousTrackCommand(),3),(center.nextTrackCommand(),4),(Retained::cast_unchecked(center.changePlaybackPositionCommand()),5)] {
  let client=client.clone();command.setEnabled(true);
  let block=block2::RcBlock::new(move|event:NonNull<MPRemoteCommandEvent>|->MPRemoteCommandHandlerStatus {
   let snapshot=client.snapshot();if snapshot.track.is_none(){return MPRemoteCommandHandlerStatus::NoActionableNowPlayingItem;}
   let action=match action {0=>{if snapshot.is_playing{return MPRemoteCommandHandlerStatus::Success;}LocalPlaybackCommand::PlayPause},1=>LocalPlaybackCommand::Pause,2=>LocalPlaybackCommand::PlayPause,3=>LocalPlaybackCommand::Previous,4=>LocalPlaybackCommand::Next,_=>{let event=&*event.as_ptr().cast::<MPChangePlaybackPositionCommandEvent>();let position=event.positionTime();if !position.is_finite(){return MPRemoteCommandHandlerStatus::CommandFailed;}LocalPlaybackCommand::SeekAbsolute(position.max(0.) as f32)}};
   if client.send(action).is_ok(){MPRemoteCommandHandlerStatus::Success}else{MPRemoteCommandHandlerStatus::CommandFailed}
  });let target=command.addTargetWithHandler(&block);targets.push((command,target));
 }
 *slot.borrow_mut()=Some(Adapter{targets,art_key:None,art:None});
}});}
pub fn publish(snapshot:&LocalPlaybackSnapshot){ADAPTER.with(|slot|{let mut slot=slot.borrow_mut();let Some(adapter)=slot.as_mut()else{return;};unsafe{
 let center=MPNowPlayingInfoCenter::defaultCenter();let Some(track)=&snapshot.track else{center.setNowPlayingInfo(None);center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);return;};
 if adapter.art_key!=track.cover_art_path {
  adapter.art_key=track.cover_art_path.clone();adapter.art=None;
  if let Some(path)=&track.cover_art_path {if let Some(image)=objc2_app_kit::NSImage::initWithContentsOfFile(objc2_app_kit::NSImage::alloc(),&NSString::from_str(&path.to_string_lossy())) {
   let size=image.size();let block=block2::RcBlock::new(move|_size:objc2_foundation::NSSize|NonNull::from(image.as_ref()));
   adapter.art=Some(MPMediaItemArtwork::initWithBoundsSize_requestHandler(MPMediaItemArtwork::alloc(),size,&block));
  }}
 }
 let mut keys=vec![MPMediaItemPropertyTitle,MPMediaItemPropertyArtist,MPMediaItemPropertyAlbumTitle,MPMediaItemPropertyPlaybackDuration,MPNowPlayingInfoPropertyElapsedPlaybackTime,MPNowPlayingInfoPropertyPlaybackRate];
 let title=NSString::from_str(&track.title);let artist=NSString::from_str(&track.artist);let album=NSString::from_str(&track.album);
 let duration=NSNumber::new_f64(track.duration as f64);let position=NSNumber::new_f64(snapshot.position as f64);let rate=NSNumber::new_f64(if snapshot.is_playing{snapshot.listening.speed as f64}else{0.});
 let mut values:Vec<&AnyObject>=vec![&title,&artist,&album,&duration,&position,&rate];
 if let Some(art)=&adapter.art {keys.push(MPMediaItemPropertyArtwork);values.push(art);}
 center.setNowPlayingInfo(Some(&NSDictionary::from_slices(&keys,&values)));
 center.setPlaybackState(if snapshot.is_playing{MPNowPlayingPlaybackState::Playing}else{MPNowPlayingPlaybackState::Paused});
}});}
pub fn shutdown(){ADAPTER.with(|slot|{if let Some(adapter)=slot.borrow_mut().take(){unsafe{for(command,target)in adapter.targets{command.removeTarget(Some(&target));command.setEnabled(false);}let center=MPNowPlayingInfoCenter::defaultCenter();center.setNowPlayingInfo(None);center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);}}});}
