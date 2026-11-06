import {test,expect} from '@playwright/test';
test('right click is pointer anchored, bounded, keyboard dismissible, and offers manual playlists only',async({page})=>{
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/1',kind:'manual',name:'Manual destination',track_ids:[]},{id:'audio/2',kind:'audio',name:'Audio excluded',track_ids:[]},{id:'spark/3',kind:'rule',name:'Rules excluded',spark:{rules:[]},track_ids:[]}])));await page.reload();await page.getByRole('button',{name:'Songs',exact:true}).click();const row=page.locator('.track-row').first();await row.click({button:'right',position:{x:12,y:12}});const menu=page.getByRole('menu');await expect(menu).toBeVisible();await expect(menu.getByRole('menuitem')).toHaveCount(4);const box=(await menu.boundingBox())!;expect(box.x).toBeGreaterThanOrEqual(0);expect(box.y).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(720);expect(box.y+box.height).toBeLessThanOrEqual(480);
 await menu.getByRole('menuitem',{name:'Add to playlist…'}).click();const picker=page.getByRole('dialog',{name:'Add to playlist',exact:true});await expect(picker.getByRole('button',{name:/Manual destination/})).toBeVisible();await expect(picker.getByText(/excluded/)).toHaveCount(0);await picker.getByRole('searchbox',{name:'Find playlist'}).fill('missing');await expect(picker.getByText('No matching playlists.')).toBeVisible();await page.keyboard.press('Escape');await expect(picker).not.toBeVisible();
 await row.getByRole('button',{name:'More actions',exact:true}).click();await expect(menu).toBeVisible();await page.keyboard.press('Escape');await expect(menu).not.toBeVisible();
});
test('ignored metadata survives reload and restores to review',async({page})=>{
 await page.goto('/?fixture=1');
 const workspace=async()=>{
  await page.getByRole('button',{name:'Choose section',exact:true}).click();
  await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
  await page.getByRole('tab',{name:'Library',exact:true}).click();
  await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();
 };
 const ignored=async()=>{
  await page.getByRole('button',{name:'More metadata tools'}).click();
  await page.getByRole('menuitem',{name:'Ignored suggestions',exact:true}).click();
 };
 await workspace();
 await page.getByRole('button',{name:'Has suggestions',exact:true}).click();
 await page.locator('.tag-albums button').first().click();
 await page.getByRole('tab',{name:'Details',exact:true}).click();
 await page.getByText('Recording and release matches',{exact:true}).click();
 await page.getByRole('button',{name:'Ignore suggestion',exact:true}).first().click();
 await expect(page.getByRole('status').filter({hasText:'Suggestion moved'})).toBeVisible();
 await page.reload();
 await workspace();
 await ignored();
 await expect(page.getByRole('button',{name:'Restore suggestion',exact:true})).toHaveCount(1);
 await page.getByRole('button',{name:'Restore suggestion',exact:true}).click();
 await expect(page.getByRole('button',{name:'Restore suggestion',exact:true})).toHaveCount(0);
});
test('playlist creation selects songs separately and persists their order',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Create playlist',exact:true}).click();const creation=page.getByRole('dialog',{name:'Create playlist',exact:true});await creation.getByRole('textbox',{name:'Name',exact:true}).fill('Picked songs');await creation.getByRole('button',{name:'Choose songs (optional)'}).click();const picker=page.getByRole('dialog',{name:'Choose playlist songs'});await picker.locator('[data-track-id]').nth(1).click();await picker.locator('[data-track-id]').nth(0).click();const expected=await picker.locator('[data-track-id][aria-pressed=true]').evaluateAll(els=>els.map(el=>Number(el.getAttribute('data-track-id'))));await picker.getByRole('button',{name:'Use 2 songs'}).click();await creation.getByRole('button',{name:'Create playlist',exact:true}).click();await expect(page.getByRole('heading',{name:'Picked songs',exact:true})).toBeVisible();const ids=await page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-collections')!).find((c:any)=>c.name==='Picked songs').track_ids);expect(ids).toEqual(expected.reverse());
});
test('compact integrations use supplied glyphs and keep credentials behind setup',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Integrations',exact:true}).click();await expect(page.locator('[data-icon=Discord] svg')).toBeVisible();await expect(page.locator('[data-icon=LastFM] svg')).toBeVisible();await expect(page.getByRole('textbox',{name:'Application ID',exact:true})).not.toBeVisible();await expect(page.getByText('Setup needed',{exact:true})).toBeVisible();await page.screenshot({path:'out/rebuild-qa/integrations-compact-rows.png',animations:'disabled'});
});
test('pointer menus clamp all four corners and restore keyboard focus',async({page})=>{
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();const row=page.locator('.track-row').first();
 for(const [x,y]of [[0,0],[719,0],[0,479],[719,479]]){await row.dispatchEvent('contextmenu',{clientX:x,clientY:y,bubbles:true});const menu=page.getByRole('menu');await expect(menu).toBeVisible();await expect.poll(async()=>{const b=(await menu.boundingBox())!;return b.x>=0&&b.y>=0&&b.x+b.width<=720&&b.y+b.height<=480;}).toBe(true);await page.keyboard.press('Escape');await expect(menu).not.toBeVisible();await expect(row.getByRole('button',{name:'More actions',exact:true})).toBeFocused();}
 await row.getByRole('button',{name:'More actions',exact:true}).press('Shift+F10');await expect(page.getByRole('menu')).toBeVisible();await page.keyboard.press('Escape');
});
