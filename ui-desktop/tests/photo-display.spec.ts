import {test,expect,type Page} from '@playwright/test';
test.use({launchOptions:{args:['--enable-webgl','--use-angle=swiftshader','--enable-unsafe-swiftshader']}});
test.setTimeout(60000);
async function start(page:Page,depth=false){
 await page.addInitScript(depth=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:100,artwork_depth:depth})),depth);
 await page.goto('/?fixture=1&depthFixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();
}
test('expanded and moving photos keep the prepared display source instead of loading raw HDR originals',async({page})=>{
 let rawRequests=0;await page.route('**/raw-hdr-photo.png',async route=>{rawRequests++;await route.abort();});
 await page.route('**/src/fixtures.ts*',async route=>{const response=await route.fetch(),body=await response.text();expect(body).toMatch(/photo:\s*a\.path/);await route.fulfill({response,body:body.replace(/photo:\s*a\.path/,"photo:'/raw-hdr-photo.png'")});});
 await start(page,true);await page.getByRole('button',{name:'Open Clockwork Confetti',exact:true}).click();const cover=page.getByRole('button',{name:'Expand Clockwork Confetti artwork',exact:true});await expect(cover.locator('img')).toHaveAttribute('data-artwork-path',/./);
 await cover.locator('img').evaluate(image=>{image.dataset.fullSrc='/raw-hdr-photo.png';});const source=await cover.locator('img').getAttribute('src');await cover.click();const dialog=page.getByRole('dialog',{name:'Clockwork Confetti artwork'});
 await expect(dialog.locator('.viewer-artwork>img')).toHaveAttribute('src',source!);await dialog.locator('.viewer-artwork').hover({position:{x:80,y:80}});await expect(dialog.locator('.viewer-artwork')).toHaveAttribute('data-depth-ready','true');
 await expect(dialog.locator('.viewer-artwork>img')).toHaveCSS('dynamic-range-limit','standard');expect(rawRequests).toBe(0);await page.screenshot({path:'out/rebuild-qa/photo-sdr-expanded.png',animations:'disabled'});
 await page.keyboard.press('Escape');await expect(dialog).not.toBeVisible();await expect(cover).toBeFocused();
});
test('photo expansion fits every desktop scale and restores focus with Reduced Motion',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await start(page);
 for(const width of [720,1280])for(const scale of [80,100,125,150]){
  await page.setViewportSize({width,height:width===720?480:800});await page.evaluate(scale=>document.documentElement.style.fontSize=`${scale}%`,scale);
  await page.getByRole('button',{name:'Open Clockwork Confetti',exact:true}).click();const cover=page.getByRole('button',{name:'Expand Clockwork Confetti artwork',exact:true});await cover.click();const dialog=page.getByRole('dialog',{name:'Clockwork Confetti artwork'}),image=dialog.locator('.viewer-artwork>img');await expect(image).toBeVisible();const box=(await image.boundingBox())!;expect(box.x).toBeGreaterThanOrEqual(0);expect(box.y).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(width+1);expect(box.y+box.height).toBeLessThanOrEqual((width===720?480:800)+1);
  await page.keyboard.press('Escape');await expect(cover).toBeFocused();await page.getByRole('button',{name:'Back to Albums',exact:true}).click();
 }
});
