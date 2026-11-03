import { fireEvent, render, screen, waitFor, cleanup } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import PlaybackProgress from "../src/PlaybackProgress";
import { smoothPath } from "../src/smoothCorners";
afterEach(cleanup);
test("progress fill follows real time, clamps, and resets for a new track", () => {
  const seek=vi.fn().mockResolvedValue(undefined);
  const {container,rerender}=render(<PlaybackProgress trackId={1} position={50} duration={200} seek={seek}/>);
  expect(container.querySelector(".progress-rail > span")).toHaveStyle({transform:"scaleX(0.25)"});
  expect(screen.getByRole("slider")).toHaveAttribute("aria-valuetext","0:50 of 3:20");
  rerender(<PlaybackProgress trackId={2} position={300} duration={100} seek={seek}/>);
  expect(container.querySelector(".progress-rail > span")).toHaveStyle({transform:"scaleX(1)"});
});
test('pointer scrubbing uses frozen geometry even when hover changes the visible rail',async()=>{
 const seek=vi.fn().mockResolvedValue(undefined);render(<PlaybackProgress revealTime trackId={55} live={false} position={20} duration={200} seek={seek}/>);
 const slider=screen.getByRole('slider');const rect=vi.spyOn(slider,'getBoundingClientRect').mockReturnValue({left:10,width:200} as DOMRect);
 const pointer=(type:string,x:number)=>{const event=new MouseEvent(type,{bubbles:true,clientX:x,button:0});Object.defineProperty(event,'pointerId',{value:1});fireEvent(slider,event);};
 pointer('pointerdown',60);expect(slider).toHaveValue('50');
 rect.mockReturnValue({left:10,width:100} as DOMRect);pointer('pointermove',110);expect(slider).toHaveValue('100');
 pointer('pointerup',110);await waitFor(()=>expect(seek).toHaveBeenCalledExactlyOnceWith(100));
});
test('pointer cancellation restores acknowledged progress without seeking',()=>{
 const seek=vi.fn();render(<PlaybackProgress trackId={55} live={false} position={20} duration={200} seek={seek}/>);
 const slider=screen.getByRole('slider');fireEvent.pointerDown(slider);fireEvent.change(slider,{target:{value:'100'}});fireEvent.pointerCancel(slider);
 expect(slider).toHaveValue('20');expect(seek).not.toHaveBeenCalled();
});
test("scrubbing stays steady during playback events and commits once on release", async () => {
  const seek=vi.fn().mockResolvedValue(undefined);
  const {rerender}=render(<PlaybackProgress trackId={1} position={10} duration={200} seek={seek}/>);
  const slider=screen.getByRole("slider");
  fireEvent.pointerDown(slider);fireEvent.change(slider,{target:{value:"120"}});
  rerender(<PlaybackProgress trackId={1} position={11} duration={200} seek={seek}/>);
  expect(slider).toHaveValue("120");expect(seek).not.toHaveBeenCalled();
  fireEvent.pointerUp(slider);await waitFor(()=>expect(seek).toHaveBeenCalledExactlyOnceWith(120));
});
test("seek failures are visible and empty playback is disabled", async () => {
  const seek=vi.fn().mockRejectedValue(new Error("decoder"));
  const {rerender}=render(<PlaybackProgress trackId={1} position={10} duration={200} seek={seek}/>);
  fireEvent.change(screen.getByRole("slider"),{target:{value:"20"}});
  await waitFor(()=>expect(screen.getByRole("alert")).toHaveTextContent("Could not seek"));
  rerender(<PlaybackProgress position={0} duration={0} seek={seek}/>);
  expect(screen.getByRole("slider")).toBeDisabled();
});
test("100% smooth artwork matches Sketch exported control points and scales its radius", () => {
  const path=smoothPath(128,128,16,1);
  expect(path).toContain("15.0849");expect(path).toContain("22.6274");expect(path).toContain("27.3137");expect(path).toContain("4.6863");
  expect(smoothPath(256,256,32,1)).toContain("30.1699");
  expect(smoothPath(128,128,16,.6)).not.toEqual(path);
});
test('time visibility and confirmed transition marks follow authoritative state',()=>{
 const seek=vi.fn().mockResolvedValue(undefined);
 const transition={outgoing_entry:1,incoming_entry:2,status:'preparing' as const,active:false,outgoing_cue:178,incoming_cue:0,seconds:2,tempo:1};
 const {container,rerender}=render(<PlaybackProgress trackId={1} position={60} duration={180} seek={seek} transition={transition}/>);
 expect(container.querySelector('output')).toHaveTextContent('1:00 / 3:00');expect(container.querySelector('.transition-region')).toBeNull();
 rerender(<PlaybackProgress trackId={1} position={60} duration={180} seek={seek} showTime={false} transition={{...transition,status:'scheduled'}}/>);
 expect(container.querySelector('output')).toBeNull();expect(screen.getByRole('button',{name:/DJ transition scheduled at 2:58/})).toBeInTheDocument();
 rerender(<PlaybackProgress trackId={1} position={61} duration={180} seek={seek}/>);expect(container.querySelector('.transition-region')).toBeNull();
});
