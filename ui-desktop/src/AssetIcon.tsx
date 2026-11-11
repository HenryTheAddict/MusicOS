import {playerViewMorph,resetCycleShape,type CycleGlyph} from './playerViewMorph';
import {shuffleMorph} from './shuffleMorph';
import {menuDotsMorph} from './menuDotsMorph';
import { useId, useLayoutEffect, useRef } from "react";
import {playLevelsMorph} from "./playLevelsMorph";
import { morphSvg } from "./svgMorph";
import {menuCrossMorph} from "./menuCrossMorph";
import { playPauseMorph } from "./playPauseMorph";

const exports = import.meta.glob("../../assets/icons/ui/*.svg", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const icons = Object.fromEntries(Object.entries(exports).map(([path, svg]) => [path.split("/").at(-1)!.replace(/\.svg$/, ""), svg.slice(svg.indexOf("<svg"))]));
icons.menuPlus = icons.Plus;
icons.playerQueue=icons.queue;icons.playerLyrics=icons.Lyrics.replace('viewBox="0 0 32 32"','viewBox="8 7 16 16"');icons.playerPlaying=icons.Song;
const cycleNames=['playerQueue','playerLyrics','playerPlaying'];
icons.controlClose=icons.tinyX.replace('viewBox="0 0 8 8"','viewBox="-2 -2 12 12"').replace(/#979797/g,'var(--icon-stroke,currentColor)');
icons.menuDotsClose=icons.controlClose;
icons.menuClose=icons.menuPlus.replace('<g ', '<g transform="rotate(45 16 16)" ');
icons.rowLevels = '<svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg"><g fill="var(--icon-fill,currentColor)" stroke="var(--icon-stroke,currentColor)" stroke-width="1.5"><rect x="5" y="6" width="5" height="20" rx="2"/><rect x="13.5" y="6" width="5" height="20" rx="2"/><rect x="22" y="6" width="5" height="20" rx="2"/></g></svg>';
const groups = [["queue","controlClose"],["3dots_(nocircle)","menuDotsClose"],["3dots(incircle)","menuDotsClose"],["menuPlus","menuClose"],["play", "pause", "rowLevels"], ["play", "pause"], ["tinyPlay", "tinyPause"], ["hearted", "noheart"], ["RepeatOFF", "RepeatON", "RepeatONE"]];

// Only playback/collection controls retain shading; other UI assets stay solid.
export function solidIcon(svg:string, name:string) {
  const shaded=name==='CollectionShuffle';
  const gradients=new Set<string>();
  const flat=svg.replace(/<(?:linearGradient|radialGradient)\b[^>]*id="([^"]+)"[^>]*>[\s\S]*?<\/(?:linearGradient|radialGradient)>/g,(markup,id:string)=>{
    if(shaded){let stop=0;return markup.replace(/stop-color="[^"]+"/g,()=>`stop-color="var(--shuffle-${stop++===0?'top':'bottom'},var(--control-surface))"`);}
    gradients.add(id);return '';
  });
  // This export includes Sketch's white canvas as a <use>, not icon geometry.
  // Keep the authored strokes and play glyph transparent in menus.
  const canvasFree=['ShufflePlay','noARTIST'].includes(name)?flat.replace(/<use\b[^>]*fill="#FFFFFF"[^>]*><\/use>/g,''):flat;
  let result=canvasFree.replace(/url\(#([^)]+)\)/g,(reference,id:string)=>gradients.has(id)?'currentColor':reference);
  if(name==='ShuffleON'||name==='ShuffleOFF')result=result.replace(/stroke="#3B3B3B"/g,'stroke="var(--icon-fill,var(--secondary-ink,currentColor))"');
  if(['play','pause','prev','next'].includes(name)){
    result=result.replace(/(<path\b[^>]*?)fill="#000000"/g,'$1fill="url(#transport-shade)"');
    result=result.replace(/(<svg\b[^>]*>)/,'$1<defs><linearGradient id="transport-shade" x1="0" y1="0" x2="0" y2="32" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="var(--transport-top,var(--icon-fill,currentColor))"/><stop offset="1" stop-color="var(--transport-bottom,var(--icon-fill,currentColor))"/></linearGradient></defs>');
  }
  if(name==='tinyX'||name==='tinyMinimize')result=result.replace(/stroke="#979797"/g,'stroke="var(--icon-stroke,currentColor)"');
  if(name==='Library')result=result.replace(/fill="#FFFFFF"/g,'fill="var(--icon-surface,var(--control-surface))"');
  return result;
}

// The image fallback keeps Sketch's supplied path coordinates, without its
// white export canvas. Its surrounding surface is owned by the active theme.
export const portraitPlaceholder='data:image/svg+xml,'+encodeURIComponent(solidIcon(icons.noARTIST,'noARTIST'));

export function AssetIcon({ name, className = "", motion = true }: { name: string; className?: string; motion?: boolean }) {
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
    if(old!==name&&cycleNames.includes(old)&&cycleNames.includes(name)){if(!motion||reduced.matches){host.innerHTML=markup;resetCycleShape(host,name as CycleGlyph);}else cancel=playerViewMorph(host,old as CycleGlyph,name as CycleGlyph,markup);
    }else if(old!==name&&['ShuffleON','ShuffleOFF'].includes(old)&&['ShuffleON','ShuffleOFF'].includes(name)){
      if(!motion||reduced.matches){host.innerHTML=markup;host.dataset.shuffleProgress=String(name==='ShuffleON'?1:0);}else cancel=shuffleMorph(host,name==='ShuffleON',markup);
    } else if (old !== name && host.innerHTML && motion && !reduced.matches && groups.some(group => group.includes(old) && group.includes(name))) {
      cancel = [old,name].every(value=>['3dots_(nocircle)','3dots(incircle)','menuDotsClose'].includes(value)) ? menuDotsMorph(host,name==='menuDotsClose',markup)
        : [old,name].every(value=>value==='menuPlus'||value==='menuClose') ? menuCrossMorph(host,name==='menuClose',markup)
        : [old,name].includes('rowLevels') ? playLevelsMorph(host,name==='rowLevels',markup,(name==='pause'||old==='pause')?'pause':'play')
        : [old,name].every(value=>value==='play'||value==='pause') ? playPauseMorph(host,name==='pause',markup)
        : morphSvg(host, host.innerHTML, markup, ['queue','controlClose','menuDotsClose','tinyX','menuPlus','3dots_(nocircle)','3dots(incircle)'].includes(name)?140:180);
    } else {host.innerHTML = markup;if(cycleNames.includes(name))resetCycleShape(host,name as CycleGlyph);host.dataset.morphProgress=String(name==='rowLevels'?1:0);host.dataset.transportMorphProgress=String(name==='pause'?1:0);host.dataset.crossProgress=String(name==='menuClose'?1:0);host.dataset.dotsProgress=String(name==='menuDotsClose'?1:0);host.dataset.shuffleProgress=String(name==='ShuffleON'?1:0);}
    const finish = () => { if (!motion || reduced.matches) { cancel(); host.innerHTML = markup;if(cycleNames.includes(name))resetCycleShape(host,name as CycleGlyph);host.dataset.morphProgress=String(name==='rowLevels'?1:0);host.dataset.transportMorphProgress=String(name==='pause'?1:0);host.dataset.crossProgress=String(name==='menuClose'?1:0);host.dataset.dotsProgress=String(name==='menuDotsClose'?1:0);host.dataset.shuffleProgress=String(name==='ShuffleON'?1:0); } };
    reduced.addEventListener("change", finish);
    return () => { cancel(); reduced.removeEventListener("change", finish); };
  }, [name, prefix, motion]);
  return <i ref={element} className={`icon asset-icon ${className}`} data-icon={name} aria-hidden="true" />;
}
