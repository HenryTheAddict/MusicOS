import { useLayoutEffect } from "react";
import { getSvgPath } from "figma-squircle";

// Values read from musicosdesk.sketch: panels 16px / .6, artwork 16px / 1.
// The generator's 128px, r16, smoothing1 outline matches the exported Sketch
// Bézier control points (15.084945, 22.627417, 27.313708, 4.6862915).
const surfaces = ".app-header > .menu-trigger, .now-playing, .library-sidebar, .library-toolbar, .search, .page-frame, .card-artwork, .cover, .genre-art, .settings-tile, .beta-integrations, .integration-card, .album-strip, .artist-about, .personal-library, .library-shortcuts, .library-sidebar nav button, .personal-library > button, .sketch-button, .puffy-toggle, .puffy-toggle > button, .personal-collection";
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
    const watched=new Set<HTMLElement>();
    const apply=(node:HTMLElement) => {
      const style=getComputedStyle(node);
      const width=node.offsetWidth, height=node.offsetHeight;
      if (!width || !height) return;
      const raw=style.borderTopLeftRadius;
      const radius=raw.endsWith("%") ? Math.min(width,height)*parseFloat(raw)/100 : parseFloat(raw);
      const smoothing=node.matches(".portrait") ? .6 : node.matches(".cover, .genre-art, .card-artwork") ? 1 : .6;
      const path=smoothPath(width,height,radius,smoothing);
      node.style.clipPath=`path('${path}')`;
      node.dataset.cornerSmoothing=String(smoothing);
    };
    const resize=new ResizeObserver(entries=>entries.forEach(e=>apply(e.target as HTMLElement)));
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
    const scale=new MutationObserver(()=>watched.forEach(apply));
    scale.observe(document.documentElement,{attributes:true,attributeFilter:["style"]});
    return ()=>{resize.disconnect();changes.disconnect();scale.disconnect();watched.forEach(node=>{node.style.clipPath="";delete node.dataset.cornerSmoothing;});};
  }, []);
}
