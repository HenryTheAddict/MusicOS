vi.mock('../src/AppleEnrichment',()=>({default:()=>null}));
import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,expect,it,vi} from 'vitest';
import type {IntegrationStatus} from '../src/types';
const mocks=vi.hoisted(()=>({active:true,call:vi.fn()}));
vi.mock('../src/ViewScope',()=>({useViewActive:()=>mocks.active}));
vi.mock('../src/components',()=>({Icon:()=>null}));
vi.mock('../src/bridge',()=>({call:mocks.call,native:true,errorText:String}));
import BetaIntegrations from '../src/BetaIntegrations';
const status:IntegrationStatus={config:{discord_enabled:false,discord_client_id:'123',lastfm_enabled:false,lastfm_username:null},discord:'Off',lastfm:'Off',queued:0};
afterEach(()=>{cleanup();mocks.active=true;mocks.call.mockReset();});
it('retains a non-secret application ID draft across inactive visits',async()=>{
 mocks.call.mockResolvedValue(status);const view=render(<BetaIntegrations/>);await waitFor(()=>expect(screen.getByLabelText('Application ID')).toHaveValue('123'));fireEvent.change(screen.getByLabelText('Application ID'),{target:{value:'987'}});mocks.active=false;view.rerender(<BetaIntegrations/>);mocks.active=true;view.rerender(<BetaIntegrations/>);await waitFor(()=>expect(screen.getByLabelText('Application ID')).toHaveValue('987'));
});
it('recovers controls when an acknowledged action finishes while its page is inactive',async()=>{
 let finish:(value:unknown)=>void=()=>{};mocks.call.mockImplementation((command:string)=>command==='integration_status'?Promise.resolve(status):new Promise(resolve=>{finish=resolve;}));const view=render(<BetaIntegrations/>);const toggle=await screen.findByRole('switch',{name:'Discord presence'});await waitFor(()=>expect(toggle).not.toBeDisabled());fireEvent.click(toggle);expect(toggle).toBeDisabled();mocks.active=false;view.rerender(<BetaIntegrations/>);finish({status:{...status,config:{...status.config,discord_enabled:true}}});await waitFor(()=>expect(mocks.call).toHaveBeenCalledWith('integration_action',expect.anything()));await new Promise(resolve=>setTimeout(resolve,0));mocks.active=true;view.rerender(<BetaIntegrations/>);await waitFor(()=>expect(toggle).not.toBeDisabled());
});
