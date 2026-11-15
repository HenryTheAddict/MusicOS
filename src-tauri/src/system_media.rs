//! Public MediaPlayer integration, confined to the desktop main thread.
use musicos_desktop::local::{LocalPlaybackClient, LocalPlaybackCommand, LocalPlaybackSnapshot};
use objc2::{rc::Retained, runtime::AnyObject, AnyThread};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSArray};
use objc2_media_player::*;
use std::{cell::RefCell, ptr::NonNull};
use std::{path::PathBuf,time::{Duration,Instant}};

#[derive(PartialEq)]
struct MediaState {
    identity:Option<(i64,String,String,String,i32)>,
    cover:Option<PathBuf>,
    playing:bool,
    speed:f32,
    queue_revision:u64,
    queue_index:Option<usize>,
    shuffle:bool,
    repeat:String,
    failed:Vec<u64>,
}

// MediaPlayer extrapolates time from the last time and rate we publish.
#[derive(Default)]
pub struct Publication {
    last:Option<(MediaState,f32,Instant)>,
}
impl Publication {
    pub fn should_publish(&mut self,snapshot:&LocalPlaybackSnapshot,artwork:Option<&Option<PathBuf>>)->bool {
        self.changed_at(snapshot,artwork,Instant::now())
    }
    fn changed_at(&mut self,snapshot:&LocalPlaybackSnapshot,artwork:Option<&Option<PathBuf>>,now:Instant)->bool {
        let state=MediaState{
            identity:snapshot.track.as_ref().map(|track|(track.id,track.title.clone(),track.artist.clone(),track.album.clone(),track.duration)),
            cover:artwork.cloned().unwrap_or_else(||snapshot.track.as_ref().and_then(|track|track.cover_art_path.clone())),
            playing:snapshot.is_playing,
            speed:system_playback_rate(snapshot),
            queue_revision:snapshot.queue_revision,
            queue_index:snapshot.queue_index,
            shuffle:snapshot.shuffle,
            repeat:snapshot.repeat.clone(),
            failed:snapshot.failed_entries.clone(),
        };
        if let Some((prior,position,when))=&self.last {
            let age=now.saturating_duration_since(*when);
            let expected=position+age.as_secs_f32()*prior.speed;
            let tolerance=if state.playing{0.65}else{0.005};
            if state==*prior&&(snapshot.position-expected).abs()<tolerance&&(!state.playing||age<Duration::from_secs(8)) {
                return false;
            }
        }
        self.last=Some((state,snapshot.position,now));
        true
    }
}
fn system_playback_rate(snapshot:&LocalPlaybackSnapshot)->f32 {
    if !snapshot.is_playing{return 0.;}
    let entry=snapshot.queue_index.and_then(|index|snapshot.queue_entry_ids.get(index));
    snapshot.listening.speed*snapshot.dj_transition.as_ref().filter(|transition|transition.active&&entry==Some(&transition.incoming_entry)).map_or(1.,|transition|transition.tempo)
}

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
 let duration=NSNumber::new_f64(track.duration as f64);let position=NSNumber::new_f64(snapshot.position as f64);let rate=NSNumber::new_f64(f64::from(system_playback_rate(snapshot)));
 let index=NSNumber::new_u64(snapshot.queue_index.unwrap_or(0) as u64);let count=NSNumber::new_u64(snapshot.queue.len() as u64);let default_rate=NSNumber::new_f64(1.);
 let mut values:Vec<&AnyObject>=vec![&title,&artist,&album,&duration,&position,&rate,&index,&count,&default_rate];
 if let Some(art)=&adapter.art {keys.push(MPMediaItemPropertyArtwork);values.push(art);}
 center.setNowPlayingInfo(Some(&NSDictionary::from_slices(&keys,&values)));
 center.setPlaybackState(if snapshot.is_playing{MPNowPlayingPlaybackState::Playing}else{MPNowPlayingPlaybackState::Paused});
}});}
pub fn shutdown(){ADAPTER.with(|slot|{if let Some(adapter)=slot.borrow_mut().take(){unsafe{for(command,target)in adapter.targets{command.removeTarget(Some(&target));command.setEnabled(false);}let center=MPNowPlayingInfoCenter::defaultCenter();center.setNowPlayingInfo(None);center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);}}});}

fn repeat_steps(current:&str,target:&str)->usize{let index=|value:&str|match value{"all"=>1,"one"=>2,_=>0};(index(target)+3-index(current))%3}
#[cfg(test)]mod tests{#[test]fn repeat_requests_follow_the_existing_runtime_cycle(){for (from,to,count) in [("none","all",1),("all","one",1),("one","none",1),("none","one",2),("all","none",2),("one","one",0)]{assert_eq!(super::repeat_steps(from,to),count);}}}

#[cfg(test)]
mod publication_tests {
    use super::*;

    #[test]
    fn idle_updates_are_suppressed_and_seeks_and_transport_publish_immediately() {
        let mut publication=Publication::default();
        let mut snapshot=LocalPlaybackSnapshot::default();
        let start=Instant::now();
        assert!(publication.changed_at(&snapshot,None,start));
        assert!(!publication.changed_at(&snapshot,None,start+Duration::from_secs(40)));
        snapshot.position=34.;
        assert!(publication.changed_at(&snapshot,None,start+Duration::from_secs(41)));
        snapshot.is_playing=true;
        assert!(publication.changed_at(&snapshot,None,start+Duration::from_secs(42)));
        snapshot.position=35.9;
        assert!(!publication.changed_at(&snapshot,None,start+Duration::from_secs(44)));
        snapshot.position=9.;
        assert!(publication.changed_at(&snapshot,None,start+Duration::from_secs(44)));
        snapshot.is_playing=false;
        assert!(publication.changed_at(&snapshot,None,start+Duration::from_millis(44100)));
        assert!(!publication.changed_at(&snapshot,None,start+Duration::from_secs(90)));
    }
}
