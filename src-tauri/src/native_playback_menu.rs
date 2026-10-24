//! Native queue controls resolve stable entry IDs against acknowledged playback.
use musicos_desktop::local::{LocalPlaybackClient,LocalPlaybackCommand,LocalPlaybackSnapshot};
use tauri::{AppHandle,Emitter,menu::{Menu,MenuItem,CheckMenuItem,PredefinedMenuItem,Submenu}};
pub struct PlaybackMenu {app:AppHandle,toggle:MenuItem<tauri::Wry>,shuffle:CheckMenuItem<tauri::Wry>,repeat:MenuItem<tauri::Wry>,queue:Submenu<tauri::Wry>,stamp:Option<(u64,Option<usize>,Vec<u64>)>,state:Option<(bool,bool,String,usize,bool)>}
pub fn entry_index(ids:&[u64],entry:u64)->Option<usize>{ids.iter().position(|id|*id==entry)}
impl PlaybackMenu {
 pub fn new(app:AppHandle,client:LocalPlaybackClient)->tauri::Result<Self>{
  let toggle=MenuItem::with_id(&app,"musicos.playback.toggle","Play",true,None::<&str>)?;
  let previous=MenuItem::with_id(&app,"musicos.playback.previous","Previous song",true,None::<&str>)?;
  let next=MenuItem::with_id(&app,"musicos.playback.next","Next song",true,None::<&str>)?;
  let backward=MenuItem::with_id(&app,"musicos.playback.backward","Back 15 seconds",true,None::<&str>)?;
  let forward=MenuItem::with_id(&app,"musicos.playback.forward","Forward 15 seconds",true,None::<&str>)?;
  let shuffle=CheckMenuItem::with_id(&app,"musicos.playback.shuffle","Shuffle songs",true,false,None::<&str>)?;
  let repeat=MenuItem::with_id(&app,"musicos.playback.repeat","Repeat: Off",true,None::<&str>)?;
  let queue=Submenu::new(&app,"Up next",true)?;
  let playback=Submenu::with_items(&app,"Playback",true,&[&toggle,&previous,&next,&PredefinedMenuItem::separator(&app)?,&backward,&forward,&PredefinedMenuItem::separator(&app)?,&shuffle,&repeat,&queue])?;
  let menu=app.menu().map(Ok).unwrap_or_else(||Menu::default(&app))?;menu.insert(&playback,3.min(menu.items()?.len()))?;app.set_menu(menu)?;
  app.on_menu_event(move|app,event|{let id=event.id().as_ref();let snapshot=client.snapshot();let command=match id {
   "musicos.playback.toggle"=>Some(LocalPlaybackCommand::PlayPause),"musicos.playback.previous"=>Some(LocalPlaybackCommand::Previous),"musicos.playback.next"=>Some(LocalPlaybackCommand::Next),"musicos.playback.backward"=>Some(LocalPlaybackCommand::SeekRelative(-15.)),"musicos.playback.forward"=>Some(LocalPlaybackCommand::SeekRelative(15.)),"musicos.playback.shuffle"=>Some(LocalPlaybackCommand::SetShuffle(!snapshot.shuffle)),"musicos.playback.repeat"=>Some(LocalPlaybackCommand::CycleRepeat),"musicos.playback.queue"=>{let _=app.emit("native-open-queue",());None},_=>id.strip_prefix("musicos.queue.").and_then(|value|value.parse::<u64>().ok()).and_then(|entry|entry_index(&snapshot.queue_entry_ids,entry)).map(LocalPlaybackCommand::SelectQueueIndex)
  };if let Some(command)=command{if snapshot.track.is_some(){let _=client.send(command);}}});
  Ok(Self{app,toggle,shuffle,repeat,queue,stamp:None,state:None})
 }
 pub fn publish(&mut self,snapshot:&LocalPlaybackSnapshot)->tauri::Result<()>{
  let state=(snapshot.is_playing,snapshot.shuffle,snapshot.repeat.clone(),snapshot.queue.len(),snapshot.track.is_some());if self.state.as_ref()!=Some(&state){self.toggle.set_text(if snapshot.is_playing{"Pause"}else{"Play"})?;self.toggle.set_enabled(snapshot.track.is_some())?;self.shuffle.set_checked(snapshot.shuffle)?;self.shuffle.set_enabled(snapshot.queue.len()>1)?;self.repeat.set_enabled(snapshot.track.is_some())?;self.repeat.set_text(format!("Repeat: {}",match snapshot.repeat.as_str(){"one"=>"One song","all"=>"All songs",_=>"Off"}))?;self.state=Some(state);}
  let stamp=(snapshot.queue_revision,snapshot.queue_index,snapshot.failed_entries.clone());if self.stamp.as_ref()!=Some(&stamp){
   for item in self.queue.items()?{self.queue.remove(&item)?;}
   let start=snapshot.queue_index.unwrap_or(0);for (index,track) in snapshot.queue.iter().enumerate().skip(start).take(21){if let Some(entry)=snapshot.queue_entry_ids.get(index){let label=format!("{} · {}",track.title,track.artist).chars().take(100).collect::<String>();self.queue.append(&CheckMenuItem::with_id(&self.app,format!("musicos.queue.{entry}"),label,!snapshot.failed_entries.contains(entry),snapshot.queue_index==Some(index),None::<&str>)?)?;}}
   if !snapshot.queue.is_empty(){self.queue.append(&PredefinedMenuItem::separator(&self.app)?)?;}
   self.queue.append(&MenuItem::with_id(&self.app,"musicos.playback.queue","Open full Up next…",true,None::<&str>)?)?;self.stamp=Some(stamp);
  }Ok(())
 }
}
#[cfg(test)]mod tests{#[test]fn queue_selection_follows_entries_after_reordering_and_rejects_removed_entries(){assert_eq!(super::entry_index(&[9,4,7],4),Some(1));assert_eq!(super::entry_index(&[4,9,7],4),Some(0));assert_eq!(super::entry_index(&[9,7],4),None);}}
