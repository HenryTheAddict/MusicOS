import {afterEach,describe,expect,it,vi} from 'vitest';
import {cleanup,render,within} from '@testing-library/react';
import BeatPulseGrid,{beatPosition,downbeatGlyph,gridSize,ripplePulse} from '../src/BeatPulseGrid';
vi.mock('../src/AssetIcon',()=>({AssetIcon:({name}:{name:string})=><i data-icon={name}/>}));
vi.mock('../src/playbackFeed',()=>({currentPlaybackClock:()=>({trackId:1,playing:true,position:2,levels:[]}),clockPosition:()=>2,subscribePlaybackClock:()=>()=>{}}));
afterEach(()=>{cleanup();vi.unstubAllGlobals();});
describe('beat pulse math',()=>{
 it('uses the heart phase and offset and follows seeks',()=>{expect(beatPosition(2,120,.5)).toBeCloseTo(3.1);expect(beatPosition(0,120)).toBeCloseTo(.1);});
 it('does not invent beats for missing or invalid tempo',()=>{for(const bpm of [undefined,0,NaN,Infinity,401])expect(beatPosition(2,bpm)).toBeNull();expect(beatPosition(NaN,120)).toBeNull();expect(ripplePulse(null,0)).toBe(0);});
 it('delays the pulse by distance from center',()=>{expect(ripplePulse(0,0)).toBe(1);expect(ripplePulse(.32,2)).toBeCloseTo(1);expect(ripplePulse(0,2)).toBe(0);});
 it('swaps only a few glyphs on fourth beats and cycles',()=>{expect(downbeatGlyph(3.99,0)).toBe(0);expect(downbeatGlyph(4,0)).toBe(1);expect(downbeatGlyph(4,1)).toBe(0);expect(downbeatGlyph(32,0)).toBe(2);expect(downbeatGlyph(60,0)).toBe(0);expect(downbeatGlyph(null,0)).toBe(0);});
 it('balances wide and compact counts',()=>{expect(gridSize(480,480)).toEqual({columns:4,rows:4});expect(gridSize(1280,800)).toEqual({columns:11,rows:7});});
});
describe('motion lifecycle',()=>{
 function setup(){vi.stubGlobal('ResizeObserver',class{observe(){}disconnect(){}});const request=vi.fn(()=>1),cancel=vi.fn();vi.stubGlobal('requestAnimationFrame',request);vi.stubGlobal('cancelAnimationFrame',cancel);return {request,cancel};}
 it('renders a static decorative grid with no rAF in Reduced Motion',()=>{const {request}=setup();const view=render(<BeatPulseGrid playing active reduced trackId={1} bpm={120}/>);expect(within(view.container).queryAllByRole('button')).toHaveLength(0);expect(view.container.querySelector('[aria-hidden=true]')).not.toBeNull();expect(view.container.querySelectorAll('[data-icon=Song]')).toHaveLength(9);expect(request).not.toHaveBeenCalled();});
 it('cancels its sole frame on pause and unmount',()=>{const {request,cancel}=setup();const view=render(<BeatPulseGrid playing active reduced={false} trackId={1} bpm={120}/>);expect(request).toHaveBeenCalledTimes(1);view.rerender(<BeatPulseGrid playing={false} active reduced={false} trackId={1} bpm={120}/>);expect(cancel).toHaveBeenCalledWith(1);expect(request).toHaveBeenCalledTimes(1);view.unmount();});
 it('does not schedule frames for inactive or hidden views',()=>{const {request}=setup();const view=render(<BeatPulseGrid playing active={false} reduced={false} trackId={1} bpm={120}/>);expect(request).not.toHaveBeenCalled();view.unmount();vi.spyOn(document,'hidden','get').mockReturnValue(true);render(<BeatPulseGrid playing active reduced={false} trackId={1} bpm={120}/>);expect(request).not.toHaveBeenCalled();vi.restoreAllMocks();});
});
