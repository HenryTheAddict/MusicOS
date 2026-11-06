import {cleanup,render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,expect,test,vi} from 'vitest';
import type {Album,AlbumReview} from '../src/types';

const mocks=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/bridge',()=>({call:mocks.call,errorText:String,native:false}));
vi.mock('../src/components',()=>({Cover:({title}:{title:string})=><span aria-hidden="true">{title}</span>,Icon:()=>null}));
import AlbumMetadata from '../src/AlbumMetadata';
import {invalidateDisplay} from '../src/displayData';

const album={id:'record',title:'Record',artist:'Artist',cover:'old.png',track_ids:[1,2]} as Album;
const comparison:AlbumReview={
 album_id:album.id,title:album.title,artist:album.artist,token:'version-1',cover:'old.png',
 rows:[1,2].map(track_id=>({track_id,current:{title:`Song ${track_id}`,album:'Record',artist:'Artist',year:2000,revision:1},
  candidate:{album:'Different edition',year:2020,artwork_path:`cover-${track_id}.png`},candidate_token:`candidate-${track_id}`})),
};
afterEach(()=>{cleanup();invalidateDisplay();mocks.call.mockReset();});
test('cover review requires a proposal and submits artwork only even with checked tags',async()=>{
 mocks.call.mockImplementation(async (_command,args)=>args.request?.action==='detail'?{album:comparison}:{count:2});
 const props={album,lyrics:false,epoch:0,playing:false,onTab:()=>{},onSong:()=>{},onLyrics:()=>{},
  onDirty:()=>{},onSaving:()=>{},onSaved:()=>{},onRefresh:async()=>{}};
 const user=userEvent.setup();
 const view=render(<AlbumMetadata {...props} tab="details"/>);
 const choice=await screen.findByRole('checkbox',{name:'Apply album to 2 songs'});
 expect(choice).not.toBeChecked();
 await user.click(choice);
 view.rerender(<AlbumMetadata {...props} tab="cover"/>);
 expect(screen.getByRole('button',{name:'Update cover only'})).toBeDisabled();
 await user.click(screen.getByRole('radio',{name:/Cover 2/}));
 await user.click(screen.getByRole('button',{name:'Update cover only'}));
 expect(screen.getByRole('heading',{name:'Review cover update'})).toBeVisible();
 expect(screen.queryByText('Different edition')).toBeNull();
 await user.click(screen.getByRole('button',{name:'Save in musicOS'}));
 await waitFor(()=>expect(mocks.call).toHaveBeenCalledWith('album_metadata',{request:{
  action:'apply',album_id:'record',token:'version-1',rows:[{track_id:2,fields:['artwork']}],write_files:false,
 }}));
 expect(screen.getByRole('heading',{name:'Record'})).toBeVisible();
});

test('new comparison invalidates confirmation and preserves a manual draft for a new review',async()=>{
 let current=comparison;
 mocks.call.mockImplementation(async (_command,args)=>args.request?.action==='detail'?{album:current}:2);
 const props={album,lyrics:false,playing:false,onTab:()=>{},onSong:()=>{},onLyrics:()=>{},
  onDirty:()=>{},onSaving:()=>{},onSaved:()=>{},onRefresh:async()=>{}};
 const user=userEvent.setup();
 const view=render(<AlbumMetadata {...props} tab="details" epoch={0}/>);
 const year=await screen.findByRole('textbox',{name:'Year',exact:true});
 await user.clear(year);
 await user.type(year,'2023');
 await user.click(screen.getByRole('button',{name:'Review changes',exact:true}));
 expect(screen.getByRole('button',{name:'Save in musicOS'})).toBeVisible();
 current={...comparison,token:'version-2',rows:comparison.rows.map(row=>({...row,current:{...row.current,revision:2,year:2022}}))};
 view.rerender(<AlbumMetadata {...props} tab="details" epoch={1}/>);
 await screen.findByText(/This album changed/);
 expect(screen.queryByRole('button',{name:'Save in musicOS'})).toBeNull();
 expect(screen.getByRole('textbox',{name:'Year',exact:true})).toHaveValue('2023');
 await user.click(screen.getByRole('button',{name:'Review changes',exact:true}));
 expect(screen.getByText('2022',{exact:true})).toBeVisible();
 await user.click(screen.getByRole('button',{name:'Save in musicOS'}));
 await waitFor(()=>expect(mocks.call).toHaveBeenCalledWith('metadata_edit',{
  ids:[1,2],patch:{year:2023},revisions:[[1,2],[2,2]],writeFiles:false,
 }));
});

test('mixed album values identify their songs and stay out of unrelated edits',async()=>{
 const mixed={...comparison,rows:comparison.rows.map(row=>({...row,current:{...row.current,album_artist:row.track_id===1?'First artist':'Second artist'}}))};
 mocks.call.mockImplementation(async (_command,args)=>args.request?.action==='detail'?{album:mixed}:2);
 const user=userEvent.setup();
 render(<AlbumMetadata album={album} lyrics={false} playing={false} tab="details" epoch={0}
  onTab={()=>{}} onSong={()=>{}} onLyrics={()=>{}} onDirty={()=>{}} onSaving={()=>{}} onSaved={()=>{}} onRefresh={async()=>{}}/>);
 expect(await screen.findByRole('textbox',{name:'Album artist'})).toHaveAttribute('placeholder','Mixed values');
 await user.click(screen.getByText('See affected songs',{exact:true}));
 expect(screen.getByText('First artist',{exact:true})).toBeVisible();
 expect(screen.getByText('Second artist',{exact:true})).toBeVisible();
 await user.type(screen.getByRole('textbox',{name:'Genre',exact:true}),'Electronic');
 await user.click(screen.getByRole('button',{name:'Review changes',exact:true}));
 await user.click(screen.getByRole('button',{name:'Save in musicOS'}));
 await waitFor(()=>expect(mocks.call).toHaveBeenCalledWith('metadata_edit',{
  ids:[1,2],patch:{genre:'Electronic'},revisions:[[1,1],[2,1]],writeFiles:false,
 }));
});
