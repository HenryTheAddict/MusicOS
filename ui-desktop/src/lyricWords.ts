import type {LyricLine} from './lyrics';

/** Keep the source line's punctuation and spaces, never reconstruct it by
 * concatenating a provider's word labels. Incompatible labels use line timing. */
export function lyricWordSegments(line:LyricLine):{text:string;word?:number}[]|null {
 if(!line.words?.length)return null;
 const segments:{text:string;word?:number}[]=[];let cursor=0;
 for(let i=0;i<line.words.length;i++){
  const token=line.words[i].text.trim();if(!token)continue;
  const at=line.text.indexOf(token,cursor);if(at<0)return null;
  if(at>cursor)segments.push({text:line.text.slice(cursor,at)});
  segments.push({text:line.text.slice(at,at+token.length),word:i});cursor=at+token.length;
 }
 if(cursor<line.text.length)segments.push({text:line.text.slice(cursor)});
 return segments.some(segment=>segment.word!==undefined)?segments:null;
}
export function lyricWordEnd(line:LyricLine,index:number):number|undefined {
 const word=line.words?.[index];if(!word)return;
 const valid=(end:number|undefined)=>end!==undefined&&Number.isFinite(end)&&end>word.start_ms;
 if(valid(word.end_ms))return word.end_ms;
 // Simultaneous syllables/voices can share a start. A zero-length neighbour
 // must not turn genuine word timing into an instantaneous full highlight.
 const end=line.words?.slice(index+1).find(next=>next.start_ms>word.start_ms)?.start_ms??line.end_ms;
 return valid(end)?end:undefined;
}
export function lyricWordActive(line:LyricLine,index:number,time:number){
 const word=line.words?.[index],end=lyricWordEnd(line,index);
 return !!word&&time>=word.start_ms&&(end===undefined||time<end);
}
/** End boundaries matter even when the next word has not started. */
export function lyricWordState(line:LyricLine,time:number){
 return (line.words??[]).map((word,i)=>time<word.start_ms?'0':lyricWordActive(line,i,time)?'1':'2').join('');
}
export function wordProgress(start:number,end:number|undefined,time:number){
 return time<start?0:end===undefined?1:Math.max(0,Math.min(1,(time-start)/Math.max(1,end-start)));
}

/** A short light envelope around real word timing, independent of font geometry. */
export function wordGlow(start:number,end:number|undefined,time:number){
 if(time<start)return 0;
 const duration=Math.max(1,(end??start+350)-start),attack=Math.min(100,duration*.25),release=Math.min(140,duration*.3);
 const smooth=(v:number)=>{v=Math.max(0,Math.min(1,v));return v*v*(3-2*v);};
 return .48*smooth((time-start)/attack)*(1-smooth((time-(start+duration-release))/(release+90)));
}
