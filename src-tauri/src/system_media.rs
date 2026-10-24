//! Public MediaPlayer integration, confined to the desktop main thread.
use musicos_desktop::local::{LocalPlaybackClient, LocalPlaybackCommand, LocalPlaybackSnapshot};
use objc2::{rc::Retained, runtime::AnyObject, AnyThread};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSArray};
use objc2_media_player::*;
use std::{cell::RefCell, ptr::NonNull};
struct Adapter {targets:Vec<(Retained<MPRemoteCommand>,Retained<AnyObject>)>,art_key:Option<std::path::PathBuf>,art:Option<Retained<MPMediaItemArtwork>>,menu:crate::native_playback_menu::PlaybackMenu}
thread_local! {static ADAPTER:RefCell<Option<Adapter>>=const{RefCell::new(None)};}
pub fn register(client:LocalPlaybackClient,app:tauri::AppHandle)->tauri::Result<()>{if ADAPTER.with(|slot|slot.borrow().is_some()){return Ok(());}let menu=crate::native_playback_menu::PlaybackMenu::new(app,client.clone())?;ADAPTER.with(|slot|{if slot.borrow().is_some(){return;}unsafe{
 let center=MPRemoteCommandCenter::sharedCommandCenter();let mut targets=Vec::new();
 let intervals=NSArray::from_retained_slice(&[NSNumber::new_f64(15.)]);center.skipForwardCommand().setPreferredIntervals(&intervals);center.skipBackwardCommand().setPreferredIntervals(&intervals);
 for (command,action) in [(center.playCommand(),0),(center.pauseCommand(),1),(center.togglePlayPauseCommand(),2),(center.previousTrackCommand(),3),(center.nextTrackCommand(),4),(Retained::cast_unchecked(center.changePlaybackPositionCommand()),5),(Retained::cast_unchecked(center.skipForwardCommand()),6),(Retained::cast_unchecked(center.skipBackwardCommand()),7),(Retained::cast_unchecked(center.changeShuffleModeCommand()),8),(Retained::cast_unchecked(center.changeRepeatModeCommand()),9)] {
  let client=client.clone();command.setEnabled(true);
  let block=block2::RcBlock::new(move|event:NonNull<MPRemoteCommandEvent>|->MPRemoteCommandHandlerStatus {
   let snapshot=client.snapshot();if snapshot.track.is_none(){return MPRemoteCommandHandlerStatus::NoActionableNowPlayingItem;}
   let action=match action {0=>{if snapshot.is_playing{return MPRemoteCommandHandlerStatus::Success;}LocalPlaybackCommand::PlayPause},1=>LocalPlaybackCommand::Pause,2=>LocalPlaybackCommand::PlayPause,3=>LocalPlaybackCommand::Previous,4=>LocalPlaybackCommand::Next,6|7=>{let interval=(&*event.as_ptr().cast::<MPSkipIntervalCommandEvent>()).interval();if !interval.is_finite()||interval<=0.||interval>600.{return MPRemoteCommandHandlerStatus::CommandFailed;}LocalPlaybackCommand::SeekRelative((interval*if action==6{1.}else{-1.}) as f32)},8=>{let value=(&*event.as_ptr().cast::<MPChangeShuffleModeCommandEvent>()).shuffleType();if value!=MPShuffleType::Off&&value!=MPShuffleType::Items{return MPRemoteCommandHandlerStatus::CommandFailed;}LocalPlaybackCommand::SetShuffle(value==MPShuffleType::Items)},9=>{let value=(&*event.as_ptr().cast::<MPChangeRepeatModeCommandEvent>()).repeatType();let target=if value==MPRepeatType::Off{"none"}else if value==MPRepeatType::All{"all"}else if value==MPRepeatType::One{"one"}else{return MPRemoteCommandHandlerStatus::CommandFailed;};for _ in 0..repeat_steps(&snapshot.repeat,target){if client.send(LocalPlaybackCommand::CycleRepeat).is_err(){return MPRemoteCommandHandlerStatus::CommandFailed;}}return MPRemoteCommandHandlerStatus::Success;},_=>{let event=&*event.as_ptr().cast::<MPChangePlaybackPositionCommandEvent>();let position=event.positionTime();if !position.is_finite(){return MPRemoteCommandHandlerStatus::CommandFailed;}LocalPlaybackCommand::SeekAbsolute(position.max(0.) as f32)}};
   if client.send(action).is_ok(){MPRemoteCommandHandlerStatus::Success}else{MPRemoteCommandHandlerStatus::CommandFailed}
  });let target=command.addTargetWithHandler(&block);targets.push((command,target));
 }
 *slot.borrow_mut()=Some(Adapter{targets,art_key:None,art:None,menu});
}});Ok(())}
pub fn publish(snapshot:&LocalPlaybackSnapshot){ADAPTER.with(|slot|{let mut slot=slot.borrow_mut();let Some(adapter)=slot.as_mut()else{return;};unsafe{
 let _=adapter.menu.publish(snapshot);let commands=MPRemoteCommandCenter::sharedCommandCenter();let playable=snapshot.track.is_some();for (command,_) in &adapter.targets{command.setEnabled(playable);}commands.nextTrackCommand().setEnabled(playable&&(snapshot.queue_index.is_some_and(|i|i+1<snapshot.queue.len())||snapshot.repeat=="all"));commands.changeShuffleModeCommand().setEnabled(snapshot.queue.len()>1);commands.changeShuffleModeCommand().setCurrentShuffleType(if snapshot.shuffle{MPShuffleType::Items}else{MPShuffleType::Off});commands.changeRepeatModeCommand().setCurrentRepeatType(match snapshot.repeat.as_str(){"all"=>MPRepeatType::All,"one"=>MPRepeatType::One,_=>MPRepeatType::Off});
 let center=MPNowPlayingInfoCenter::defaultCenter();let Some(track)=&snapshot.track else{center.setNowPlayingInfo(None);center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);return;};
 if adapter.art_key!=track.cover_art_path {
  adapter.art_key=track.cover_art_path.clone();adapter.art=None;
  if let Some(path)=&track.cover_art_path {if let Some(image)=objc2_app_kit::NSImage::initWithContentsOfFile(objc2_app_kit::NSImage::alloc(),&NSString::from_str(&path.to_string_lossy())) {
   let size=image.size();let block=block2::RcBlock::new(move|_size:objc2_foundation::NSSize|NonNull::from(image.as_ref()));
   adapter.art=Some(MPMediaItemArtwork::initWithBoundsSize_requestHandler(MPMediaItemArtwork::alloc(),size,&block));
  }}
 }
 let mut keys=vec![MPMediaItemPropertyTitle,MPMediaItemPropertyArtist,MPMediaItemPropertyAlbumTitle,MPMediaItemPropertyPlaybackDuration,MPNowPlayingInfoPropertyElapsedPlaybackTime,MPNowPlayingInfoPropertyPlaybackRate,MPNowPlayingInfoPropertyPlaybackQueueIndex,MPNowPlayingInfoPropertyPlaybackQueueCount,MPNowPlayingInfoPropertyDefaultPlaybackRate];
 let title=NSString::from_str(&track.title);let artist=NSString::from_str(&track.artist);let album=NSString::from_str(&track.album);
 let duration=NSNumber::new_f64(track.duration as f64);let position=NSNumber::new_f64(snapshot.position as f64);let rate=NSNumber::new_f64(if snapshot.is_playing{snapshot.listening.speed as f64}else{0.});
 let index=NSNumber::new_u64(snapshot.queue_index.unwrap_or(0) as u64);let count=NSNumber::new_u64(snapshot.queue.len() as u64);let default_rate=NSNumber::new_f64(1.);
 let mut values:Vec<&AnyObject>=vec![&title,&artist,&album,&duration,&position,&rate,&index,&count,&default_rate];
 if let Some(art)=&adapter.art {keys.push(MPMediaItemPropertyArtwork);values.push(art);}
 center.setNowPlayingInfo(Some(&NSDictionary::from_slices(&keys,&values)));
 center.setPlaybackState(if snapshot.is_playing{MPNowPlayingPlaybackState::Playing}else{MPNowPlayingPlaybackState::Paused});
}});}
pub fn shutdown(){ADAPTER.with(|slot|{if let Some(adapter)=slot.borrow_mut().take(){unsafe{for(command,target)in adapter.targets{command.removeTarget(Some(&target));command.setEnabled(false);}let center=MPNowPlayingInfoCenter::defaultCenter();center.setNowPlayingInfo(None);center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);}}});}

fn repeat_steps(current:&str,target:&str)->usize{let index=|value:&str|match value{"all"=>1,"one"=>2,_=>0};(index(target)+3-index(current))%3}
#[cfg(test)]mod tests{#[test]fn repeat_requests_follow_the_existing_runtime_cycle(){for (from,to,count) in [("none","all",1),("all","one",1),("one","none",1),("none","one",2),("all","none",2),("one","one",0)]{assert_eq!(super::repeat_steps(from,to),count);}}}
