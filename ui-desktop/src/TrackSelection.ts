import {createContext,type MouseEvent} from 'react';
export const TrackSelection=createContext<{ids:Set<number>;select:(id:number,index:number,event:MouseEvent)=>void;remove:(ids?:number[])=>void;canRemove?:boolean;selectAll?:()=>void}|null>(null);
