import {defaultAppearance,themePalette,readableInk,secondaryInk} from './appearance';
import type {Appearance,Preferences} from './types';
declare global {interface Window {__MUSICOS_LAUNCH_APPEARANCE__?:Appearance}}
const key='musicos-launch-appearance-v1';
function valid(value:unknown):value is Appearance {const a=value as Appearance|undefined;return !!a&&['system','light','dark'].includes(a.mode)&&['blue','artwork','custom'].includes(a.source)&&/^#[0-9a-f]{6}$/i.test(a.accent)&&Number.isFinite(a.tint)&&a.tint>=0&&a.tint<=40&&Number.isFinite(a.saturation)&&a.saturation>=0&&a.saturation<=150;}
export function launchAppearance():Appearance {
 if(valid(window.__MUSICOS_LAUNCH_APPEARANCE__))return window.__MUSICOS_LAUNCH_APPEARANCE__;
 try {
  const fixture=new URLSearchParams(location.search).has('fixture');
  const cached=JSON.parse(localStorage.getItem(fixture?'musicos-fixture-preferences':key)??'null');
  const a=fixture?cached?.appearance:cached;
  if(valid(a))return a;
  // Old fixture profiles explicitly exercise the legacy Light appearance.
  if(fixture)return {...defaultAppearance,mode:'light'};
 }catch{}
 return {...defaultAppearance};
}
export function rememberLaunchAppearance(value:Appearance){try{localStorage.setItem(key,JSON.stringify(value));}catch{}}
export function launchPreferences(defaults:Preferences):Preferences{return {...defaults,display_name:'',appearance:launchAppearance()};}
// Establish the solid palette before React enables launch interpolation. A dark
// profile must never interpolate from the registered bright-blue CSS defaults.
export function prepareLaunchTheme(){
 const look=launchAppearance(),dark=look.mode==='dark'||look.mode==='system'&&matchMedia('(prefers-color-scheme: dark)').matches;
 const palette=themePalette(look,null,dark),root=document.documentElement;
 const surface=palette?.background??'#cbf1ff',blob=palette?.blob??'#54b9ff',ink=palette?.text??'#111820',accent=palette?.accent??'#0064df';
 root.dataset.themeMode=dark?'dark':'light';
 const tokens={'--page-color':palette?.background??'#00aeff','--shell-color':palette?.blob??'#00baff','--surface-color':surface,'--blob-color':blob,'--ink':ink,'--accent':accent,'--accent-ink':readableInk(accent),'--secondary-ink':secondaryInk(ink,surface,blob,dark)};
 for(const [key,value] of Object.entries(tokens))root.style.setProperty(key,value);
 // Flush once, before any transition owner is installed; no frame loop.
 getComputedStyle(root).getPropertyValue('--shell-color');
}
