import {test,expect,type Page} from '@playwright/test';
async function start(page:Page,mode='dark',percent=100){
 await page.addInitScript(({mode,percent})=>{
  localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,appearance:{mode,source:'artwork',accent:'#0064df',saturation:100,tint:15}}));
  localStorage.setItem('musicos-fixture-analysis',JSON.stringify([{track_id:1,bpm:120}]));
 },{mode,percent});
 await page.route('**/src/fixtures.ts*',async route=>{
  const response=await route.fetch(),body=await response.text(),marker='case "artwork_palette":';
  await route.fulfill({response,body:body.replace(marker,`${marker}
   if(String(a.path).includes('Pier'))return {background:'#ecc4c4',blob:'#bc7777',accent:'#802525',text:'#12202a'};
   return {background:'#c8ded9',blob:'#55937a',accent:'#20574d',text:'#12202a'};
  `)});
 });
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await expect(page.getByRole('button',{name:'Favorite playing song',exact:true})).toHaveAttribute('data-pulsing','true');
}
for(const mode of ['light','dark'])test(`${mode}: player colors belong to the playing album and its context menu`,async({page})=>{
 await start(page,mode);const bar=page.getByRole('region',{name:'Now playing',exact:true});
 const colors=()=>bar.evaluate(el=>({surface:el.style.getPropertyValue('--surface-color'),accent:el.style.getPropertyValue('--accent'),ink:el.style.getPropertyValue('--ink'),fill:el.style.getPropertyValue('--transport-fill'),stroke:el.style.getPropertyValue('--transport-stroke')}));
 const expected=await page.evaluate(async mode=>{const path='/src/appearance.ts';const {themePalette,defaultAppearance}=await import(/* @vite-ignore */ path);return themePalette({...defaultAppearance,mode,source:'artwork'},{background:'#c8ded9',blob:'#55937a',accent:'#20574d',text:'#12202a'},false);},mode);
 await expect.poll(async()=>(await colors()).accent).toBe(expected.accent);const before=await colors();
 await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();
 await expect.poll(()=>page.locator('html').evaluate(el=>el.style.getPropertyValue('--surface-color'))).not.toBe(before.surface);
 expect(await colors()).toEqual(before);
 const tones=await bar.locator('.transport path[stroke]').first().evaluate(el=>({fill:getComputedStyle(el).fill,stroke:getComputedStyle(el).stroke}));expect(tones.fill).not.toBe(tones.stroke);
 await bar.click({button:'right'});const menu=page.getByRole('menu');await expect(menu).toBeVisible();
 expect(await menu.evaluate(el=>el.style.getPropertyValue('--accent'))).toBe(before.accent);
 await expect(menu).not.toHaveCSS('border-top-color','rgb(255, 255, 255)');await page.keyboard.press('Escape');
 await page.screenshot({path:`out/rebuild-qa/player-polish-${mode}.png`,animations:'disabled'});
});
for(const percent of [80,100,125,150])test(`heart sits beside title without covering labels at compact and large sizes (${percent}%)`,async({page})=>{
 await start(page,'dark',percent);const heart=page.locator('.now-playing-title .track-favorite');
 for(const size of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(size);
  const geometry=await heart.evaluate(el=>{const a=el.getBoundingClientRect(),label=el.parentElement!.querySelector('.overflow-label')!.getBoundingClientRect(),bar=el.closest('.now-playing')!.getBoundingClientRect();return {gap:a.left-label.right,fits:a.top>=bar.top&&a.bottom<=bar.bottom&&a.right<=bar.right};});
  expect(geometry.gap).toBeGreaterThanOrEqual(0);expect(geometry.gap).toBeLessThan(12);expect(geometry.fits).toBe(true);
  expect(await heart.evaluate(el=>{const button=el.getBoundingClientRect(),svg=el.querySelector('svg')!.getBoundingClientRect();return svg.width<=button.width*.9&&svg.height<=button.height*.9;})).toBe(true);
 }
 await expect(heart.locator('.heart-beat')).toHaveCSS('animation-duration','0.5s');
 await heart.click();await expect(page.getByRole('button',{name:'Unfavorite playing song',exact:true})).toHaveAttribute('aria-pressed','true');
 await page.getByRole('button',{name:'Pause',exact:true}).click();await expect(heart.locator('.heart-beat')).toHaveCSS('animation-name','none');
});
test('overflow fades, reveals on hover, and stays still with Reduced Motion; audio uses a chevron',async({page})=>{
 await start(page);await page.setViewportSize({width:720,height:480});
 const album=page.locator('.now-playing-album');await album.evaluate(el=>{el.style.maxWidth='45px';});
 const label=album.locator('.overflow-label');await expect(label).toHaveAttribute('data-overflow','true');
 await expect(label).not.toHaveCSS('mask-image','none');await album.hover();await expect(label).toHaveCSS('mask-image','none');
 await page.emulateMedia({reducedMotion:'reduce'});await expect(label.locator(':scope>span')).toHaveCSS('transform','none');
 await page.setViewportSize({width:1280,height:800});await page.locator('.now-playing .audio-quality button').click();const panel=page.getByRole('dialog',{name:'Audio controls',exact:true});
 await panel.getByText('Playback details',{exact:true}).click();await expect(panel.locator('details')).toHaveAttribute('open','');
 await expect(panel.locator('summary')).toHaveCSS('list-style-type','none');await expect(panel.getByText('Up to date',{exact:true})).toBeVisible();
 await page.screenshot({path:'out/rebuild-qa/player-audio-disclosure.png',animations:'disabled'});
});
test('shape morph stays inside the two-tone transport and respects Reduced Motion',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 const glyph=page.locator('.now-playing .transport .asset-icon[data-icon=play]');
 const tones=await glyph.locator('path[stroke]').evaluate(el=>({fill:getComputedStyle(el).fill,stroke:getComputedStyle(el).stroke}));expect(tones.fill).not.toBe(tones.stroke);
 await page.getByRole('button',{name:'Play collection',exact:true}).click();
 const water=page.locator('.now-playing .transport [data-transport-morph]');await expect(water).toHaveCount(1);
 await expect(page.locator('.page-frame .playback-ripple')).toHaveCount(0);
 await expect(water).toHaveCount(0);
 const pause=page.locator('.now-playing .transport .asset-icon[data-icon=pause]');
 const finalTones=await pause.locator('path[stroke]').first().evaluate(el=>({fill:getComputedStyle(el).fill,stroke:getComputedStyle(el).stroke}));expect(finalTones.fill).not.toBe(finalTones.stroke);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Pause',exact:true}).click();
 await expect(page.locator('[data-transport-morph]')).toHaveCount(0);
});
