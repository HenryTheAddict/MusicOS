import {test,expect} from '@playwright/test';
test('song menus enqueue repeated entries, queue context menu removes only one',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();
 const song=page.locator('.track-row').first();await song.click({button:'right'});
 await page.getByRole('menuitem',{name:'Play next',exact:true}).click();
 await song.click({button:'right'});await page.getByRole('menuitem',{name:'Add to Up next',exact:true}).click();
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const rows=page.locator('.queue-row');await expect(rows).toHaveCount(2);
 await rows.first().click({button:'right'});await page.getByRole('menuitem',{name:'Remove from Up next',exact:true}).click();await expect(rows).toHaveCount(1);
});
test('floating Audio stays open while a song is dragged in and still closes on outside click',async({page})=>{
 await page.goto('/?fixture=1');await page.evaluate(()=>{const key='musicos-fixture-preferences';localStorage.setItem(key,JSON.stringify({version:1,interface_percent:100,display_name:'Henry',coverflow_colors:false,...JSON.parse(localStorage.getItem(key)??'{}'),audio_presentation:'floating'}));});await page.reload();
 await page.getByRole('button',{name:'Songs',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const panel=page.locator('.audio-panel');await expect(panel).toHaveClass(/audio-floating/);
 const source=await page.locator('.track-label').first().boundingBox(),target=await panel.boundingBox();
 await page.mouse.move(source!.x+50,source!.y+8);await page.mouse.down();await expect(panel).toBeVisible();
 await page.mouse.move(target!.x+40,target!.y+target!.height-40,{steps:12});await expect(panel).toHaveAttribute('data-song-drop','true');await page.mouse.up();
 await expect(panel).toBeVisible();await expect(page.locator('.queue-row')).toHaveCount(1);
 expect(await page.evaluate(()=>window.getSelection()?.toString())).toBe('');await expect(page.getByText('Drop music or folders',{exact:true})).toHaveCount(0);
 await page.mouse.move(source!.x+50,source!.y+8);await page.mouse.down();await page.mouse.move(target!.x+40,target!.y+target!.height-40,{steps:8});await page.keyboard.press('Escape');await page.mouse.up();
 await expect(panel).toBeVisible();await expect(page.locator('.queue-row')).toHaveCount(1);
 await page.locator('.library-sidebar').click({position:{x:10,y:12}});await expect(panel).toHaveCount(0);
});
test('dropping a library song before a queue row honors the insertion marker',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();
 const songs=page.locator('.track-row');
 for(const index of [0,1]){await songs.nth(index).click({button:'right'});await page.getByRole('menuitem',{name:'Add to Up next',exact:true}).click();}
 const expected=await songs.nth(2).locator('.track-label > span').textContent();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const rows=page.locator('.queue-row');await expect(rows).toHaveCount(2);const first=await rows.first().locator('strong').textContent();
 const source=await songs.nth(2).locator('.track-label').boundingBox(),target=await rows.first().boundingBox();
 await page.mouse.move(source!.x+50,source!.y+8);await page.mouse.down();await page.mouse.move(target!.x+40,target!.y+4,{steps:12});
 await expect(rows.first()).toHaveAttribute('data-incoming-drop','true');await page.mouse.up();await expect(rows).toHaveCount(3);
 await expect(rows.first().locator('strong')).toHaveText(expected!);await expect(rows.nth(1).locator('strong')).toHaveText(first!);
});
test('pointer song drop adds to Audio, Escape cancels without file import',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const label=page.locator('.track-label').first(),panel=page.locator('.audio-panel');
 const from=await label.boundingBox(),to=await panel.boundingBox();
 await page.mouse.move(from!.x+85,from!.y+8);await page.mouse.down();await page.mouse.move(to!.x+40,to!.y+to!.height-40,{steps:12});
 await expect(panel).toHaveAttribute('data-song-drop','true');await page.mouse.up();await expect(page.locator('.queue-row')).toHaveCount(1);
 await page.mouse.move(from!.x+85,from!.y+8);await page.mouse.down();await page.mouse.move(to!.x+40,to!.y+to!.height-40,{steps:12});await page.keyboard.press('Escape');await page.mouse.up();
 await expect(page.locator('.queue-row')).toHaveCount(1);await expect(page.getByText('Drop music or folders',{exact:true})).toHaveCount(0);await expect(page.locator('.song-drag-preview')).toHaveCount(0);
});
