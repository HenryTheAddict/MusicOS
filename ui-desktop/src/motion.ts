import { useLayoutEffect, useRef, type RefObject } from "react";

// Native resizing remains continuous. At a column breakpoint, animate artwork
// between the old and new layout using transforms; never tween layout or scroll.
export function useGridReflow(root: RefObject<HTMLElement | null>, columns: number, width: number) {
  const previous = useRef<{ columns: number; boxes: Map<string, { x: number; y: number; width: number }> } | null>(null);
  const running = useRef<Animation[]>([]);
  useLayoutEffect(() => {
    const node = root.current;
    if (!node) return;
    const rearranged = previous.current && columns !== previous.current.columns;
    // Width changes during native resizing must not restart a layout animation.
    // Animate only once, when the settled column count actually changes.
    if (rearranged) {
      running.current.forEach(a => a.cancel());
      running.current = [];
    }
    const origin = node.getBoundingClientRect();
    const boxes = new Map<string, { x: number; y: number; width: number }>();
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    node.querySelectorAll<HTMLElement>("[data-collection-id]").forEach(card => {
      const rect = card.getBoundingClientRect();
      const id = card.dataset.collectionId!;
      const target = { x: rect.left - origin.left, y: rect.top - origin.top, width: rect.width };
      boxes.set(id, target);
      if (!rearranged || reduced.matches || rect.bottom < 0 || rect.top > innerHeight) return;
      const old = previous.current?.boxes.get(id);
      const enter = card.animate(old
        ? [{ transform: `translate(${old.x - target.x}px, ${old.y - target.y}px)` }, { transform: "none" }]
        : [{ opacity: 0 }, { opacity: 1 }],
        { duration: 280, easing: "cubic-bezier(.25,.1,.25,1)" });
      enter.id = "grid-reflow";
      running.current.push(enter);
    });
    previous.current = { columns, boxes };
    const changed = () => { if (reduced.matches) running.current.forEach(a => a.cancel()); };
    reduced.addEventListener("change", changed);
    return () => reduced.removeEventListener("change", changed);
  }, [root, columns, width]);
  useLayoutEffect(() => () => running.current.forEach(a => a.cancel()), []);
}

// Animate the existing page rather than remounting it: scroll restoration, focus,
// virtualization and the authoritative playback state stay intact.
export function usePageEntrance(
  root: RefObject<HTMLElement | null>,
  location: string,
  ready: boolean,
) {
  useLayoutEffect(() => {
    const node = root.current;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    if (!node || !ready || reduced.matches) return;
    const animations: Animation[] = [];
    const enter = (selector: string, delay: number, kind: "art" | "text" | "control" = "text") => {
      node.querySelectorAll<HTMLElement>(selector).forEach((element, index) => {
        const transform = kind === "art"
          ? "translateY(8px) scale(.97)"
          : kind === "control"
            ? "translateY(4px) scale(.8)"
            : "translateY(6px)";
        const animation = element.animate(
          [{ opacity: 0, transform }, { opacity: 1, transform: "none" }],
          { duration: kind === "art" ? 240 : 180, delay: delay + index * 24, easing: "cubic-bezier(.2,.7,.2,1)", fill: "backwards" },
        );
        animation.id = "page-entrance";
        animations.push(animation);
      });
    };
    if (node.querySelector(".album-detail, .artist-detail")) {
      enter(".album-hero > .cover, .album-artwork-stage, .artist-hero > .cover", 0, "art");
      enter(".album-copy h1, .artist-hero > h1", 24);
      enter(".artist-link, .quality, .artist-about, .artist-statistics > :not(.collection-actions)", 48);
      enter(".collection-actions > button", 72, "control");
      enter(".section-title, .album-strip, .track-table", 100);
    } else if (node.querySelector(".settings")) {
      enter(".page-heading", 0);
      enter(".settings-tile", 24, "art");
      enter(".beta-integrations", 96);
    } else {
      enter(".page-content > *", 0);
    }
    const stop = () => animations.forEach(a => a.cancel());
    const changed = () => { if (reduced.matches) stop(); };
    reduced.addEventListener("change", changed);
    return () => { stop(); reduced.removeEventListener("change", changed); };
  }, [root, location, ready]);
}

