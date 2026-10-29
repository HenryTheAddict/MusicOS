import {test,expect} from '@playwright/test';

test('square desktop retains navigation and stationary player across scales',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(new URLSearchParams(location.search).get('scale'))||100})));
 for(const scale of [80,100,125,150]){
  await page.setViewportSize({width:720,height:720});await page.goto(`/?fixture=1&scale=${scale}`);await expect(page.locator('.adaptive-shell')).toHaveAttribute('data-size',String(scale));
  await expect(page.locator('.adaptive-shell')).toHaveAttribute('data-layout','wide');
  await expect(page.locator('.personal-home')).toBeVisible();await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');await page.screenshot({path:`out/companion-2026-10-29/square-${scale}.png`});const positions=await page.evaluate(()=>['.section-control','.now-playing'].map(selector=>{const r=document.querySelector(selector)!.getBoundingClientRect();return {left:r.left,right:r.right};}));expect(positions[0].right).toBeLessThanOrEqual(positions[1].left);
  await expect(page.getByRole('navigation',{name:'Library',exact:true})).toBeVisible();
  expect((await page.locator('.now-playing-info').boundingBox())!.width).toBeGreaterThanOrEqual(100);
  expect(await page.locator('.page-scroll').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);
  await page.locator('.now-playing').evaluate(el=>(window as any).__playerNode=el);
  await page.setViewportSize({width:480,height:720});await expect(page.locator('.adaptive-shell')).toHaveAttribute('data-layout','compact');
  await page.setViewportSize({width:1024,height:768});await expect(page.locator('.adaptive-shell')).toHaveAttribute('data-layout','wide');
  expect(await page.locator('.now-playing').evaluate(el=>el===(window as any).__playerNode)).toBe(true);
 }
});
test('standard mini artwork fills available space without stretching; queue retains footer',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await page.getByRole('button',{name:'musicOS actions',exact:true}).click();await page.getByRole('menuitem',{name:'Mini Player',exact:true}).click();
 for(const size of [{width:244,height:320},{width:334,height:480}]){
  await page.setViewportSize(size);const mini=page.locator('.mini-shell>.mini-player');await expect(mini).toHaveCSS('height',`${size.height}px`);
  await mini.evaluate(el=>Promise.allSettled(el.getAnimations().map(a=>a.finished)));
  const art=await mini.locator('.mini-player-art').boundingBox();expect(art!.width).toBeGreaterThanOrEqual(size.width===244?155:300);expect(Math.abs(art!.width-art!.height)).toBeLessThan(1);
  const footer=await mini.locator('.mini-player-transport').boundingBox();await mini.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(mini).toHaveAttribute('data-layout','queue');expect(await mini.locator('.mini-player-transport').boundingBox()).toEqual(footer);
  await mini.getByRole('button',{name:'Close Up next',exact:true}).click();await expect(mini.locator('.mini-tall-view')).toHaveCSS('opacity','1');await expect(mini.locator('.mini-queue-view')).toHaveCSS('visibility','hidden');await page.screenshot({path:`out/companion-2026-10-29/mini-${size.width}.png`});
 }
});
test('unknown tempo still has a gentle playing heart and cached artist visits do not recheck',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();const row=page.locator('[data-track-id="1"]');await row.hover();await row.locator('.row-play').click();
 const heart=row.locator('.track-favorite');await expect(heart).toHaveAttribute('data-tempo-source','gentle');await expect(heart).toHaveAttribute('data-pulsing','true');
 await page.getByRole('button',{name:'Artists',exact:true}).click();await expect(page.locator('[data-artist-name]').first()).toBeVisible();await expect.poll(()=>page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='artist_info').length)).toBeGreaterThan(0);
 const before=await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='artist_info').length);
 await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Artists',exact:true}).click();await page.waitForTimeout(300);
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='artist_info').length)).toBe(before);
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='artist_info').every((r:any)=>r.args.refresh===false))).toBe(true);
 await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('.now-playing .heart-beat')).toHaveCSS('animation-name','none');
});
