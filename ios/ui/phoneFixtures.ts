// Isolated browser fixtures only; production always uses the native companion.
const tracks=Array.from({length:72},(_,i)=>({id:i+1,title:['Kazoo Sunrise','Soft signal','Transit','Silver morning'][i%4]+(i>3?` ${i+1}`:''),artist:['Velvet Static','Moonspindle','Pixel Ferrets'][i%3],album:`Imaginary Teacup Volume ${Math.floor(i/6)+1}`,album_id:`album-${Math.floor(i/6)}`,disc_number:Math.floor(i%6/3)+1,track_number:i%3+1,duration:210,available:true,cover_art_path:null,favorited:i%7===0,path:`fixture/${i}.ogg`,mime:'audio/ogg'}));
let state:any={tracks:[...tracks].reverse(),collections:[{id:'one',name:'Weekend',kind:'manual',track_ids:[1,3,1,7],complete:true},{id:'two',name:'Late afternoon',kind:'manual',track_ids:[2,4,6],complete:true}],preferences:{appearance:'light',colors:'blue',animations:true,interactionFeedback:true,musicHaptics:false,automix:false},paired:false,pending_changes:0,conflicts:[],storage_bytes:123000,text_scale:1,sync:{state:'idle'},playback:{track:tracks[0],queue:[1,2,1,3,4,5],queue_index:0,queue_entry_ids:[101,102,103,104,105,106],is_playing:false,position:24,shuffle:false,repeat:'none',volume:1,backend:'fixture'}};
const scenario=new URLSearchParams(location.search).get('fixture');
if(scenario==='long'){
 Object.assign(tracks[0],{title:'UnbrokenSongTitle'.repeat(12),artist:'AnExtremelyLongArtistCredit'.repeat(10),album:'AnExtremelyLongAlbumTitle'.repeat(9)});
 state.collections[0].name='LongPlaylistName'.repeat(7);
 state.collections[0].track_ids=[9999,1,3,1,7];
 state.collections[0].complete=false;
}
if(scenario==='listening')state.paired=true;
if(scenario==='sync'){
 state.paired=true;
 state.pending_changes=2;
}
let cloud:any={phase:'idle',automatic:false,connected:false,copies:[],song_count:tracks.length};
const iCloudCopy={device:'fixture',name:'Studio iPhone',songs:6,updated:1700000000,current:false};
let snapshots=0;
let syncTimer=0;
let serial=1000;
export async function fixtureRequest(p:any){const a=p.action,s=state.playback;switch(a){case 'snapshot':snapshots++;return structuredClone(state);
case 'cloud_status':return structuredClone(cloud);
case 'cloud_choose':cloud={...cloud,connected:true,folder:'Music copies'};return structuredClone(cloud);
case 'cloud_list':cloud.copies=[iCloudCopy];return structuredClone(cloud);
case 'cloud_auto':cloud.automatic=p.enabled;return structuredClone(cloud);
case 'cloud_upload':cloud={...cloud,last_saved:1700000000,message:'Saved to your Files folder.'};return structuredClone(cloud);
case 'cloud_import':cloud.message='Library copy merged. Songs are available offline.';return structuredClone(cloud);
case 'cloud_disconnect':cloud={...cloud,connected:false,automatic:false,copies:[],message:undefined};return structuredClone(cloud);
case 'fixture_snapshot_count':return snapshots;
case 'listening_status':return {state:state.paired?'connected':'unpaired',lastfm_ready:state.paired,discord_ready:state.paired,pending:state.preferences.lastfmListening?2:0,storage_error:false};
case 'sync_status':return {paired:state.paired,nearby_macs:[],offline_songs:tracks.length,storage_bytes:state.storage_bytes,pending_changes:state.pending_changes,conflicts:[],sync:{...state.sync}};
case 'sync':state.sync={state:'preparing',completed:1,total:3};syncTimer=window.setTimeout(()=>{state.sync={state:'downloading',completed:1,total:2,message:'LongDownloadTitle'.repeat(12)};syncTimer=window.setTimeout(()=>{state.sync={state:'complete',message:'2 songs copied.'};},2500);},1200);return {...state.sync};
case 'cancel':clearTimeout(syncTimer);state.sync={state:'cancelled',message:'Downloaded songs are kept. Retry to continue.'};return {...state.sync};case 'haptic':return {};case 'settings':state.preferences[p.key]=p.value;return structuredClone(state);case 'lyrics':return {document:null,state:'not_found',candidates:[],revision:0};case 'play':if(p.ids){s.queue=p.ids;s.queue_entry_ids=p.ids.map(()=>++serial);s.queue_index=p.index??0;s.position=0;s.track=tracks.find(t=>t.id===s.queue[s.queue_index]);}s.is_playing=true;break;case 'toggle':s.is_playing=!s.is_playing;break;case 'pause':s.is_playing=false;break;case 'next':case 'previous':s.queue_index=Math.max(0,Math.min(s.queue.length-1,s.queue_index+(a==='next'?1:-1)));s.track=tracks.find(t=>t.id===s.queue[s.queue_index]);s.position=0;break;case 'queue_play':s.queue_index=p.index;s.track=tracks.find(t=>t.id===s.queue[p.index]);s.is_playing=true;break;case 'queue_move':{const active=s.queue_entry_ids[s.queue_index],[id]=s.queue.splice(p.from,1),[key]=s.queue_entry_ids.splice(p.from,1);s.queue.splice(p.to,0,id);s.queue_entry_ids.splice(p.to,0,key);s.queue_index=s.queue_entry_ids.indexOf(active);break;}case 'queue_remove':s.queue.splice(p.index,1);s.queue_entry_ids.splice(p.index,1);break;case 'queue_clear_upcoming':s.queue=s.queue.slice(0,s.queue_index+1);s.queue_entry_ids=s.queue_entry_ids.slice(0,s.queue_index+1);break;case 'queue_add':{const i=p.next?s.queue_index+1:s.queue.length;s.queue.splice(i,0,...p.ids);s.queue_entry_ids.splice(i,0,...p.ids.map(()=>++serial));break;}case 'shuffle':s.shuffle=!s.shuffle;break;case 'repeat':s.repeat=s.repeat==='none'?'all':s.repeat==='all'?'one':'none';break;case 'seek':s.position=p.position;break;case 'favorite':tracks.find(t=>t.id===p.id)!.favorited=p.value;return structuredClone(state);case 'playlist':{const id=p.id??`new-${++serial}`;state.collections=state.collections.filter((c:any)=>c.id!==id);state.collections.push({id,name:p.name,kind:'manual',track_ids:p.track_ids,complete:true});return structuredClone(state);}default:return {};}
return structuredClone(s);}
