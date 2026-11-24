export function ownsSpaceKey(target:EventTarget|null,pointerButtonPlayback=false){
 if(!(target instanceof Element))return false;
 if(target.closest('input,textarea,select,a,summary,[contenteditable]:not([contenteditable=false]),[role=menu],[role=region][tabindex],[data-native-space]'))return true;
 return !pointerButtonPlayback&&!!target.closest('button,[role=button]');
}

/** Pointer interaction keeps Space available for playback. Tab/arrow navigation
 * deliberately hands keys and visible focus back to the focused control. */
export function observeInputModality(){
 const root=document.documentElement;
 let modality:'pointer'|'keyboard'='pointer';
 const set=(next:typeof modality)=>{modality=next;root.dataset.inputModality=next;};
 const pointer=()=>set('pointer');
 const keyboard=(event:KeyboardEvent)=>{if(event.key==='Tab'||event.key==='Enter'||event.key.startsWith('Arrow'))set('keyboard');};
 set('pointer');
 document.addEventListener('pointerdown',pointer,true);
 document.addEventListener('keydown',keyboard,true);
 return {pointerFocus:()=>modality==='pointer',dispose:()=>{document.removeEventListener('pointerdown',pointer,true);document.removeEventListener('keydown',keyboard,true);delete root.dataset.inputModality;}};
}
