import {test,expect,type Page} from '@playwright/test';
async function playlist(page:Page){
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/1',name:'Evening',kind:'manual',spark:null,track_ids:[25,1,26,37,2,25],artwork_choice:'collage'}])));await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open playlist Evening',exact:true}).click();
}
test('toolbar Find filters only its playlist, preserves order and never changes the saved sequence or playback',async({page})=>{
 await playlist(page);await page.getByRole('button',{name:'Find in this list',exact:true}).click();const field=page.getByRole('searchbox',{name:'Find in Evening',exact:true});await expect(field).toBeFocused();await field.fill('artist:"Pixel Ferrets" title:Opening');
 const ids=await page.locator('.track-row').evaluateAll(rows=>rows.map(row=>row.getAttribute('data-track-id')));expect(ids).toEqual(['25','37','25']);await expect(page.locator('.list-find-count')).toContainText('3 of 6 songs');await expect(page.getByRole('region',{name:'Suggested additions'})).toHaveCount(0);
 expect(await page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-collections')!)[0].track_ids)).toEqual([25,1,26,37,2,25]);expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((call:any)=>call.command==='playback').length)).toBe(0);
 await field.fill('something totally missing');await expect(page.getByText('No matching songs. Try another term or clear Find.')).toBeVisible();await page.keyboard.press('Escape');await expect(page.getByRole('button',{name:'Find in this list',exact:true})).toBeFocused();await expect(page.locator('.list-find')).toHaveCount(0);await expect(page.locator('.track-row')).toHaveCount(6);
});
test('Command F and Control F refocus local Find, while Command K remains global',async({page})=>{
 await playlist(page);await page.keyboard.press('Meta+f');const field=page.getByRole('searchbox',{name:'Find in Evening',exact:true});await expect(field).toBeFocused();await field.fill('artist:Velvet Static');await page.keyboard.press('Control+f');await page.keyboard.type('title:Opening');await expect(field).toHaveValue('title:Opening');
 await page.keyboard.press('Meta+k');await expect(page.getByRole('combobox',{name:'SuperCommand search'})).toBeVisible();await page.keyboard.press('Escape');await expect(field).toHaveValue('title:Opening');
 await page.getByRole('button',{name:'Clear',exact:true}).click();await expect(field).toBeFocused();await expect(page.locator('.list-find-count')).toContainText('6 of 6 songs');
});
test('Find survives detail navigation and restores the unfiltered scroll when dismissed',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();const scroll=page.locator('.page-scroll');await scroll.evaluate(el=>el.scrollTop=250);const before=await scroll.evaluate(el=>el.scrollTop);
 await page.keyboard.press('Control+f');const field=page.getByRole('searchbox',{name:'Find in Songs',exact:true});await field.fill('artist:Moonspindle title:Opening');await page.locator('.track-row').first().getByRole('button',{name:'Moonspindle',exact:true}).click();await expect(page.locator('.artist-hero')).toBeVisible();await page.locator('.sidebar-back').click();await expect(field).toHaveValue('artist:Moonspindle title:Opening');await page.keyboard.press('Control+f');await page.keyboard.press('Escape');expect(await scroll.evaluate(el=>el.scrollTop)).toBe(before);
});
test('collection Find reuses the existing field and album Find respects field qualifiers',async({page})=>{
 await playlist(page);await page.getByRole('button',{name:'More library views',exact:true}).click();await page.getByRole('menuitem',{name:'Playlists',exact:true}).click();await page.getByRole('button',{name:'Find in this list',exact:true}).click();await expect(page.getByRole('searchbox',{name:'Find collections',exact:true})).toBeFocused();await expect(page.locator('.list-find')).toHaveCount(0);await page.keyboard.type('Evening');await page.keyboard.press('Escape');await expect(page.getByRole('searchbox',{name:'Find collections',exact:true})).toHaveValue('');
 await page.getByRole('button',{name:'Albums',exact:true}).click();await page.keyboard.press('Control+f');await page.getByRole('searchbox',{name:'Find in Albums',exact:true}).fill('artist:Moonspindle');await expect(page.locator('.collection-card')).toHaveCount(3);await expect(page.locator('.list-find-count')).toContainText('3 of 12 albums');
});
for(const percent of [80,100,125,150])for(const mode of ['light','dark'])test(`compact list Find geometry at ${percent}% ${mode}`,async({page})=>{
 await page.addInitScript(({percent,mode})=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,appearance:{mode,source:'artwork',accent:'#ff00ff',saturation:150,tint:20}})),{percent,mode});await playlist(page);
 for(const viewport of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(viewport);const toolbar=page.locator('.library-toolbar');const icons=await toolbar.locator('.icon').evaluateAll(nodes=>nodes.map(node=>{const rect=node.getBoundingClientRect();return {w:rect.width,h:rect.height,rem:parseFloat(getComputedStyle(document.documentElement).fontSize)};}));for(const icon of icons){expect(icon.w/icon.rem).toBeLessThanOrEqual(1.01);expect(icon.h/icon.rem).toBeLessThanOrEqual(1.01);}
  await page.getByRole('button',{name:'Find in this list',exact:true}).click();const field=page.getByRole('searchbox',{name:'Find in Evening',exact:true});await field.fill('artist:Daft');expect(await page.locator('.list-find').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);const box=(await field.boundingBox())!;expect(box.width).toBeGreaterThan(70);await page.screenshot({path:`out/rebuild-qa/list-find-${mode}-${percent}-${viewport.width}.png`});await page.keyboard.press('Escape');
 }
});
test('Reduced Motion has no find animation and dialogs retain keyboard ownership',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await playlist(page);await page.keyboard.press('Control+f');expect(await page.locator('.list-find').evaluate(el=>el.getAnimations().length)).toBe(0);await page.keyboard.press('Escape');await page.getByRole('button',{name:'Edit details',exact:true}).click();await page.keyboard.press('Control+f');await expect(page.getByRole('dialog',{name:'Edit playlist details',exact:true})).toBeVisible();await expect(page.locator('.list-find')).toHaveCount(0);
});
