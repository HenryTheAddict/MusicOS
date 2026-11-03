import {cleanup,fireEvent,render,screen} from '@testing-library/react';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import VisualizerSettings from '../src/VisualizerSettings';
import {defaultVisualizer} from '../src/visualizer';
beforeEach(()=>{HTMLDialogElement.prototype.showModal=function(){this.open=true;};HTMLDialogElement.prototype.close=function(){this.open=false;};});
afterEach(cleanup);
it('nested Escape closes options without cancelling the player and restores focus',()=>{
 const outerCancel=vi.fn();render(<dialog open onCancel={outerCancel}><VisualizerSettings value={defaultVisualizer} onChange={()=>{}} fallback={false}/></dialog>);
 const trigger=screen.getByRole('button',{name:'Visualizer options'});fireEvent.click(trigger);
 const dialog=screen.getByRole('dialog',{name:'Visualizer'});expect(dialog).toHaveAttribute('open');
 fireEvent(dialog,new Event('cancel',{bubbles:true,cancelable:true}));
 expect(outerCancel).not.toHaveBeenCalled();expect(screen.queryByRole('dialog',{name:'Visualizer'})).toBeNull();expect(document.activeElement).toBe(trigger);
});
it('keeps unknown beat timing explicit and updates lyrics only on the chosen toggle',()=>{
 const change=vi.fn();render(<VisualizerSettings value={{...defaultVisualizer,rotation:'beats'}} onChange={change} fallback/>);fireEvent.click(screen.getByRole('button',{name:'Visualizer options'}));
 expect(screen.getByRole('status')).toHaveTextContent('30 seconds');expect(change).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('switch',{name:'Visualizer lyrics'}));expect(change).toHaveBeenLastCalledWith(expect.objectContaining({lyrics:true,rotation:'beats'}));
});
it('offers remembered album colors without changing preset or playback options',()=>{
 const change=vi.fn();render(<VisualizerSettings value={defaultVisualizer} onChange={change} fallback={false}/>);
 fireEvent.click(screen.getByRole('button',{name:'Visualizer options'}));
 const toggle=screen.getByRole('switch',{name:'Album colors'});expect(toggle).not.toBeChecked();
 fireEvent.click(toggle);expect(change).toHaveBeenLastCalledWith({...defaultVisualizer,album_colors:true});
});
