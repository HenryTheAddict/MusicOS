import {test,expect} from '@playwright/test';

test('filter option menus show selection, support keyboard and stay inside the viewport',async({page})=>{
 for(const size of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(size);await page.goto('/?fixture=1&consistency=1');
  await page.getByRole('button',{name:'Filter',exact:true}).click();
  const panel=page.getByRole('region',{name:'Library filters'}),sort=panel.getByRole('button',{name:'Sort',exact:true});
  await panel.getByRole('button',{name:'Reset filters',exact:true}).click();
  await sort.click();await expect(page.getByRole('menuitemradio',{name:'Recently Added',exact:true})).toHaveAttribute('aria-checked','true');
  await page.getByRole('menuitemradio',{name:'Album Name',exact:true}).click();await expect(sort).toHaveText('Album Name');await expect(panel).toBeVisible();
  await sort.focus();await page.keyboard.press('ArrowDown');await expect(page.getByRole('menu',{name:'Sort',exact:true})).toBeVisible();
  await page.keyboard.press('Escape');await expect(panel).toBeVisible();await expect(sort).toBeFocused();
  await panel.getByRole('button',{name:'Availability',exact:true}).click();
  const menu=page.getByRole('menu',{name:'Availability',exact:true});await expect(menu).toBeVisible();const bounds=(await menu.boundingBox())!;
  expect(bounds.x).toBeGreaterThanOrEqual(7);expect(bounds.y).toBeGreaterThanOrEqual(7);expect(bounds.x+bounds.width).toBeLessThanOrEqual(size.width-7);expect(bounds.y+bounds.height).toBeLessThanOrEqual(size.height-7);
  expect(await menu.evaluate(el=>{const box=el.getBoundingClientRect();return [0.25,0.5,0.75].every(y=>el.contains(document.elementFromPoint(box.right-12,box.top+box.height*y)));})).toBe(true);
  await page.screenshot({path:`out/rebuild-qa/filter-options-${size.width}.png`,animations:'disabled'});
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');await expect(panel).not.toBeVisible();
 }
});

test('resizing a virtualized grid preserves cards until columns settle and ends without overlaps',async({page})=>{
 await page.goto('/?fixture=1&largeAlbums=1');await expect(page.locator('.virtual-grid').first()).toBeVisible();
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
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
 const glyph=page.locator('.transport [data-icon=pause]');await expect(glyph).not.toHaveAttribute('data-morphing','true');
 await page.getByRole('button',{name:'Pause',exact:true}).click();
 await expect(page.locator('.transport [data-morphing=true] path').first()).toHaveAttribute('d',/Q/);
 await page.waitForTimeout(120);await page.getByRole('button',{name:'Play',exact:true}).click();
 await expect(glyph).not.toHaveAttribute('data-morphing','true');expect(await glyph.locator('path').last().getAttribute('d')).toContain('M21.2,4');
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Pause',exact:true}).click();await expect(page.locator('.transport [data-morphing=true]')).toHaveCount(0);
});

test('native resize fallback updates grid columns when its ResizeObserver does not deliver',async({page})=>{
 await page.addInitScript(()=>{const Original=ResizeObserver;window.ResizeObserver=class extends Original{observe(target:Element,options?:ResizeObserverOptions){if(!target.matches('.collection-grid-container'))super.observe(target,options);}};});
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');const grid=page.locator('.collection-grid');const columns=()=>grid.evaluate(el=>getComputedStyle(el).gridTemplateColumns.split(' ').length);const small=await columns();await page.setViewportSize({width:1800,height:1000});await expect.poll(columns).toBeGreaterThan(small);await page.setViewportSize({width:720,height:480});await expect.poll(columns).toBe(small);
});