export function parallaxOffset(center:number,top:number,height:number) {
 return Math.max(-14,Math.min(14,((center-top)/Math.max(1,height)-.5)*40));
}
export function useArtworkParallax(root:RefObject<HTMLElement|null>,scroll:RefObject<HTMLElement|null>,enabled=true){
 useLayoutEffect(()=>{const node=root.current,viewport=scroll.current;if(!node||!viewport||!enabled)return;
 const reduced=matchMedia('(prefers-reduced-motion: reduce)');let frame=0,last=0,dirty=true;
 const visible=new Set<HTMLElement>(),positions=new Map<HTMLElement,number>(),centers=new Map<HTMLElement,number>(),hovered=new Set<HTMLElement>();
 let top=0,height=1,measuredScroll=0;
 const schedule=()=>{if(!frame)frame=requestAnimationFrame(update);};
 const measure=()=>{const bounds=viewport.getBoundingClientRect();top=bounds.top;height=bounds.height;measuredScroll=viewport.scrollTop;
  for(const art of visible){const rect=art.getBoundingClientRect();centers.set(art,rect.top+rect.height/2-(art.querySelector('.portrait')?(positions.get(art)??0):0));}dirty=false;};
 const update=(now:number)=>{frame=0;if(dirty)measure();const dt=last?Math.min(200,now-last):8.33;last=now;const blend=1-Math.exp(-dt/32);let moving=false;
  for(const art of visible){const current=positions.get(art)??0;
   const target=reduced.matches||hovered.has(art)||art.dataset.depthReady==='true'?0:Math.max(-3,Math.min(3,parallaxOffset((centers.get(art)??0)-(viewport.scrollTop-measuredScroll),top,height)));
   const next=reduced.matches||Math.abs(target-current)<.02?target:current+(target-current)*blend;
   positions.set(art,next);art.style.setProperty('--parallax-y',`${next}px`);
   // Stable overscan prevents zoom pumping as the scroll direction changes.
   art.style.setProperty('--parallax-scale',art.querySelector('.portrait')||hovered.has(art)||reduced.matches?'1':'1.04');
   moving ||= Math.abs(target-next)>=.02;
  }if(moving)schedule();else last=0;
 };
 const observer=new IntersectionObserver(entries=>{for(const entry of entries){const art=entry.target as HTMLElement;if(entry.isIntersecting)visible.add(art);else{visible.delete(art);positions.delete(art);centers.delete(art);art.style.removeProperty('--parallax-y');art.style.removeProperty('--parallax-scale');}}dirty=true;schedule();},{root:viewport});
 const observed=new Set<HTMLElement>();
 const scan=()=>{const current=new Set(node.querySelectorAll<HTMLElement>('.card-artwork'));for(const art of observed){if(!current.has(art)){observer.unobserve(art);observed.delete(art);visible.delete(art);positions.delete(art);centers.delete(art);hovered.delete(art);}}for(const art of current){if(!observed.has(art)){observed.add(art);observer.observe(art);}}dirty=true;schedule();};
 const mutations=new MutationObserver(records=>{if(records.some(r=>r.type==='childList'))scan();else schedule();});mutations.observe(node,{childList:true,subtree:true,attributes:true,attributeFilter:['data-depth-ready']});
 const artAt=(target:EventTarget|null)=>target instanceof Element?target.closest<HTMLElement>('.card-open')?.querySelector<HTMLElement>('.card-artwork'):null;
 const enter=(e:Event)=>{const art=artAt(e.target);if(art){hovered.add(art);schedule();}};
 const leave=(e:Event)=>{const art=artAt(e.target);if(art&&!art.closest('.card-open')?.contains((e as PointerEvent).relatedTarget as Node|null)){hovered.delete(art);schedule();}};
 const pointer=(e:PointerEvent)=>{const art=artAt(e.target);if(!art||!art.querySelector('.portrait')||art.dataset.depthReady==='true'||reduced.matches)return;const box=art.getBoundingClientRect();art.style.setProperty('--cover-tilt-x',`${Math.max(-3,Math.min(3,-(e.clientY-box.top-box.height/2)/box.height*6))}deg`);art.style.setProperty('--cover-tilt-y',`${Math.max(-3,Math.min(3,(e.clientX-box.left-box.width/2)/box.width*6))}deg`);};
 const neutral=(e:Event)=>{const art=artAt(e.target);if(art&&!art.closest('.card-open')?.contains((e as PointerEvent).relatedTarget as Node|null)){art.style.removeProperty('--cover-tilt-x');art.style.removeProperty('--cover-tilt-y');}};
 node.addEventListener('pointermove',pointer);node.addEventListener('pointerout',neutral);
 const resize=()=>{dirty=true;schedule();};const ro=new ResizeObserver(resize);ro.observe(viewport);ro.observe(node);
 scan();viewport.addEventListener('scroll',schedule,{passive:true});window.addEventListener('resize',resize);reduced.addEventListener('change',schedule);
 node.addEventListener('pointerover',enter);node.addEventListener('pointerout',leave);node.addEventListener('focusin',enter);node.addEventListener('focusout',leave);
 return()=>{node.removeEventListener('pointermove',pointer);node.removeEventListener('pointerout',neutral);cancelAnimationFrame(frame);observer.disconnect();mutations.disconnect();ro.disconnect();viewport.removeEventListener('scroll',schedule);window.removeEventListener('resize',resize);reduced.removeEventListener('change',schedule);
 node.removeEventListener('pointerover',enter);node.removeEventListener('pointerout',leave);node.removeEventListener('focusin',enter);node.removeEventListener('focusout',leave);
 for(const art of observed){art.style.removeProperty('--cover-tilt-x');art.style.removeProperty('--cover-tilt-y');art.style.removeProperty('--parallax-y');art.style.removeProperty('--parallax-scale');}};
 },[root,scroll,enabled]);
}

