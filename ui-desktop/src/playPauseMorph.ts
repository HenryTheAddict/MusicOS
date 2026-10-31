import pauseAsset from '../../assets/icons/ui/pause.svg?raw';
const ns='http://www.w3.org/2000/svg';
type Point=[number,number];type Curve=[Point,Point,Point];type Shape={start:Point;curves:Curve[]};
const p=(x:number,y:number):Point=>[x,y];
const line=(a:Point,b:Point):Curve=>[p(a[0]+(b[0]-a[0])/3,a[1]+(b[1]-a[1])/3),p(a[0]+2*(b[0]-a[0])/3,a[1]+2*(b[1]-a[1])/3),b];
function halves(start:Point,[a,b,end]:Curve):[Curve,Curve]{const mid=(x:Point,y:Point)=>p((x[0]+y[0])/2,(x[1]+y[1])/2),ab=mid(start,a),bc=mid(a,b),cd=mid(b,end),abc=mid(ab,bc),bcd=mid(bc,cd),center=mid(abc,bcd);return [[ab,abc,center],[bcd,cd,end]];}
const still=(at:Point):Curve=>[at,at,at];
// Sixteen authored cubic segments per half. Only the two original corner
// curves need subdivision; no perimeter sampling or correspondence search.
const a=p(4,6.91320122),b=p(6.60564639,5.30282319),c=p(13,8.5),d=p(13,23.5),e=p(6.60564639,26.6971768),f=p(4,25.0867988);
const left:Shape={start:a,curves:[...halves(a,[p(4,5.67806539),p(4,5.06049747),p(4.47426889,4.76738318)]),[p(4.94853778,4.47426889),p(5.50090731,4.75045366),b],line(b,c),still(c),still(c),still(c),line(c,d),still(d),still(d),still(d),line(d,e),...halves(e,[p(5.50090731,27.2495463),p(4.94853778,27.5257311),p(4.47426889,27.2326168)]),[p(4,26.9395025),p(4,26.3219346),f],line(f,a)]};
const tip=p(26.763932,16),top=p(24.9503024,14.4751512),bottom=p(24.9503024,17.5248488);
const right:Shape={start:c,curves:[still(c),still(c),still(c),line(c,top),...halves(top,[p(26.1593888,15.0796944),p(26.763932,15.381966),tip]),[p(26.763932,16.618034),p(26.1593888,16.9203056),bottom],line(bottom,d),still(d),still(d),still(d),still(d),still(d),still(d),still(d),line(d,c)]};
function pauseShape(data:string):Shape{const tokens=data.match(/[MLCZ]|-?\d+(?:\.\d+)?/gi)!;let i=1;const point=():Point=>p(Number(tokens[i++]),Number(tokens[i++]));const start=point(),curves:Curve[]=[];let at=start;while(i<tokens.length){const command=tokens[i++];if(command==='L'){const end=point();curves.push(line(at,end));at=end;}else if(command==='C'){const curve:Curve=[point(),point(),point()];curves.push(curve);at=curve[2];}}return {start:curves[12][2],curves:[...curves.slice(13),...curves.slice(0,13)]};}
const source=[left,right],target=[...pauseAsset.matchAll(/<path\b([^>]+)>/g)].filter(match=>match[1].includes('stroke=')).map(match=>pauseShape(match[1].match(/\bd="([^"]+)"/)![1]));
// The asset starts at the top edge. Rotate each contour to the left edge.
function shapes(progress:number){const t=Math.max(0,Math.min(1,progress)),mix=(a:Point,b:Point):Point=>p(a[0]+(b[0]-a[0])*t,a[1]+(b[1]-a[1])*t);return source.map((shape,index)=>({start:mix(shape.start,target[index].start),curves:shape.curves.map((curve,j)=>curve.map((at,k)=>mix(at,target[index].curves[j][k])) as Curve)}));}
export function transportContours(progress:number){return shapes(progress).map(shape=>[shape.start,...shape.curves.flat()]);}
const layouts=source.map((shape,index)=>{const from=[...shape.start,...shape.curves.flat(2)],to=[...target[index].start,...target[index].curves.flat(2)];return {from,delta:to.map((value,i)=>value-from[i])};});
export function transportDrawing(progress:number){
 const t=Math.max(0,Math.min(1,progress)),outlines:string[]=[],seams:string[]=[],fills:string[]=[];
 for(let index=0;index<layouts.length;index++){
  const {from,delta}=layouts[index],n=(i:number)=>Math.round((from[i]+delta[i]*t)*10000)/10000;
  let previous=`${n(0)},${n(1)}`,fill=`M${previous} `,outline='',seam='';
  for(let j=0;j<16;j++){const k=2+j*6,command=`C${n(k)},${n(k+1)} ${n(k+2)},${n(k+3)} ${n(k+4)},${n(k+5)} `;fill+=command;
   if(j===(index===0?7:15))seam=`M${previous} ${command}`;else outline+=`${j===0||j===(index===0?8:16)?`M${previous} `:''}${command}`;
   previous=`${n(k+4)},${n(k+5)}`;
  }
  outlines.push(outline);seams.push(seam);fills.push(fill+'Z');
 }
 return {outlines,seams,fill:fills.join(' ')};
}
export function playPauseMorph(host:HTMLElement,toPause:boolean,finalMarkup:string){
 const value=Number(host.dataset.transportMorphProgress??(toPause?0:1)),start=Number.isFinite(value)?Math.max(0,Math.min(1,value)):(toPause?0:1),end=toPause?1:0;
 const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 32 32');svg.setAttribute('aria-hidden','true');svg.dataset.transportMorph='true';
 const template=document.createElement('template');template.innerHTML=finalMarkup;template.content.querySelectorAll('defs').forEach(defs=>svg.append(defs.cloneNode(true)));const gradient=template.content.querySelector('linearGradient')?.id;
 // Match the supplied assets: fill first, then the full-width outline.
 const fill=document.createElementNS(ns,'path');fill.setAttribute('fill',gradient?`url(#${gradient})`:'var(--icon-fill,currentColor)');svg.append(fill);
 const stroke=()=>{const path=document.createElementNS(ns,'path');for(const [name,value] of Object.entries({fill:'none',stroke:'var(--icon-stroke,currentColor)','stroke-width':'3','stroke-linejoin':'round','stroke-linecap':'round'}))path.setAttribute(name,value);svg.append(path);return path;};
 const outlines=source.map(stroke),seams=source.map(stroke);host.replaceChildren(svg);host.dataset.morphing='true';const began=performance.now(),duration=360*Math.abs(end-start);let frame=0;
 const draw=(progress:number)=>{host.dataset.transportMorphProgress=String(progress);const drawing=transportDrawing(progress);fill.setAttribute('d',drawing.fill);drawing.outlines.forEach((d,i)=>outlines[i].setAttribute('d',d));drawing.seams.forEach((d,i)=>{seams[i].setAttribute('d',d);seams[i].setAttribute('stroke-opacity',String(progress));});};
 const tick=(now:number)=>{const t=Math.min(1,(now-began)/(duration||1)),ease=1-Math.pow(1-t,3);draw(start+(end-start)*ease);if(t<1)frame=requestAnimationFrame(tick);else{host.dataset.transportMorphProgress=String(end);host.innerHTML=finalMarkup;delete host.dataset.morphing;}};tick(began);return()=>{cancelAnimationFrame(frame);delete host.dataset.morphing;};
}
