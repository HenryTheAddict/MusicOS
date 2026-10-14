import {useLayoutEffect,useSyncExternalStore,type RefObject} from 'react';
import {call,native} from './bridge';
import {springScale} from './motion';
const revealed=new Set<string>();
const prepared=new Map<string,Promise<{photo:HTMLCanvasElement;depth:HTMLCanvasElement}>>();
const requests=new Map<string,Promise<import("./types").DepthArtwork|null>>();
const VERTEX=`#version 300 es
in vec2 point;out vec2 uv;uniform vec4 rect;uniform vec2 viewport;
void main(){uv=point;vec2 p=rect.xy+point*rect.zw;gl_Position=vec4(p/viewport*vec2(2.,-2.)+vec2(-1.,1.),0.,1.);}`;
const FRAGMENT=`#version 300 es
precision mediump float;in vec2 uv;out vec4 color;uniform sampler2D photo;uniform sampler2D depth;uniform vec2 size;uniform float radius;uniform float motion;uniform float strength;uniform vec2 pointer;
void main(){vec2 p=uv*size;vec2 q=abs(p-size*.5)-(size*.5-vec2(radius));float edge=length(max(q,0.))+min(max(q.x,q.y),0.)-radius;
float border=min(min(uv.x,1.-uv.x),min(uv.y,1.-uv.y));float fade=smoothstep(0.,.12,border);vec2 direction=vec2(pointer.x,motion+pointer.y)/size;vec2 sampleUV=uv;
float d=texture(depth,uv).r-.5;sampleUV=clamp(uv+direction*d*2.*fade,vec2(.001),vec2(.999));color=texture(photo,sampleUV);color.a*=(1.-smoothstep(-1.,0.,edge))*strength;}`;
type Entry={art:HTMLElement;image:HTMLImageElement;path:string;version:string;photo:WebGLTexture;depth:WebGLTexture;top:number;left:number;width:number;height:number;radius:number;motion:number;strength:number;hover:boolean;hero:boolean;pointerX:number;pointerY:number;targetX:number;targetY:number;visible:boolean;used:number};
function program(gl:WebGL2RenderingContext){
 const compile=(kind:number,source:string)=>{const shader=gl.createShader(kind)!;gl.shaderSource(shader,source);gl.compileShader(shader);if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(shader)??'Depth shader failed');return shader;};
 const vertex=compile(gl.VERTEX_SHADER,VERTEX),fragment=compile(gl.FRAGMENT_SHADER,FRAGMENT),program=gl.createProgram()!;gl.attachShader(program,vertex);gl.attachShader(program,fragment);gl.linkProgram(program);gl.deleteShader(vertex);gl.deleteShader(fragment);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw new Error('Depth program failed');return program;
}
const subscribeMotion=(change:()=>void)=>{const query=matchMedia('(prefers-reduced-motion: reduce)');query.addEventListener('change',change);return()=>query.removeEventListener('change',change);};
export function useDepthArtwork(root:RefObject<HTMLElement|null>,scroll:RefObject<HTMLElement|null>,enabled:boolean){
 const reducedMotion=useSyncExternalStore(subscribeMotion,()=>matchMedia('(prefers-reduced-motion: reduce)').matches);
 useLayoutEffect(()=>{
 const node=root.current,viewport=scroll.current,reduced=matchMedia('(prefers-reduced-motion: reduce)');if(!node||!viewport||!enabled||(!native&&!(import.meta.env.DEV&&new URLSearchParams(location.search).has('depthFixture')))||reduced.matches)return;
 let viewLeft=0,viewTop=0;
 let disposed=false,lost=false,ready=false,frame=0,last=0,dirty=true,pending=false,waitingLayout=false,loadFrame=0,velocity=0,previousScroll=viewport.scrollTop,previousScrollTime=performance.now(),readinessTimer:ReturnType<typeof setTimeout>|undefined,baseScroll=0,viewWidth=0,viewHeight=0,clock=0;
 const hero=node.matches('.artist-portrait-stage,.album-artwork-stage,.viewer-depth-stage'),area=hero?node:viewport;
 const interfacePercent=Math.round((parseFloat(getComputedStyle(document.documentElement).fontSize)||16)/16*100);
 const intervals:number[]=[],costs:number[]=[];let reported=0;
 const entries=new Map<HTMLElement,Entry>(),visible=new Set<HTMLElement>(),observed=new Set<HTMLElement>();
 const wrapper=document.createElement('div');wrapper.className='depth-compositor';wrapper.setAttribute('aria-hidden','true');
 const canvas=document.createElement('canvas');wrapper.append(canvas);wrapper.dataset.hero=String(hero);(hero?node:document.body).append(wrapper);
 const context=canvas.getContext('webgl2',{alpha:true,premultipliedAlpha:false,antialias:false,depth:false,stencil:false});if(!context){wrapper.remove();return;}const gl:WebGL2RenderingContext=context;
 let shader:WebGLProgram;try{shader=program(gl);}catch{wrapper.remove();return;}
 gl.useProgram(shader);const buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([0,0,1,0,0,1,0,1,1,0,1,1]),gl.STATIC_DRAW);
 const point=gl.getAttribLocation(shader,'point');gl.enableVertexAttribArray(point);gl.vertexAttribPointer(point,2,gl.FLOAT,false,0,0);
 const uniforms=Object.fromEntries(['rect','viewport','photo','depth','size','radius','motion','strength','pointer'].map(name=>[name,gl.getUniformLocation(shader,name)]));
 gl.uniform1i(uniforms.photo,0);gl.uniform1i(uniforms.depth,1);gl.disable(gl.BLEND);
 const remove=(art:HTMLElement)=>{const entry=entries.get(art);if(entry){gl.deleteTexture(entry.photo);gl.deleteTexture(entry.depth);entries.delete(art);}delete art.dataset.depthReady;};
 const texture=(image:TexImageSource)=>{const texture=gl.createTexture()!;gl.bindTexture(gl.TEXTURE_2D,texture);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);return texture;};
 Object.defineProperty(canvas,'depthGeometry',{value:()=>[...entries.values()].filter(entry=>visible.has(entry.art)).map(entry=>({path:entry.path,left:entry.left,top:entry.top-(hero?0:viewport.scrollTop-baseScroll),width:entry.width,height:entry.height,motion:entry.motion,hover:entry.hover,pointerX:entry.pointerX,pointerY:entry.pointerY}))});
 const schedule=()=>{if(!frame&&!disposed&&!lost)frame=requestAnimationFrame(draw);};
 const measure=()=>{const bounds=area.getBoundingClientRect(),stretch=hero?1:parseFloat(viewport.style.getPropertyValue('--overscroll-scale'))||1,offset=hero?0:parseFloat(viewport.style.getPropertyValue('--overscroll-y'))||0,originShift=!hero&&offset<0?area.offsetHeight*(1-stretch):0;viewWidth=area.clientWidth;viewHeight=area.clientHeight;baseScroll=viewport.scrollTop;
  viewLeft=bounds.left+area.clientLeft;viewTop=bounds.top+area.clientTop;
  if(!hero){wrapper.style.left=`${bounds.left+area.clientLeft}px`;wrapper.style.top=`${bounds.top-offset-originShift+area.clientTop}px`;wrapper.style.borderRadius=getComputedStyle(viewport).borderTopLeftRadius;}
  wrapper.style.width=`${viewWidth}px`;wrapper.style.height=`${viewHeight}px`;
  const dpr=Math.min(devicePixelRatio,2),width=Math.ceil(viewWidth*dpr),height=Math.ceil(viewHeight*dpr);if(canvas.width!==width||canvas.height!==height){canvas.width=width;canvas.height=height;}canvas.style.width=`${viewWidth}px`;canvas.style.height=`${viewHeight}px`;gl.viewport(0,0,canvas.width,canvas.height);
  for(const entry of entries.values()){const r=entry.art.getBoundingClientRect(),spring=springScale(entry.art),width=r.width/spring.x,height=r.height/spring.y/stretch;entry.left=r.left-bounds.left-area.clientLeft+(r.width-width)/2;entry.top=(r.top-bounds.top)/stretch-area.clientTop+(r.height/stretch-height)/2;entry.width=width;entry.height=height;const radius=getComputedStyle(entry.art).borderTopLeftRadius;entry.radius=radius.endsWith('%')?r.width*parseFloat(radius)/100:parseFloat(radius);}
  dirty=false;
 };
 const deferLayout=()=>{const layout=[...(node.closest('.page-content')?.getAnimations({subtree:true})??[]),...(node.closest('dialog.artwork-viewer')?.getAnimations()??[])].filter(a=>(a.id==='page-entrance'||a.id==='grid-reflow'||('animationName' in a&&a.animationName==='artwork-open'))&&a.playState==='running');
  if(!layout.length)return false;for(const entry of entries.values())delete entry.art.dataset.depthReady;wrapper.style.visibility='hidden';if(!waitingLayout){waitingLayout=true;void Promise.allSettled(layout.map(a=>a.finished)).then(()=>{waitingLayout=false;if(!disposed){dirty=true;wrapper.style.visibility='';for(const entry of entries.values())entry.art.dataset.depthReady='true';schedule();scheduleLoad();}});}return true;};
 const scheduleLoad=()=>{if(!loadFrame&&!disposed&&!lost)loadFrame=requestAnimationFrame(()=>{loadFrame=0;setTimeout(()=>{if(!disposed)void loadNext();},0);});};
 const loadNext=async()=>{if(pending||!ready||disposed||lost||(entries.size>=24&&[...entries.values()].every(e=>visible.has(e.art))))return;if(deferLayout())return;
  const art=[...visible].find(art=>{const img=art.querySelector<HTMLImageElement>('img[data-artwork-path]');return img?.complete&&img.naturalWidth>0&&img.dataset.artworkPath&&!entries.has(art)&&art.dataset.depthUnavailable!==`${img.dataset.artworkPath}:${img.dataset.artworkVersion??'0'}`;});if(!art)return;
  const image=art.querySelector<HTMLImageElement>('img[data-artwork-path]')!,path=image.dataset.artworkPath!,version=image.dataset.artworkVersion??'0',key=`${path}:${version}`;pending=true;
  let photo:WebGLTexture|null=null,depth:WebGLTexture|null=null;
  try{let request=requests.get(key);if(!request){request=call('depth_artwork',{path});if(requests.size>=32)requests.delete(requests.keys().next().value!);requests.set(key,request);request.then(()=>{let bytes=0;const items=[...requests.entries()];void Promise.all(items.map(async([k,p])=>[k,(await p)?.photo.length??0] as const)).then(values=>{for(let i=values.length-1;i>=0;i--){bytes+=values[i][1];if(bytes>32*1024*1024)requests.delete(values[i][0]);}}).catch(()=>{});}).catch(()=>requests.delete(key));}const map=await request;
   if(disposed||lost||!art.isConnected||image.dataset.artworkPath!==path||(image.dataset.artworkVersion??'0')!==version)return;
   if(!map){art.dataset.depthUnavailable=key;return;}
   const full=art.classList.contains("viewer-artwork"),preparedKey=`${map.identity??key}:${full}`;
   let decoded=prepared.get(preparedKey);
   if(!decoded){decoded=(async()=>{const depthImage=new Image();depthImage.src=full?(map.full_map??map.map):map.map;const photoImage=new Image();photoImage.src=map.photo;await Promise.all([depthImage.decode(),photoImage.decode()]);
    const photo=document.createElement('canvas');photo.width=full?Math.max(1,Math.round(512*photoImage.naturalWidth/Math.max(photoImage.naturalWidth,photoImage.naturalHeight))):512;photo.height=full?Math.max(1,Math.round(512*photoImage.naturalHeight/Math.max(photoImage.naturalWidth,photoImage.naturalHeight))):512;const side=Math.min(photoImage.naturalWidth,photoImage.naturalHeight);if(full)photo.getContext('2d')!.drawImage(photoImage,0,0,photo.width,photo.height);else photo.getContext('2d')!.drawImage(photoImage,(photoImage.naturalWidth-side)/2,(photoImage.naturalHeight-side)/2,side,side,0,0,512,512);
    const depth=document.createElement('canvas');depth.width=depth.height=256;const depthContext=depth.getContext('2d')!;depthContext.filter='blur(3px)';depthContext.drawImage(depthImage,-3,-3,262,262);return {photo,depth};})();
    if(prepared.size>=16)prepared.delete(prepared.keys().next().value!);prepared.set(preparedKey,decoded);decoded.catch(()=>prepared.delete(preparedKey));
   }else{prepared.delete(preparedKey);prepared.set(preparedKey,decoded);}
   const images=await decoded;
   if(disposed||lost||!art.isConnected||image.dataset.artworkPath!==path||(image.dataset.artworkVersion??'0')!==version)return;
   // Decoded 512/256 canvases are shared across navigation (20MiB maximum).
   photo=texture(images.photo);depth=texture(images.depth);
   if(entries.size>=24){const oldest=[...entries.values()].filter(e=>!visible.has(e.art)).sort((a,b)=>a.used-b.used)[0];if(oldest)remove(oldest.art);else{gl.deleteTexture(photo);gl.deleteTexture(depth);photo=depth=null;return;}}
   entries.set(art,{art,image,path,version,photo,depth,top:0,left:0,width:0,height:0,radius:0,motion:0,strength:0,hover:!!art.closest('.card-open')?.matches(':hover,:focus-visible'),hero:art.matches('.artist-portrait-button,.album-artwork,.viewer-artwork'),pointerX:0,pointerY:0,targetX:0,targetY:0,visible:true,used:clock++});photo=depth=null;art.dataset.depthReady='true';if(map.fresh&&!revealed.has(map.identity??key)){revealed.add(map.identity??key);art.animate([{opacity:.96},{opacity:1}],{duration:220,easing:'ease-out'});}dirty=true;schedule();
  }catch{if(!disposed)art.dataset.depthUnavailable=key;}finally{if(photo)gl.deleteTexture(photo);if(depth)gl.deleteTexture(depth);pending=false;scheduleLoad();}
 };
 function draw(now:number){frame=0;if(disposed||lost||reduced.matches)return;const started=performance.now();if(dirty&&deferLayout()){last=0;return;}if(dirty)measure();gl.clearColor(0,0,0,0);gl.clear(gl.COLOR_BUFFER_BIT);gl.useProgram(shader);gl.uniform2f(uniforms.viewport,viewWidth,viewHeight);
  const interval=last?now-last:0;if(interval>0&&interval<100){intervals.push(interval);if(intervals.length>240)intervals.shift();}const dt=last?Math.min(200,now-last):8.33;last=now;let moving=false;const delta=hero?0:viewport!.scrollTop-baseScroll;velocity*=Math.exp(-dt/85);
  for(const entry of entries.values()){
   if(!visible.has(entry.art))continue;entry.used=clock++;const top=entry.top-delta;
   const target=entry.hero||entry.hover?0:Math.max(-6,Math.min(6,((top+entry.height/2)/Math.max(1,viewHeight)-.5)*24+velocity*2));
   const next=entry.motion+(target-entry.motion)*(1-Math.exp(-dt/32));entry.motion=Math.abs(target-next)<.02?target:next;moving ||=Math.abs(target-entry.motion)>.02;
   const strength=1;entry.strength+=(strength-entry.strength)*(1-Math.exp(-dt/32));if(Math.abs(strength-entry.strength)<.01)entry.strength=strength;moving ||= Math.abs(strength-entry.strength)>.01;
   entry.pointerX+=(entry.targetX-entry.pointerX)*(1-Math.exp(-dt/45));entry.pointerY+=(entry.targetY-entry.pointerY)*(1-Math.exp(-dt/45));moving ||= Math.abs(entry.targetX-entry.pointerX)+Math.abs(entry.targetY-entry.pointerY)>.02;
   const spring=springScale(entry.art);moving ||= spring.moving;const w=entry.width*spring.x,h=entry.height*spring.y;
   gl.uniform4f(uniforms.rect,entry.left+(entry.width-w)/2,top+(entry.height-h)/2,w,h);gl.uniform1f(uniforms.strength,entry.strength);gl.uniform2f(uniforms.size,entry.width,entry.height);gl.uniform1f(uniforms.radius,entry.radius);gl.uniform1f(uniforms.motion,entry.motion);gl.uniform2f(uniforms.pointer,entry.pointerX,entry.pointerY);
   gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,entry.photo);gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,entry.depth);gl.drawArrays(gl.TRIANGLES,0,6);
  }
  const cost=performance.now()-started;costs.push(cost);if(costs.length>240)costs.shift();
  if(now-reported>3000&&intervals.length>=20){reported=now;const percentile=(values:number[],p:number)=>[...values].sort((a,b)=>a-b)[Math.min(values.length-1,Math.floor(values.length*p))]??0;
   void call('artwork_performance',{sample:{width:Math.round(viewWidth),height:Math.round(viewHeight),frames:intervals.length,interface_percent:interfacePercent,raf_median_ms:percentile(intervals,.5),raf_p95_ms:percentile(intervals,.95),cpu_draw_p95_ms:percentile(costs,.95),over_120hz_budget:intervals.filter(v=>v>12.5).length/intervals.length}}).catch(()=>{});
  }
  canvas.dataset.drawMs=cost.toFixed(2);canvas.dataset.visible=String(visible.size);
  if(moving)schedule();else last=0;scheduleLoad();
 }
 const observe=new IntersectionObserver(records=>{for(const record of records){const art=record.target as HTMLElement;if(record.isIntersecting)visible.add(art);else visible.delete(art);}dirty=true;schedule();scheduleLoad();},{root:viewport});
 const scan=()=>{const current=new Set(node.querySelectorAll<HTMLElement>('.card-artwork,.artist-portrait-button,.album-artwork,.viewer-artwork'));for(const art of observed)if(!current.has(art)){observe.unobserve(art);observed.delete(art);visible.delete(art);remove(art);}for(const art of current)if(!observed.has(art)){observed.add(art);observe.observe(art);}for(const [art,entry] of entries)if(entry.image.dataset.artworkPath!==entry.path||(entry.image.dataset.artworkVersion??'0')!==entry.version)remove(art);for(const art of current)if(art.dataset.depthUnavailable!==`${art.querySelector<HTMLImageElement>('img')?.dataset.artworkPath}:${art.querySelector<HTMLImageElement>('img')?.dataset.artworkVersion??'0'}`)delete art.dataset.depthUnavailable;dirty=true;schedule();scheduleLoad();};
 const mutations=new MutationObserver(scan);mutations.observe(node,{childList:true,subtree:true,attributes:true,attributeFilter:['data-artwork-path','data-artwork-version']});
 const resize=()=>{dirty=true;schedule();};const ro=new ResizeObserver(resize);ro.observe(node);ro.observe(viewport);

 const hover=(e:Event)=>{const control=e.target instanceof Element?e.target.closest('.card-open,.artist-portrait-button,.album-artwork,.viewer-artwork'):null,art=control?.matches('.artist-portrait-button,.album-artwork,.viewer-artwork')?control as HTMLElement:control?.querySelector<HTMLElement>('.card-artwork');if(!art)return;const entry=entries.get(art);if(!entry)return;if((e.type==='pointerout'||e.type==='focusout')&&control?.contains((e as PointerEvent).relatedTarget as Node|null))return;entry.hover=e.type==='pointerover'||e.type==='focusin';if(!entry.hover){entry.targetX=entry.targetY=0;}schedule();};
 const pointer=(e:PointerEvent)=>{const art=e.target instanceof Element?e.target.closest('.card-open,.artist-portrait-button,.album-artwork,.viewer-artwork')?.querySelector<HTMLElement>('.card-artwork')??e.target.closest<HTMLElement>('.artist-portrait-button,.album-artwork,.viewer-artwork'):null;
  const entry=hero?[...entries.values()][0]:art?entries.get(art):undefined;if(!entry)return;
  const left=viewLeft+entry.left,top=viewTop+entry.top-(hero?0:viewport.scrollTop-baseScroll);
  entry.targetX=Math.max(-6,Math.min(6,(e.clientX-left-entry.width/2)/Math.max(1,entry.width)*12));entry.targetY=Math.max(-6,Math.min(6,(e.clientY-top-entry.height/2)/Math.max(1,entry.height)*12));schedule();};
 const scrolled=()=>{const now=performance.now(),position=viewport.scrollTop;velocity=Math.max(-2,Math.min(2,(position-previousScroll)/Math.max(8,now-previousScrollTime)));previousScroll=position;previousScrollTime=now;schedule();};
 const fallback=()=>{lost=true;cancelAnimationFrame(frame);for(const art of observed)delete art.dataset.depthReady;canvas.style.visibility='hidden';};
 const contextLost=(e:Event)=>{e.preventDefault();fallback();};canvas.addEventListener('webglcontextlost',contextLost);
 const readiness=(e:Event)=>{const next=!!(e as CustomEvent).detail;if(next===ready)return;ready=next;requests.clear();prepared.clear();if(!ready){for(const art of observed)remove(art);}else{for(const art of observed)delete art.dataset.depthUnavailable;}schedule();};
 const invalidated=()=>{requests.clear();prepared.clear();for(const art of observed){remove(art);delete art.dataset.depthUnavailable;}scan();};
 window.addEventListener('musicos-artwork-invalidated',invalidated);
 const imageLoaded=()=>{scan();};node.addEventListener('load',imageLoaded,true);
 const motionChanged=()=>{if(reduced.matches)fallback();};
 viewport.addEventListener('scroll',scrolled,{passive:true});node.addEventListener('pointermove',pointer);window.addEventListener('resize',resize);window.addEventListener('musicos-depth-ready',readiness);reduced.addEventListener('change',motionChanged);
 for(const event of ['pointerover','pointerout','focusin','focusout'])node.addEventListener(event,hover);
 const checkReady=()=>void call('depth_setup',{action:'status'}).then(s=>{if(disposed)return;ready=s.ready;scan();if(s.running)readinessTimer=setTimeout(checkReady,1500);}).catch(()=>{});checkReady();scan();
 return()=>{disposed=true;window.removeEventListener('musicos-artwork-invalidated',invalidated);clearTimeout(readinessTimer);node.removeEventListener('load',imageLoaded,true);cancelAnimationFrame(frame);cancelAnimationFrame(loadFrame);node.removeEventListener('pointermove',pointer);observe.disconnect();mutations.disconnect();ro.disconnect();viewport.removeEventListener('scroll',scrolled);window.removeEventListener('resize',resize);window.removeEventListener('musicos-depth-ready',readiness);reduced.removeEventListener('change',motionChanged);for(const event of ['pointerover','pointerout','focusin','focusout'])node.removeEventListener(event,hover);for(const art of observed){remove(art);delete art.dataset.depthUnavailable;}gl.deleteBuffer(buffer);gl.deleteProgram(shader);gl.getExtension('WEBGL_lose_context')?.loseContext();wrapper.remove();};
 },[root,scroll,enabled,reducedMotion]);
}
