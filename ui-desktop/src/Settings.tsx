import {product} from './product';
import MiniPresetPicker from './MiniPresetPicker';
import {personalName} from './personalName';
import {KeepView} from './ViewScope';
import {useContentFade} from './motion';
import ThemedChoice from './ThemedChoice';
import AppearanceControls from './AppearanceControls';
import {appearance} from './appearance';
import {defaultListening} from "./QuickAudio";
import DepthSetup from "./DepthSetup";
import ThemedSwitch from "./ThemedSwitch";
import VolumeControl from "./VolumeControl";
import SparkSetup from "./SparkSetup";
import BetaIntegrations from "./BetaIntegrations";
import { Suspense, useEffect, useRef, useState, type ReactNode, type KeyboardEvent, type CSSProperties } from "react";
import { Cover, Icon } from "./components";
import type { Preferences, Palette, Playback } from "./types";
const tabs=["Appearance","Audio","Library","Integrations","General"] as const;
type Tab=typeof tabs[number];
const categoryInfo:Record<Tab,{icon:string;description:string}>={
 Appearance:{icon:'Album',description:'Colors, artwork motion, and a comfortable fit for your screen.'},
 Audio:{icon:'Lossless',description:'Playback, analysis, and Sparklist setup.'},
 Library:{icon:'Storage',description:'Manage your music folders, metadata, and artwork.'},
 Integrations:{icon:'sync',description:'Share your music or save a listening history. You choose what connects.'},
 General:{icon:'Artist',description:'Your name and information about musicOS.'}
};
export default function Settings({onSetup,preferences,onChange,error,cover,palette,playback,onVolume,libraryView,onLibraryView,onManageCollections,libraryContent,audioContent,miniTheme}:{onSetup?:()=>void;miniTheme?:CSSProperties;preferences:Preferences;onChange:(p:Preferences)=>void;error:string;cover?:string|null;palette?:Palette|null;playback:Playback;onVolume:(volume:number)=>void|Promise<void>;libraryView:"sources"|"review"|"playlists"|"sparklists";onLibraryView:(view:"sources"|"review"|"playlists"|"sparklists")=>void;onManageCollections?:()=>void;libraryContent:ReactNode;audioContent:ReactNode}) {
  const [tab,setTab]=useState<Tab>(()=>{const saved=sessionStorage.getItem("musicos-settings-tab");return tabs.find(t=>t===saved)??"Appearance";});
  const [audioPage,setAudioPage]=useState<'playback'|'quality'|'spark'>(()=>{const saved=sessionStorage.getItem('musicos-audio-page');return saved==='quality'||saved==='spark'?saved:'playback';});
  function audioSelect(value:'playback'|'quality'|'spark'){setAudioPage(value);sessionStorage.setItem('musicos-audio-page',value);}
  useEffect(()=>{const open=(event:Event)=>{const value=(event as CustomEvent).detail;if(['playback','quality','spark'].includes(value))audioSelect(value);};window.addEventListener('musicos-audio-page',open);return()=>window.removeEventListener('musicos-audio-page',open);},[]);
  const [name,setName]=useState(preferences.display_name);
  const [nameError,setNameError]=useState("");
  const dj=preferences.dj_config??{enabled:false,mode:"smart_transition" as const,character:"subtle" as const,preserve_energy:true};
  const list=useRef<HTMLDivElement>(null);
  const panel=useRef<HTMLDivElement>(null);
  const contentMounted=useRef(false);
  useContentFade(panel,`${tab}:${audioPage}`,contentMounted.current);
  useEffect(()=>{contentMounted.current=true;},[]);
  useEffect(()=>setName(preferences.display_name),[preferences.display_name]);
  function select(next:Tab){window.dispatchEvent(new CustomEvent("musicos-settings-selected",{detail:next}));setTab(next);sessionStorage.setItem("musicos-settings-tab",next);}
  function keyboard(e:KeyboardEvent<HTMLButtonElement>){
    let index=tabs.indexOf(tab);
    if(e.key==="ArrowRight"||e.key==="ArrowDown")index=(index+1)%tabs.length;
    else if(e.key==="ArrowLeft"||e.key==="ArrowUp")index=(index+tabs.length-1)%tabs.length;
    else if(e.key==="Home")index=0;else if(e.key==="End")index=tabs.length-1;else return;
    e.preventDefault();select(tabs[index]);list.current?.querySelectorAll<HTMLButtonElement>("button")[index].focus();
  }
  useEffect(()=>{const open=(event:Event)=>{const next=(event as CustomEvent).detail;if(tabs.includes(next))select(next);};window.addEventListener('musicos-settings-tab',open);return()=>window.removeEventListener('musicos-settings-tab',open);},[]);
  function editName(value:string){setName(value);setNameError("");onChange({...preferences,display_name:value.trim()});}
  return <section className="settings" data-library-view={tab==='Library'?libraryView:undefined}>
    <header className="page-heading"><span className="settings-wordmark">{product.name}</span><h1>Settings</h1><button className="lyric-settings-back" onClick={()=>{select('Library');onLibraryView('sources');}}><Icon name="Back"/>Library settings</button></header>
    <div className="settings-layout"><div className="settings-tabs" role="tablist" aria-orientation="vertical" aria-label="Settings categories" ref={list}>
      {tabs.map(t=><button key={t} role="tab" id={`settings-tab-${t}`} aria-controls={`settings-panel-${t}`} aria-selected={tab===t} tabIndex={tab===t?0:-1} onClick={()=>select(t)} onKeyDown={keyboard}><Icon name={categoryInfo[t].icon}/><span>{t}</span><span className="settings-chevron" aria-hidden="true">›</span></button>)}
    </div>
    <div ref={panel} role="tabpanel" className="settings-panel" id={`settings-panel-${tab}`} aria-labelledby={`settings-tab-${tab}`} tabIndex={0}>
      <header className="settings-category-heading"><h2>{tab}</h2><p>{categoryInfo[tab].description}</p></header>
      <KeepView active={tab==="Appearance"}><div className="bento appearance-bento"><AppearanceControls preferences={preferences} onChange={onChange}/>
        <article className="settings-tile size-tile"><div className="settings-summary"><h2>Interface size</h2><p>Text, controls, and spacing scale together.</p></div><div className="size-options" role="group" aria-label="Interface size">{([80,100,125,150] as const).map(size=><button key={size} aria-pressed={preferences.interface_percent===size} onClick={()=>onChange({...preferences,interface_percent:size})}>{size}%</button>)}</div></article>
        <MiniPresetPicker theme={miniTheme} playback={playback} cover={cover} value={preferences.mini_player_density??'standard'} onChange={mini_player_density=>onChange({...preferences,mini_player_density})}/><article className="settings-tile motion-tile"><div className="settings-summary"><h2>Artwork motion</h2><p>Gentle depth on hover and while scrolling.</p></div><DepthSetup enabled={!!preferences.artwork_depth} onChange={checked=>onChange({...preferences,artwork_depth:checked,artwork_parallax:checked})}/></article><article className="settings-tile timing-tile"><Icon name="Song"/><h2>Now Playing</h2><div className="settings-group-row"><span>Artwork opens</span><ThemedChoice label="Now Playing artwork opens" value={preferences.now_playing_artwork_action??'album'} onChange={value=>onChange({...preferences,now_playing_artwork_action:value as NonNullable<Preferences['now_playing_artwork_action']>})} options={[{value:'album',label:'Album'},{value:'mini_player',label:'Mini Player'},{value:'artwork',label:'Artwork'}]}/></div><ThemedSwitch label="Show song durations" checked={preferences.show_song_durations!==false} onChange={checked=>onChange({...preferences,show_song_durations:checked})}/><ThemedSwitch label="Show Now Playing time" checked={preferences.show_now_playing_time!==false} onChange={checked=>onChange({...preferences,show_now_playing_time:checked})}/></article>
      </div></KeepView>
      <KeepView active={tab==="Audio"}><><div className="library-settings-nav" role="group" aria-label="Audio settings sections">{(['playback','quality','spark'] as const).map(value=><button key={value} aria-pressed={audioPage===value} onClick={()=>audioSelect(value)}>{value==='playback'?'Playback':value==='quality'?'Quality & analysis':'Sparklist'}</button>)}</div><KeepView active={audioPage==='playback'}><div className="bento audio-bento"><article className="settings-tile"><Icon name="Lossless"/><h2>Listening volume</h2><p>musicOS volume, independent of your system volume.</p><VolumeControl value={playback.volume} onChange={onVolume}/>
        <details className="settings-playback-details"><summary>Playback details</summary><dl className="audio-status"><div><dt>Output</dt><dd>System default</dd></div><div><dt>Source</dt><dd>Local files</dd></div></dl><small>Choose your output device in system sound settings.</small>{playback.error&&<p className="inline-error" role="alert">{playback.error}</p>}</details></article></div><article className="settings-tile setting-inline"><div className="settings-summary"><h2>Quick equalizer</h2><p>Bands shown in the audio controls.</p></div><div className="size-options" role="group" aria-label="Equalizer bands">{([3,8] as const).map(bands=><button key={bands} aria-pressed={(preferences.listening?.bands??3)===bands} onClick={()=>onChange({...preferences,listening:{...defaultListening,...preferences.listening,bands}})}>{bands} bands</button>)}</div></article><article className="settings-tile dj-settings"><Icon name="DJmode"/><h2>DJmode</h2><div className="settings-choice-row"><span>Transition</span><div className="size-options" role="group" aria-label="DJmode transitions">{(['smart_transition','full_song'] as const).map(mode=><button key={mode} aria-pressed={dj.mode===mode} onClick={()=>onChange({...preferences,dj_config:{...dj,mode}})}>{mode==='full_song'?'Full song':'Smart transition'}</button>)}</div></div><div className="settings-choice-row"><span>Character</span><div className="size-options" role="group" aria-label="DJmode character">{(['subtle','creative'] as const).map(character=><button key={character} aria-pressed={dj.character===character} onClick={()=>onChange({...preferences,dj_config:{...dj,character}})}>{character==='subtle'?'Subtle':'Creative'}</button>)}</div></div><ThemedSwitch label="Preserve energy" checked={dj.preserve_energy} onChange={checked=>onChange({...preferences,dj_config:{...dj,preserve_energy:checked}})}/></article><article className="settings-tile audio-presentation setting-inline"><div className="settings-summary"><h2>Up next</h2><p>Small windows open the queue inside the player.</p></div><div className="size-options" role="group" aria-label="Audio menu presentation">{(['sidebar','floating'] as const).map(value=><button key={value} aria-pressed={(preferences.audio_presentation??'sidebar')===value} onClick={()=>onChange({...preferences,audio_presentation:value})}>{value==='sidebar'?'Sidebar':'Floating panel'}</button>)}</div></article></KeepView><KeepView active={audioPage==='spark'}><SparkSetup enabled={!!preferences.sparklist_beta} onChange={value=>onChange({...preferences,sparklist_beta:value})}/></KeepView><KeepView active={audioPage==='quality'}><article className="settings-tile"><Icon name="Lossless"/><h2>Lossless & lossy audio</h2><p>FLAC and ALAC are lossless. AAC, MP3, and E-AC-3 are lossy. The waveform opens audio controls and shows the playing song’s specifications on hover.</p></article>{audioContent}</KeepView></></KeepView>
      <KeepView active={tab==="Library"}><><div className="library-settings-nav" role="group" aria-label="Library management"><button aria-pressed={libraryView==='sources'} onClick={()=>onLibraryView('sources')}>Music folders</button><button aria-pressed={libraryView==='review'} onClick={()=>onLibraryView('review')}>Metadata & artwork</button></div><Suspense fallback={<div className="settings-subpage-placeholder" role="status" aria-label="Loading library settings" aria-busy="true"/>}>{libraryContent}</Suspense>{libraryView==='sources'&&<div className="settings-group-row settings-collections-row"><span><strong>Collections</strong><small>Manage playlists, Sparklists, and Smart playlists in Library.</small></span><button onClick={onManageCollections}><Icon name="Playlist"/>Manage collections</button></div>}</></KeepView>
      <KeepView active={tab==="Integrations"}><BetaIntegrations cover={cover}/></KeepView>
      <KeepView active={tab==="General"}><div className="bento general-bento"><article className="settings-tile"><Icon name="Artist"/><h2>Your name</h2><p>Optional. Leave it blank to use “Your library”.</p><label className="field-label" htmlFor="display-name">Name</label><input id="display-name" className="name-input" value={name} maxLength={48} onChange={e=>editName(e.target.value)} onBlur={()=>{if(!name.trim()){setName(preferences.display_name);setNameError("");}}} aria-invalid={!!nameError}/><small className="name-preview">{personalName(name.trim()||preferences.display_name)}</small>{nameError&&<p role="alert" className="inline-error">{nameError}</p>}</article>
        <article className="settings-tile about-musicos"><Icon name="Album"/><h2>{product.name}</h2><p>Your albums. Your artists. Your own library.</p><small>Desktop beta · {product.version}<br/>Typography: Figtree, SIL Open Font License.</small>{onSetup&&<button className="soft-action welcome-setup-action" onClick={onSetup}><Icon name="Home"/>Open welcome setup</button>}</article></div></KeepView>
    </div>
    </div>
    {error&&<p className="inline-error" role="alert">{error}</p>}
  </section>;
}
