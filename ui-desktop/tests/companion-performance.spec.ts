import {test,expect} from '@playwright/test';
test('companion interaction pacing sample',async({page})=>{
 await page.setViewportSize({width:1200,height:800});await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await expect(page.locator('.collection-grid .collection-card').first()).toBeVisible();
 await page.evaluate(()=>{const samples:number[]=[];(window as any).__pacing=samples;let last=performance.now();const tick=(now:number)=>{samples.push(now-last);last=now;if(samples.length<160)requestAnimationFrame(tick);};requestAnimationFrame(tick);});
 const sizes=[];for(let width=1200;width>=800;width-=20){await page.setViewportSize({width,height:800});await page.waitForTimeout(16);sizes.push(await page.locator('.collection-grid .card-artwork').first().evaluate(el=>el.getBoundingClientRect().width));}
 const immediate=sizes.at(-1)!;await page.waitForTimeout(220);const settled=await page.locator('.collection-grid .card-artwork').first().evaluate(el=>el.getBoundingClientRect().width);
 const pacing=await page.evaluate(()=>(window as any).__pacing as number[]);const sorted=pacing.slice().sort((a,b)=>a-b);
 expect(Math.abs(immediate-settled)).toBeLessThan(1);
 await test.info().attach('interaction-pacing.json',{body:JSON.stringify({pacing:'browser fixture; not native display FPS',samples:pacing.length,median_ms:sorted[Math.floor(sorted.length/2)],p95_ms:sorted[Math.floor(sorted.length*.95)],over_25ms:pacing.filter(t=>t>25).length,resize_card_widths:sizes,immediate,settled,settle_jump:Math.abs(immediate-settled)},null,2),contentType:'application/json'});
});
