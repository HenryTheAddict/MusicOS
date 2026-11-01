import {AssetIcon} from './AssetIcon';
import {haptic} from './haptics';
import './playerViewCycle.css';
export type CyclingView='playing'|'queue'|'lyrics';
export const nextPlayerView=(view:CyclingView):CyclingView=>view==='playing'?'queue':view==='queue'?'lyrics':'playing';
const labels={playing:'Now playing',queue:'Up next',lyrics:'Lyrics'};
export default function PlayerViewCycle({value,onChange,disabled=false}:{value:CyclingView;onChange:(value:CyclingView)=>void;disabled?:boolean}){const next=nextPlayerView(value);return <button type="button" className="icon-button player-view-cycle" data-player-cycle={value} data-active={value!=='playing'} disabled={disabled} aria-label={next==='queue'?'Open Up next':`Show ${labels[next]}`} title={`${labels[value]} · ${labels[next]} next`} onClick={()=>{haptic('toggle',`player-view:${next}`);onChange(next);}}><AssetIcon name={value==='playing'?'playerQueue':value==='queue'?'playerLyrics':'playerPlaying'}/></button>;}
