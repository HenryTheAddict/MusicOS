import catalog from '../../assets/visualizers/manifest.json';
export const visualizerCatalog=catalog.presets;
export type VisualizerPreferences={preset:string;rotation:'manual'|'each_song'|'timed'|'beats';timed_seconds:15|30|60;beat_interval:16|32|64;beat_sync:boolean;album_colors?:boolean;lyrics:boolean};
export const defaultVisualizer:VisualizerPreferences={preset:'boxes',rotation:'manual',timed_seconds:30,beat_interval:32,beat_sync:true,album_colors:false,lyrics:false};
export type VisualizerStatus={requestId:number;phase:string;preset:string;error?:string|null;frames:number;width:number;height:number;missedFrames:number;submissionMs:number;beatFallback:boolean};
export type VisualizerSettings={requestId:number;preferences:VisualizerPreferences;albumPalette?:AlbumPalette;beat:{trackId?:number;bpm?:number;offset?:number;confidence:number};reduced:boolean};
export function isEditing(target:EventTarget|null){return target instanceof Element&&!!target.closest('input:not([type=range]):not([type=checkbox]):not([type=radio]):not([type=button]):not([type=submit]):not([type=reset]):not([type=image]):not([type=color]):not([type=file]),textarea,select,[contenteditable]:not([contenteditable=false]),[role=textbox],[role=combobox]');}
export function playerShortcut(event:Pick<KeyboardEvent,'metaKey'|'ctrlKey'|'altKey'|'shiftKey'|'key'|'repeat'|'target'>):'previous'|'next'|'visualizer'|'panel'|null{
 if(!(event.metaKey||event.ctrlKey)||event.altKey||event.shiftKey||event.repeat||isEditing(event.target))return null;
 return ({p:'previous',n:'next',v:'visualizer',b:'panel'} as const)[event.key.toLowerCase() as 'p']??null;
}

export type AlbumPalette=[number[],number[],number[]];
