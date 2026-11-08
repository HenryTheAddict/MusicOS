/** Source time remains authoritative. Absorb small delivery jitter without
 * pulling word fills backwards on every native telemetry packet. */
export type SourceClock={receivedAt:number;playing:boolean;speed:number;trackId:number|undefined;position:number;correction?:number};
export function sourcePosition(value:SourceClock,at:number){
 const elapsed=Math.max(0,(at-value.receivedAt)/1000);
 return value.position+(value.playing?Math.min(.5,elapsed)*value.speed+(value.correction??0)*Math.exp(-Math.min(.5,elapsed)/.18):0);
}
export function sourceCorrection(previous:SourceClock,next:SourceClock,at:number){
 if(!previous.playing||!next.playing||previous.trackId!==next.trackId||previous.speed!==next.speed)return 0;
 const delta=sourcePosition(previous,at)-next.position;
 // A seek, repeat, output discontinuity or stale packet reanchors immediately.
 return Math.abs(delta)<=Math.min(.12,.15*next.speed)?delta:0;
}
