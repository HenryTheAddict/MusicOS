import type{AppleResponse,AppleStatus}from'./AppleEnrichment';
const status:AppleStatus={enabled:false,signed_in:false,sign_in:'not_checked',library:'not_checked',lyrics:'not_checked',artwork:'not_checked',phase:'disconnected',message:'Demo connection. No Apple account is accessed.',scanned:0,matched:0,ambiguous:0,completed:0,total:0,lyrics_ready:0,lyrics_failed:0};
export async function appleFixture(r:{action:string}):Promise<AppleResponse>{
 if(r.action==='matches')return{matches:Array.from({length:12},(_,i)=>({trackId:i+1,catalogId:String(i+100),title:i===0?'Opening':`Demo song ${i+1}`,artist:'Demo artist',album:'Demo album',cached:status.lyrics_ready>0}))};
 if(r.action==='lyrics')return{document:{version:'1.0',metadata:{title:'Opening',artist:'Demo artist',album:'Demo album',offset_ms:0,instrumental:false},plain:'Words for this moment',lines:[{text:'Words for this moment',start_ms:0,end_ms:5000}]},cached:true};
 if(r.action==='begin'){Object.assign(status,{enabled:true,phase:'signing_in',sign_in:'waiting'});}
 if(r.action==='verify'){Object.assign(status,{signed_in:true,phase:'connected',sign_in:'available',message:'Demo signed in. Other capabilities have not been checked.'});}
 if(r.action==='sync'){Object.assign(status,{phase:'matching',scanned:0});await new Promise(r=>setTimeout(r,300));Object.assign(status,{phase:'ready',library:'available',scanned:140,matched:12,message:'12 demo local recordings matched. Local files are unchanged.'});}
 if(r.action==='sync_lyrics'){Object.assign(status,{phase:'lyrics_sync',total:12,completed:0});await new Promise(r=>setTimeout(r,300));Object.assign(status,{phase:'ready',lyrics:'available',total:12,completed:12,lyrics_ready:12,message:'12 demo lyric previews ready. Saved selections were not replaced.'});}
 if(r.action==='disconnect'){Object.assign(status,{enabled:false,signed_in:false,phase:'disconnected',sign_in:'not_checked',library:'not_checked',lyrics:'not_checked',artwork:'not_checked',message:'Disconnected. Selected lyrics are preserved.'});}
 return {status:{...status},choices:[]};
}
