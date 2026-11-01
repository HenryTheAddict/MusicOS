import queue from '../../assets/icons/ui/queue.svg?raw';
import song from '../../assets/icons/ui/Song.svg?raw';
type Shape={outline:number[];lines:number[];weights:number[]};
type Point=[number,number];
export type CycleGlyph='playerQueue'|'playerLyrics'|'playerPlaying';
// Exact cubic subdivision preserves the Sketch curves; no contour sampling.
function cubicPath(svg:string){const source=svg.match(/<path d="(M7,|M24,)[^"]+/)![0].slice(9),tokens=source.match(/[MLCZ]|-?\d+(?:\.\d+)?/g)!;let i=0,point:Point=[0,0],origin:Point=[0,0];const curves:number[][]=[];while(i<tokens.length){const command=tokens[i++];if(command==='M'){point=[+tokens[i++],+tokens[i++]];origin=point;}else if(command==='L'){const end:Point=[+tokens[i++],+tokens[i++]];curves.push([point[0],point[1],point[0]+(end[0]-point[0])/3,point[1]+(end[1]-point[1])/3,point[0]+(end[0]-point[0])*2/3,point[1]+(end[1]-point[1])*2/3,...end]);point=end;}else if(command==='C'){const c=tokens.slice(i,i+6).map(Number);i+=6;curves.push([...point,...c]);point=[c[4],c[5]];}else if(command==='Z'&&(point[0]!==origin[0]||point[1]!==origin[1])){curves.push([...point,...point,...origin,...origin]);point=origin;}}return curves;}
function split(c:number[]){const p:Point[]=[0,2,4,6].map(i=>[c[i],c[i+1]]),mid=(a:Point,b:Point):Point=>[(a[0]+b[0])/2,(a[1]+b[1])/2],a=mid(p[0],p[1]),b=mid(p[1],p[2]),d=mid(p[2],p[3]),e=mid(a,b),f=mid(b,d),g=mid(e,f);return[[...p[0],...a,...e,...g],[...g,...f,...d,...p[3]]];}
const outlines=[cubicPath(queue),cubicPath(song)],count=Math.max(...outlines.map(p=>p.length));
for(const path of outlines){while(path.length<count){let at=0,longest=-1;path.forEach((c,i)=>{const length=Math.hypot(c[6]-c[0],c[7]-c[1]);if(length>longest){at=i;longest=length;}});path.splice(at,1,...split(path[at]));}}
const flatten=(p:number[][])=>[...p[0].slice(0,2),...p.flatMap(c=>c.slice(2))];
const states:Record<CycleGlyph,Shape>={
 playerQueue:{outline:flatten(outlines[0]),lines:[10,10,22,10,10,16,22,16,10,22,19,22],weights:[2,2,2,2]},
 playerLyrics:{outline:Array(2+count*6).fill(16),lines:[6,10,26,10,6,16,18,16,6,22,22,22],weights:[0,2,2,2]},
 playerPlaying:{outline:flatten(outlines[1]),lines:[24,10,24,10,16,16,16,16,12,22,12,22],weights:[2,0,0,0]},
};
const shown=new WeakMap<HTMLElement,Shape>();
export function cycleShape(name:CycleGlyph){return states[name];}
export function resetCycleShape(host:HTMLElement,name:CycleGlyph){shown.set(host,states[name]);}
export function cycleDrawing(shape:Shape){return 'M'+shape.outline.slice(0,2).join(' ')+Array.from({length:count},(_,i)=>'C'+shape.outline.slice(2+i*6,8+i*6).join(' ')).join('');}
export function playerViewMorph(host:HTMLElement,from:CycleGlyph,to:CycleGlyph,markup:string){const start=shown.get(host)??states[from],end=states[to],began=performance.now();let frame=0;host.innerHTML='<svg viewBox="0 0 32 32" data-player-view-morph="true" fill="none" stroke="var(--icon-stroke,currentColor)" stroke-linecap="round" stroke-linejoin="round"><path/><path/><path/><path/></svg>';const paths=host.querySelectorAll('path');const hidden=()=>{if(document.hidden){cancelAnimationFrame(frame);host.innerHTML=markup;shown.set(host,end);document.removeEventListener('visibilitychange',hidden);}};document.addEventListener('visibilitychange',hidden);
 const tick=(at:number)=>{const t=Math.min(1,(at-began)/180),p=1-(1-t)**3,mix=(a:number[],b:number[])=>a.map((n,i)=>n+(b[i]-n)*p),value={outline:mix(start.outline,end.outline),lines:mix(start.lines,end.lines),weights:mix(start.weights,end.weights)};shown.set(host,value);paths[0].setAttribute('d',cycleDrawing(value));paths.forEach((path,i)=>{path.setAttribute('stroke-width',String(value.weights[i]));if(i)path.setAttribute('d',`M${value.lines.slice((i-1)*4,(i-1)*4+2).join(' ')}L${value.lines.slice((i-1)*4+2,i*4).join(' ')}`);});if(t<1)frame=requestAnimationFrame(tick);else{frame=0;host.innerHTML=markup;shown.set(host,end);document.removeEventListener('visibilitychange',hidden);}};tick(began);return()=>{cancelAnimationFrame(frame);document.removeEventListener('visibilitychange',hidden);};
}
