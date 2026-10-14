export type VibeArt={version:1|2;hue:number;energy:number;shape:number;seed:number;variant:number;tempo:number|null};
export function parseVibe(choice?:string|null):VibeArt|null {try{if(!choice?.startsWith('vibe/')||choice.length>1024)return null;const a=JSON.parse(atob(choice.slice(5).replaceAll('-','+').replaceAll('_','/'))) as VibeArt;return [1,2].includes(a.version)&&Number.isFinite(a.hue)&&a.hue>=0&&a.hue<360&&Number.isFinite(a.energy)&&a.energy>=0&&a.energy<=1&&[0,1,2].includes(a.shape)&&[a.seed,a.variant].every(v=>Number.isInteger(v)&&v>=0&&v<=4294967295)&&(a.tempo===null||(Number.isFinite(a.tempo)&&a.tempo>=20&&a.tempo<=300))?a:null;}catch{return null;}}
export function encodeVibe(a:VibeArt){return `vibe/${btoa(JSON.stringify(a)).replaceAll('+','-').replaceAll('/','_').replaceAll('=','')}`;}
const cache=new Map<string,string>();
export function vibeCover(choice?:string|null){const a=parseVibe(choice);if(!a)return undefined;if(cache.has(choice!))return cache.get(choice!);const h=a.hue,s=65+a.energy*25;const color=(shift:number,l=55)=>`hsl(${(h+shift)%360} ${s}% ${l}%)`;const offset=a.seed%51-25;
 const points=Array.from({length:16},(_,i)=>{const theta=i*Math.PI/8,r=i%2?145:180;return [256+Math.cos(theta)*r,256+Math.sin(theta)*r];});
 const path=points.map((p,i)=>{const next=points[(i+1)%16];return `${i?'Q':'M'}${i?`${p[0]},${p[1]} `:''}${(p[0]+next[0])/2},${(p[1]+next[1])/2}`;}).join(' ')+' Z';
 let content=a.shape===2?`<path d="${path}" fill="url(#g)"/>`:`<g filter="url(#blur)"><ellipse cx="${160+offset}" cy="180" rx="200" ry="${180+a.energy*50}" fill="${color(0)}"/><ellipse cx="360" cy="340" rx="210" ry="190" fill="${color(110)}"/><ellipse cx="150" cy="420" rx="150" ry="170" fill="${color(235)}"/></g>`;
 if(a.version===2){
  const style=a.variant%4;
  if(style===1)content=`<g filter="url(#blur)">${Array.from({length:6},(_,i)=>`<ellipse cx="${70+(i%3)*180+offset}" cy="${100+Math.floor(i/3)*310}" rx="${140+(i%2)*40}" ry="${100+a.energy*65}" fill="${color(i*55)}"/>`).join('')}</g>`;
  if(style===2)content=`<g filter="url(#blur)" fill="none" stroke-linecap="round"><path d="M-50 70 Q250 450 560 100" stroke="${color(0)}" stroke-width="180"/><path d="M-50 250 Q250 -20 560 400" stroke="${color(95)}" stroke-width="140"/><path d="M-30 470 Q250 130 560 540" stroke="${color(230)}" stroke-width="170"/></g>`;
  if(style===3)content=`<g filter="url(#blur)"><ellipse cx="130" cy="130" rx="230" ry="190" fill="${color(0)}"/><ellipse cx="380" cy="420" rx="220" ry="210" fill="${color(110)}"/></g><path d="${path}" fill="url(#g)" opacity=".75" transform="translate(92 90) scale(.65)"/>`;
 }
 const svg=`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><defs><filter id="blur"><feGaussianBlur stdDeviation="48"/></filter><linearGradient id="g" x2="0.7" y2="1"><stop stop-color="${color(0)}"/><stop offset=".5" stop-color="${color(65)}"/><stop offset="1" stop-color="${color(150)}"/></linearGradient></defs><rect width="512" height="512" fill="${a.shape===2?color(155,65):color(35,93)}"/>${content}</svg>`;
 const result=`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;if(cache.size>=64)cache.delete(cache.keys().next().value!);cache.set(choice!,result);return result;
}
