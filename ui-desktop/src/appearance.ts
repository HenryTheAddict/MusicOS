import type {Appearance,Palette,Preferences} from './types';
export const defaultAppearance:Appearance={mode:'system',source:'blue',accent:'#0064df',saturation:100,tint:15};
export function appearance(p:Preferences):Appearance{return p.appearance??{...defaultAppearance,mode:'light',source:p.coverflow_colors?'artwork':'blue'};}
const rgb=(hex:string)=>[1,3,5].map(i=>parseInt(hex.slice(i,i+2),16));
const hex=(values:number[])=>'#'+values.map(v=>Math.round(Math.max(0,Math.min(255,v))).toString(16).padStart(2,'0')).join('');
const mix=(a:string,b:string,amount:number)=>hex(rgb(a).map((v,i)=>v*(1-amount)+rgb(b)[i]*amount));
export function luminance(color:string){const c=rgb(color).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;});return c[0]*.2126+c[1]*.7152+c[2]*.0722;}
export function contrast(a:string,b:string){const values=[luminance(a),luminance(b)].sort((a,b)=>b-a);return (values[0]+.05)/(values[1]+.05);}
export function readableInk(color:string){return contrast(color,'#000000')>=contrast(color,'#ffffff')?'#000000':'#ffffff';}
function saturate(color:string,amount:number){const c=rgb(color),mid=(Math.max(...c)+Math.min(...c))/2;return hex(c.map(v=>mid+(v-mid)*amount));}
const colored=(color:string)=>{const c=rgb(color),hi=Math.max(...c),difference=hi-Math.min(...c);return difference>=12&&difference/Math.max(32,hi)>=.15;};
// Native palettes retain a deliberately dark canonical accent. Use its hue
// at a useful control luminance without changing saved artwork colors.
function artworkAccent(color:string,dark:boolean){
 const channels=rgb(color),hi=Math.max(...channels),lo=Math.min(...channels),chroma=hi-lo;
 if(!chroma)return dark?'#b7bac3':'#535965';
 const ceiling=dark?220:168,saturation=Math.min(.72,chroma/Math.max(1,hi)),floor=ceiling*(1-saturation);
 return hex(channels.map(channel=>floor+(channel-lo)/chroma*(ceiling-floor)));
}
export function themePalette(a:Appearance,art:Palette|null,systemDark:boolean):Palette|null{
 const dark=a.mode==='dark'||(a.mode==='system'&&systemDark);if(!dark&&(a.source==='blue'||(a.source==='artwork'&&!art))&&a.saturation===100&&a.tint===15)return null;
 const artworkColor=art?[art.accent,art.blob].find(colored):null;
 // Muted artwork still supplies its own hue. Falling back to the original
 // blue here gave green/gray artwork an unrelated saturated blue accent.
 const artworkBase=artworkColor??art?.accent;
 const base=a.source==='custom'?a.accent:a.source==='artwork'&&artworkBase?artworkBase:'#0064df';
 let color=a.source==='artwork'&&artworkBase?artworkAccent(saturate(base,a.saturation/100),dark):saturate(base,a.saturation/100),ink=dark?'#dedfe3':'#12202a';
 let accent=color;
 if(dark){
  // Color lives in the solid surfaces, gently bounded even at high saturation.
  // Monochrome artwork and a zero tint retain the neutral charcoal foundation.
  const tint=Math.max(0,Math.min(40,a.tint))/100;
  const tone=a.source==='artwork'&&!artworkColor?null:color;
  let background=tone?mix('#202124',tone,tint*.45):'#202124';
  let blob=tone?mix('#2b2d32',tone,tint*.5):'#2b2d32';
  for(let n=0;n<30&&contrast(ink,background)<7;n++)background=mix(background,'#202124',.08);
  for(let n=0;n<30&&contrast(ink,blob)<7;n++)blob=mix(blob,'#202124',.08);
  for(let n=0;n<30&&Math.min(contrast(accent,background),contrast(accent,blob))<4.5;n++)accent=mix(accent,'#ffffff',.08);
  return {background,blob,accent,text:ink};
 }
 for(let n=0;n<30&&contrast(accent,'#ffffff')<4.5;n++)accent=mix(accent,'#000000',.06);
 // Artwork supplies a tint, not the entire window paint. Bound large Light
 // surfaces even when the cover itself contains luminous, saturated colors.
 const artwork=a.source==='artwork'&&art;
 const artBackground=artwork?saturate(artwork.background,a.saturation/100):null;
 const artSurface=artwork?saturate(artwork.blob,a.saturation/100):null;
 let background=artBackground&&artSurface?mix('#f7f9fc',artBackground,a.tint/100*.6):mix('#f7f9fc',color,a.tint/100);
 // Reserve a little contrast through perceptual color interpolation, whose
 // midpoint can be darker than either of two differently hued surfaces.
 const surfaceContrast=a.source==='artwork'?4.8:4.5;
 for(let n=0;n<30&&contrast(background,ink)<surfaceContrast;n++)background=mix(background,'#ffffff',.03);
 let blob=artSurface?mix('#e6ebf4',artSurface,a.tint/100*.9):mix('#e6ebf4',color,Math.min(.6,.2+a.tint/100));
 for(let n=0;n<30&&contrast(blob,ink)<surfaceContrast;n++)blob=mix(blob,'#ffffff',.03);
 return {background,blob,accent,text:ink};
}

/** Keep secondary copy readable on both page and raised solid surfaces. */
export function secondaryInk(ink:string,surface:string,blob:string,dark=false){const controls=dark?mix(blob,ink,.08):mix(surface,blob,.22);for(let amount=.18;amount>=0;amount-=.01){const candidate=mix(ink,surface,Math.max(0,amount));if([surface,blob,controls].every(background=>contrast(candidate,background)>=4.5))return candidate;}return ink;}
