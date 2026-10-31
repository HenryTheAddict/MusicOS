import {useEffect,useState} from 'react';
import {Icon} from './components';
import {usePlaybackClock,clockPosition} from './playbackFeed';
export default function LyricActivity({loading=false,active=true,playing=false,start=0,end=0}:{loading?:boolean;active?:boolean;playing?:boolean;start?:number;end?:number}){
 const clock=usePlaybackClock(active),[visible,setVisible]=useState(!document.hidden);
 useEffect(()=>{const change=()=>setVisible(!document.hidden);document.addEventListener('visibilitychange',change);return()=>document.removeEventListener('visibilitychange',change);},[]);
 const progress=end>start?Math.max(0,Math.min(1,(clockPosition(clock)*1000-start)/(end-start))):0;
 return <span className="lyric-activity" role={loading?'status':undefined} aria-label={loading?'Finding lyrics':'Instrumental break'} data-loading={loading||undefined} data-moving={active&&visible&&(loading||playing)||undefined}>{!loading&&<Icon name="Song"/>}<span className="lyric-activity-dots" aria-hidden="true">{[0,1,2].map(i=><i key={i} data-filled={!loading&&progress*3>i||undefined}/>)}</span></span>;
}
