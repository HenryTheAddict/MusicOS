import playAsset from '../../assets/icons/ui/play.svg?raw';
import pauseAsset from '../../assets/icons/ui/pause.svg?raw';
const ns='http://www.w3.org/2000/svg';
type Point=[number,number];
const count=96;
// Sample the supplied cubic contours once. The temporary morph then has the
// same edge coverage as both final assets, without the old rounded trapezoids.
function contour(data:string):Point[]{
 const tokens=data.match(/[MLCZ]|-?\d+(?:\.\d+)?(?:e[-+]?\d+)?/gi)??[];
 const dense:Point[]=[];let at:Point=[0,0],first:Point=at,index=0;
 const point=():Point=>[Number(tokens[index++]),Number(tokens[index++])];
 while(index<tokens.length){const command=tokens[index++];
  if(command==='M'){at=point();first=at;dense.push(at);}
  else if(command==='L'){at=point();dense.push(at);}
  else if(command==='C'){const a=at,b=point(),c=point(),d=point();for(let step=1;step<=24;step++){const t=step/24,u=1-t;dense.push([0,1].map(axis=>u*u*u*a[axis]+3*u*u*t*b[axis]+3*u*t*t*c[axis]+t*t*t*d[axis]) as Point);}at=d;}
  else if(command==='Z'){dense.push(first);at=first;}
  else throw Error(`Unexpected transport path command ${command}`);
 }
 const lengths=[0];for(let i=1;i<dense.length;i++)lengths.push(lengths[i-1]+Math.hypot(dense[i][0]-dense[i-1][0],dense[i][1]-dense[i-1][1]));
 const total=lengths.at(-1)!;let segment=1;
 return Array.from({length:count},(_,i)=>{const distance=i*total/count;while(segment<lengths.length-1&&lengths[segment]<distance)segment++;const t=(distance-lengths[segment-1])/(lengths[segment]-lengths[segment-1]||1);return dense[segment-1].map((v,axis)=>v+(dense[segment][axis]-v)*t) as Point;});
}
function paths(asset:string){return [...asset.matchAll(/<path\b([^>]+)>/g)].filter(([,attributes])=>attributes.includes('stroke=')).map(([,attributes])=>contour(attributes.match(/\bd="([^"]+)"/)![1]));}
const play=paths(playAsset)[0];
function align(target:Point[]){let best=target,cost=Infinity;
 for(const points of [target,[...target].reverse()])for(let offset=0;offset<count;offset++){
  const distance=play.reduce((sum,p,i)=>sum+(p[0]-points[(i+offset)%count][0])**2+(p[1]-points[(i+offset)%count][1])**2,0);
  if(distance<cost){cost=distance;best=points.map((_,i)=>points[(i+offset)%count]);}
 }return best;
}
const pause=paths(pauseAsset).map(align);
export function transportContours(progress:number){return pause.map(target=>play.map((p,i)=>p.map((v,axis)=>v+(target[i][axis]-v)*progress) as Point));}
export function playPauseMorph(host:HTMLElement,toPause:boolean,finalMarkup:string){
 const value=Number(host.dataset.transportMorphProgress??(toPause?0:1));
 const start=Number.isFinite(value)?Math.min(1,Math.max(0,value)):(toPause?0:1),end=toPause?1:0;
 const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 32 32');svg.setAttribute('aria-hidden','true');svg.dataset.transportMorph='true';
 const template=document.createElement('template');template.innerHTML=finalMarkup;
 // Use the actual gradient definition rather than assuming it is the first defs.
 for(const defs of template.content.querySelectorAll('defs'))svg.append(defs.cloneNode(true));
 const gradient=template.content.querySelector('linearGradient')?.id;
 const outlines=pause.map(()=>{const path=document.createElementNS(ns,'path');path.setAttribute('fill','none');path.setAttribute('stroke','var(--icon-stroke,currentColor)');path.setAttribute('stroke-width','3');path.setAttribute('stroke-linejoin','round');svg.append(path);return path;});
 const fills=outlines.map(()=>{const path=document.createElementNS(ns,'path');path.setAttribute('fill',gradient?`url(#${gradient})`:'var(--icon-fill,currentColor)');svg.append(path);return path;});
 host.replaceChildren(svg);host.dataset.morphing='true';const began=performance.now();let frame=0;
 function tick(now:number){const t=Math.min(1,(now-began)/360),ease=t*t*(3-2*t),progress=start+(end-start)*ease;host.dataset.transportMorphProgress=String(progress);
  transportContours(progress).forEach((points,i)=>{const d=`M${points.map(p=>p.join(',')).join(' L')} Z`;outlines[i].setAttribute('d',d);fills[i].setAttribute('d',d);});
  if(t<1)frame=requestAnimationFrame(tick);else{host.innerHTML=finalMarkup;delete host.dataset.morphing;}
 }tick(began);return()=>{cancelAnimationFrame(frame);delete host.dataset.morphing;};
}
