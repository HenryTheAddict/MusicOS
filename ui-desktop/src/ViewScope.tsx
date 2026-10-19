import {createContext,useContext,useEffect,useRef,useSyncExternalStore,type ReactNode} from 'react';
const Visible=createContext(true);
const subscribeDocument=(change:()=>void)=>{document.addEventListener('visibilitychange',change);return()=>document.removeEventListener('visibilitychange',change);};
export function useViewActive(){const active=useContext(Visible),visible=useSyncExternalStore(subscribeDocument,()=>document.visibilityState!=='hidden');return active&&visible;}
/** Retain visited views and their drafts; hidden views receive no UI work. */
export function KeepView({active,children,className}:{active:boolean;children:ReactNode;className?:string}){
 const parent=useContext(Visible),drawing=useViewActive(),visited=useRef(active),retained=useRef(children);if(active)visited.current=true;if(parent&&active)retained.current=children;
 return visited.current?<Visible.Provider value={drawing&&active}><div className={className} hidden={!active} inert={!active} data-view-active={drawing&&active}>{retained.current}</div></Visible.Provider>:null;
}
export function useVisiblePolling(work:()=>void,interval:number,dependencies:unknown[]=[]){
 const active=useViewActive(),latest=useRef(work);latest.current=work;
 useEffect(()=>{if(!active)return;latest.current();const timer=setInterval(()=>latest.current(),interval);return()=>clearInterval(timer);},[active,interval,...dependencies]);
}
