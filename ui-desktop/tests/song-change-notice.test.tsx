import {act,cleanup,render,screen} from '@testing-library/react';
import {afterEach,expect,it,vi} from 'vitest';
import SongChangeNotice from '../src/SongChangeNotice';
import type {Track} from '../src/types';
afterEach(()=>{cleanup();vi.useRealTimers();});
it('announces acknowledged entry changes once, including duplicates, and releases timers',()=>{
 vi.useFakeTimers();const track={id:1,title:'First song',artist:'Artist'} as Track;
 const {rerender,unmount}=render(<SongChangeNotice track={track} entryId={1} active/>);
 expect(screen.getByRole('status')).not.toHaveAttribute('data-visible');
 rerender(<SongChangeNotice track={track} entryId={2} active/>);expect(screen.getByRole('status')).toHaveAttribute('data-visible','true');
 act(()=>vi.advanceTimersByTime(1000));rerender(<SongChangeNotice track={{...track}} entryId={2} active/>);
 act(()=>vi.advanceTimersByTime(1200));expect(screen.getByRole('status')).not.toHaveAttribute('data-visible');
 act(()=>vi.advanceTimersByTime(180));expect(vi.getTimerCount()).toBe(0);
 rerender(<SongChangeNotice track={track} entryId={3} active/>);unmount();expect(vi.getTimerCount()).toBe(0);
});