// One shared frame loop. Reversing hover retains both displacement and velocity.
export type JellyAnimation={cancel:()=>void;onfinish:(()=>void)|null};
type Spring={element:HTMLElement;x:number;y:number;vx:number;vy:number;last:number;start:number;active:boolean;handle:JellyAnimation};
const springs=new Map<HTMLElement,Spring>();let springFrame=0;
export function springStep(x:number,v:number,target:number,dt:number):[number,number]{
 const w=20,z=.78,a=z*w,b=w*Math.sqrt(1-z*z),d=x-target,e=Math.exp(-a*dt),c=Math.cos(b*dt),s=Math.sin(b*dt),q=(v+a*d)/b;
 return [target+e*(d*c+q*s),e*(v*c-(a*q+b*d)*s)];
}
function tickSprings(now:number){springFrame=0;
 for(const [element,state] of springs){const dt=Math.min(1,Math.max(0,(now-state.last)/1000));state.last=now;
  const pulse=state.active&&now-state.start<85;
  [state.x,state.vx]=springStep(state.x,state.vx,pulse ? .009 : 0,dt);[state.y,state.vy]=springStep(state.y,state.vy,pulse ? -.006 : 0,dt);
  element.style.scale=`${1+Math.max(-.015,Math.min(.015,state.x))} ${1+Math.max(-.015,Math.min(.015,state.y))}`;
  if(!element.isConnected||now-state.start>=440||(!pulse&&Math.abs(state.x)+Math.abs(state.y)+Math.abs(state.vx)+Math.abs(state.vy)<.0005)){
   element.style.removeProperty('scale');springs.delete(element);state.handle.onfinish?.();
  }
 }if(springs.size)springFrame=requestAnimationFrame(tickSprings);
}
export function springScale(element:HTMLElement){const s=springs.get(element);return {x:1+(s?.x??0),y:1+(s?.y??0),moving:!!s};}
export function jelly(element:HTMLElement,active:boolean,_previous?:JellyAnimation){
 const existing=springs.get(element);if(existing){existing.active=active;if(active)existing.start=performance.now();return existing.handle;}
 const handle:JellyAnimation={onfinish:null,cancel:()=>{springs.delete(element);element.style.removeProperty('scale');}};
 const now=performance.now();springs.set(element,{element,x:0,y:0,vx:0,vy:0,last:now,start:now,active,handle});
 if(!springFrame)springFrame=requestAnimationFrame(tickSprings);return handle;
}
export function useJellyMotion(){useLayoutEffect(()=>{
 const reduced=matchMedia('(prefers-reduced-motion: reduce)'),running=new Map<HTMLElement,JellyAnimation>();
 const selector='.card-open, .album-strip > button, .collection-actions > button, .collection-shuffle, .row-play, .sketch-button';
 const target=(node:EventTarget|null)=>node instanceof Element?node.closest<HTMLElement>(selector):null;
 const animate=(control:HTMLElement,active:boolean)=>{const element=control.matches('.card-open,.album-strip > button')?control.querySelector<HTMLElement>('.card-artwork,.cover'):control;if(!element||control.matches('.card-open')&&!control.closest('[data-artist-name]'))return;
  if(reduced.matches){running.get(element)?.cancel();running.delete(element);return;}
  const animation=jelly(element,active,running.get(element));running.set(element,animation);animation.onfinish=()=>running.delete(element);
 };
 const enter=(e:Event)=>{const control=target(e.target);if(control&&!control.contains((e as PointerEvent).relatedTarget as Node|null))animate(control,true);};
 const leave=(e:Event)=>{const control=target(e.target);if(control&&!control.contains((e as PointerEvent).relatedTarget as Node|null))animate(control,false);};
 const stop=()=>{running.forEach(a=>a.cancel());running.clear();};
 document.addEventListener('pointerover',enter);document.addEventListener('pointerout',leave);document.addEventListener('focusin',enter);document.addEventListener('focusout',leave);reduced.addEventListener('change',stop);
 return()=>{stop();document.removeEventListener('pointerover',enter);document.removeEventListener('pointerout',leave);document.removeEventListener('focusin',enter);document.removeEventListener('focusout',leave);reduced.removeEventListener('change',stop);};
},[]);}

