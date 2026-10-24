import {transportContours} from './playPauseMorph';
const ns='http://www.w3.org/2000/svg';
type Contours=number[][];
const clamp=(value:number,min:number,max:number)=>Math.max(min,Math.min(max,value));
// The two action contours are the authored Sketch transport paths. The third
// contour collapses to a point at the action endpoint, then becomes a meter bar.
function actionContours(action:'play'|'pause'):Contours{
 const contours=transportContours(action==='pause'?1:0).map(points=>points.flat());
 contours.push(Array.from({length:49},()=>[24.5,16]).flat());return contours;
}
function bar(x:number,level:number):number[]{
 const bottom=26,top=bottom-20*clamp(level,0,1),radius=Math.min(2,(bottom-top)/2),right=x+5,k=.5522847498;
 const start=[x,top+radius],out=[...start];let at=start;
 const line=(end:number[])=>{out.push(at[0]+(end[0]-at[0])/3,at[1]+(end[1]-at[1])/3,at[0]+2*(end[0]-at[0])/3,at[1]+2*(end[1]-at[1])/3,...end);at=end;};
 const curve=(a:number[],b:number[],end:number[])=>{out.push(...a,...b,...end);at=end;};
 // Sixteen compatible cubic segments: straight thirds and four rounded corners.
 curve([x,top+radius*(1-k)],[x+radius*(1-k),top],[x+radius,top]);
 for(let i=1;i<=3;i++)line([x+radius+(5-2*radius)*i/3,top]);
 curve([right-radius*(1-k),top],[right,top+radius*(1-k)],[right,top+radius]);
 for(let i=1;i<=3;i++)line([right,top+radius+(bottom-top-2*radius)*i/3]);
 curve([right,bottom-radius*(1-k)],[right-radius*(1-k),bottom],[right-radius,bottom]);
 for(let i=1;i<=3;i++)line([right-radius-(5-2*radius)*i/3,bottom]);
 curve([x+radius*(1-k),bottom],[x,bottom-radius*(1-k)],[x,bottom-radius]);
 for(let i=1;i<=3;i++)line([x,bottom-radius-(bottom-top-2*radius)*i/3]);
 return out;
}
export function levelContours(levels:number[]):Contours{return [5,13.5,22].map((x,i)=>bar(x,levels[i]??1));}
const current=new WeakMap<HTMLElement,{contours:Contours;progress:number;action:'play'|'pause'}>();
export function playLevelsMorph(host:HTMLElement,toLevels:boolean,finalMarkup:string,action:'play'|'pause'='play'){
 const live=host.closest<HTMLElement>('.live-meter');
 const levels=()=>[0,1,2].map(i=>{const value=parseFloat((live??host).style.getPropertyValue(`--level-${i+1}`));return Number.isFinite(value)?clamp(value,0,1):1;});
 const previous=current.get(host),start=previous?.progress??Number(host.dataset.morphProgress??(toLevels?0:1)),end=toLevels?1:0;
 const from=previous?.contours??(start===1?levelContours(levels()):actionContours(action));
 const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 32 32');svg.setAttribute('aria-hidden','true');
 const make=(fill:string)=>{const path=document.createElementNS(ns,'path');path.setAttribute('fill',fill);if(fill==='none'){path.setAttribute('stroke','var(--icon-stroke,currentColor)');path.setAttribute('stroke-linejoin','round');path.setAttribute('stroke-linecap','round');}svg.append(path);return path;};
 const fill=make('var(--icon-fill,currentColor)'),outline=make('none'),seam=make('none');
 const drawing=(contours:Contours)=>{
  let fillD='',outlineD='',seamD='';
  for(let i=0;i<contours.length;i++){
   const points=contours[i],n=(at:number)=>Math.round(points[at]*10000)/10000;let previous=`${n(0)},${n(1)}`,d=`M${previous} `,edge='',join='';const seamIndex=i===0?7:15;
   for(let j=0;j<16;j++){const at=2+j*6,command=`C${n(at)},${n(at+1)} ${n(at+2)},${n(at+3)} ${n(at+4)},${n(at+5)} `;d+=command;
    if(action==='play'&&j===seamIndex)join=`M${previous} ${command}`;else edge+=`${j===0||j===seamIndex+1?`M${previous} `:''}${command}`;
    previous=`${n(at+4)},${n(at+5)}`;
   }
   fillD+=d+'Z ';if(i===2)seamD+=edge+join;else{outlineD+=edge;seamD+=join;}
  }return [fillD,outlineD,seamD];
 };
 const set=(contours:Contours,progress:number)=>{const paths=drawing(contours);[fill,outline,seam].forEach((path,i)=>path.setAttribute('d',paths[i]));for(const path of [outline,seam])path.setAttribute('stroke-width',String(3-1.5*progress));seam.setAttribute('stroke-opacity',String(progress));};
 host.replaceChildren(svg);let frame=0;const began=performance.now(),duration=180*Math.abs(end-start);
 const finish=()=>{host.innerHTML=finalMarkup;host.dataset.morphProgress=String(end);host.dataset.transportMorphProgress=String(action==='pause'?1:0);current.delete(host);};
 if(typeof CSS!=='undefined'&&CSS.supports?.('d','path("M0 0 L1 1")')&&typeof fill.animate==='function'){
  const target=toLevels?levelContours(levels()):actionContours(action),first=drawing(from),last=drawing(target);let finished=false;
  set(from,start);svg.dataset.meterMorph='native';current.set(host,{contours:from,progress:start,action});
  const animations=[fill,outline,seam].map((path,i)=>path.animate([{d:`path("${first[i]}")`,...(i?{strokeWidth:String(3-1.5*start)}:{}),...(i===2?{strokeOpacity:start}:{})},{d:`path("${last[i]}")`,...(i?{strokeWidth:String(3-1.5*end)}:{}),...(i===2?{strokeOpacity:end}:{})}],{duration:duration||1,easing:'cubic-bezier(.42,0,.58,1)',fill:'both'}));
  animations[0].onfinish=()=>{finished=true;animations.forEach(animation=>animation.cancel());finish();};
  return()=>{if(!finished){
   const elapsed=clamp(Number(animations[0].currentTime??0)/(duration||1),0,1);
   // Sample the same cubic easing only when interrupted, not on every frame.
   const cubic=(t:number,a:number,b:number)=>3*(1-t)*(1-t)*t*a+3*(1-t)*t*t*b+t*t*t;let lo=0,hi=1;for(let i=0;i<20;i++){const mid=(lo+hi)/2;if(cubic(mid,.42,.58)<elapsed)lo=mid;else hi=mid;}const ease=cubic((lo+hi)/2,0,1),progress=start+(end-start)*ease;
   current.set(host,{contours:from.map((points,i)=>points.map((value,j)=>value+(target[i][j]-value)*ease)),progress,action});host.dataset.morphProgress=String(progress);
  }animations.forEach(animation=>animation.cancel());};
 }
 function tick(now:number){
  const t=clamp((now-began)/(duration||1),0,1),ease=t*t*(3-2*t),progress=start+(end-start)*ease,target=toLevels?levelContours(levels()):actionContours(action);
  const contours=from.map((points,i)=>points.map((value,j)=>value+(target[i][j]-value)*ease));
  current.set(host,{contours,progress,action});host.dataset.morphProgress=String(progress);
  set(contours,progress);
  if(t<1)frame=requestAnimationFrame(tick);else{finish();}
 }
 tick(began);return()=>cancelAnimationFrame(frame);
}
