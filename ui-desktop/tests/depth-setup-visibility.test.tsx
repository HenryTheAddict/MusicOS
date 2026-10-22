import {act,cleanup,render} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
const bridge=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/bridge',()=>({call:bridge.call,errorText:String,native:true}));
vi.mock('../src/AssetIcon',()=>({AssetIcon:()=>null}));
import DepthSetup from '../src/DepthSetup';
import {KeepView} from '../src/ViewScope';
import {invalidateDisplay} from '../src/displayData';
const status={ready:true,supported:true,running:false,downloaded:0,total:100,error:null};
afterEach(()=>{cleanup();invalidateDisplay();bridge.call.mockReset();vi.useRealTimers();});
test('idle and hidden artwork setup do not keep requesting status',async()=>{
 vi.useFakeTimers();bridge.call.mockResolvedValue(status);
 const view=render(<KeepView active><DepthSetup enabled onChange={()=>{}}/></KeepView>);
 await act(async()=>{});expect(bridge.call).toHaveBeenCalledTimes(1);
 await act(async()=>vi.advanceTimersByTimeAsync(10000));expect(bridge.call).toHaveBeenCalledTimes(1);
 view.rerender(<KeepView active={false}><DepthSetup enabled onChange={()=>{}}/></KeepView>);
 await act(async()=>vi.advanceTimersByTimeAsync(10000));expect(bridge.call).toHaveBeenCalledTimes(1);
 view.rerender(<KeepView active><DepthSetup enabled onChange={()=>{}}/></KeepView>);
 await act(async()=>{});expect(bridge.call).toHaveBeenCalledTimes(2);
});
test('visible download refreshes until completion and then stops',async()=>{
 vi.useFakeTimers();bridge.call.mockResolvedValueOnce({...status,ready:false,running:true}).mockResolvedValue(status);
 render(<DepthSetup enabled={false} onChange={()=>{}}/>);await act(async()=>{});
 await act(async()=>vi.advanceTimersByTimeAsync(1500));expect(bridge.call).toHaveBeenCalledTimes(2);
 await act(async()=>vi.advanceTimersByTimeAsync(10000));expect(bridge.call).toHaveBeenCalledTimes(2);
});
