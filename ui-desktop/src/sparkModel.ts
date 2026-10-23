import {useEffect,useState} from 'react';
import {call,errorText} from './bridge';
import {displayData,invalidateDisplay,peekDisplayData} from './displayData';
import {useViewActive} from './ViewScope';
import type {SparkState} from './types';
let installation:Promise<SparkState>|undefined;
/** One installation request even when the creator and Settings meet mid-navigation. */
export function installSparkModel(){
 if(!installation)installation=call('sparklist',{request:{action:'install_staged'}}).then(state=>{invalidateDisplay('sparklist');return state;}).finally(()=>{installation=undefined;});
 return installation;
}
/** Automatic setup only while an enabled audio feature is visible. Jobs survive navigation. */
export function useAutomaticSparkModel(enabled:boolean,visible:boolean){
 const active=useViewActive();
 const [state,setState]=useState<SparkState|undefined>(()=>peekDisplayData('sparklist',{request:{action:'status'}}));
 const [error,setError]=useState('');
 useEffect(()=>{
  if(!enabled||!visible||!active)return;
  let alive=true,pending=false,timer:ReturnType<typeof setTimeout>|undefined,attempted=false,wasWorking=false,wasReady=!!state?.ready;
  async function refresh(){
   if(pending||!alive)return;pending=true;clearTimeout(timer);
   try{
    let next=await displayData('sparklist',{request:{action:'status'}},true);
    if(alive&&!attempted&&next.beta!==false&&!next.ready&&!next.installed&&!next.installation?.running&&!next.installation?.error&&next.staged_available){attempted=true;next=await installSparkModel();}
    if(!alive)return;setState(next);setError('');
    if((wasWorking&&!next.installation?.running)||(next.ready&&!wasReady)){invalidateDisplay('playlist_suggestions');invalidateDisplay('spark_recommendations');}
    wasWorking=!!next.installation?.running;wasReady=next.ready;
    if(next.installation?.running)timer=setTimeout(()=>void refresh(),1500);
   }catch(e){if(alive)setError(errorText(e));}finally{pending=false;}
  }
  const changed=(event:Event)=>{if((event as CustomEvent).detail==='sparklist')void refresh();};
  void refresh();window.addEventListener('musicos-display-invalidated',changed);
  return()=>{alive=false;clearTimeout(timer);window.removeEventListener('musicos-display-invalidated',changed);};
 },[enabled,visible,active]);
 return {state:enabled?state:undefined,error};
}
