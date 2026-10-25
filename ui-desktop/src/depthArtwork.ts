import {useViewActive} from './ViewScope';
import {smoothPath} from './smoothCorners';
import {useLayoutEffect,useSyncExternalStore,useState,useRef,type RefObject} from 'react';
import {call,native} from './bridge';
import {ByteCache,reserveTexture,registerTexture,textureBytes} from './depthCache';
import {springScale} from './motion';
type Images={photo:HTMLCanvasElement;depth:HTMLCanvasElement};
const prepared=new ByteCache<Images>(240*1024*1024,v=>(v.photo.width*v.photo.height+v.depth.width*v.depth.height)*4);
const masks=new ByteCache<HTMLCanvasElement>(16*1024*1024,v=>v.width*v.height*4);
const requests=new ByteCache<import("./types").DepthArtwork|null>(64*1024*1024,v=>v?((v.photo.length+v.map.length+(v.full_map?.length??0))*2):0);
const VERTEX=`#version 300 es
in vec2 point;out vec2 uv;uniform vec4 rect;uniform vec2 viewport;
void main(){uv=point;vec2 p=rect.xy+point*rect.zw;gl_Position=vec4(p/viewport*vec2(2.,-2.)+vec2(-1.,1.),0.,1.);}`;
const FRAGMENT=`#version 300 es
precision mediump float;in vec2 uv;out vec4 color;uniform sampler2D photo;uniform sampler2D depth;uniform sampler2D edgeMask;uniform vec2 size;uniform float radius;uniform float motion;uniform float strength;uniform vec2 pointer;
void main(){
float border=min(min(uv.x,1.-uv.x),min(uv.y,1.-uv.y));float fade=smoothstep(0.,.12,border);vec2 direction=vec2(pointer.x,motion+pointer.y)/size;vec2 sampleUV=uv;
float d=texture(depth,uv).r-.5;sampleUV=clamp(uv+direction*d*2.*fade,vec2(.001),vec2(.999));color=texture(photo,sampleUV);color.a*=texture(edgeMask,uv).a*strength;}`;
type Entry={art:HTMLElement;image:HTMLImageElement;path:string;version:string;photoSource:string;photo:WebGLTexture;depth:WebGLTexture;mask:WebGLTexture;maskKey:string;maskWidth:number;maskHeight:number;top:number;left:number;width:number;height:number;radius:number;motion:number;strength:number;hover:boolean;hero:boolean;pointerX:number;pointerY:number;targetX:number;targetY:number;visible:boolean;used:number;cacheKey:string;cpuPinned:boolean;releaseTexture:()=>void;resident:{used:number;bytes:number}};
function program(gl:WebGL2RenderingContext){
 const compile=(kind:number,source:string)=>{const shader=gl.createShader(kind)!;gl.shaderSource(shader,source);gl.compileShader(shader);if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(shader)??'Depth shader failed');return shader;};
 const vertex=compile(gl.VERTEX_SHADER,VERTEX),fragment=compile(gl.FRAGMENT_SHADER,FRAGMENT),program=gl.createProgram()!;gl.attachShader(program,vertex);gl.attachShader(program,fragment);gl.linkProgram(program);gl.deleteShader(vertex);gl.deleteShader(fragment);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw new Error('Depth program failed');return program;
}
const subscribeMotion=(change:()=>void)=>{const query=matchMedia('(prefers-reduced-motion: reduce)');query.addEventListener('change',change);return()=>query.removeEventListener('change',change);};
export function useDepthArtwork(root:RefObject<HTMLElement|null>,scroll:RefObject<HTMLElement|null>,enabled:boolean){
 const active=useViewActive();
 const visibility=useRef<((visible:boolean)=>void)|null>(null);
 const [viewportEpoch,setViewportEpoch]=useState(0);const recoveries=useRef(0);
 const reducedMotion=useSyncExternalStore(subscribeMotion,()=>matchMedia('(prefers-reduced-motion: reduce)').matches);
 useLayoutEffect(()=>{
 const node=root.current,viewport=scroll.current,reduced=matchMedia('(prefers-reduced-motion: reduce)');if(!node||!viewport||!enabled||!active||(!native&&!(import.meta.env.DEV&&new URLSearchParams(location.search).has('depthFixture')))||reduced.matches)return;
 let flingTimer:ReturnType<typeof setTimeout>|undefined,flinging=false;
 let viewLeft=0,viewTop=0,scrolling=false,pointerClientX=-1,pointerClientY=-1;
 let modelRevision="",disposed=false,lost=false,suspended=false,resizeTimer:ReturnType<typeof setTimeout>|undefined,ready=false,frame=0,last=0,dirty=true,pending=false,waitingLayout=false,loadFrame=0,velocity=0,previousScroll=viewport.scrollTop,previousScrollTime=performance.now(),readinessTimer:ReturnType<typeof setTimeout>|undefined,baseScroll=0,viewWidth=0,viewHeight=0,clock=0;
 const hero=node.matches('.artist-portrait-stage,.album-artwork-stage,.viewer-depth-stage'),area=hero?node:viewport;
 const interfacePercent=Math.round((parseFloat(getComputedStyle(document.documentElement).fontSize)||16)/16*100);
 const intervals:number[]=[],costs:number[]=[];let reported=0;
 const entries=new Map<HTMLElement,Entry>(),visible=new Set<HTMLElement>(),observed=new Set<HTMLElement>(),near=new Set<HTMLElement>();
 const wrapper=document.createElement('div');wrapper.className='depth-compositor';wrapper.setAttribute('aria-hidden','true');
 const canvas=document.createElement('canvas');wrapper.append(canvas);wrapper.dataset.hero=String(hero);wrapper.style.visibility='hidden';const clip=hero?node:document.createElement('div');if(!hero){clip.className='depth-clip';clip.setAttribute('aria-hidden','true');node.append(clip);}clip.append(wrapper);
 const context=canvas.getContext('webgl2',{alpha:true,premultipliedAlpha:false,antialias:false,depth:false,stencil:false});if(!context){if(!hero)clip.remove();else wrapper.remove();return;}const gl:WebGL2RenderingContext=context;
 // Ordinary covers and their motion layer share standard-range sRGB output.
 if('drawingBufferColorSpace' in gl)gl.drawingBufferColorSpace='srgb';
 if('unpackColorSpace' in gl)gl.unpackColorSpace='srgb';
 let shader:WebGLProgram;try{shader=program(gl);}catch{if(!hero)clip.remove();else wrapper.remove();return;}
 gl.useProgram(shader);const buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([0,0,1,0,0,1,0,1,1,0,1,1]),gl.STATIC_DRAW);
 const point=gl.getAttribLocation(shader,'point');gl.enableVertexAttribArray(point);gl.vertexAttribPointer(point,2,gl.FLOAT,false,0,0);
 const uniforms=Object.fromEntries(['rect','viewport','photo','depth','edgeMask','size','radius','motion','strength','pointer'].map(name=>[name,gl.getUniformLocation(shader,name)]));
 gl.uniform1i(uniforms.photo,0);gl.uniform1i(uniforms.depth,1);gl.uniform1i(uniforms.edgeMask,2);gl.disable(gl.BLEND);
 const remove=(art:HTMLElement)=>{const entry=entries.get(art);if(entry){entry.releaseTexture();if(entry.cpuPinned)prepared.pin(entry.cacheKey,false);gl.deleteTexture(entry.photo);gl.deleteTexture(entry.depth);gl.deleteTexture(entry.mask);entries.delete(art);}delete art.dataset.depthReady;};
 const texture=(image:TexImageSource)=>{const texture=gl.createTexture()!;gl.bindTexture(gl.TEXTURE_2D,texture);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);return texture;};
 Object.defineProperty(canvas,'depthGeometry',{value:()=>[...entries.values()].filter(entry=>visible.has(entry.art)).map(entry=>({path:entry.path,left:entry.left,top:entry.top-(hero?0:viewport!.scrollTop-baseScroll),width:entry.width,height:entry.height,motion:entry.motion,hover:entry.hover,pointerX:entry.pointerX,pointerY:entry.pointerY}))});
 const schedule=()=>{if(!frame&&!disposed&&!lost&&!suspended&&!flinging)frame=requestAnimationFrame(draw);};
 const measure=()=>{const bounds=area.getBoundingClientRect();viewWidth=area.clientWidth;viewHeight=area.clientHeight;baseScroll=Math.max(0,Math.min(viewport.scrollTop,viewport.scrollHeight-viewport.clientHeight));
  viewLeft=bounds.left+area.clientLeft;viewTop=bounds.top+area.clientTop;
  if(!hero){const rootBounds=node.getBoundingClientRect();wrapper.style.left=`${bounds.left+area.clientLeft-rootBounds.left}px`;wrapper.style.top=`${bounds.top+area.clientTop-rootBounds.top}px`;wrapper.style.transform="";wrapper.style.borderRadius=getComputedStyle(viewport).borderTopLeftRadius;}
  wrapper.style.width=`${viewWidth}px`;wrapper.style.height=`${viewHeight}px`;
  const dpr=Math.min(devicePixelRatio,hero?2:1.5,Math.sqrt((hero?4_000_000:2_000_000)/Math.max(1,viewWidth*viewHeight))),width=Math.ceil(viewWidth*dpr),height=Math.ceil(viewHeight*dpr);if(canvas.width!==width||canvas.height!==height){canvas.width=width;canvas.height=height;}canvas.style.width=`${viewWidth}px`;canvas.style.height=`${viewHeight}px`;gl.viewport(0,0,canvas.width,canvas.height);
  for(const entry of entries.values()){const r=entry.art.getBoundingClientRect(),spring=springScale(entry.art),width=r.width/spring.x,height=r.height/spring.y;entry.left=r.left-bounds.left-area.clientLeft+(r.width-width)/2;entry.top=r.top-bounds.top-area.clientTop+(r.height-height)/2;entry.width=width;entry.height=height;if(r.right>bounds.left&&r.left<bounds.right&&r.bottom>bounds.top&&r.top<bounds.bottom)visible.add(entry.art);else{visible.delete(entry.art);delete entry.art.dataset.depthReady;}const radius=getComputedStyle(entry.art).borderTopLeftRadius;entry.radius=radius.endsWith('%')?r.width*parseFloat(radius)/100:parseFloat(radius);
   const smoothing=Number(entry.art.dataset.cornerSmoothing??1),curve=entry.art.style.clipPath;
   const maskWidth=Math.max(1,Math.min(2048,Math.ceil(width*Math.min(devicePixelRatio,2)))),maskHeight=Math.max(1,Math.min(2048,Math.ceil(height*Math.min(devicePixelRatio,2))));
   const extra=(maskWidth*maskHeight-entry.maskWidth*entry.maskHeight)*4;
   if(extra>0&&!reserveTexture(extra)){remove(entry.art);scheduleLoad();continue;}
   if(!entries.has(entry.art))continue;
   entry.resident.bytes+=extra;entry.maskWidth=maskWidth;entry.maskHeight=maskHeight;
   const maskKey=`${maskWidth}:${maskHeight}:${entry.radius}:${smoothing}:${curve}`;
   if(entry.maskKey!==maskKey){let pendingMask=masks.get(maskKey);if(!pendingMask){const image=document.createElement('canvas');image.width=maskWidth;image.height=maskHeight;const ctx=image.getContext('2d')!;ctx.scale(maskWidth/width,maskHeight/height);const path=new Path2D(curve.startsWith('path(')?curve.slice(6,-2):smoothPath(width,height,entry.radius,smoothing));ctx.fillStyle='#fff';ctx.fill(path);ctx.globalCompositeOperation='destination-out';ctx.lineWidth=2;ctx.stroke(path);pendingMask=Promise.resolve(image);masks.set(maskKey,pendingMask);}
    void pendingMask.then(image=>{if(disposed||lost||!entries.has(entry.art))return;gl.activeTexture(gl.TEXTURE2);gl.bindTexture(gl.TEXTURE_2D,entry.mask);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);entry.maskKey=maskKey;schedule();});
   }
  }
  dirty=false;
 };
 const deferLayout=()=>{const layout=[...(node.closest('.page-content')?.getAnimations({subtree:true})??[]),...(node.closest('dialog.artwork-viewer')?.getAnimations()??[])].filter(a=>(a.id==='page-entrance'||a.id==='grid-reflow'||('animationName' in a&&a.animationName==='artwork-open'))&&a.playState==='running');
  if(!layout.length)return false;for(const entry of entries.values())delete entry.art.dataset.depthReady;wrapper.style.visibility='hidden';if(!waitingLayout){waitingLayout=true;void Promise.allSettled(layout.map(a=>a.finished)).then(()=>{waitingLayout=false;if(!disposed){dirty=true;schedule();scheduleLoad();}});}return true;};
 const scheduleLoad=()=>{if(!loadFrame&&!disposed&&!lost&&!flinging&&!suspended)loadFrame=requestAnimationFrame(()=>{loadFrame=0;setTimeout(()=>{if(!disposed)void loadNext();},0);});};
 const loadNext=async()=>{if(pending||!ready||disposed||lost||suspended||flinging)return;if(deferLayout())return;
  const art=[...new Set([...visible,...near])].find(art=>{const img=art.querySelector<HTMLImageElement>('img[data-artwork-path]');return img?.complete&&img.naturalWidth>0&&img.dataset.artworkPath&&!entries.has(art)&&art.dataset.depthUnavailable!==`${img.dataset.artworkPath}:${img.dataset.artworkVersion??'0'}`;});if(!art)return;
  const image=art.querySelector<HTMLImageElement>('img[data-artwork-path]')!,path=image.dataset.artworkPath!,version=image.dataset.artworkVersion??'0',photoSource=image.currentSrc||image.src,key=`${modelRevision}:${path}:${version}`;pending=true;
  let photo:WebGLTexture|null=null,depth:WebGLTexture|null=null,mask:WebGLTexture|null=null;
  try{let request=requests.get(key);if(!request){request=call('depth_artwork',{path});requests.set(key,request);}const map=await request;
   if(disposed||lost||flinging||suspended||!art.isConnected||image.dataset.artworkPath!==path||(image.dataset.artworkVersion??'0')!==version||(image.currentSrc||image.src)!==photoSource)return;
   if(!map){requests.delete(key);art.dataset.depthUnavailable=`${path}:${version}`;setTimeout(()=>{if(!disposed){delete art.dataset.depthUnavailable;scheduleLoad();}},2000);return;}
   const full=art.classList.contains("viewer-artwork"),preparedKey=`${map.identity??key}:${full}:${photoSource}`;
   let decoded=prepared.get(preparedKey);
   if(!decoded){decoded=(async()=>{const depthImage=new Image();depthImage.src=full?(map.full_map??map.map):map.map;const photoImage=image;await Promise.all([depthImage.decode(),photoImage.decode()]);
    const photo=document.createElement('canvas');photo.width=full?Math.max(1,Math.round(2048*photoImage.naturalWidth/Math.max(photoImage.naturalWidth,photoImage.naturalHeight))):512;photo.height=full?Math.max(1,Math.round(2048*photoImage.naturalHeight/Math.max(photoImage.naturalWidth,photoImage.naturalHeight))):512;const side=Math.min(photoImage.naturalWidth,photoImage.naturalHeight);if(full)photo.getContext('2d',{colorSpace:'srgb'})!.drawImage(photoImage,0,0,photo.width,photo.height);else photo.getContext('2d',{colorSpace:'srgb'})!.drawImage(photoImage,(photoImage.naturalWidth-side)/2,(photoImage.naturalHeight-side)/2,side,side,0,0,512,512);
    const depth=document.createElement('canvas');depth.width=depth.height=256;const depthContext=depth.getContext('2d')!;depthContext.filter='blur(3px)';depthContext.drawImage(depthImage,-3,-3,262,262);return {photo,depth};})();
    prepared.set(preparedKey,decoded);decoded.catch(()=>prepared.delete(preparedKey));
   }
   const images=await decoded;
   if(disposed||lost||flinging||suspended||!art.isConnected||image.dataset.artworkPath!==path||(image.dataset.artworkVersion??'0')!==version||(image.currentSrc||image.src)!==photoSource)return;
   const rect=art.getBoundingClientRect(),maskWidth=Math.max(1,Math.min(2048,Math.ceil(rect.width*Math.min(devicePixelRatio,2)))),maskHeight=Math.max(1,Math.min(2048,Math.ceil(rect.height*Math.min(devicePixelRatio,2))));
   const bytes=(images.photo.width*images.photo.height+images.depth.width*images.depth.height+maskWidth*maskHeight)*4;
   if(!reserveTexture(bytes)){art.dataset.depthUnavailable=`${path}:${version}`;setTimeout(()=>{if(!disposed){delete art.dataset.depthUnavailable;scheduleLoad();}},500);return;}
   photo=texture(images.photo);depth=texture(images.depth);const edgeCanvas=document.createElement('canvas');edgeCanvas.width=maskWidth;edgeCanvas.height=maskHeight;mask=texture(edgeCanvas);
   const resident={bytes,used:performance.now(),pinned:()=>visible.has(art),evict:()=>remove(art)};
   const releaseTexture=registerTexture(resident),cpuPinned=visible.has(art);if(cpuPinned)prepared.pin(preparedKey,true);
   entries.set(art,{art,image,path,version,photoSource,photo,depth,mask,maskKey:'',maskWidth,maskHeight,cacheKey:preparedKey,cpuPinned,releaseTexture,resident,top:0,left:0,width:0,height:0,radius:0,motion:0,strength:1,hover:!scrolling&&(art.matches(':hover,:focus-within')||!!art.closest('.card-open')?.matches(':hover,:focus-visible')),hero:art.matches('.artist-portrait-button,.album-artwork,.viewer-artwork'),pointerX:0,pointerY:0,targetX:0,targetY:0,visible:true,used:clock++});photo=depth=mask=null;dirty=true;schedule();
  }catch{requests.delete(key);if(!disposed){art.dataset.depthUnavailable=`${path}:${version}`;setTimeout(()=>{if(!disposed){delete art.dataset.depthUnavailable;scheduleLoad();}},2000);}}finally{if(photo)gl.deleteTexture(photo);if(depth)gl.deleteTexture(depth);if(mask)gl.deleteTexture(mask);pending=false;scheduleLoad();}
 };
 function draw(now:number){frame=0;if(disposed||lost||suspended||reduced.matches)return;const started=performance.now();if(dirty&&deferLayout()){last=0;return;}const layoutChanged=dirty;if(dirty)measure();for(const [art,entry] of entries){const top=entry.top-(hero?0:viewport!.scrollTop-baseScroll),onScreen=top+entry.height>0&&top<viewHeight&&entry.left+entry.width>0&&entry.left<viewWidth;if(onScreen)visible.add(art);else visible.delete(art);if(entry.cpuPinned!==onScreen){prepared.pin(entry.cacheKey,onScreen);entry.cpuPinned=onScreen;}if(onScreen)entry.resident.used=performance.now();}
 gl.clearColor(0,0,0,0);gl.clear(gl.COLOR_BUFFER_BIT);gl.useProgram(shader);gl.uniform2f(uniforms.viewport,viewWidth,viewHeight);
  const interval=last?now-last:0;if(interval>0&&interval<100){intervals.push(interval);if(intervals.length>240)intervals.shift();}const dt=last?Math.min(200,now-last):8.33;last=now;let moving=false;const delta=hero?0:viewport!.scrollTop-baseScroll;velocity*=Math.exp(-dt/85);
  for(const entry of entries.values()){
   if(!visible.has(entry.art))continue;if(Math.abs(entry.targetX)+Math.abs(entry.targetY)<.04&&Math.abs(entry.pointerX)+Math.abs(entry.pointerY)<.04&&Math.abs(velocity)<.025&&Math.abs(entry.motion)<.02){delete entry.art.dataset.depthReady;continue;}entry.used=clock++;const top=entry.top-delta;
   const target=Math.abs(velocity)<.025?0:Math.max(-1.5,Math.min(1.5,velocity*.35));
   const next=entry.motion+(target-entry.motion)*(1-Math.exp(-dt/32));entry.motion=Math.abs(target-next)<.02?target:next;moving ||=Math.abs(target-entry.motion)>.02;
   const strength=1;entry.strength+=(strength-entry.strength)*(1-Math.exp(-dt/32));if(Math.abs(strength-entry.strength)<.01)entry.strength=strength;moving ||= Math.abs(strength-entry.strength)>.01;
   entry.pointerX+=(entry.targetX-entry.pointerX)*(1-Math.exp(-dt/45));entry.pointerY+=(entry.targetY-entry.pointerY)*(1-Math.exp(-dt/45));moving ||= Math.abs(entry.targetX-entry.pointerX)+Math.abs(entry.targetY-entry.pointerY)>.02;
   const spring=springScale(entry.art);moving ||= spring.moving;const w=entry.width*spring.x,h=entry.height*spring.y;
   gl.uniform4f(uniforms.rect,entry.left+(entry.width-w)/2,top+(entry.height-h)/2,w,h);gl.uniform1f(uniforms.strength,entry.strength);gl.uniform2f(uniforms.size,entry.width,entry.height);gl.uniform1f(uniforms.radius,entry.radius);gl.uniform1f(uniforms.motion,entry.motion);gl.uniform2f(uniforms.pointer,entry.pointerX,entry.pointerY);
   gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,entry.photo);gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,entry.depth);gl.activeTexture(gl.TEXTURE2);gl.bindTexture(gl.TEXTURE_2D,entry.mask);gl.drawArrays(gl.TRIANGLES,0,6);
  }
  const cost=performance.now()-started;costs.push(cost);if(costs.length>240)costs.shift();
  if(now-reported>3000&&intervals.length>=20){reported=now;const percentile=(values:number[],p:number)=>[...values].sort((a,b)=>a-b)[Math.min(values.length-1,Math.floor(values.length*p))]??0;
   void call('artwork_performance',{sample:{width:Math.round(viewWidth),height:Math.round(viewHeight),frames:intervals.length,interface_percent:interfacePercent,raf_median_ms:percentile(intervals,.5),raf_p95_ms:percentile(intervals,.95),cpu_draw_p95_ms:percentile(costs,.95),over_120hz_budget:intervals.filter(v=>v>12.5).length/intervals.length}}).catch(()=>{});
  }
  canvas.dataset.drawMs=cost.toFixed(2);canvas.dataset.visible=String(visible.size);canvas.dataset.ramBytes=String(prepared.bytes+masks.bytes);canvas.dataset.textureBytes=String(textureBytes());canvas.dataset.cacheHits=String(prepared.hits);canvas.dataset.resident=String(entries.size);
  if(gl.isContextLost()||(layoutChanged&&gl.getError()!==gl.NO_ERROR)){fallback();return;}
  for(const entry of entries.values())if(visible.has(entry.art)&&entry.width>0&&entry.height>0&&(entry.hover||Math.abs(entry.pointerX)+Math.abs(entry.pointerY)>=.04||Math.abs(entry.motion)>=.02))entry.art.dataset.depthReady='true';
  wrapper.style.visibility='';
  if(moving)schedule();else last=0;scheduleLoad();
 }
 const observe=new IntersectionObserver(records=>{for(const record of records){const art=record.target as HTMLElement;if(record.isIntersecting)near.add(art);else near.delete(art);}scheduleLoad();},{root:viewport,rootMargin:'400px 0px'});
 const scan=()=>{const current=new Set(node.querySelectorAll<HTMLElement>('.card-artwork,.artist-portrait-button,.album-artwork,.viewer-artwork'));for(const art of observed)if(!current.has(art)){observe.unobserve(art);ro.unobserve(art);observed.delete(art);visible.delete(art);near.delete(art);remove(art);}for(const art of current)if(!observed.has(art)){observed.add(art);observe.observe(art);ro.observe(art);}for(const [art,entry] of entries)if(entry.image.dataset.artworkPath!==entry.path||(entry.image.dataset.artworkVersion??'0')!==entry.version||(entry.image.currentSrc||entry.image.src)!==entry.photoSource)remove(art);for(const art of current)if(art.dataset.depthUnavailable!==`${art.querySelector<HTMLImageElement>('img')?.dataset.artworkPath}:${art.querySelector<HTMLImageElement>('img')?.dataset.artworkVersion??'0'}`)delete art.dataset.depthUnavailable;dirty=true;schedule();scheduleLoad();};
 const mutations=new MutationObserver(records=>{if(records.some(record=>record.type==='attributes'||[...record.addedNodes,...record.removedNodes].some(child=>child instanceof Element&&(child.matches('.card-artwork,.artist-portrait-button,.album-artwork,.viewer-artwork,img[data-artwork-path]')||child.querySelector('.card-artwork,.artist-portrait-button,.album-artwork,.viewer-artwork,img[data-artwork-path]')))))scan();});mutations.observe(node,{childList:true,subtree:true,attributes:true,attributeFilter:['data-artwork-path','data-artwork-version']});
 const resize=()=>{dirty=true;schedule();};const rebuild=()=>{dirty=true;schedule();};const ro=new ResizeObserver(()=>{if(viewWidth>0&&(area.clientWidth!==viewWidth||area.clientHeight!==viewHeight))rebuild();else resize();});ro.observe(node);ro.observe(viewport);

 const hover=(e:Event)=>{if(e.type.startsWith('pointer')&&(e as PointerEvent).pointerType==='touch')return;const control=e.target instanceof Element?e.target.closest('.card-open,.artist-portrait-button,.album-artwork,.viewer-artwork'):null,art=control?.matches('.artist-portrait-button,.album-artwork,.viewer-artwork')?control as HTMLElement:control?.querySelector<HTMLElement>('.card-artwork');if(!art)return;const entry=entries.get(art);if(!entry)return;if((e.type==='pointerout'||e.type==='focusout')&&control?.contains((e as PointerEvent).relatedTarget as Node|null))return;entry.hover=e.type==='focusin'||(e.type==='pointerover'&&!scrolling);if(!entry.hover){entry.targetX=entry.targetY=0;}else if(e.type==='focusin'){entry.targetX=1;entry.targetY=-.5;}schedule();};
 const pointer=(e:PointerEvent)=>{if(e.pointerType==='touch'||flinging)return;const moved=Math.abs(e.clientX-pointerClientX)+Math.abs(e.clientY-pointerClientY)>.5;if(scrolling&&!moved)return;pointerClientX=e.clientX;pointerClientY=e.clientY;scrolling=false;const art=e.target instanceof Element?e.target.closest('.card-open,.artist-portrait-button,.album-artwork,.viewer-artwork')?.querySelector<HTMLElement>('.card-artwork')??e.target.closest<HTMLElement>('.artist-portrait-button,.album-artwork,.viewer-artwork'):null;
  const entry=hero?[...entries.values()][0]:art?entries.get(art):undefined;if(!entry)return;entry.hover=true;
  const left=viewLeft+entry.left,top=viewTop+entry.top-(hero?0:viewport!.scrollTop-baseScroll);
  entry.targetX=Math.max(-3,Math.min(3,(e.clientX-left-entry.width/2)/Math.max(1,entry.width)*6));entry.targetY=Math.max(-3,Math.min(3,(e.clientY-top-entry.height/2)/Math.max(1,entry.height)*6));schedule();};
 const scrolled=()=>{const now=performance.now(),position=Math.max(0,Math.min(viewport.scrollTop,viewport.scrollHeight-viewport.clientHeight));velocity=(position-previousScroll)/Math.max(8,now-previousScrollTime);previousScroll=position;previousScrollTime=now;scrolling=true;if(!hero)wrapper.style.transform=`translateY(${Math.max(0,Math.min(viewport.scrollTop,viewport.scrollHeight-viewport.clientHeight))-baseScroll}px)`;for(const entry of entries.values()){entry.hover=entry.art.matches(':focus-within');entry.targetX=entry.targetY=entry.pointerX=entry.pointerY=0;delete entry.art.dataset.depthReady;}clearTimeout(flingTimer);flinging ||= Math.abs(velocity)>1.2;
  if(flinging){wrapper.style.visibility='hidden';cancelAnimationFrame(frame);frame=0;last=0;}else{schedule();scheduleLoad();}
  flingTimer=setTimeout(()=>{flinging=false;scrolling=false;velocity=0;
   for(const entry of entries.values())if(visible.has(entry.art)){
    const focused=entry.art.matches(':focus-within')||!!entry.art.closest('.card-open')?.matches(':focus-visible');
    const hovered=entry.art.matches(':hover')||!!entry.art.closest('.card-open')?.matches(':hover');
    entry.hover=focused||hovered;if(focused){entry.targetX=1;entry.targetY=-.5;}
   }
   dirty=true;schedule();scheduleLoad();},100);
 };
 const fallback=()=>{if(disposed||lost)return;wrapper.style.visibility='hidden';lost=true;cancelAnimationFrame(frame);for(const art of observed)delete art.dataset.depthReady;canvas.style.visibility='hidden';if(!reduced.matches&&recoveries.current<2){recoveries.current++;resizeTimer=setTimeout(()=>{if(!disposed)setViewportEpoch(epoch=>epoch+1);},250);}};
 const contextLost=(e:Event)=>{e.preventDefault();fallback();};canvas.addEventListener('webglcontextlost',contextLost);
 const readiness=(e:Event)=>{const next=!!(e as CustomEvent).detail;if(next===ready)return;ready=next;if(!ready){for(const art of observed)remove(art);}else{for(const art of observed)delete art.dataset.depthUnavailable;checkReady();}schedule();};
 const invalidated=()=>{for(const art of observed)delete art.dataset.depthUnavailable;scan();};
 window.addEventListener('musicos-artwork-invalidated',invalidated);
 const imageLoaded=()=>{scan();};node.addEventListener('load',imageLoaded,true);
 const motionChanged=()=>{if(reduced.matches)fallback();};
 viewport.addEventListener('scroll',scrolled,{passive:true});node.addEventListener('pointermove',pointer);node.addEventListener('musicos-grid-layout',resize);window.addEventListener('resize',rebuild);window.visualViewport?.addEventListener('resize',rebuild);window.addEventListener('musicos-depth-ready',readiness);reduced.addEventListener('change',motionChanged);
 for(const event of ['pointerover','pointerout','focusin','focusout'])node.addEventListener(event,hover);
 const checkReady=()=>void call('depth_setup',{action:'status'}).then(s=>{if(disposed)return;ready=s.ready;modelRevision=s.revision??"current";scan();if(s.running&&!suspended)readinessTimer=setTimeout(checkReady,1500);}).catch(()=>{});checkReady();scan();
 visibility.current=isVisible=>{
  suspended=!isVisible;clearTimeout(readinessTimer);
  if(suspended){cancelAnimationFrame(frame);cancelAnimationFrame(loadFrame);frame=loadFrame=0;last=0;wrapper.style.visibility='hidden';
   for(const entry of entries.values()){entry.hover=false;entry.targetX=entry.targetY=entry.pointerX=entry.pointerY=entry.motion=0;if(entry.cpuPinned){prepared.pin(entry.cacheKey,false);entry.cpuPinned=false;}delete entry.art.dataset.depthReady;}visible.clear();
  }else{previousScroll=viewport.scrollTop;previousScrollTime=performance.now();dirty=true;scan();if(!ready)checkReady();}
 };
 return()=>{visibility.current=null;disposed=true;clearTimeout(flingTimer);clearTimeout(resizeTimer);canvas.removeEventListener('webglcontextlost',contextLost);window.removeEventListener('musicos-artwork-invalidated',invalidated);clearTimeout(readinessTimer);node.removeEventListener('load',imageLoaded,true);cancelAnimationFrame(frame);cancelAnimationFrame(loadFrame);node.removeEventListener('pointermove',pointer);node.removeEventListener('musicos-grid-layout',resize);window.visualViewport?.removeEventListener('resize',rebuild);observe.disconnect();mutations.disconnect();ro.disconnect();viewport.removeEventListener('scroll',scrolled);window.removeEventListener('resize',rebuild);window.removeEventListener('musicos-depth-ready',readiness);reduced.removeEventListener('change',motionChanged);for(const event of ['pointerover','pointerout','focusin','focusout'])node.removeEventListener(event,hover);for(const art of observed){remove(art);delete art.dataset.depthUnavailable;}gl.deleteBuffer(buffer);gl.deleteProgram(shader);gl.getExtension('WEBGL_lose_context')?.loseContext();if(!hero)clip.remove();else wrapper.remove();};
 },[root,scroll,enabled,reducedMotion,viewportEpoch]);
 useLayoutEffect(()=>{if(visibility.current)visibility.current(active);else if(active&&enabled)setViewportEpoch(epoch=>epoch+1);},[active,enabled]);
}
