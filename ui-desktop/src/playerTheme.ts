import type {CSSProperties} from 'react';
import {readableInk,contrast,luminance,secondaryInk} from './appearance';
import type {Palette} from './types';
const mix=(a:string,b:string,t:number)=>'#'+[1,3,5].map(i=>Math.round(parseInt(a.slice(i,i+2),16)*(1-t)+parseInt(b.slice(i,i+2),16)*t).toString(16).padStart(2,'0')).join('');
/** Retain the design's hue in both tones while bounding their surface contrast. */
export function transportTones(palette:Palette|null){
 const surface=palette?.background??'#f7fbff',accent=palette?.accent??'#0064df',ink=palette?.text??'#12202a';
 let fill=mix(ink,accent,.34),stroke=mix(surface,accent,.48);
 for(let i=0;i<40&&contrast(fill,surface)<4.5;i++)fill=mix(fill,ink,.12);
 for(let i=0;i<40&&contrast(stroke,surface)<3;i++)stroke=mix(stroke,ink,.08);
 // Avoid two nearly identical colors when an extreme custom accent is used.
 for(let i=0;i<30&&contrast(fill,stroke)<1.4;i++)fill=mix(fill,ink,.12);
 return {fill,stroke};
}
/** A player owns its palette; browsing a different release cannot recolor it. */
export function playerTheme(palette:Palette|null):CSSProperties{
 const accent=palette?.accent??'#0064df',ink=palette?.text??'#12202a',tones=transportTones(palette);
 const surface=palette?.background??'#f7fbff',blob=palette?.blob??'#e6f1fc';
 const dark=luminance(surface)<.18;
 return {
  colorScheme:dark?'dark':'light',
  '--surface-color':surface,
  '--blob-color':blob,
  '--accent':accent,'--accent-ink':readableInk(accent),'--ink':ink,
  '--playback-tone':accent,'--icon-fill':ink,'--icon-stroke':ink,
  '--transport-fill':tones.fill,'--transport-stroke':tones.stroke,
  '--transport-top':mix(tones.fill,ink,.08),'--transport-bottom':tones.fill,
  '--control-surface':'color-mix(in srgb,var(--surface-color) 78%,var(--blob-color))',
  '--control-border':'color-mix(in srgb,var(--ink) 18%,transparent)',
  '--secondary-ink':secondaryInk(ink,surface,blob,dark),
  color:ink,background:surface,
 } as CSSProperties;
}
