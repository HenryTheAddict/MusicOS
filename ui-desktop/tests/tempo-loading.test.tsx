import {renderHook,act,waitFor,cleanup} from '@testing-library/react';
import {afterEach,describe,it,expect,vi} from 'vitest';
import useAnalyzedTempos from '../src/useAnalyzedTempos';
import {call} from '../src/bridge';
vi.mock('../src/bridge',()=>({call:vi.fn()}));
afterEach(()=>{cleanup();vi.useRealTimers();vi.clearAllMocks();});
describe('tempo display loading',()=>{
 it('uses one lightweight request when the library and playing song arrive together',async()=>{
  vi.mocked(call).mockResolvedValue({running:false,tempos:[{track_id:1,bpm:120}]} as never);
  const {result}=renderHook(()=>useAnalyzedTempos(true,'library',1));await waitFor(()=>expect(result.current.get(1)).toBe(120));expect(call).toHaveBeenCalledTimes(1);expect(call).toHaveBeenCalledWith('audio_tempos');
 });
 it('recovers from an initial busy response without needing a rescan',async()=>{
  vi.useFakeTimers();vi.mocked(call).mockRejectedValueOnce(Error('busy')).mockResolvedValue({running:false,tempos:[{track_id:1,bpm:143.6}]} as never);
  const {result}=renderHook(()=>useAnalyzedTempos(true,'library',1));await act(async()=>{});expect(result.current.size).toBe(0);await act(async()=>{await vi.advanceTimersByTimeAsync(1000);});expect(result.current.get(1)).toBe(143.6);expect(call).toHaveBeenCalledTimes(2);
 });
 it('preserves valid tempos while a refresh fails and cancels retries on unmount',async()=>{
  vi.useFakeTimers();vi.mocked(call).mockResolvedValueOnce({running:false,tempos:[{track_id:1,bpm:120}]} as never).mockRejectedValue(Error('busy'));
  const {result,unmount}=renderHook(()=>useAnalyzedTempos(true,'library',1));await act(async()=>{});await act(async()=>{window.dispatchEvent(new Event('musicos-audio-rescanned'));});expect(result.current.get(1)).toBe(120);unmount();await act(async()=>{await vi.runAllTimersAsync();});expect(call).toHaveBeenCalledTimes(2);
 });
});
