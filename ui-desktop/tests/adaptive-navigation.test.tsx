import {act,renderHook} from '@testing-library/react';
import {beforeEach,expect,it} from 'vitest';
import {useDestinationNavigation} from '../src/useDestinationNavigation';
import {shellLayout} from '../src/useShellLayout';
beforeEach(()=>sessionStorage.clear());
it('keeps independent detail stacks, route filters and scroll; reselecting does nothing',()=>{
 const {result}=renderHook(useDestinationNavigation);
 act(()=>result.current.dispatch({type:'patch',patch:{scroll:110}}));
 act(()=>result.current.dispatch({type:'open',route:{view:'album',id:'home-album',search:'',scroll:23}}));
 act(()=>result.current.switchDestination('library'));
 act(()=>result.current.dispatch({type:'patch',patch:{search:'Moonspindle',scroll:640}}));
 act(()=>result.current.dispatch({type:'open',route:{view:'artist',id:'library-artist',search:'',scroll:0}}));
 const library=result.current.navigation;
 act(()=>result.current.switchDestination('library'));
 expect(result.current.navigation).toBe(library);
 act(()=>result.current.switchDestination('search'));
 act(()=>result.current.dispatch({type:'open',route:{view:'album',id:'search-album',search:'',scroll:90}}));
 act(()=>result.current.switchDestination('home'));
 expect(result.current.navigation.current.id).toBe('home-album');
 act(()=>result.current.dispatch({type:'back'}));
 expect(result.current.navigation.current.scroll).toBe(110);
 act(()=>result.current.switchDestination('library'));
 expect(result.current.navigation).toBe(library);
 act(()=>result.current.dispatch({type:'back'}));
 expect(result.current.navigation.current).toMatchObject({view:'albums',search:'Moonspindle',scroll:640});
 act(()=>result.current.switchDestination('search'));
 expect(result.current.navigation.current.id).toBe('search-album');
 act(()=>result.current.dispatch({type:'back'}));
 expect(result.current.navigation.current.view).toBe('search');
});
it('reconciles removed metadata groups in inactive stacks and restores Search detail ownership',()=>{
 sessionStorage.setItem('musicos-navigation-v1',JSON.stringify({current:{view:'album',id:'old',scroll:0,search:''},history:[{view:'search',scroll:120,search:''}]}));
 const {result}=renderHook(useDestinationNavigation);
 expect(result.current.destination).toBe('search');
 act(()=>result.current.switchDestination('library'));
 act(()=>result.current.dispatch({type:'reconcile',albums:[],artists:[],genres:[]}));
 act(()=>result.current.switchDestination('search'));
 expect(result.current.navigation.current).toMatchObject({view:'search',scroll:120});
});
it('keeps square desktop windows wide and uses a bounded scale-aware phone threshold',()=>{
 for(const percent of [80,100,125,150]){const font=16*percent/100;const boundary=Math.min(720,40*font);expect(shellLayout(boundary-.01,font)).toBe('compact');expect(shellLayout(boundary,font)).toBe('wide');expect(shellLayout(720,font)).toBe('wide');}
});
