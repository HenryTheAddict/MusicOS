import{render,waitFor,cleanup,fireEvent}from'@testing-library/react';
import{afterEach,beforeEach,expect,it,vi}from'vitest';
vi.mock('../src/bridge',()=>({native:true,artwork:(path:string)=>path}));
vi.mock('../src/useArtworkRendition',()=>({artworkRendition:vi.fn(async(path:string)=>path)}));
vi.mock('../src/AppleEnrichment',()=>({appleCall:vi.fn(async(action:string,id?:number)=>action==='motion_policy'?{lowPower:false}:{url:`https://example.test/${id}.m3u8`})}));
vi.mock('../src/ViewScope',()=>({useViewActive:()=>true}));
import MotionArtwork from'../src/MotionArtwork';
import {appleCall} from '../src/AppleEnrichment';
let reduced=false;const listeners=new Set<()=>void>();
beforeEach(()=>{reduced=false;vi.stubGlobal('IntersectionObserver',class{constructor(private cb:(entries:any[])=>void){}observe(){this.cb([{isIntersecting:true}]);}disconnect(){}});vi.stubGlobal('matchMedia',()=>({get matches(){return reduced;},addEventListener:(_:string,fn:()=>void)=>listeners.add(fn),removeEventListener:(_:string,fn:()=>void)=>listeners.delete(fn)}));vi.spyOn(HTMLMediaElement.prototype,'play').mockResolvedValue();vi.spyOn(HTMLMediaElement.prototype,'pause').mockImplementation(()=>{});vi.spyOn(HTMLMediaElement.prototype,'load').mockImplementation(()=>{});window.dispatchEvent(new CustomEvent('musicos-apple-status',{detail:{signed_in:true}}));});
afterEach(()=>{cleanup();listeners.clear();window.dispatchEvent(new CustomEvent('musicos-apple-status',{detail:{signed_in:false}}));vi.restoreAllMocks();vi.unstubAllGlobals();});
it('only the highest visible presentation owns a decoder and reduced motion releases it',async()=>{const view=render(<><MotionArtwork trackId={1} priority={1}><img className="cover" alt="Album"/></MotionArtwork><MotionArtwork trackId={2} priority={2}><img className="cover" alt="Mini"/></MotionArtwork><MotionArtwork trackId={3} priority={3}><img className="cover" alt="Expanded"/></MotionArtwork></>);await waitFor(()=>expect(document.querySelectorAll('video')).toHaveLength(1));expect(document.querySelector('video')!.src).toContain('/3.m3u8');view.rerender(<><MotionArtwork trackId={1} priority={1}><img className="cover" alt="Album"/></MotionArtwork><MotionArtwork trackId={2} priority={2}><img className="cover" alt="Mini"/></MotionArtwork></>);await waitFor(()=>expect(document.querySelector('video')!.src).toContain('/2.m3u8'));reduced=true;listeners.forEach(fn=>fn());await waitFor(()=>expect(document.querySelectorAll('video')).toHaveLength(0));});

it('expanded still artwork stays visible when motion fails and duplicate presentations share one request',async()=>{
 vi.mocked(appleCall).mockImplementation(async(action,id)=>action==='motion_policy'?{lowPower:false}:action==='expanded_artwork'?{staticPath:`/covers/${id}-2048.jpg`,url:`https://example.test/${id}.m3u8`}:{});
 const view=render(<><MotionArtwork trackId={40} priority={3}><img className="cover" alt="First"/></MotionArtwork><MotionArtwork trackId={40} priority={3}><img className="cover" alt="Second"/></MotionArtwork></>);
 await waitFor(()=>expect(document.querySelectorAll('.expanded-static-artwork')).toHaveLength(2));
 expect(vi.mocked(appleCall).mock.calls.filter(([action,id])=>action==='expanded_artwork'&&id===40)).toHaveLength(1);
 const still=document.querySelector<HTMLImageElement>('.expanded-static-artwork')!;expect(still.style.opacity).toBe('0');fireEvent.load(still);expect(still.style.opacity).toBe('1');
 await waitFor(()=>expect(document.querySelector('video')).not.toBeNull());fireEvent.error(document.querySelector('video')!);
 await waitFor(()=>expect(document.querySelector('video')).not.toBeNull()); // the second visible owner may take over
 expect(still.style.opacity).toBe('1');view.unmount();expect(document.querySelectorAll('video')).toHaveLength(0);
});
it('a late cover cannot replace the next song and still mode never starts a decoder',async()=>{
 let finish:(value:any)=>void=()=>{};
 vi.mocked(appleCall).mockImplementation(async(action,id)=>action==='motion_policy'?{lowPower:false}:action==='expanded_artwork'?id===50?await new Promise(resolve=>{finish=resolve;}):{staticPath:`/covers/${id}-2048.jpg`,url:`https://example.test/${id}.m3u8`}:{});
 const view=render(<MotionArtwork trackId={50} priority={3} animated={false}><img className="cover" alt="Base"/></MotionArtwork>);
 await waitFor(()=>expect(vi.mocked(appleCall).mock.calls.some(([action,id])=>action==='expanded_artwork'&&id===50)).toBe(true));
 view.rerender(<MotionArtwork trackId={51} priority={3} animated={false}><img className="cover" alt="Base"/></MotionArtwork>);
 await waitFor(()=>expect(document.querySelector<HTMLImageElement>('.expanded-static-artwork')?.src).toContain('51-2048.jpg'));
 finish({staticPath:'/covers/50-2048.jpg',url:'https://example.test/50.m3u8'});
 await waitFor(()=>expect(document.querySelector<HTMLImageElement>('.expanded-static-artwork')?.src).toContain('51-2048.jpg'));
 expect(document.querySelectorAll('video')).toHaveLength(0);
});
