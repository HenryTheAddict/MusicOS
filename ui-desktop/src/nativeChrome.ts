import { useLayoutEffect } from "react";
import { call, native } from "./bridge";
export function useNativeChrome(enabled=true){
  useLayoutEffect(()=>{
    if(!native||!enabled)return;
    const region=document.querySelector<HTMLElement>(".window-drag-region");if(!region)return;
    let frame=0;
    const update=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>{const box=region.getBoundingClientRect();void call("sync_window_chrome",{centerY:box.top+box.height/2}).catch(()=>{});});};
    // Do not also configure Tauri's static trafficLightPosition: TAO reapplies
    // that inset on every draw and would overwrite this scaled alignment.
    const observer=new ResizeObserver(update);observer.observe(region);
    window.addEventListener("resize",update);window.addEventListener("focus",update);update();
    return ()=>{cancelAnimationFrame(frame);observer.disconnect();window.removeEventListener("resize",update);window.removeEventListener("focus",update);};
  },[enabled]);
}
