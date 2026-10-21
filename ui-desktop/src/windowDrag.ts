import type {MouseEvent} from 'react';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {native} from './bridge';

// AppKit expects a mouse-down event. Starting from pointer-down can race
// WebKit's mouse dispatch; only backgrounds and text initiate window moves.
export function dragWindow(event:MouseEvent<HTMLElement>,failed:(message:string)=>void){
 if(!native||event.button!==0||event.defaultPrevented)return;
 const target=event.target as Element;
 if(target.closest('button,a,input,textarea,select,[role=menu],[role=slider],[role=separator],[tabindex="0"],.playback-progress,.queue-scroll')||document.querySelector('.mini-actions-popover:popover-open'))return;
 event.preventDefault();
 void getCurrentWindow().startDragging().catch(()=>failed('Could not move the window. Try dragging the song title again.'));
}
