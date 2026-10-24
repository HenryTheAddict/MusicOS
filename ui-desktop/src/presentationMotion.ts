/** One finite animation per element/property owner, shared by navigation panels. */
const owners=new WeakMap<HTMLElement,Map<string,Animation>>();
export function animateOwned(node:HTMLElement,key:string,frames:Keyframe[],duration:number){
 const map=owners.get(node)??new Map<string,Animation>();owners.set(node,map);const old=[...map.values()].find(animation=>animation.playState==='running');
 if(old?.playState==='running'){const style=getComputedStyle(node);frames=[{...frames[0],...(frames[0].opacity!==undefined?{opacity:style.opacity}:{}),...(frames[0].transform!==undefined?{transform:style.transform}:{})},...frames.slice(1)];}map.forEach(animation=>animation.cancel());map.clear();
 if(typeof matchMedia!=='function'||typeof node.animate!=='function'||matchMedia('(prefers-reduced-motion: reduce)').matches)return ()=>{};
 const animation=node.animate(frames,{duration,easing:'cubic-bezier(.2,.75,.2,1)'});animation.id=key;map.set(key,animation);
 const media=matchMedia('(prefers-reduced-motion: reduce)');const changed=()=>{if(media.matches)cleanup();};const cleanup=()=>{media.removeEventListener('change',changed);if(map.get(key)===animation)map.delete(key);animation.cancel();};media.addEventListener('change',changed);void animation.finished.then(cleanup).catch(()=>{media.removeEventListener('change',changed);if(map.get(key)===animation)map.delete(key);});return cleanup;
}
export function cancelOwned(node:HTMLElement){owners.get(node)?.forEach(animation=>animation.cancel());owners.delete(node);}
