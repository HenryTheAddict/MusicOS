// A plus and a close are the same two strokes, rotated around a fixed center.
// Keeping that topology avoids contour pairing and changes in stroke weight.
export function menuCrossMorph(host:HTMLElement,closing:boolean,finalMarkup:string){
 const start=Number(host.dataset.crossProgress??(closing?0:1)),end=closing?1:0;
 const began=performance.now(),duration=140*Math.abs(end-start);
 host.innerHTML='<svg viewBox="0 0 32 32" aria-hidden="true" data-cross-morph="true"><path d="M4 16H28M16 4V28" fill="none" stroke="var(--icon-stroke,currentColor)" stroke-width="2" stroke-linecap="round"/></svg>';
 const path=host.querySelector('path')!;let frame=0;
 function tick(now:number){const t=Math.min(1,(now-began)/(duration||1)),ease=1-(1-t)**3,progress=start+(end-start)*ease;host.dataset.crossProgress=String(progress);path.setAttribute('transform',`rotate(${45*progress} 16 16)`);if(t<1)frame=requestAnimationFrame(tick);else host.innerHTML=finalMarkup;}
 tick(began);return()=>cancelAnimationFrame(frame);
}
