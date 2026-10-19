import { useLayoutEffect } from "react";
import { getSvgPath } from "figma-squircle";

// Values read from musicosdesk.sketch: panels 16px / .6, artwork 16px / 1.
// The generator's 128px, r16, smoothing1 outline matches the exported Sketch
// Bézier control points (15.084945, 22.627417, 27.313708, 4.6862915).
const surfaces = ".section-control > .menu-trigger:first-child, .now-playing, .library-toolbar, .search, .card-artwork, .mini-player-art, .album-artwork, .viewer-artwork, .artist-portrait-button, .cover, .genre-art, .album-strip, .artist-about, .library-shortcuts, .library-sidebar nav button, .personal-library > button, .sketch-button, .puffy-toggle, .puffy-toggle > button, .personal-collection";
const paths = new Map<string, string>();
export function smoothPath(width: number, height: number, radius: number, smoothing: number) {
  const key = `${width}:${height}:${radius}:${smoothing}`;
  let path = paths.get(key);
  if (!path) {
    path = getSvgPath({width,height,cornerRadius:radius,cornerSmoothing:smoothing});
    if (paths.size >= 256) paths.delete(paths.keys().next().value!);
    paths.set(key,path);
  }
  return path;
}
export function useSmoothCorners() {
  useLayoutEffect(() => {
    const watched=new Set<HTMLElement>(),pending=new Set<HTMLElement>();let frame=0;
    const apply=(node:HTMLElement) => {
      // A nested image inherits its framing surface. Clipping both antialiased
      // edges composites the backing twice, especially visibly on dark themes.
      if(node.matches('.cover')&&node.parentElement?.matches('.card-artwork,.mini-player-art,.album-artwork,.viewer-artwork,.artist-portrait-button')){node.style.clipPath='none';node.style.background='transparent';return;}
      const style=getComputedStyle(node);
      const width=node.offsetWidth, height=node.offsetHeight;
      if (!width || !height) return;
      const raw=style.borderTopLeftRadius;
      const radius=raw.endsWith("%") ? Math.min(width,height)*parseFloat(raw)/100 : parseFloat(raw);
      const smoothing=node.matches(".portrait,.artist-portrait-button") ? .6 : node.matches(".cover,.genre-art,.card-artwork,.mini-player-art,.album-artwork,.viewer-artwork") ? 1 : .6;
      const path=smoothPath(width,height,radius,smoothing);
      const clip=`path('${path}')`;if(node.style.clipPath!==clip)node.style.clipPath=clip;
      node.dataset.cornerSmoothing=String(smoothing);
    };
    const schedule=(node:HTMLElement)=>{pending.add(node);if(!frame)frame=requestAnimationFrame(()=>{frame=0;for(const item of pending)if(item.isConnected)apply(item);pending.clear();});};
    const resize=new ResizeObserver(entries=>entries.forEach(e=>schedule(e.target as HTMLElement)));
    const scan=()=>{
      for(const node of watched) if(!node.isConnected){resize.unobserve(node);watched.delete(node);}
      document.querySelectorAll<HTMLElement>(surfaces).forEach(node=>{if(!watched.has(node)){watched.add(node);resize.observe(node);apply(node);}});
    };
    scan();
    // SVG morphs replace paths each frame. They are not new rounded surfaces.
    const changes=new MutationObserver(records=>{
      if(records.some(r=>[...r.addedNodes,...r.removedNodes].some(n=>n instanceof Element&&(n.matches(surfaces)||n.querySelector(surfaces)))))scan();
    });
    changes.observe(document.body,{subtree:true,childList:true});
    // Interface-size changes alter the radii as well as dimensions.
    let fontSize=document.documentElement.style.fontSize;
    const scale=new MutationObserver(()=>{const next=document.documentElement.style.fontSize;if(next!==fontSize){fontSize=next;watched.forEach(schedule);}});
    scale.observe(document.documentElement,{attributes:true,attributeFilter:["style"]});
    return ()=>{cancelAnimationFrame(frame);pending.clear();resize.disconnect();changes.disconnect();scale.disconnect();watched.forEach(node=>{node.style.clipPath="";delete node.dataset.cornerSmoothing;});};
  }, []);
}
