import {native,errorText} from './bridge';
import {dragWindow} from './windowDrag';
import {useEffect,useRef,useState,type CSSProperties} from 'react';
import {Icon} from './components';
import {appearance,themePalette,readableInk,secondaryInk} from './appearance';
import {animateOwned} from './presentationMotion';
import {product} from './product';
import type {Preferences,Task,Palette} from './types';
const steps=['Appearance','Music folders'];
export default function Onboarding({preferences,artPalette,task,songs,onImport,onFinish,onDismiss}:{preferences:Preferences;artPalette?:Palette|null;task:Task;songs:number;onImport:()=>Promise<string[]>;onFinish:(preferences:Preferences,devices:boolean)=>Promise<boolean>;onDismiss:()=>void}){
 const dialog=useRef<HTMLDialogElement>(null),body=useRef<HTMLDivElement>(null),heading=useRef<HTMLHeadingElement>(null),direction=useRef(1);
 const [step,setStep]=useState(0),[draft,setDraft]=useState(preferences),[folders,setFolders]=useState<string[]>([]),[pending,setPending]=useState(false),[picking,setPicking]=useState(false),[error,setError]=useState('');
 const look=appearance(draft),[dark,setDark]=useState(()=>matchMedia('(prefers-color-scheme: dark)').matches);
 useEffect(()=>{const media=matchMedia('(prefers-color-scheme: dark)'),update=()=>setDark(media.matches);media.addEventListener('change',update);return()=>media.removeEventListener('change',update);},[]);
 const palette=themePalette(look,artPalette??null,dark)??{background:'#cbf1ff',blob:'#54b9ff',accent:'#0064df',text:'#12202a'};
 const style={'--surface-color':palette.background,'--blob-color':palette.blob,'--shell-color':palette.blob,'--accent':palette.accent,'--accent-ink':readableInk(palette.accent),'--ink':palette.text,'--secondary-ink':secondaryInk(palette.text,palette.background,palette.blob,look.mode==='dark'||look.mode==='system'&&dark),'--control-surface':`color-mix(in srgb,${palette.background} 78%,${palette.blob})`,'--control-border':`color-mix(in srgb,${palette.text} 20%,transparent)`} as CSSProperties;
 useEffect(()=>{const node=dialog.current!,before=document.activeElement as HTMLElement|null;node.showModal();return()=>{if(node.open)node.close();if(before?.isConnected)before.focus({preventScroll:true});};},[]);
 useEffect(()=>{if(body.current)body.current.scrollTop=0;heading.current?.focus({preventScroll:true});if(body.current)return animateOwned(body.current,'onboarding-step',[{opacity:0,transform:`translateX(${direction.current*10}px)`},{opacity:1,transform:'none'}],180);},[step]);
 function navigate(next:number){direction.current=next>step?1:-1;setError('');setStep(next);}
 async function finish(devices=false,defaults=false){if(pending||picking)return;setPending(true);setError('');try{if(!await onFinish({...(defaults?preferences:draft),display_name:(defaults?preferences:draft).display_name.trim(),onboarding_version:product.onboarding.version},devices))setError('Your setup could not be saved. Try again.');}catch{setError('Your setup could not be saved. Try again.');}finally{setPending(false);}}
 async function add(){setPicking(true);setError('');try{const selected=await onImport();if(selected.length)setFolders(old=>[...new Set([...old,...selected])]);}catch(e){setError(errorText(e));}finally{setPicking(false);}}
 return <dialog ref={dialog} className="onboarding" data-step={step} data-native-titlebar={native&&/Mac/.test(navigator.platform)} style={style} aria-labelledby="onboarding-title" onCancel={event=>{event.preventDefault();if(!pending&&!picking)onDismiss();}}>
  {native&&/Mac/.test(navigator.platform)&&<div className="onboarding-drag-region" onMouseDown={event=>dragWindow(event,()=>{})}/>}
  <header className="onboarding-top"><span><Icon name="Album"/>{product.name}<small>Setup</small></span><button aria-label="Close setup" disabled={pending||picking} onClick={onDismiss}><Icon name="controlClose"/></button></header>
  <div className="onboarding-layout">
   <aside className="onboarding-aside">
    <div className="onboarding-preview" aria-hidden="true" inert>
     <div className="setup-preview-top"><Icon name="Library"/><span>{draft.display_name.trim()||product.defaults.libraryLabel}</span><Icon name="Plus"/></div>
     <div className="setup-preview-nav"><span>Albums</span><span>Artists</span><span>Songs</span></div>
     <div className="setup-preview-art"><div><Icon name="Album"/></div><div><Icon name="Song"/></div><div><Icon name="Sparklist"/></div><div><Icon name="hearted"/></div></div>
     <div className="setup-preview-player"><Icon name="Song"/><span/><Icon name="play"/></div>
    </div>
    <div className="onboarding-preview-caption"><Icon name="Light Mode"/><span>{look.mode==='system'?'Follows your system':look.mode==='dark'?'Dark appearance':'Light appearance'}<small>{look.source==='artwork'?'Colors from your artwork':`${product.name} blue`}</small></span></div>
   </aside>
   <div className="onboarding-main">
    <ol className="onboarding-steps" aria-label="Setup progress">{steps.map((name,index)=><li key={name} aria-current={step===index?'step':undefined}><span>{index+1}</span>{name}</li>)}</ol>
    <div className="onboarding-copy" ref={body}>
     <h1 ref={heading} id="onboarding-title" tabIndex={-1}>{steps[step]}</h1>
     {step===0?<>
      <p className="onboarding-lead">Choose how {product.name} looks. You can change these in Settings.</p>
      <label className="onboarding-name">Your name <small>Optional</small><input value={draft.display_name} maxLength={48} autoComplete="nickname" placeholder={product.defaults.libraryLabel} onChange={event=>setDraft({...draft,display_name:event.target.value.replace(/[\r\n\t]/g,'')})}/></label>
      <fieldset><legend>Appearance</legend><div className="onboarding-choices appearance-choices">{(['system','light','dark'] as const).map(mode=><button key={mode} data-mode={mode} aria-pressed={look.mode===mode} onClick={()=>setDraft({...draft,appearance:{...look,mode}})}><span className="appearance-sample" aria-hidden="true"><i/><i/><i/></span><span>{mode==='system'?'System':mode==='light'?'Light':'Dark'}</span></button>)}</div></fieldset>
      <fieldset><legend>Colors</legend><div className="onboarding-choices color-choices">{(['blue','artwork'] as const).map(source=><button key={source} aria-pressed={look.source===source} onClick={()=>setDraft({...draft,appearance:{...look,source}})}><span className={`setup-color-swatch ${source}`} aria-hidden="true"/>{source==='blue'?`${product.name} blue`:'From artwork'}</button>)}</div></fieldset>
     </>:<>
      <p className="onboarding-lead">Add a music folder. Your original files stay where they are.</p>
      <button className="onboarding-folder" onClick={()=>void add()} disabled={picking||task.running||pending}><Icon name="Open In Folder"/><span><strong>{picking?'Choosing a folder…':'Choose music folders'}</strong><small>Albums, artists, and songs appear in your library.</small></span><Icon name="Plus"/></button>
      {folders.length>0&&<ul className="onboarding-folders">{folders.map(folder=><li key={folder}><Icon name="Storage"/><span title={folder}>{folder.split(/[\\/]/).filter(Boolean).at(-1)}</span><Icon name="Downloaded"/></li>)}</ul>}
      {task.running?<div className="onboarding-import" role="status"><strong>{task.message||'Adding your music…'}</strong>{task.total>0&&<progress value={task.processed} max={task.total}/>}<small>You can open your library while this finishes.</small></div>:songs>0?<p className="onboarding-import" role="status"><Icon name="Song"/>{songs} {songs===1?'song is':'songs are'} ready.</p>:<p className="onboarding-note">You can skip this and add folders later.</p>}
      <div className="onboarding-phone"><Icon name="device"/><span><strong>Android companion</strong><small>Copy music over Wi-Fi for offline listening.</small></span><button disabled={pending||picking} onClick={()=>void finish(true)}>Set up a phone</button></div>
     </>}
    </div>
    {error&&<p role="alert" className="onboarding-error">{error}</p>}
    <footer className="onboarding-footer">{step>0?<button disabled={pending||picking} onClick={()=>navigate(0)}><Icon name="Back"/>Back</button>:<button disabled={pending||picking} onClick={()=>void finish(false,true)}>{preferences.onboarding_version?'Keep my setup':'Use defaults'}</button>}<button className="onboarding-primary" disabled={pending||picking} onClick={()=>{if(step===1)void finish();else navigate(1);}}>{pending?'Saving…':step===1?'Open my library':'Continue'}{!pending&&<Icon name="Back"/>}</button></footer>
   </div>
  </div>
 </dialog>;
}
