import {act,cleanup,renderHook,waitFor} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
import type {SparkState} from '../src/types';
const bridge=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/components',()=>({Icon:()=>null}));
vi.mock('../src/bridge',()=>({call:bridge.call,native:true,errorText:(e:unknown)=>String(e)}));
import {installSparkModel,useAutomaticSparkModel} from '../src/sparkModel';
import {invalidateDisplay} from '../src/displayData';
const missing={ready:false,beta:true,installed:false,staged_available:true,installation:{running:false,error:null,copied:0,total:281234043}} as SparkState;
afterEach(()=>{cleanup();bridge.call.mockReset();invalidateDisplay();});
test('concurrent feature views share one verified model installation and can retry failures',async()=>{
 let finish!:(value:SparkState)=>void;bridge.call.mockReturnValueOnce(new Promise<SparkState>(resolve=>{finish=resolve;}));
 const first=installSparkModel(),second=installSparkModel();expect(first).toBe(second);expect(bridge.call).toHaveBeenCalledTimes(1);finish({...missing,installation:{running:true,error:null,copied:0,total:281234043}});await first;
 bridge.call.mockRejectedValueOnce(Error('Not enough space'));await expect(installSparkModel()).rejects.toThrow('Not enough space');
 bridge.call.mockResolvedValueOnce({...missing,ready:true});expect((await installSparkModel()).ready).toBe(true);
});
test('an enabled visible feature installs automatically, refreshes suggestions on completion, and stops polling',async()=>{
 let installed=false,running=false;bridge.call.mockImplementation(async(_command,args)=>{if(args.request.action==='install_staged'){installed=true;running=true;return {...missing,installation:{running:true,error:null,copied:0,total:281234043}};}if(running){running=false;return {...missing,ready:true,installed:true};}return installed?{...missing,ready:true,installed:true}:missing;});
 const changed=vi.fn();window.addEventListener('musicos-display-invalidated',changed);
 const hook=renderHook(({enabled,visible})=>useAutomaticSparkModel(enabled,visible),{initialProps:{enabled:false,visible:true}});expect(bridge.call).not.toHaveBeenCalled();
 hook.rerender({enabled:true,visible:true});await waitFor(()=>expect(bridge.call.mock.calls.filter(([,args])=>args.request.action==='install_staged')).toHaveLength(1));
 await waitFor(()=>expect(hook.result.current.state?.ready).toBe(true),{timeout:2500});expect(changed.mock.calls.some(([event])=>event.detail==='playlist_suggestions')).toBe(true);
 hook.rerender({enabled:true,visible:false});const count=bridge.call.mock.calls.length;await act(async()=>new Promise(resolve=>setTimeout(resolve,50)));expect(bridge.call).toHaveBeenCalledTimes(count);window.removeEventListener('musicos-display-invalidated',changed);
});
test('cancellation and consent off do not silently restart an installation',async()=>{
 bridge.call.mockResolvedValue({...missing,installation:{running:false,copied:0,total:281234043,error:'Model installation cancelled'}});
 const hook=renderHook(()=>useAutomaticSparkModel(true,true));await waitFor(()=>expect(hook.result.current.state?.installation?.error).toContain('cancelled'));
 expect(bridge.call.mock.calls.some(([,args])=>args.request.action==='install_staged')).toBe(false);
});
test('explicit model removal disables automatic setup instead of immediately reinstalling',async()=>{
 const {render,screen,fireEvent}=await import('@testing-library/react');
 const {default:SparkSetup}=await import('../src/SparkSetup');let removed=false;
 bridge.call.mockImplementation(async(_command,args)=>{if(args.request.action==='remove_model')removed=true;return {...missing,ready:!removed,installed:!removed,analyzed:3,available:3,analysis:{running:false,paused:false,processed:0,total:3}};});
 const changed=vi.fn();render(<SparkSetup enabled onChange={changed}/>);await waitFor(()=>expect(screen.getByText('Model installed',{exact:true})).toBeVisible());fireEvent.click(screen.getByText('Model, storage & details'));fireEvent.click(screen.getByRole('button',{name:'Remove model'}));await waitFor(()=>expect(changed).toHaveBeenCalledWith(false));expect(bridge.call.mock.calls.some(([,args])=>args.request.action==='install_staged')).toBe(false);
});
