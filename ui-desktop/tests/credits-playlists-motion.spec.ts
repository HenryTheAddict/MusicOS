import {test,expect} from '@playwright/test';
test('saved playlists live below More, preserve collections, and keep their own artwork colors',async({page})=>{
 const collections=[{id:'playlist/1',name:'A long playlist name for the road',track_ids:[1,2],kind:'manual',spark:null,artwork_choice:'collage'},{id:'playlist/2',name:'Quiet mornings',track_ids:[4],kind:'manual',spark:null,artwork_choice:'collage'},{id:'audio/1',name:'Saved sparkle',track_ids:[1,3],kind:'audio',spark:null}];
 await page.addInitScript(items=>localStorage.setItem('musicos-fixture-collections',JSON.stringify(items)),collections);await page.goto('/?fixture=1');
 const box=page.getByRole('region',{name:'Your playlists'}),more=page.getByRole('button',{name:'More library views'});
 await expect(box.getByRole('button',{name:'Open playlist Quiet mornings'})).toBeVisible();await expect(box.getByRole('button',{name:/Saved sparkle/})).toHaveCount(0);
 expect((await box.boundingBox())!.y).toBeGreaterThan((await more.boundingBox())!.y+(await more.boundingBox())!.height);
 await expect.poll(()=>box.locator('[data-collection-tone]').first().evaluate(el=>el.style.getPropertyValue('--collection-hue-1'))).not.toBe('');
 await box.getByRole('button',{name:'Open playlist Quiet mornings'}).click({button:'right'});await expect(page.getByRole('menuitem',{name:'Edit details…',exact:true})).toBeVisible();await page.keyboard.press('Escape');await expect(box.getByRole('button',{name:'Open playlist Quiet mornings'})).toBeFocused();
 await box.getByRole('button',{name:'Open playlist Quiet mornings'}).click();await expect(page.getByRole('heading',{name:'Quiet mornings',exact:true})).toBeVisible();
 await expect(box.getByRole('button',{name:'Open playlist Quiet mornings'})).toHaveAttribute('aria-current','page');
 expect(await page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-collections')!))).toEqual(collections);
 await page.screenshot({path:'out/rebuild-qa/credits-playlists-wide.png'});
});
for(const percent of [80,100,125,150])for(const theme of ['light','dark'])test(`named contributors and playlist box fit at ${percent}% ${theme}`,async({page})=>{
 await page.addInitScript(({percent,theme})=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,appearance:{mode:theme,source:'artwork',accent:'#0064df',saturation:100,tint:15}}));localStorage.setItem('musicos-fixture-collections',JSON.stringify(Array.from({length:12},(_,i)=>({id:`playlist/${i}`,name:`Playlist ${i+1}`,track_ids:[1],kind:'manual',spark:null}))));},{percent,theme});
 for(const size of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(size);await page.goto('/?fixture=1&contributors=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();
  const credits=page.getByRole('group',{name:'Album artists',exact:true});
  await expect(credits.getByRole('button',{name:'View artist M83',exact:true})).toBeVisible();await expect(credits.locator('.album-collaborator .artist-credit-name').first()).toBeVisible();
  expect(await credits.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);
  const contributorBottom=(await credits.boundingBox())!.y+(await credits.boundingBox())!.height;
  expect((await page.getByRole('button',{name:'Play collection',exact:true}).boundingBox())!.y).toBeGreaterThanOrEqual(contributorBottom);
  const all=page.getByRole('button',{name:'15 more contributing artists',exact:true});await all.click();await expect(page.getByRole('menuitem',{name:'Guest artist 17',exact:true})).toBeVisible();await page.keyboard.press('End');await expect(page.getByRole('menuitem',{name:'Guest artist 17',exact:true})).toBeFocused();await page.keyboard.press('Escape');await expect(all).toBeFocused();
  const box=page.getByRole('region',{name:'Your playlists'});await box.scrollIntoViewIfNeeded();expect((await box.boundingBox())!.height).toBeGreaterThan(32);expect(await page.locator('.library-sidebar').evaluate(el=>el.scrollHeight-el.clientHeight)).toBeLessThan(2);
  await page.screenshot({path:`out/rebuild-qa/credits-${theme}-${percent}-${size.width}.png`});
 }
});
test('creation and transport morphs reverse without changing targets and respect Reduced Motion',async({page})=>{
 await page.goto('/?fixture=1');const create=page.locator('.sidebar-personal>.menu-trigger'),before=await create.boundingBox();await create.click();await expect(create.locator('[data-icon=menuClose]')).toBeVisible();await page.keyboard.press('Escape');await expect(create).toBeFocused();expect(await create.boundingBox()).toEqual(before);
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();const bar=page.getByRole('region',{name:'Now playing'});const pause=bar.getByRole('button',{name:'Pause',exact:true});await expect(pause).toBeVisible();const bounds=await pause.boundingBox();
 await pause.click();await page.waitForTimeout(70);await bar.getByRole('button',{name:'Play',exact:true}).click();await page.waitForTimeout(400);expect(await pause.boundingBox()).toEqual(bounds);await expect(pause.locator('[data-morphing]')).toHaveCount(0);
 await page.emulateMedia({reducedMotion:'reduce'});await pause.click();await expect(bar.locator('[data-transport-morph]')).toHaveCount(0);await create.click();await expect(create.locator('[data-cross-morph]')).toHaveCount(0);await page.keyboard.press('Escape');
});
test('transport motion keeps a single split silhouette through its intermediate frames',async({page})=>{
 await page.goto('/?fixture=1');
 await page.evaluate(async()=>{
  const source='/src/playPauseMorph.ts',module=await import(source),host=document.createElement('div');host.id='transport-frames';host.style.cssText='position:fixed;inset:100px auto auto 20px;z-index:99999;background:#252a30;color:#e7c4bf;display:flex;gap:20px;padding:24px;border-radius:16px';
  for(const progress of [0,.25,.5,.75,1]){
   const drawing=module.transportDrawing(progress) as {outlines:string[];seams:string[];fill:string};
   const cell=document.createElement('div');cell.style.cssText='text-align:center;font:14px system-ui';
   cell.innerHTML=`<svg width="80" height="80" viewBox="0 0 32 32"><defs><linearGradient id="frame-${progress}" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="32"><stop offset="0" stop-color="#eed4d1"/><stop offset="1" stop-color="#bc7772"/></linearGradient></defs>${drawing.outlines.map(d=>`<path d="${d}" fill="none" stroke="#a86e69" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>`).join('')}${drawing.seams.map(d=>`<path d="${d}" fill="none" stroke="#a86e69" stroke-width="3" stroke-opacity="${progress}" stroke-linejoin="round"/>`).join('')}<path d="${drawing.fill}" fill="url(#frame-${progress})"/></svg><div>${progress*100}%</div>`;host.append(cell);
  }document.body.append(host);
 });
 await page.locator('#transport-frames').screenshot({path:'out/rebuild-qa/credits-transport-frames.png'});
});
