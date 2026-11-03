// Only the isolated visualizer Vite configuration resolves this module.
import {call as actualCall} from '../../src/bridge';
import {fixtureCall} from '../../src/fixtures';
export * from '../../src/bridge';
export const call:typeof actualCall=(async(command:string,args:any)=>{
 if(command==='visualizer')return actualCall(command,args);
 if(command==='apple_enrichment')return args.request.action==='motion_policy'?{lowPower:false}:{status:{signed_in:false}};
 if(command==='trackpad_feedback')return;
 return fixtureCall(command,args);
}) as typeof actualCall;
