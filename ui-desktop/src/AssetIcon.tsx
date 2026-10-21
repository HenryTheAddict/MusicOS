import { useId, useLayoutEffect, useRef } from "react";
import {playLevelsMorph} from "./playLevelsMorph";
import { morphSvg } from "./svgMorph";
import { playPauseMorph } from "./playPauseMorph";

const exports = import.meta.glob("../../assets/icons/ui/*.svg", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const icons = Object.fromEntries(Object.entries(exports).map(([path, svg]) => [path.split("/").at(-1)!.replace(/\.svg$/, ""), svg.slice(svg.indexOf("<svg"))]));
icons.rowLevels = '<svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg"><g fill="var(--icon-fill,currentColor)" stroke="var(--icon-stroke,currentColor)" stroke-width="1.5"><rect x="5" y="6" width="5" height="20" rx="2"/><rect x="13.5" y="6" width="5" height="20" rx="2"/><rect x="22" y="6" width="5" height="20" rx="2"/></g></svg>';
const groups = [["play", "rowLevels"], ["play", "pause"], ["tinyPlay", "tinyPause"], ["hearted", "noheart"], ["RepeatOFF", "RepeatON", "RepeatONE"]];

// Only playback/collection controls retain shading; other UI assets stay solid.
export function solidIcon(svg:string, name:string) {
  const shaded=name==='CollectionShuffle';
  const gradients=new Set<string>();
  const flat=svg.replace(/<(?:linearGradient|radialGradient)\b[^>]*id="([^"]+)"[^>]*>[\s\S]*?<\/(?:linearGradient|radialGradient)>/g,(markup,id:string)=>{
    if(shaded){let stop=0;return markup.replace(/stop-color="[^"]+"/g,()=>`stop-color="var(--shuffle-${stop++===0?'top':'bottom'},var(--control-surface))"`);}
    gradients.add(id);return '';
  });
  let result=flat.replace(/url\(#([^)]+)\)/g,(reference,id:string)=>gradients.has(id)?'currentColor':reference);
  if(['play','pause','prev','next'].includes(name)){
    result=result.replace(/(<path\b[^>]*?)fill="#000000"/g,'$1fill="url(#transport-shade)"');
    result=result.replace(/(<svg\b[^>]*>)/,'$1<defs><linearGradient id="transport-shade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--transport-top,var(--icon-fill,currentColor))"/><stop offset="1" stop-color="var(--transport-bottom,var(--icon-fill,currentColor))"/></linearGradient></defs>');
  }
  return result;
}

export function AssetIcon({ name, className = "" }: { name: string; className?: string }) {
  const element = useRef<HTMLElement>(null);
  const previous = useRef(name);
  const prefix = useId().replace(/[^a-zA-Z0-9]/g, "");
  useLayoutEffect(() => {
    const host = element.current!;
    const source = icons[name]&&solidIcon(icons[name],name).replace(/<title\b[^>]*>[\s\S]*?<\/title>/g,'').replace(/(fill|stroke|stop-color)="(#(?:000000|000|3[Bb]3[Bb]3[Bb]|111820))"/g,(_,attribute:string)=>`${attribute}="${attribute==='fill'?'var(--icon-fill,currentColor)':'var(--icon-stroke,currentColor)'}"`);
    if (!source) { host.replaceChildren(); previous.current = name; return; }
    // Sketch exports reuse identifiers such as path-1; scope every reference.
    const counts = new Map<string, number>();
    const markup = source.replace(/id="([^"]+)"/g, (_, id: string) => {
      const occurrence = counts.get(id) ?? 0;
      counts.set(id, occurrence + 1);
      return `id="${prefix}-${id}${occurrence ? `-${occurrence}` : ""}"`;
    }).replace(/(href=")#([^"]+)/g, `$1#${prefix}-$2`).replace(/url\(#([^)]+)\)/g, `url(#${prefix}-$1)`);
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const old = previous.current;
    previous.current = name;
    let cancel = () => {};
    if(old!==name&&['ShuffleON','ShuffleOFF'].includes(old)&&['ShuffleON','ShuffleOFF'].includes(name)){
      host.innerHTML=markup;
      if(!reduced.matches){const fade=host.animate([{opacity:.5},{opacity:1}],{duration:100,easing:'ease-out'});cancel=()=>fade.cancel();}
    } else if (old !== name && host.innerHTML && !reduced.matches && groups.some(group => group.includes(old) && group.includes(name))) {
      cancel = [old,name].every(value=>value==='play'||value==='rowLevels') ? playLevelsMorph(host,name==='rowLevels',markup)
        : [old,name].every(value=>value==='play'||value==='pause') ? playPauseMorph(host,name==='pause',markup)
        : morphSvg(host, host.innerHTML, markup);
    } else {host.innerHTML = markup;host.dataset.morphProgress=String(name==='rowLevels'?1:0);host.dataset.transportMorphProgress=String(name==='pause'?1:0);}
    const finish = () => { if (reduced.matches) { cancel(); host.innerHTML = markup;host.dataset.morphProgress=String(name==='rowLevels'?1:0);host.dataset.transportMorphProgress=String(name==='pause'?1:0); } };
    reduced.addEventListener("change", finish);
    return () => { cancel(); reduced.removeEventListener("change", finish); };
  }, [name, prefix]);
  return <i ref={element} className={`icon asset-icon ${className}`} data-icon={name} aria-hidden="true" />;
}
