// Share one observer across the visible/virtualized song rows.
const hearts=new Map<HTMLElement,boolean>();
let observer:IntersectionObserver|undefined;
function updateVisibility(){for(const [heart,visible] of hearts)heart.dataset.pulsing=String(visible&&!document.hidden);}
export function watchHeart(heart:HTMLElement){
 if(!observer){
  observer=new IntersectionObserver(entries=>{for(const entry of entries)if(hearts.has(entry.target as HTMLElement))hearts.set(entry.target as HTMLElement,entry.isIntersecting);updateVisibility();});
  document.addEventListener('visibilitychange',updateVisibility);
 }
 hearts.set(heart,false);observer.observe(heart);
 return()=>{
  observer?.unobserve(heart);hearts.delete(heart);delete heart.dataset.pulsing;
  if(!hearts.size){observer?.disconnect();observer=undefined;document.removeEventListener('visibilitychange',updateVisibility);}
 };
}
