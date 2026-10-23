import {test,expect} from '@playwright/test';
const phase=(globalThis as any).process?.env.MUSICOS_MOTION_PHASE;
test('compare transport drawing and visible animation work',async({page})=>{
 test.skip(!phase,'Opt-in before/after benchmark, not native display pacing.');
 await page.goto('/?fixture=1');
 const report=await page.evaluate(async()=>{
  const modulePath='/src/playPauseMorph.ts';const {transportDrawing,playPauseMorph}=await import(modulePath);
  const drawing=[];for(let run=0;run<7;run++){const start=performance.now();for(let i=0;i<2000;i++)transportDrawing((i%101)/100);drawing.push(performance.now()-start);}
  const host=document.createElement('i');host.style.cssText='position:fixed;right:20px;bottom:20px;width:48px;height:48px';document.body.append(host);
  const frames:number[]=[];let last=0,live=true;const tick=(now:number)=>{if(last)frames.push(now-last);last=now;if(live)requestAnimationFrame(tick);};requestAnimationFrame(tick);
  for(let i=0;i<6;i++){playPauseMorph(host,i%2===0,'<svg viewBox="0 0 32 32"></svg>');await new Promise(resolve=>setTimeout(resolve,420));}
  live=false;host.remove();return {drawing_ms:drawing,frame_intervals_ms:frames,userAgent:navigator.userAgent};
 });
 expect(report.drawing_ms).toHaveLength(7);const fsModule='node:fs';const {writeFileSync}=await import(fsModule);writeFileSync(`out/rebuild-qa/connected-consistency/morph-${phase}.json`,JSON.stringify({kind:'Headless Chromium, seven 2000-drawing runs and six 360ms morphs; not native refresh-rate measurement',...report},null,2));
});
