export {};
// Browser layout tests use Tauri's official mocks. The native fixture skips
// this branch and reaches the actual Metal surface, with no audio output.
if(!(window as any).isTauri){
 (window as any).isTauri=true;
 const {mockIPC,mockWindows,mockConvertFileSrc}=await import('@tauri-apps/api/mocks');
 const {emit}=await import('@tauri-apps/api/event');
 mockWindows('proof');mockConvertFileSrc('macos');
 let full=false,status:any={};
 let failRenderer=new URLSearchParams(location.search).has('failRenderer');
 mockIPC(async(command,args:any)=>{
  if(command==='plugin:window|is_fullscreen')return full;
  if(command==='plugin:window|set_fullscreen'){full=args.value;await emit('tauri://resize',{width:innerWidth,height:innerHeight});return;}
  if(command==='visualizer'){
   if(args.request.action==='close')return status;
   if(args.request.action==='open'&&failRenderer){failRenderer=false;throw Error('Renderer failed');}
   if(args.request.settings){const s=args.request.settings;status={requestId:s.requestId,phase:'frozen',preset:s.preferences.preset,frames:0,width:0,height:0,missedFrames:0,submissionMs:0,beatFallback:s.preferences.rotation==='beats'&&s.beat.offset===undefined};
    if(args.request.action==='refresh')await emit('musicos-visualizer-status',status);
   }
   return status;
  }
  if(!command.startsWith('plugin:')){
   const {fixtureCall}=await import('../../src/fixtures');
   return fixtureCall(command,args);
  }
 },{shouldMockEvents:true});
}
await import('./visualizer-player');
