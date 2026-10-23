import type {HTMLAttributes} from 'react';
import {Overflow,type MenuItem} from './components';
/** One action source for pointer overflow, secondary click, and keyboard context. */
export default function ActionSurface({items,children,...props}:HTMLAttributes<HTMLDivElement>&{items:MenuItem[]}){
 function open(node:HTMLElement,point?:{x:number;y:number}){node.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:point}));}
 return <article {...props} data-action-surface onContextMenu={e=>{e.preventDefault();e.stopPropagation();open(e.currentTarget,{x:e.clientX,y:e.clientY});}} onKeyDown={e=>{if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.stopPropagation();open(e.currentTarget);}}}>{children}<Overflow items={items}/></article>;
}