export function useOverscrollFeedback(scroll:RefObject<HTMLElement|null>,ready:boolean){
 useLayoutEffect(()=>{const viewport=scroll.current;if(!viewport||!ready)return;const reduced=matchMedia('(prefers-reduced-motion: reduce)');let frame=0,last=0,offset=0,velocity=0,impulse=0;
 const write=()=>{const height=Math.max(1,viewport.clientHeight),stretch=1+Math.min(.012,Math.abs(offset)/height),blur=Math.min(.45,Math.abs(offset)/30);
  const apply=(el:HTMLElement)=>{el.style.setProperty('--overscroll-y',`${offset.toFixed(3)}px`);el.style.setProperty('--overscroll-scale',String(stretch));el.style.setProperty('--overscroll-origin',offset>=0?'top':'bottom');el.style.setProperty('--overscroll-blur',`${blur.toFixed(3)}px`);};
  apply(viewport);document.querySelectorAll<HTMLElement>('.depth-compositor[data-hero=false]').forEach(apply);
 };
 const settle=(now:number)=>{frame=0;const dt=last?Math.min(.2,(now-last)/1000):1/120;last=now;
  [offset,velocity]=springStep(offset,velocity,0,dt);if(Math.abs(offset)+Math.abs(velocity)<.05){offset=velocity=0;last=0;impulse=0;}write();if(offset||velocity)frame=requestAnimationFrame(settle);
 };
 const wheel=(e:WheelEvent)=>{if(reduced.matches||e.ctrlKey)return;const delta=e.deltaY*(e.deltaMode===1?16:e.deltaMode===2?viewport.clientHeight:1);
  if((viewport.scrollTop<=0&&delta<0)||(viewport.scrollTop>=viewport.scrollHeight-viewport.clientHeight-1&&delta>0)){
   velocity=Math.max(-180,Math.min(180,velocity-delta*.7));impulse=0;if(!frame){last=performance.now();frame=requestAnimationFrame(settle);}
  }
 };
 const reset=()=>{cancelAnimationFrame(frame);frame=0;offset=velocity=impulse=last=0;write();};viewport.addEventListener('wheel',wheel,{passive:true});reduced.addEventListener('change',reset);
 return()=>{reset();viewport.removeEventListener('wheel',wheel);reduced.removeEventListener('change',reset);for(const name of ['--overscroll-y','--overscroll-scale','--overscroll-origin','--overscroll-blur'])viewport.style.removeProperty(name);};
 },[scroll,ready]);
}
