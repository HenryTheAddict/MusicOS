import {test,expect} from '@playwright/test';
test('a recommendation excerpt leaves the playing queue unchanged and stops explicitly',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-home-activity',JSON.stringify({frequent:[{id:1,plays:3}],recent:[1,2]})));
 await page.goto('/?fixture=1');await page.locator('.sidebar-home').click();
 const row=page.locator('.home-recent>[data-action-surface]').first();
 await row.locator('.home-recent-copy').click({button:'right'});
 const preview=page.getByRole('menuitem',{name:'Preview 30 seconds'});
 await expect(preview).toBeVisible();
 const before=await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback').length);
 await preview.click();await expect(page.getByRole('complementary',{name:'Song preview'})).toBeVisible();await expect(page.locator('.song-preview-tray progress')).toHaveAttribute('max','30');
 await expect.poll(()=>page.locator('.song-preview-tray progress').getAttribute('value')).not.toBe('0');
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback').length)).toBe(before);
 await page.getByRole('button',{name:'Stop song preview',exact:true}).click();await expect(page.getByRole('complementary',{name:'Song preview'})).toHaveCount(0);
 const calls=await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='audition').map((c:any)=>c.args.action));expect(calls).toContain('start');expect(calls).toContain('stop');
});
test('recently played actions fit at narrow and scaled desktop widths',async({page})=>{
 for(const [width,height,scale] of [[480,480,100],[720,480,125],[1280,800,150],[1280,800,75]]){
  await page.setViewportSize({width,height});await page.goto('/?fixture=1&homeActivity=1');await page.evaluate(scale=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:scale})),scale);await page.reload();
  const home=page.locator('.sidebar-home');if(await home.isVisible())await home.click();else await page.getByRole('button',{name:'Home',exact:true}).click();
  await expect(page.locator('.personal-home')).toBeVisible();expect(await page.locator('.page-scroll').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);
  for(const row of await page.locator('.home-recent>[data-action-surface]').all()){const box=await row.boundingBox();const actions=await row.getByRole('button',{name:/^Play /}).boundingBox();expect(actions!.x+actions!.width).toBeLessThanOrEqual(box!.x+box!.width+1);}
 }
});
