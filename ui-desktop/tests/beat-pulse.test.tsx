import {afterEach,describe,expect,it,vi} from 'vitest';
import {cleanup,render,within} from '@testing-library/react';
import BeatPulseGrid,{beatPosition,downbeatGlyph,gridSize,ripplePulse} from '../src/BeatPulseGrid';
vi.mock('../src/AssetIcon',()=>({AssetIcon:({name}:{name:string})=><i className="asset-icon" data-icon={name}/>}));
const feed=vi.hoisted(()=>({clock:{trackId:1,playing:true,position:2,levels:[] as number[]},position:vi.fn(()=>2)}));
vi.mock('../src/playbackFeed',()=>({currentPlaybackClock:()=>feed.clock,clockPosition:feed.position,subscribePlaybackClock:()=>()=>{}}));
afterEach(()=>{cleanup();vi.unstubAllGlobals();});
describe('beat pulse math',()=>{
 it('uses the heart phase and offset and follows seeks',()=>{expect(beatPosition(2,120,.5)).toBeCloseTo(3.1);expect(beatPosition(0,120)).toBeCloseTo(.1);});
 it('does not invent beats for missing or invalid tempo',()=>{for(const bpm of [undefined,0,NaN,Infinity,401])expect(beatPosition(2,bpm)).toBeNull();expect(beatPosition(NaN,120)).toBeNull();expect(ripplePulse(null,0)).toBe(0);});
 it('delays the pulse by distance from center',()=>{expect(ripplePulse(0,0)).toBe(1);expect(ripplePulse(.32,2)).toBeCloseTo(1);expect(ripplePulse(0,2)).toBe(0);});
 it('swaps only a few glyphs on fourth beats and cycles',()=>{expect(downbeatGlyph(3.99,0)).toBe(0);expect(downbeatGlyph(4,0)).toBe(1);expect(downbeatGlyph(4,1)).toBe(0);expect(downbeatGlyph(32,0)).toBe(2);expect(downbeatGlyph(60,0)).toBe(0);expect(downbeatGlyph(null,0)).toBe(0);});
 it('balances counts within the chrome-free wide and compact areas',()=>{expect(gridSize(440,264)).toEqual({columns:4,rows:2});expect(gridSize(1240,584)).toEqual({columns:11,rows:5});expect(gridSize(0,0)).toEqual({columns:1,rows:1});});
 it('leaves room for maximum pulse translation, scale and rotation',()=>{const extent=48*1.32*(Math.cos(8*Math.PI/180)+Math.sin(8*Math.PI/180))+24;for(const [width,height] of [[440,264],[1240,584]]){const {columns,rows}=gridSize(width,height);expect(width/columns).toBeGreaterThan(extent);expect(height/rows).toBeGreaterThan(extent);}});
});
describe('motion lifecycle',()=>{
 function setup(){vi.stubGlobal('ResizeObserver',class{observe(){}disconnect(){}});const request=vi.fn<(callback:FrameRequestCallback)=>number>(()=>1),cancel=vi.fn();vi.stubGlobal('requestAnimationFrame',request);vi.stubGlobal('cancelAnimationFrame',cancel);return {request,cancel};}
 it('switches from BPM and downbeat glyphs to levels only when Beat Sync is off',()=>{
  const {request}=setup();feed.clock.levels=[.9];feed.position.mockClear();
  const view=render(<BeatPulseGrid playing active reduced={false} trackId={1} bpm={120}/>);
  request.mock.calls.at(-1)![0](0);expect(feed.position).toHaveBeenCalled();
  const cell=view.container.querySelector('.beat-pulse-cell') as HTMLElement;
  expect((cell.children[1] as HTMLElement).style.opacity).toBe('1');
  view.rerender(<BeatPulseGrid beatSync={false} playing active reduced={false} trackId={1} bpm={120}/>);feed.position.mockClear();
  request.mock.calls.at(-1)![0](1);const high=cell.style.transform;
  expect((cell.children[0] as HTMLElement).style.opacity).toBe('1');expect((cell.children[1] as HTMLElement).style.opacity).toBe('0');
  feed.clock.levels=[.45];request.mock.calls.at(-1)![0](2);expect(cell.style.transform).not.toBe(high);
  feed.clock.levels=[];request.mock.calls.at(-1)![0](3);expect(cell.style.transform).not.toContain('NaN');expect(feed.position).not.toHaveBeenCalled();
 });
 it('renders a static decorative grid with no rAF in Reduced Motion',()=>{const {request}=setup();const view=render(<BeatPulseGrid playing active reduced trackId={1} bpm={120}/>);expect(within(view.container).queryAllByRole('button')).toHaveLength(0);expect(view.container.querySelector('[aria-hidden=true]')).not.toBeNull();expect(new Set(Array.from(view.container.querySelectorAll('[data-icon]'),icon=>icon.getAttribute('data-icon'))).size).toBe(7);expect(request).not.toHaveBeenCalled();});
 it('cancels its sole frame on pause and unmount',()=>{const {request,cancel}=setup();const view=render(<BeatPulseGrid playing active reduced={false} trackId={1} bpm={120}/>);expect(request).toHaveBeenCalledTimes(1);view.rerender(<BeatPulseGrid playing={false} active reduced={false} trackId={1} bpm={120}/>);expect(cancel).toHaveBeenCalledWith(1);expect(request).toHaveBeenCalledTimes(1);view.unmount();});
 it('does not schedule frames for inactive or hidden views',()=>{const {request}=setup();const view=render(<BeatPulseGrid playing active={false} reduced={false} trackId={1} bpm={120}/>);expect(request).not.toHaveBeenCalled();view.unmount();vi.spyOn(document,'hidden','get').mockReturnValue(true);render(<BeatPulseGrid playing active reduced={false} trackId={1} bpm={120}/>);expect(request).not.toHaveBeenCalled();vi.restoreAllMocks();});
});
