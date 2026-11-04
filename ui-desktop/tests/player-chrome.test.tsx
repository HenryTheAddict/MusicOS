import {act,render,screen,fireEvent,cleanup} from '@testing-library/react';
import {useRef} from 'react';
import {afterEach,expect,it,vi} from 'vitest';
import {usePlayerChrome} from '../src/usePlayerChrome';
function Player(){const ref=useRef<HTMLDialogElement>(null),chrome=usePlayerChrome(ref,true);return <dialog open ref={ref} data-chrome={chrome}><header className="expanded-player-header"><button>Close</button></header><button>Timed line</button></dialog>;}
afterEach(()=>{cleanup();vi.useRealTimers();delete document.documentElement.dataset.inputModality;});
it('one idle timeout releases on close, and keyboard focus keeps controls available',()=>{
 vi.useFakeTimers();document.documentElement.dataset.inputModality='pointer';const view=render(<Player/>),dialog=screen.getByRole('dialog');
 act(()=>vi.advanceTimersByTime(2200));expect(dialog).toHaveAttribute('data-chrome','hidden');expect(vi.getTimerCount()).toBe(0);
 fireEvent.pointerMove(dialog);expect(dialog).toHaveAttribute('data-chrome','visible');
 document.documentElement.dataset.inputModality='keyboard';act(()=>screen.getByText('Close').focus());act(()=>vi.advanceTimersByTime(2200));expect(dialog).toHaveAttribute('data-chrome','visible');expect(vi.getTimerCount()).toBe(0);
 fireEvent.keyDown(dialog,{key:'Tab'});expect(vi.getTimerCount()).toBe(1);view.unmount();expect(vi.getTimerCount()).toBe(0);
});

it('native keyboard actions reveal controls and release their one idle timeout',()=>{vi.useFakeTimers();const view=render(<Player/>);const dialog=screen.getByRole('dialog');expect(dialog).toHaveAttribute('data-chrome','hidden');act(()=>window.dispatchEvent(new Event('musicos-player-interaction')));expect(dialog).toHaveAttribute('data-chrome','visible');act(()=>vi.advanceTimersByTime(2200));expect(dialog).toHaveAttribute('data-chrome','hidden');expect(vi.getTimerCount()).toBe(0);view.unmount();});

it('controller interaction reveals chrome and controller focus holds it without a frame loop',()=>{vi.useFakeTimers();document.documentElement.dataset.controllerInput='true';const view=render(<Player/>),dialog=screen.getByRole('dialog');act(()=>window.dispatchEvent(new Event('musicos-player-interaction')));act(()=>screen.getByText('Close').focus());act(()=>vi.advanceTimersByTime(2200));expect(dialog).toHaveAttribute('data-chrome','visible');expect(vi.getTimerCount()).toBe(0);view.unmount();delete document.documentElement.dataset.controllerInput;expect(vi.getTimerCount()).toBe(0);});
