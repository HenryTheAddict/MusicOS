import {test,expect} from '@playwright/test';
// Playwright executes this test in Node; application TypeScript excludes Node globals.
// @ts-expect-error Node builtins are supplied by the Playwright runner.
import {writeFileSync,mkdirSync} from 'node:fs';
declare const process:{env:Record<string,string|undefined>};
test('record comparable browser frame pacing and animation scripting',async({page,browserName})=>{
 test.setTimeout(90000);const origin=process.env.MUSICOS_PERF_ORIGIN??'http://127.0.0.1:1420';
 await page.addInitScript(()=>{const raf=window.requestAnimationFrame.bind(window);(window as any).__frameWork=[];window.requestAnimationFrame=callback=>raf(time=>{const began=performance.now();callback(time);(window as any).__frameWork.push(performance.now()-began);});(window as any).__measureFrames=(ms:number)=>new Promise(resolve=>{const frames:number[]=[];const until=performance.now()+ms;const tick=(time:number)=>{frames.push(time);if(time<until)raf(tick);else resolve(frames.slice(1).map((v,i)=>v-frames[i]));};raf(tick);});});
 await page.goto(origin+'/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();await page.locator('.track-row').first().hover();await page.locator('.row-play').first().click();await page.mouse.move(3,3);await page.getByRole('button',{name:'Choose section',exact:true}).focus();await page.waitForTimeout(500);
 const framePromise=page.evaluate(()=>(window as any).__measureFrames(5000));const rows=page.locator('.track-row');
 for(let i=0;i<12;i++){await rows.nth(i%3).hover();await page.waitForTimeout(95);await page.mouse.move(3,3);await page.waitForTimeout(95);}
 const intervals=await framePromise;const work=await page.evaluate(()=>(window as any).__frameWork as number[]);const stats=(values:number[])=>{const v=[...values].sort((a,b)=>a-b);return {count:v.length,median:v[Math.floor(v.length*.5)]??0,p95:v[Math.floor(v.length*.95)]??0,total:v.reduce((a,b)=>a+b,0)};};
 const frameStats=stats(intervals as number[]),report={origin,browserName,scope:'Automated browser fixture; not native display pacing',frames:frameStats,intervalsOver1_5Median:(intervals as number[]).filter(v=>v>frameStats.median*1.5).length,rafCallbackMilliseconds:stats(work)};mkdirSync('out/rebuild-qa/expressive-performance',{recursive:true});writeFileSync(`out/rebuild-qa/expressive-performance/${process.env.MUSICOS_PERF_LABEL??'updated'}.json`,JSON.stringify(report,null,2));expect(frameStats.count).toBeGreaterThan(10);
});
test('measure old and restored meter morph scripting in the same browser',async({page})=>{
 await page.goto('/?fixture=1');const report=await page.evaluate(async()=>{
  const load=new Function('url','return import(url)') as (url:string)=>Promise<any>;const old=await load('/tests/support/baseline/playLevelsMorph.ts'),updated=await load('/src/playLevelsMorph.ts');
  const originalRaf=window.requestAnimationFrame,originalCancel=window.cancelAnimationFrame,originalStyle=window.getComputedStyle;let next=0,reads=0;const queued=new Map<number,FrameRequestCallback>();window.requestAnimationFrame=cb=>{queued.set(++next,cb);return next;};window.cancelAnimationFrame=id=>{queued.delete(id);};window.getComputedStyle=(...args)=>{reads++;return originalStyle(...args);};
  const host=document.createElement('i');host.style.setProperty('--level-1','.2');host.style.setProperty('--level-2','.5');host.style.setProperty('--level-3','.8');document.body.append(host);
  const measure=(fn:any)=>{reads=0;const samples=[];for(let run=0;run<5;run++){const began=performance.now();for(let repeat=0;repeat<50;repeat++){const time=performance.now();host.dataset.morphProgress='0';const cancel=fn(host,true,'<svg/>');for(let i=1;i<=12;i++){const callbacks=[...queued.values()];queued.clear();callbacks.forEach(cb=>cb(time+i*17));}cancel();}samples.push(performance.now()-began);}return {millisecondsPer50Morphs:samples,computedStyleReads:reads};};
  try{return {scope:'Same-page isolated morph scripting, not native FPS',baseline:measure(old.playLevelsMorph),updated:measure(updated.playLevelsMorph)};}finally{host.remove();window.requestAnimationFrame=originalRaf;window.cancelAnimationFrame=originalCancel;window.getComputedStyle=originalStyle;}
 });mkdirSync('out/rebuild-qa/expressive-performance',{recursive:true});writeFileSync('out/rebuild-qa/expressive-performance/morph-cost.json',JSON.stringify(report,null,2));expect(report.updated.computedStyleReads).toBe(0);expect(report.baseline.computedStyleReads).toBeGreaterThan(0);
});
