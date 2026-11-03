import * as Dropdown from '@radix-ui/react-dropdown-menu';
import {useLayoutEffect,useRef,useState} from 'react';
import {useMenuPresentation,menuOwnsFocus} from './menuPresentation';
import {visualizerCatalog} from './visualizer';
import {Icon} from './components';
const pictures=import.meta.glob<string>('../../assets/visualizers/thumbnails/*.png',{query:'?url',import:'default',eager:true});
export default function VisualizerPicker({value,onChange}:{value:string;onChange:(id:string)=>void}){
 const trigger=useRef<HTMLButtonElement>(null),[container,setContainer]=useState<HTMLElement|undefined>();useLayoutEffect(()=>setContainer(trigger.current?.closest('dialog')??undefined),[]);const panel=useMenuPresentation(trigger);
 return <Dropdown.Root modal={false} open={panel.open} onOpenChange={panel.setOpen}><Dropdown.Trigger ref={trigger} onClick={e=>{if(e.detail===0)panel.setOpen(!panel.open);}} className="visualizer-picker-trigger" aria-label="Choose visualizer"><Icon name="Visualizer"/><span>{visualizerCatalog.find(p=>p.id===value)?.name??'boxes'}</span></Dropdown.Trigger>{panel.present&&<Dropdown.Portal container={container} forceMount><Dropdown.Content forceMount ref={panel.ref} inert={!panel.open||undefined} aria-hidden={!panel.open||undefined} className="menu-panel visualizer-picker" sideOffset={6} collisionPadding={8} onEscapeKeyDown={panel.onEscapeKeyDown} onCloseAutoFocus={e=>{e.preventDefault();if(menuOwnsFocus(trigger.current,panel.session))trigger.current?.focus({preventScroll:true});}}><div className="menu-panel-surface" aria-hidden="true"/><div className="menu-panel-items"><Dropdown.RadioGroup value={value} onValueChange={onChange}>{visualizerCatalog.map(p=><Dropdown.RadioItem key={p.id} value={p.id} className="visualizer-preset"><img src={pictures[`../../assets/visualizers/thumbnails/${p.id}.png`]} alt=""/><span>{p.name}</span></Dropdown.RadioItem>)}</Dropdown.RadioGroup></div></Dropdown.Content></Dropdown.Portal>}</Dropdown.Root>;
}
