/** Normalized 32 px geometry: dots grow into two strokes, never a huge 8 px X. */
export function menuDotsMorph(host:HTMLElement,closing:boolean,markup:string){
 const start=Number(host.dataset.dotsProgress??(closing?0:1)),end=closing?1:0,began=performance.now(),duration=140*Math.abs(end-start);let frame=0;
 host.innerHTML='<svg viewBox="0 0 32 32" aria-hidden="true"><path fill="none" stroke="var(--icon-stroke,currentColor)" stroke-linecap="round"/><path fill="none" stroke="var(--icon-stroke,currentColor)" stroke-linecap="round"/><circle cx="16" cy="16" r="2" fill="var(--icon-stroke,currentColor)"/></svg>';
 const paths=host.querySelectorAll('path'),center=host.querySelector('circle')!;
 const tick=(now:number)=>{const t=Math.min(1,(now-began)/(duration||1)),q=1-(1-t)**3,value=start+(end-start)*q;host.dataset.dotsProgress=String(value);[[16,10,9,9,23,23],[16,22,23,9,9,23]].forEach(([x,y,a,b,c,d],i)=>{paths[i].setAttribute('d',`M${x+(a-x)*value} ${y+(b-y)*value}L${x+(c-x)*value+.0001} ${y+(d-y)*value}`);paths[i].setAttribute('stroke-width',String(4+(2.333333-4)*value));});center.setAttribute('opacity',String(1-value));if(t<1)frame=requestAnimationFrame(tick);else host.innerHTML=markup;};tick(began);return()=>cancelAnimationFrame(frame);
}
