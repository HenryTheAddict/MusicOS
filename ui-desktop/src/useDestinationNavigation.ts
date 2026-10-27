import {useCallback,useState} from 'react';
import {initialNavigation,navigationReducer,restoreNavigation,type Navigation,type NavigationAction} from './navigation';
import type {Route} from './types';
export type Destination='home'|'search'|'library';
const empty=(view:Route['view']):Navigation=>({current:{view,search:'',scroll:0},history:[]});
export function useDestinationNavigation(){
 const [state,setState]=useState(()=>{
  const restored=restoreNavigation(sessionStorage.getItem('musicos-navigation-v1'));
  const active:Destination=restored.current.view==='search'||restored.history[0]?.view==='search'?'search':restored.current.view==='home'||restored.history[0]?.view==='home'?'home':'library';
  const stacks={home:initialNavigation,search:empty('search'),library:empty('albums')};stacks[active]=restored;
  return {active,stacks};
 });
 const dispatch=useCallback((action:NavigationAction)=>setState(old=>{
  if(action.type==='reconcile')return {...old,stacks:{home:navigationReducer(old.stacks.home,action),search:navigationReducer(old.stacks.search,action),library:navigationReducer(old.stacks.library,action)}};
  return {...old,stacks:{...old.stacks,[old.active]:navigationReducer(old.stacks[old.active],action)}};
 }),[]);
 const switchDestination=useCallback((active:Destination)=>setState(old=>old.active===active?old:{...old,active}),[]);
 const scope=useCallback((view:Route['view'])=>setState(old=>{const active:Destination=view==='home'?'home':view==='search'?'search':'library';return {active,stacks:{...old.stacks,[active]:navigationReducer(old.stacks[active],{type:'scope',view})}};}),[]);
 return {navigation:state.stacks[state.active],dispatch,destination:state.active,switchDestination,scope};
}
