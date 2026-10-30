import {render,screen,waitFor,cleanup} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {test,expect,vi,beforeEach,afterEach} from 'vitest';
vi.mock('../src/components',()=>({Icon:()=>null}));
import Onboarding from '../src/Onboarding';
import {defaultPreferences,emptyTask} from '../src/bridge';
beforeEach(()=>{Object.defineProperty(window,'matchMedia',{value:vi.fn(()=>({matches:false,addEventListener:vi.fn(),removeEventListener:vi.fn()})),configurable:true});HTMLDialogElement.prototype.showModal=function(){this.setAttribute('open','')};HTMLDialogElement.prototype.close=function(){this.removeAttribute('open')};HTMLElement.prototype.animate=vi.fn(()=>({finished:Promise.resolve(),cancel:vi.fn(),playState:'finished'})) as any;});
afterEach(cleanup);
test('optional identity, canceled picker and acknowledged save retain chosen settings',async()=>{
 const finish=vi.fn().mockResolvedValue(true),pick=vi.fn().mockResolvedValue([]);const user=userEvent.setup();render(<Onboarding preferences={defaultPreferences} task={emptyTask} songs={0} onImport={pick} onFinish={finish} onDismiss={()=>{}}/>);
 await user.click(screen.getByRole('button',{name:'Make it mine'}));await user.type(screen.getByRole('textbox'), '  Zoë  ');await user.click(screen.getByRole('button',{name:'Dark'}));await user.click(screen.getByRole('button',{name:'Continue'}));await user.click(screen.getByRole('button',{name:/Choose music folders/}));await waitFor(()=>expect(pick).toHaveBeenCalledTimes(1));expect(screen.queryByRole('alert')).toBeNull();await user.click(screen.getByRole('button',{name:'Open my library'}));await waitFor(()=>expect(finish).toHaveBeenCalledWith(expect.objectContaining({display_name:'Zoë',onboarding_version:1,appearance:expect.objectContaining({mode:'dark'})}),false));
});
test('failed persistence stays open with a retry and never imports automatically',async()=>{
 const finish=vi.fn().mockResolvedValueOnce(false).mockResolvedValue(true),pick=vi.fn();const user=userEvent.setup();render(<Onboarding preferences={defaultPreferences} task={emptyTask} songs={0} onImport={pick} onFinish={finish} onDismiss={()=>{}}/>);await user.click(screen.getByRole('button',{name:'Use defaults'}));expect(await screen.findByRole('alert')).toHaveTextContent('could not be saved');expect(pick).not.toHaveBeenCalled();await user.click(screen.getByRole('button',{name:'Use defaults'}));await waitFor(()=>expect(finish).toHaveBeenCalledTimes(2));
});
