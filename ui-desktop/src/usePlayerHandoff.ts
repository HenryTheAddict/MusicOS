import{useLayoutEffect,useRef,type RefObject}from'react';
import{animateOwned,cancelOwned}from'./presentationMotion';
/** Playback acknowledgements own song changes; position/theme updates never restart motion. */
export function usePlayerHandoff(root:RefObject<HTMLElement|null>,trackId:number|undefined,view:string){
 const last=useRef({trackId,view}),direction=useRef(1);
 useLayoutEffect(()=>{const host=root.current;if(!host||!host.getClientRects().length||document.hidden)return;const old=last.current;last.current={trackId,view};if(old.trackId!==trackId&&old.trackId!==undefined){for(const node of host.querySelectorAll<HTMLElement>('.expanded-player-art,.expanded-player-copy>strong,.expanded-player-copy>span:not(.lyrics-small-heart),.mini-player-art .cover,.mini-player-thumb .cover,.mini-player-copy .mini-track-content,.now-playing-cover .cover,.now-playing-title-action>span'))animateOwned(node,'song-handoff',[{opacity:.55,transform:`translateX(${direction.current*10}px)`},{opacity:1,transform:'none'}],180);}
 if(old.view!==view){direction.current=view==='playing'||view==='tall'?-1:1;for(const node of host.querySelectorAll<HTMLElement>('.expanded-side-view:not([hidden]),.mini-lyrics-view,.mini-queue-list'))if(node.getClientRects().length)animateOwned(node,'player-view-handoff',[{opacity:.45,transform:`translateX(${direction.current*8}px)`},{opacity:1,transform:'none'}],180);}},[trackId,view]);
 useLayoutEffect(()=>{const node=root.current;return()=>{node?.querySelectorAll<HTMLElement>('.expanded-player-art,.expanded-player-copy>strong,.expanded-player-copy>span:not(.lyrics-small-heart),.mini-player-art .cover,.mini-player-thumb .cover,.mini-player-copy .mini-track-content,.now-playing-cover .cover,.now-playing-title-action>span,.expanded-side-view,.mini-lyrics-view,.mini-queue-list').forEach(cancelOwned);};},[]);
 return(directionValue:number)=>{direction.current=directionValue;};
}
