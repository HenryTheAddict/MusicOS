import {test,expect} from '@playwright/test';
test('Audio docks, preserves keyboard focus, and floating preference survives reload',async({page})=>{
 await page.goto('/?fixture=1');const opener=page.getByRole('button',{name:'Open Up next',exact:true});await opener.click();
 const panel=page.getByRole('complementary',{name:'Up next',exact:true});await expect(panel).toHaveClass(/audio-docked/);await expect(panel.getByRole('heading',{name:'Up next'})).toBeVisible();
 await page.keyboard.press('Escape');await expect(panel).toHaveCount(0);await expect(opener).toBeFocused();
 await opener.click();await page.keyboard.press('Escape');await page.getByRole('button',{name:'Choose section'}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.getByRole('button',{name:'Playback',exact:true}).click();await page.getByRole('button',{name:'Floating panel',exact:true}).click();await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')??'{}').audio_presentation)).toBe('floating');await page.reload();await opener.click();await expect(panel).toHaveClass(/audio-floating/);
 await page.locator('.library-sidebar').click({position:{x:10,y:12}});await expect(panel).toHaveCount(0);
});
test('pointer and keyboard queue moves preserve current occurrence, and internal dragging does not import',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const rows=page.locator('.queue-row'),names=await rows.locator('strong').allTextContents();
 const handle=rows.nth(1).getByRole('button',{name:`Play ${names[1]} from queue`,exact:true});const from=await handle.boundingBox(),to=await rows.first().boundingBox();
 await page.mouse.move(from!.x+from!.width/2,from!.y+from!.height/2);await page.mouse.down();await page.mouse.move(to!.x+10,to!.y+to!.height/2,{steps:8});await page.mouse.up();await expect(rows.first().locator('strong')).toHaveText(names[1]);await expect(rows.locator('.active')).toHaveCount(0);await expect(page.locator('.now-playing-info > strong')).toHaveText('Opening');await expect(page.getByText('Drop music or folders',{exact:true})).toHaveCount(0);
 await rows.first().getByRole('button',{name:`Play ${names[1]} from queue`,exact:true}).focus();await page.keyboard.press('Alt+ArrowDown');await expect(rows.first().locator('strong')).toHaveText(names[0]);await expect(rows.nth(1).getByRole('button',{name:`Play ${names[1]} from queue`,exact:true})).toBeFocused();
 await expect(page.locator('.queue-drag-handle')).toHaveCount(0);
 const before=await rows.locator('strong').allTextContents();const dragBox=await rows.first().locator('.queue-track').boundingBox();await page.mouse.move(dragBox!.x+20,dragBox!.y+20);await page.mouse.down();await page.mouse.move(dragBox!.x+20,dragBox!.y+100,{steps:5});await page.keyboard.press('Escape');await page.mouse.up();await expect(page.getByRole('complementary',{name:'Up next',exact:true})).toBeVisible();expect(await rows.locator('strong').allTextContents()).toEqual(before);

 const cover=await rows.first().locator('.cover').boundingBox(),copy=await rows.first().locator('.queue-track-copy').boundingBox();expect(copy!.x).toBeGreaterThan(cover!.x+cover!.width);expect(Math.abs(copy!.y-cover!.y)).toBeLessThan(12);await page.screenshot({animations:'disabled',path:'out/rebuild-qa/audio-menu-populated.png'});
});
test('dragging a repeated song moves only that queue entry; removing it preserves now playing',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const rows=page.locator('.queue-row');await expect(rows).toHaveCount(11);
 const duplicate=rows.filter({has:page.getByRole('button',{name:'Play Opening from queue',exact:true})});
 const source=await duplicate.locator('.cover').boundingBox(),destination=await rows.first().boundingBox();
 await page.mouse.move(source!.x+10,source!.y+10);await page.mouse.down();await page.mouse.move(destination!.x+20,destination!.y+10,{steps:8});await page.mouse.up();
 await expect(rows.first().locator('strong')).toHaveText('Opening');await expect(page.locator('.now-playing-info > strong')).toHaveText('Opening');
 await rows.first().getByRole('button',{name:'Remove Opening from queue',exact:true}).click();await expect(rows).toHaveCount(10);await expect(page.locator('.now-playing-info > strong')).toHaveText('Opening');
});
test('virtual queue drag autoscrolls and Escape cancels without activating file import',async({page})=>{
 await page.goto('/?fixture=1&large=1');await page.getByRole('button',{name:'Songs',exact:true}).click();await page.locator('.track-row').first().hover();await page.locator('.row-play').first().click();await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const scroll=page.locator('.queue-scroll'),row=page.locator('.queue-row').first(),name=await row.locator('strong').textContent();const from=await row.locator('.cover').boundingBox(),box=await scroll.boundingBox();
 await page.mouse.move(from!.x+10,from!.y+10);await page.mouse.down();await page.mouse.move(box!.x+30,box!.y+box!.height-5,{steps:12});
 await expect.poll(()=>scroll.evaluate(el=>el.scrollTop)).toBeGreaterThan(500);await page.keyboard.press('Escape');await page.mouse.up();
 await expect(page.getByRole('complementary',{name:'Up next',exact:true})).toBeVisible();await expect(page.getByText('Drop music or folders',{exact:true})).toHaveCount(0);
 await scroll.evaluate(el=>{el.scrollTop=0;});await expect(page.locator('.queue-row').first().locator('strong')).toHaveText(name!);
});
