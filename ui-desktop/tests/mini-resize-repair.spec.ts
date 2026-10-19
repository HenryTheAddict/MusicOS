import {test,expect} from '@playwright/test';
test('resized queue uses available height and actions preserve its footprint',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await page.locator('[data-track-id="1"]').hover();await page.locator('[data-track-id="1"] .row-play').click();
 await page.getByRole('button',{name:'MusicOS actions',exact:true}).click();await expect(page.getByRole('menuitem',{name:'Mini Player',exact:true}).locator('[data-icon=openMiniplayer]')).toHaveCount(1);await page.getByRole('menuitem',{name:'Mini Player',exact:true}).click();
 await page.setViewportSize({width:450,height:320});const mini=page.locator('.mini-player');await expect(mini).toHaveCSS('width','450px');await page.getByRole('button',{name:'Open Up next',exact:true}).click();await page.setViewportSize({width:330,height:420});
 await expect(mini).toHaveCSS('height','420px');await expect(mini.locator('.mini-queue-list')).toHaveCSS('height','247px');
 await mini.getByRole('button',{name:'Mini Player actions',exact:true}).click();await page.setViewportSize({width:330,height:664});await expect(mini).toHaveCSS('height','420px');await expect(page.getByRole('menu',{name:'Mini Player actions',exact:true})).toHaveCSS('top','424px');
 await expect(page.getByRole('menuitem',{name:'Unpin window',exact:true}).locator('[data-icon=unPin]')).toHaveCount(1);
 await page.keyboard.press('Escape');await page.setViewportSize({width:330,height:420});await expect(mini).toHaveCSS('height','420px');await page.screenshot({path:'out/rebuild-qa/mini-resized-queue.png'});
});
test('dark artwork has no pale backing and album selection is additive',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:100,appearance:{mode:'dark',source:'custom',accent:'#00ed00',saturation:150,tint:20}})));
 await page.goto('/?fixture=1');await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');await expect(page.locator('.card-artwork>.cover:not(.placeholder)').first()).toHaveCSS('background-color','rgba(0, 0, 0, 0)');await expect(page.locator('.card-artwork>.cover:not(.placeholder)').first()).toHaveCSS('clip-path','none');
 await page.screenshot({path:'out/rebuild-qa/dark-artwork-repair.png'});
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect(page.getByRole('button',{name:'Appearance mode',exact:true}).locator('[data-icon="Dark Mode"]')).toHaveCount(1);
 await page.getByRole('tab',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();await page.getByRole('tab',{name:'Edit',exact:true}).click();
 const albums=page.locator('.batch-metadata .metadata-album-list>button');await albums.nth(0).click();const count=async()=>Number((await page.locator('.batch-metadata .selection-actions output').innerText()).split(' ')[0]);const first=await count();expect(first).toBeGreaterThan(0);
 await page.getByRole('button',{name:'Artist change',exact:true}).click();await page.getByRole('menuitemradio',{name:'Set value',exact:true}).click();await page.getByRole('textbox',{name:'New artist',exact:true}).fill('Review only');
 await albums.nth(1).click();expect(await count()).toBeGreaterThan(first);await expect(page.getByRole('textbox',{name:'New artist',exact:true})).toHaveValue('Review only');
});
test('library caption motion belongs to navigation and respects Reduced Motion',async({page})=>{
 await page.addInitScript(()=>{const original=Element.prototype.animate;(window as any).libraryAnimations=[];Element.prototype.animate=function(...args:Parameters<Element['animate']>){const animation=original.apply(this,args);(window as any).libraryAnimations.push(animation);return animation;};});
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Albums',exact:true}).click();
 const count=()=>page.evaluate(()=>(window as any).libraryAnimations.filter((a:Animation)=>a.id==='library-labels').length);expect(await count()).toBeGreaterThan(0);const before=await count();
 await page.mouse.wheel(0,900);await page.setViewportSize({width:1100,height:800});await expect.poll(count).toBe(before);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Artists',exact:true}).click();expect(await count()).toBe(before);
});
test('closing queue never steals focus from the section chooser',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Up next',exact:true}).click();await page.keyboard.press('Escape');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect(page.getByRole('tab',{name:'Appearance',exact:true})).toBeVisible();
});
test('invisible mini edges support pointer and keyboard size requests',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'MusicOS actions',exact:true}).click();await page.getByRole('menuitem',{name:'Mini Player',exact:true}).click();await page.setViewportSize({width:334,height:320});
 const edge=page.getByRole('separator',{name:'Resize Mini Player width',exact:true});await edge.focus();await page.keyboard.press('ArrowRight');await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-window-size')??'null'))).toEqual({width:344,height:320});
 const box=(await edge.boundingBox())!;await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+83,box.y+box.height/2,{steps:6});await page.mouse.up();await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-window-size')??'null')?.width)).toBeGreaterThan(400);
});
test('queue bottom edge has an interior hit target and sends height changes',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'MusicOS actions',exact:true}).click();await page.getByRole('menuitem',{name:'Mini Player',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();await page.setViewportSize({width:244,height:320});
 const edge=page.getByRole('separator',{name:'Resize Mini Player height',exact:true});const box=(await edge.boundingBox())!;expect(box.height).toBeGreaterThanOrEqual(12);await page.mouse.move(box.x+box.width/2,box.y+2);await page.mouse.down();await page.mouse.move(box.x+box.width/2,box.y-30,{steps:6});await page.mouse.up();await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-window-size')??'null')?.height)).toBe(320);
 await edge.focus();await page.keyboard.press('Shift+ArrowDown');await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-window-size')??'null')?.height)).toBe(360);
});
