import {useEffect,useRef} from 'react';
import {transportTones} from './playerTheme';
import {resolveArtworkTheme} from './artworkTheme';
type Palette={tone:string;stops:string[];surfaces:string[]};
const colors=new Map<string,Palette>();
// Calibrate against the white button surface, independently of the app palette.
export function collectionPalette(pixels:ArrayLike<number>):Palette {
 const buckets=new Map<number,{rgb:number[];weight:number}>();
 for(let i=0;i+3<pixels.length;i+=4){
  if(pixels[i+3]<128)continue;
  const rgb=[pixels[i],pixels[i+1],pixels[i+2]],hi=Math.max(...rgb),lo=Math.min(...rgb);
  if(hi<24||lo>235)continue;
  const key=rgb.reduce((n,v)=>n*8+Math.floor(v/32),0),weight=1+(hi-lo)/64;
  const old=buckets.get(key);if(old){old.rgb=old.rgb.map((v,j)=>v+rgb[j]*weight);old.weight+=weight;}else buckets.set(key,{rgb:rgb.map(v=>v*weight),weight});
 }
 const ranked=[...buckets.values()].sort((a,b)=>b.weight-a.weight);
 const chromatic=ranked.filter(b=>{const rgb=b.rgb.map(v=>v/b.weight);return Math.max(...rgb)-Math.min(...rgb)>=35&&b.weight>=(ranked[0]?.weight??0)*.15;});
 const candidates=[...chromatic,...ranked.filter(b=>!chromatic.includes(b))].map(b=>b.rgb.map(v=>v/b.weight));
 const selected:number[][]=[];
 for(const rgb of candidates){if(!selected.length||selected.every(other=>Math.hypot(...rgb.map((v,i)=>v-other[i]))>65))selected.push(rgb);if(selected.length===3)break;}
 if(!selected.length)selected.push([40,65,70]);
 while(selected.length<3)selected.push(selected[selected.length-1]);
 const stops=selected.map(rgb=>{const scale=Math.min(1,112/Math.max(...rgb));return `rgb(${rgb.map(v=>Math.round(v*scale)).join(' ')})`;});
 return {tone:stops[0],stops,surfaces:selected.map(rgb=>`rgb(${rgb.map(Math.round).join(' ')})`)};
}
export function useCollectionTone(spark:boolean,identity=''){
 const ref=useRef<HTMLSpanElement>(null);
 useEffect(()=>{
  const root=ref.current,host=root?.closest<HTMLElement>('[data-collection-tone],.personal-collection,.playlist-hero,.playlist-card,.hub-collection-open');if(!root||!host)return;
  host.dataset.spark=String(spark);
  let alive=true,current:Palette|undefined;
  const apply=(palette:Palette)=>{
   current=palette;
   const style=getComputedStyle(host),toHex=(value:string,fallback:string)=>/^#[0-9a-f]{6}$/i.test(value.trim())?value.trim():value.startsWith('rgb')?'#'+(value.match(/[\d.]+/g)??[]).slice(0,3).map(v=>Math.round(Number(v)).toString(16).padStart(2,'0')).join(''):fallback;
   const surface=toHex(style.getPropertyValue('--surface-color'),'#f7fbff'),ink=toHex(style.getPropertyValue('--ink'),'#12202a'),accent=toHex(palette.surfaces[0],'#0064df');
   const tones=transportTones({background:surface,blob:surface,text:ink,accent});
   host.style.setProperty('--collection-transport-top',tones.fill);host.style.setProperty('--collection-transport-bottom',tones.stroke);host.style.setProperty('--collection-transport-stroke',tones.stroke);

   host.style.setProperty('--collection-tone',palette.tone);
   palette.stops.forEach((stop,i)=>{
    host.style.setProperty(`--collection-color-${i+1}`,spark?stop:palette.tone);
    const color=spark?palette.surfaces[i]:palette.surfaces[0];
    host.style.setProperty(`--collection-hue-${i+1}`,color);
    host.style.setProperty(`--collection-surface-${i+1}`,`color-mix(in srgb,${color} ${spark?12:18}%,white)`);
   });
  };
  const update=()=>{
   const images=[...root.querySelectorAll('img')];if(!images.length||images.some(img=>!img.complete||!img.naturalWidth))return;
   const key=images.map(img=>img.currentSrc).join('|');let palette=colors.get(key);
   if(!palette){try{
    const canvas=document.createElement('canvas');canvas.width=12;canvas.height=12*images.length;
    const ctx=canvas.getContext('2d');if(!ctx)return;images.forEach((img,i)=>ctx.drawImage(img,0,i*12,12,12));
    palette=collectionPalette(ctx.getImageData(0,0,canvas.width,canvas.height).data);
    if(colors.size>=64)colors.delete(colors.keys().next().value!);colors.set(key,palette);
   }catch{
    // WKWebView local asset images can taint a canvas. Reuse the native
    // artwork palette service instead of silently keeping the default ink.
    const paths=images.map(img=>img.dataset.originalArtworkPath);
    void resolveArtworkTheme(paths).then(result=>{
     if(!result||!alive||images.map(img=>img.currentSrc).join('|')!==key)return;
     const pixels=[result.palette.accent,result.palette.blob,result.palette.text].flatMap(hex=>/^#[0-9a-f]{6}$/i.test(hex)?[parseInt(hex.slice(1,3),16),parseInt(hex.slice(3,5),16),parseInt(hex.slice(5,7),16),255]:[]);
     const next=collectionPalette(pixels);colors.set(key,next);apply(next);
    }).catch(()=>{});return;
   }}
   apply(palette);
  };
  const observer=new MutationObserver(()=>{if(current)apply(current);});const shell=host.closest('.app-shell');if(shell)observer.observe(shell,{attributes:true,attributeFilter:['style','data-theme-mode']});
  root.addEventListener('load',update,true);update();return()=>{alive=false;observer.disconnect();root.removeEventListener('load',update,true);};
 },[spark,identity]);return ref;
}
