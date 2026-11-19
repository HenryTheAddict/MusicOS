// Frozen benchmark baseline from b711510:ui-desktop/src/playLevelsMorph.ts.
const ns='http://www.w3.org/2000/svg';
const triangle=[[4,5,12,9,12,23,4,27],[12,9,20,13,20,19,12,23],[20,13,27,16,27,16,20,19]];
function contour(values:number[],radius:number){
 const points=Array.from({length:4},(_,i)=>[values[i*2],values[i*2+1]]);
 const corners=points.map((p,i)=>{const before=points[(i+3)%4],after=points[(i+1)%4];const a=Math.hypot(before[0]-p[0],before[1]-p[1]),b=Math.hypot(after[0]-p[0],after[1]-p[1]);const r=Math.min(radius,a/2,b/2);return {p,entry:p.map((v,j)=>v+(before[j]-v)*r/(a||1)),exit:p.map((v,j)=>v+(after[j]-v)*r/(b||1))};});
 return `M${corners[0].entry.join(',')} `+corners.map(c=>`Q${c.p.join(',')} ${c.exit.join(',')} L`).map((s,i)=>s+corners[(i+1)%4].entry.join(',')).join(' ')+' Z';
}
export function playLevelsMorph(host:HTMLElement,toLevels:boolean,finalMarkup:string){
 const start=Number(host.dataset.morphProgress??(toLevels?0:1));
 const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 32 32');
 const paths=triangle.map(()=>{const p=document.createElementNS(ns,'path');p.setAttribute('fill','var(--icon-fill,currentColor)');p.setAttribute('stroke','var(--icon-stroke,currentColor)');p.setAttribute('stroke-width','1.5');p.setAttribute('stroke-linejoin','round');svg.append(p);return p;});
 host.replaceChildren(svg);let frame=0;const began=performance.now(),end=toLevels?1:0;
 function tick(now:number){const t=Math.min(1,(now-began)/180),ease=t*t*(3-2*t),progress=start+(end-start)*ease;host.dataset.morphProgress=String(progress);
 const style=getComputedStyle(host);
 paths.forEach((path,i)=>{const raw=parseFloat(style.getPropertyValue(`--level-${i+1}`));const level=Number.isFinite(raw)?Math.max(0,Math.min(1,raw)):1;const x=[5,13.5,22][i],height=20*level;const bar=[x,26-height,x+5,26-height,x+5,26,x,26];const values=triangle[i].map((x,j)=>x+(bar[j]-x)*progress);path.setAttribute('d',contour(values,2*progress));path.setAttribute('stroke-opacity',String(progress));});
 if(t<1)frame=requestAnimationFrame(tick);else{host.innerHTML=finalMarkup;host.dataset.morphProgress=String(end);}}
 tick(began);return()=>cancelAnimationFrame(frame);
}
