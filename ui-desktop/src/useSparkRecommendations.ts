import {useEffect,useMemo,useState} from 'react';
import {displayData} from './displayData';
import type {SavedCollection,SparkRecommendation} from './types';
/** One shared request for both entry points. Position ticks never refresh it. */
export default function useSparkRecommendations(items:SavedCollection[],current:number|undefined,active:boolean,library:unknown,beta:boolean|undefined){
 const [ranked,setRanked]=useState<SparkRecommendation[]>([]);
 const identity=useMemo(()=>items.filter(c=>c.kind==='audio').map(c=>`${c.id}:${c.track_ids.join(',')}`).join('|'),[items]);
 useEffect(()=>{if(!active||!identity){if(!identity)setRanked([]);return;}let live=true;
  const refresh=()=>{void displayData('spark_recommendations',{current:current??null},true).then(value=>{if(live)setRanked(value);}).catch(()=>{if(live)setRanked([]);});};
  refresh();const invalidated=(event:Event)=>{const command=(event as CustomEvent<string|undefined>).detail;if(!command||command==='sparklist'||command==='spark_recommendations')refresh();};
  window.addEventListener('musicos-display-invalidated',invalidated);return()=>{live=false;window.removeEventListener('musicos-display-invalidated',invalidated);};
 },[identity,current,active,library,beta]);
 return useMemo(()=>ranked.flatMap(row=>{const item=items.find(item=>item.id===row.id&&item.kind==='audio');return item?[{...row,item}]:[];}),[items,ranked]);
}
