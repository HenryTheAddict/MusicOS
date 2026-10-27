import {test,expect} from '@playwright/test';
// @ts-expect-error Playwright supplies Node builtins outside the application bundle.
import {writeFileSync,mkdirSync} from 'node:fs';
declare const process:{env:Record<string,string|undefined>};
test('compare installed and adaptive wide-shell menu and navigation pacing',async({browser})=>{
 test.skip(!process.env.MUSICOS_BASELINE_ORIGIN,'Set MUSICOS_BASELINE_ORIGIN to the installed-source fixture server.');
 test.setTimeout(90000);
 const reports=[];
 for(const [label,origin] of [['installed',process.env.MUSICOS_BASELINE_ORIGIN!],['adaptive','http://127.0.0.1:1420']]){
  const page=await browser.newPage({viewport:{width:1280,height:800}});
  await page.addInitScript(()=>{
   const raf=requestAnimationFrame.bind(window);let measuring=false;const work:number[]=[],intervals:number[]=[];let previous=0;
   window.requestAnimationFrame=callback=>raf(time=>{const start=performance.now();callback(time);if(measuring)work.push(performance.now()-start);});
   (window as any).startMeasure=()=>{measuring=true;previous=0;const tick=(time:number)=>{if(!measuring)return;if(previous)intervals.push(time-previous);previous=time;raf(tick);};raf(tick);};
   (window as any).finishMeasure=()=>{measuring=false;return {work,intervals,activeAnimations:document.getAnimations().filter(a=>a.playState==='running').length};};
  });
  await page.goto(origin+'/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Songs',exact:true}).click();await page.waitForTimeout(400);
  const cdp=await page.context().newCDPSession(page);await cdp.send('Performance.enable');const before=(await cdp.send('Performance.getMetrics')).metrics;
  await page.evaluate(()=>(window as any).startMeasure());
  for(let i=0;i<6;i++){
   await page.getByRole('button',{name:'More library views',exact:true}).click({force:true});await page.waitForTimeout(150);await page.keyboard.press('Escape');await expect(page.getByRole('menu')).toHaveCount(0);
   await page.getByRole('button',{name:i%2?'Songs':'Albums',exact:true}).click();await page.waitForTimeout(200);
  }
  await page.mouse.move(1,1);await page.waitForTimeout(450);const measured=await page.evaluate(()=>(window as any).finishMeasure());const after=(await cdp.send('Performance.getMetrics')).metrics;
  const delta=(name:string)=>(after.find((v:any)=>v.name===name)?.value??0)-(before.find((v:any)=>v.name===name)?.value??0);
  const stats=(values:number[])=>{const sorted=[...values].sort((a,b)=>a-b);return {count:values.length,median:sorted[Math.floor(sorted.length*.5)]??0,p95:sorted[Math.floor(sorted.length*.95)]??0,total:values.reduce((a,b)=>a+b,0)};};
  const frame=stats(measured.intervals),missed=measured.intervals.filter((v:number)=>v>1.5*frame.median).length;
  reports.push({label,source:label==='installed'?'512d2a0':'working desktop branch',scope:'Chromium fixture at 1280×800; not native display FPS',frameMilliseconds:frame,intervalsOver1_5Median:missed,rafCallbackMilliseconds:stats(measured.work),scriptSeconds:delta('ScriptDuration'),layoutSeconds:delta('LayoutDuration'),taskSeconds:delta('TaskDuration'),activeAnimationsAfter:measured.activeAnimations});
  expect(measured.activeAnimations).toBe(0);await page.close();
 }
 mkdirSync('out/persistent-qa',{recursive:true});writeFileSync('out/persistent-qa/adaptive-performance.json',JSON.stringify(reports,null,2));
});
