import {cleanup,render,screen} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
import {TrackRows} from '../src/components';
import type {Track} from '../src/types';
afterEach(cleanup);
vi.stubGlobal('matchMedia',()=>({matches:true,addEventListener:vi.fn(),removeEventListener:vi.fn()}));
const song=(id:number,disc:number,artist='Daft Punk'):Track=>({id,disc_number:disc,track_number:1,path:'/song.flac',title:`Song ${id}`,artist,album:'Album',duration:123,genre:'Electronic',year:2005,stable_id:String(id),favorited:false,available:true,cover_art_path:null});
test('current track keeps its number and puts output levels in the Play control',()=>{
 const {container}=render(<TrackRows tracks={[song(1,1)]} scrollRef={{current:null}} scale={1} onPlay={vi.fn()} onArtist={vi.fn()} onSelect={vi.fn()} playingId={1} isPlaying levels={[.2,.4,.6]}/>);
 expect(container.querySelector('.track-index > span')?.textContent).toBe('1');
 expect(container.querySelector('.track-index .equalizer')).toBeNull();
 expect(screen.getByRole('button',{name:'Pause Song 1'})).toHaveAttribute('data-meter','true');
});
test('disc headers preserve tracks and redundant album credits are suppressed',()=>{
 const tracks=[song(1,1),song(2,2,'Daft Punk feat. Guest')];
 const {container,rerender}=render(<TrackRows tracks={tracks} scrollRef={{current:null}} scale={1} onPlay={vi.fn()} onArtist={vi.fn()} onSelect={vi.fn()} albumArtist="Daft Punk"/>);
 expect(screen.getByText('Disc 1')).toBeInTheDocument();expect(screen.getByText('Disc 2')).toBeInTheDocument();
 expect(container.querySelectorAll('.track-row')).toHaveLength(2);
 expect(container.querySelector('.track-credits')).not.toBeVisible();
 expect(screen.getByRole('button',{name:'Guest'})).toBeVisible();
 rerender(<TrackRows tracks={[song(1,1)]} scrollRef={{current:null}} scale={1} onPlay={vi.fn()} onArtist={vi.fn()} onSelect={vi.fn()} showDurations={false}/>);
 expect(screen.queryByText('Disc 1')).toBeNull();expect(container.querySelector('.track-credits')).toBeVisible();expect(container.querySelector('.track-table')).toHaveAttribute('data-durations','false');
});
