import {act,render,screen} from '@testing-library/react';
import {expect,test} from 'vitest';
import {usePlaybackFeed,usePlaybackClock} from '../src/playbackFeed';
import {emptyPlayback} from '../src/bridge';
import type {Playback} from '../src/types';
test('clock and metering ticks update progress without rendering the library, but playback acknowledgements render immediately',()=>{
 let pages=0,update!:(value:Playback|((old:Playback)=>Playback))=>void;
 function Clock(){const clock=usePlaybackClock();return <output>{clock.position}</output>;}
 function Library(){const [value,set]=usePlaybackFeed(emptyPlayback);update=set;pages++;return <><span>{value.is_playing?'Playing':'Paused'}</span><Clock/></>;}
 render(<Library/>);const before=pages;
 for(let i=1;i<=40;i++)act(()=>update(old=>({...old,position:i/20,audio_levels:[.2,.3,.4],reverb_processed_frames:i*1024})));
 expect(pages).toBe(before);expect(screen.getByRole('status')).toHaveTextContent('2');
 act(()=>update(old=>({...old,is_playing:true})));expect(pages).toBe(before+1);expect(screen.getByText('Playing')).toBeVisible();
});

test('partial native events retain the queue and deliver clock and presentation changes independently',()=>{
 let update!:(value:Partial<Playback>)=>void;
 const track={id:21,title:'Original'} as Playback['track'];
 function Player(){
  const [playback,apply]=usePlaybackFeed({...emptyPlayback,track,queue:[track!],queue_entry_ids:[1]});
  update=apply;
  return <><span>{playback.track?.title}</span><span>{playback.queue.length}</span><div>{playback.queue_entry_ids?.join(',')}</div></>;
 }
 render(<Player/>);
 act(()=>update({position:42,audio_levels:[.2,.4,.8]}));
 expect(screen.getByText('Original')).toBeVisible();
 expect(screen.getByText('1',{selector:'span'})).toBeVisible();
 act(()=>update({track:{...track!,title:'Corrected'},queue_entry_ids:[2]}));
 expect(screen.getByText('Corrected')).toBeVisible();
 expect(screen.getByText('2')).toBeVisible();
 expect(screen.getByText('1',{selector:'span'})).toBeVisible();
});
