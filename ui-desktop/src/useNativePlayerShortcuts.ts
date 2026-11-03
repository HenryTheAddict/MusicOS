import {useEffect,useRef} from 'react';
import {listen} from '@tauri-apps/api/event';
import {native} from './bridge';
export type PlayerShortcut='previous'|'next'|'visualizer'|'panel';
export function useNativePlayerShortcuts(action:(value:PlayerShortcut)=>void){
 const current=useRef(action);current.current=action;
 useEffect(()=>{if(!native)return;let alive=true,off:(()=>void)|undefined;void listen<PlayerShortcut>('musicos-player-shortcut',({payload})=>{if(alive&&['previous','next','visualizer','panel'].includes(payload)){window.dispatchEvent(new Event('musicos-player-interaction'));current.current(payload);}}).then(stop=>{if(alive)off=stop;else stop();}).catch(()=>{});return()=>{alive=false;off?.();};},[]);
}
