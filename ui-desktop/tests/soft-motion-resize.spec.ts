import {test,expect} from '@playwright/test';
import {openAlbums,settleAnimations} from './support/ui';

test('filter option menus show selection, support keyboard and stay inside the viewport',async({page})=>{
 for(const size of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(size);await page.goto('/?fixture=1&consistency=1');await openAlbums(page);
  await page.getByRole('button',{name:'Filter',exact:true}).click();
  const panel=page.getByRole('region',{name:'Library filters'}),sort=panel.getByRole('button',{name:'Sort',exact:true});
  await panel.getByRole('button',{name:'Reset filters',exact:true}).click();
  await sort.click();await expect(page.getByRole('menuitemradio',{name:'Recently Added',exact:true})).toHaveAttribute('aria-checked','true');
  await page.getByRole('menuitemradio',{name:'Album Name',exact:true}).click();await expect(sort).toHaveText('Album Name');await expect(panel).toBeVisible();
  await sort.focus();await page.keyboard.press('ArrowDown');await expect(page.getByRole('menu',{name:'Sort',exact:true})).toBeVisible();
  await page.keyboard.press('Escape');await expect(panel).toBeVisible();await expect(sort).toBeFocused();
  await panel.getByRole('button',{name:'Availability',exact:true}).click();
  const menu=page.getByRole('menu',{name:'Availability',exact:true});await expect(menu).toBeVisible();await settleAnimations(page);const bounds=(await menu.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(7);expect(bounds.y).toBeGreaterThanOrEqual(7);expect(bounds.x+bounds.width).toBeLessThanOrEqual(size.width-7);expect(bounds.y+bounds.height).toBeLessThanOrEqual(size.height-7);
  expect(await menu.evaluate(el=>{const box=el.getBoundingClientRect();return [0.25,0.5,0.75].every(y=>el.contains(document.elementFromPoint(box.right-12,box.top+box.height*y)));})).toBe(true);
  await page.screenshot({path:`out/rebuild-qa/filter-options-${size.width}.png`,animations:'disabled'});
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');await expect(panel).not.toBeVisible();
 }
});

test('long Genre choices remain clickable past the Filter panel edge',async({page},testInfo)=>{
 const genres=['Electronic','Alternative','Rock','Ambient','Blues','Classical','Country','Disco','Folk','Funk','Gospel','Jazz','Metal','Pop','Reggae','Soul'];
 await page.route('**/src/fixtures.ts*',async route=>{
  const response=await route.fetch();
  await route.fulfill({response,body:(await response.text()).replace(/genres:\s*\[\s*"Electronic",\s*"Alternative",\s*"Rock"\s*\]/,`genres: ${JSON.stringify(genres)}`)});
 });
 await page.goto('/?fixture=1');await openAlbums(page);
 const trigger=page.getByRole('button',{name:'Filter',exact:true});
 const panel=page.getByRole('region',{name:'Library filters'});
 const genre=panel.getByRole('button',{name:'Genre',exact:true});
 await trigger.click();
 await genre.click();
 const menu=page.getByRole('menu',{name:'Genre',exact:true});
 const soul=menu.getByRole('menuitemradio',{name:'Soul',exact:true});
 await expect(soul).toBeVisible();await settleAnimations(page);
 await page.screenshot({path:testInfo.outputPath('filter-genre.png')});
 await menu.evaluate(el=>el.scrollTop=el.scrollHeight);await settleAnimations(page);
 const panelBox=(await panel.boundingBox())!,soulBox=(await soul.boundingBox())!;
 expect(soulBox.y+soulBox.height/2).toBeGreaterThan(panelBox.y+panelBox.height);
 expect(await soul.evaluate(el=>{
  const box=el.getBoundingClientRect();
  return el.contains(document.elementFromPoint(box.x+box.width/2,box.y+box.height/2));
 })).toBe(true);
 await soul.click();
 await expect(genre).toHaveText('Soul');
 await expect(trigger).toHaveAttribute('aria-expanded','true');
 await genre.click();await expect(menu).toBeVisible();
 const sort=panel.getByRole('button',{name:'Sort',exact:true});
 await sort.click();
 await page.getByRole('menu',{name:'Sort',exact:true}).getByRole('menuitemradio',{name:'Album Name',exact:true}).click();
 await expect(sort).toHaveText('Album Name');
 await genre.click();await expect(menu).toBeVisible();
 await trigger.click();await expect(menu).toHaveCount(0);
 await trigger.click();await expect(genre).toHaveAttribute('aria-expanded','false');
});

test('resizing a virtualized grid preserves cards until columns settle and ends without overlaps',async({page})=>{
 await page.goto('/?fixture=1&largeAlbums=1');await openAlbums(page);await expect(page.locator('.virtual-grid').first()).toBeVisible();
 await page.waitForTimeout(400);
 await page.locator('.collection-card').first().evaluate(el=>el.setAttribute('data-resize-retained','true'));
 // A continuous drag changes artwork size, not virtual row identity, every pixel.
 for(const width of [1260,1240,1220,1200]){await page.setViewportSize({width,height:800});await expect(page.locator('[data-resize-retained]')).toHaveCount(1);}
 for(const width of [720,1920,960,2560,1280]){
  await page.setViewportSize({width,height:800});await page.waitForTimeout(500);
  await expect.poll(()=>page.locator('.collection-grid-container').evaluate(el=>el.getAnimations({subtree:true}).filter(a=>a.id==='grid-reflow').length)).toBe(0);
  const boxes=await page.locator('.collection-card').evaluateAll(cards=>cards.slice(0,24).map(card=>{const r=card.getBoundingClientRect(),caption=card.querySelector('.card-caption')!.getBoundingClientRect();return {top:r.top,bottom:r.bottom,captionBottom:caption.bottom};}));
  for(const box of boxes){expect(box.captionBottom).toBeLessThanOrEqual(box.bottom+1);const next=boxes.find(other=>other.top>box.top+2);if(next)expect(next.top).toBeGreaterThanOrEqual(box.captionBottom-1);}
 }
 await page.screenshot({path:'out/rebuild-qa/grid-resize-settled.png',animations:'disabled'});
});

test('transport uses rounded finite contours and settles after rapid reversals',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);await page.getByRole('button',{name:'Open Bubblewrap Lagoon',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
 const glyph=page.locator('.transport [data-icon=pause]');await expect(glyph).not.toHaveAttribute('data-morphing','true');const endpoint=await glyph.locator('svg').innerHTML();
 await page.getByRole('button',{name:'Pause',exact:true}).click();
 await expect(page.locator('.transport [data-transport-morph] > path').first()).toHaveAttribute('d',/C/);
 await page.waitForTimeout(120);await page.getByRole('button',{name:'Play',exact:true}).click();
 await expect(glyph).not.toHaveAttribute('data-morphing','true');await expect(glyph.locator('svg')).toHaveJSProperty('innerHTML',endpoint);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Pause',exact:true}).click();await expect(page.locator('.transport [data-morphing=true]')).toHaveCount(0);
});

test('native resize fallback updates grid columns when its ResizeObserver does not deliver',async({page})=>{
 await page.addInitScript(()=>{const Original=ResizeObserver;window.ResizeObserver=class extends Original{observe(target:Element,options?:ResizeObserverOptions){if(!target.matches('.collection-grid-container'))super.observe(target,options);}};});
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await openAlbums(page);const grid=page.locator('.collection-grid');const columns=()=>grid.evaluate(el=>getComputedStyle(el).gridTemplateColumns.split(' ').length);const small=await columns();await page.setViewportSize({width:1800,height:1000});await expect.poll(columns).toBeGreaterThan(small);await page.setViewportSize({width:720,height:480});await expect.poll(columns).toBe(small);
});
