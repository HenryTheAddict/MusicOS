import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import MiniActions from '../src/MiniActions';
afterEach(cleanup);
beforeEach(()=>{vi.stubGlobal('matchMedia',()=>({matches:true,addEventListener:()=>{},removeEventListener:()=>{}}));HTMLElement.prototype.showPopover=vi.fn(function(this:HTMLElement){this.style.display='block';});HTMLElement.prototype.hidePopover=vi.fn(function(this:HTMLElement){this.style.display='none';});});
it('prepares menu space and collapses it before selecting a playback-window action',async()=>{
 const calls:string[]=[];let finish:()=>void=()=>{};
 const resize=vi.fn(async(open:boolean)=>{calls.push(open?'expand':'collapse');if(!open)await new Promise<void>(resolve=>{finish=resolve;});});
 render(<MiniActions height={48} onResize={resize} items={[{label:'Up next',onSelect:()=>calls.push('queue')}]}/>);
 fireEvent.click(screen.getByRole('button',{name:'Mini Player actions'}));await screen.findByRole('menuitem',{name:'Up next'});
 fireEvent.click(screen.getByRole('menuitem',{name:'Up next'}));await waitFor(()=>expect(calls).toEqual(['expand','collapse']));expect(screen.getByRole('menu')).toHaveAttribute('inert');finish();await waitFor(()=>expect(calls).toEqual(['expand','collapse','queue']));
});
it('Escape collapses menu space and restores focus',async()=>{
 const resize=vi.fn().mockResolvedValue(undefined);
 render(<MiniActions height={48} onResize={resize} items={[{label:'Show artwork',onSelect:()=>{}}]}/>);
 const opener=screen.getByRole('button',{name:'Mini Player actions'});fireEvent.click(opener);const item=await screen.findByRole('menuitem',{name:'Show artwork'});fireEvent.keyDown(item,{key:'Escape'});await waitFor(()=>expect(opener).toHaveFocus());expect(resize).toHaveBeenLastCalledWith(false);
});
it('failed preparation stays actionable and can be retried',async()=>{
 const resize=vi.fn().mockRejectedValueOnce(Error('window')).mockResolvedValue(undefined);
 render(<MiniActions height={48} onResize={resize} items={[{label:'Up next',onSelect:()=>{}}]}/>);
 const opener=screen.getByRole('button',{name:'Mini Player actions'});fireEvent.click(opener);await screen.findByRole('alert');expect(screen.queryByRole('menu')).not.toBeInTheDocument();fireEvent.click(opener);await screen.findByRole('menuitem',{name:'Up next'});
});
it('does not dismiss during native window focus reconciliation',async()=>{
 const resize=vi.fn().mockResolvedValue(undefined);
 render(<MiniActions height={48} onResize={resize} items={[{label:'Up next',onSelect:()=>{}}]}/>);
 fireEvent.click(screen.getByRole('button',{name:'Mini Player actions'}));await screen.findByRole('menuitem',{name:'Up next'});
 fireEvent(window,new Event('blur'));expect(screen.getByRole('menu',{name:'Mini Player actions'})).toBeVisible();expect(resize).toHaveBeenCalledExactlyOnceWith(true);
});

it('preserves a resized player footprint while menu space opens and closes',async()=>{
 const resize=vi.fn().mockResolvedValue(undefined);
 render(<section className="mini-player"><MiniActions height={320} onResize={resize} items={[{label:'Up next',onSelect:()=>{}}]}/></section>);
 const root=document.querySelector<HTMLElement>('.mini-player')!;vi.spyOn(root,'getBoundingClientRect').mockReturnValue({height:420} as DOMRect);
 fireEvent.click(screen.getByRole('button',{name:'Mini Player actions'}));await screen.findByRole('menuitem',{name:'Up next'});
 expect(root.style.getPropertyValue('--mini-player-height')).toBe('420px');expect(screen.getByRole('menu').style.top).toBe('424px');
 fireEvent.keyDown(screen.getByRole('menuitem'),{key:'Escape'});await waitFor(()=>expect(root.style.getPropertyValue('--mini-player-height')).toBe(''));
});
it('rapid reopening keeps the footprint frozen until the last close',async()=>{
 let release:()=>void=()=>{};const resize=vi.fn(async(open:boolean)=>{if(!open)await new Promise<void>(resolve=>{release=resolve;});});
 render(<section className="mini-player"><MiniActions height={320} onResize={resize} items={[{label:'Up next',onSelect:()=>{}}]}/></section>);
 const root=document.querySelector<HTMLElement>('.mini-player')!;vi.spyOn(root,'getBoundingClientRect').mockReturnValue({height:420} as DOMRect);const opener=screen.getByRole('button',{name:'Mini Player actions'});
 fireEvent.click(opener);await screen.findByRole('menuitem');fireEvent.keyDown(screen.getByRole('menuitem'),{key:'Escape'});await waitFor(()=>expect(resize).toHaveBeenLastCalledWith(false));fireEvent.click(opener);release();await screen.findByRole('menuitem');expect(root.style.getPropertyValue('--mini-player-height')).toBe('420px');
});

it('uses available space above the controls when a tall window cannot expand further',async()=>{
 const resize=vi.fn().mockResolvedValue(undefined);render(<section className="mini-player"><MiniActions height={768} onResize={resize} items={[{label:'Up next',onSelect:()=>{}}]}/></section>);
 const root=document.querySelector<HTMLElement>('.mini-player')!;vi.spyOn(root,'getBoundingClientRect').mockReturnValue({height:768} as DOMRect);const opener=screen.getByRole('button',{name:'Mini Player actions'});vi.spyOn(opener,'getBoundingClientRect').mockReturnValue({top:680} as DOMRect);fireEvent.click(opener);const panel=await screen.findByRole('menu');expect(panel.style.top).toBe('444px');expect(panel.style.maxHeight).toBe('232px');expect(panel).toHaveAttribute('data-side','top');
});
