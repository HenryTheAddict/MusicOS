const ns='http://www.w3.org/2000/svg';
type Glyphs={play:string;pause:string;id:string};
const glyphs=new WeakMap<HTMLElement,Glyphs>();
let identity=0;
function layer(markup:string,suffix:string){
 const template=document.createElement('template');
 template.innerHTML=markup.replace(/id="([^"]+)"/g,`id="$1-${suffix}"`).replace(/(href=")#([^"]+)/g,`$1#$2-${suffix}`).replace(/url\(#([^)]+)\)/g,`url(#$1-${suffix})`);
 const group=document.createElementNS(ns,'g');
 for(const child of [...template.content.querySelector('svg')!.childNodes])group.append(child.cloneNode(true));
 return group;
}
/** A liquid reveal inside the supplied glyphs. Neither icon's geometry changes. */
export function playPauseMorph(host:HTMLElement,toPause:boolean,finalMarkup:string){
 let source=glyphs.get(host);
 if(!source){source={play:toPause?host.innerHTML:finalMarkup,pause:toPause?finalMarkup:host.innerHTML,id:`transport-water-${++identity}`};glyphs.set(host,source);}
 else source[toPause?'pause':'play']=finalMarkup;
 const parsed=Number(host.dataset.transportMorphProgress??(toPause?0:1));
 const start=Number.isFinite(parsed)?Math.min(1,Math.max(0,parsed)):(toPause?0:1),end=toPause?1:0;
 const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 32 32');svg.setAttribute('aria-hidden','true');svg.dataset.transportWater='true';
 const defs=document.createElementNS(ns,'defs');
 const clips=['play','pause'].map(name=>{
  const clip=document.createElementNS(ns,'clipPath');clip.id=`${source!.id}-${name}`;clip.setAttribute('clipPathUnits','userSpaceOnUse');
  const path=document.createElementNS(ns,'path');clip.append(path);defs.append(clip);return path;
 });
 svg.append(defs);
 for(const name of ['play','pause'] as const){const group=layer(source[name],`${source.id}-${name}`);group.setAttribute('clip-path',`url(#${source.id}-${name})`);svg.append(group);}
 host.replaceChildren(svg);host.dataset.morphing='true';
 const began=performance.now();let frame=0;
 function draw(progress:number){
  const y=36-40*progress,amplitude=1.5*Math.sin(Math.PI*progress);
  const surface=`M-4 ${y} C3 ${y-amplitude} 7 ${y+amplitude} 14 ${y} S25 ${y+amplitude} 36 ${y}`;
  clips[0].setAttribute('d',`${surface} V-4 H-4 Z`);
  clips[1].setAttribute('d',`${surface} V36 H-4 Z`);
  host.dataset.transportMorphProgress=String(progress);
 }
 function tick(now:number){
  const t=Math.min(1,(now-began)/240),ease=1-(1-t)**3;
  draw(start+(end-start)*ease);
  if(t<1)frame=requestAnimationFrame(tick);
  else{host.innerHTML=finalMarkup;delete host.dataset.morphing;}
 }
 draw(start);frame=requestAnimationFrame(tick);
 return()=>{cancelAnimationFrame(frame);delete host.dataset.morphing;};
}
