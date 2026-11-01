// Compatible authored Sketch strokes; the OFF diagonal uses collinear controls.
const on=[4, 12, 6.2862915, 12, 8.5175331, 12, 9.6331539, 12, 10.5538544, 12.2210406, 11.3737851, 12.4178886, 12.1574387, 12.7431293, 12.8757942, 13.1847181, 13.6776147, 13.6776147, 14.4517431, 14.4517431, 16, 16, 17.5482569, 17.5482569, 18.3223853, 18.3223853, 19.1242058, 18.8152819, 19.8425613, 19.2568707, 20.6262149, 19.5821114, 21.4461456, 19.7789594, 22.3668461, 20, 23.4824669, 20, 25.7137085, 20, 28, 20];
const off=on.map((v,i)=>i%2===0?v:8+(on[i-1]-4)*15/24);
const first='M4,20 L6.2862915,20 C8.5175331,20 9.6331539,20 10.5538544,19.7789594 C11.3737851,19.5821114 12.1574387,19.2568707 12.8757942,18.8152819 C13.6776147,18.3223853 14.4517431,17.5482569 16,16 C17.5482569,14.4517431 18.3223853,13.6776147 19.1242058,13.1847181 C19.8425613,12.7431293 20.6262149,12.4178886 21.4461456,12.2210406 C22.3668461,12 23.4824669,12 25.7137085,12 L28,12';
export function shuffleShape(p:number){const values=on.map((v,i)=>off[i]+(v-off[i])*p);let n=0;return 'M'+values.slice(n,n+=2).join(' ')+'L'+values.slice(n,n+=2).join(' ')+Array.from({length:6},()=> 'C'+values.slice(n,n+=6).join(' ')).join('')+'L'+values.slice(n,n+2).join(' ');}
export function shuffleMorph(host:HTMLElement,enabled:boolean,finalMarkup:string){
 const start=Number(host.dataset.shuffleProgress??(enabled?0:1)),end=enabled?1:0,began=performance.now(),duration=180*Math.abs(end-start);
 host.innerHTML=`<svg viewBox="0 0 32 32" data-shuffle-morph="true"><path d="${first}" fill="none" stroke="var(--icon-stroke,currentColor)" stroke-linecap="round"/><path fill="none" stroke="var(--icon-fill,var(--secondary-ink,currentColor))" stroke-width="2" stroke-linecap="round"/></svg>`;
 const paths=host.querySelectorAll('path');let frame=0;
 const draw=(now:number)=>{const t=Math.min(1,(now-began)/(duration||1)),p=start+(end-start)*(1-(1-t)**3);host.dataset.shuffleProgress=String(p);paths[0].setAttribute('stroke-width',String(1+p));paths[1].setAttribute('d',shuffleShape(p));if(t<1)frame=requestAnimationFrame(draw);else{frame=0;host.innerHTML=finalMarkup;}};
 draw(began);return()=>cancelAnimationFrame(frame);
}
