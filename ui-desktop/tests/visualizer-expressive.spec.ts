import {test,expect} from '@playwright/test';

test('real word timing drives a continuous fill with room for its glow',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed&lyricsCase=words');await page.getByRole('button',{name:'Open visualizer'}).click();await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');
 await page.mouse.move(300,120);const player=page.locator('.expanded-player');await expect(player.locator('.lyric-word')).not.toHaveCount(0);
 await player.getByRole('button',{name:'Play',exact:true}).click();const word=player.locator('.lyric-word[data-word-active=true]').first();await expect(word).toBeVisible();
 await expect.poll(async()=>parseFloat(await word.evaluate(el=>el.style.getPropertyValue('--word-progress')))).toBeGreaterThan(5);
 const padding=await player.locator('.lyrics-lines').evaluate(el=>parseFloat(getComputedStyle(el).paddingLeft));expect(padding).toBeGreaterThanOrEqual(32);
 expect(await word.locator('..').evaluate(el=>getComputedStyle(el).overflow)).toBe('visible');
 await player.getByRole('button',{name:'Pause',exact:true}).click();const stopped=await word.evaluate(el=>el.style.getPropertyValue('--word-progress'));await page.waitForTimeout(160);expect(await word.evaluate(el=>el.style.getPropertyValue('--word-progress'))).toBe(stopped);
 await page.screenshot({path:'out/expressive-player-word-glow.png'});
});

test('acknowledged song notices include bounded artwork and release their handoff',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();await page.mouse.move(300,120);await page.getByRole('button',{name:'Next track',exact:true}).click();
 const notice=page.locator('.song-change-notice');await expect(notice).toContainText('Now playing');await expect(notice).toContainText('The next song');await expect(notice.locator('img')).toBeVisible();
 const box=await notice.locator('img').boundingBox();expect(box!.width).toBe(44);expect(box!.height).toBe(44);await page.waitForTimeout(250);expect(await notice.evaluate(el=>el.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);
});

test('visualizer options stay compact in a short window',async({page})=>{
 await page.setViewportSize({width:720,height:480});await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();await page.mouse.move(300,120);await page.getByRole('button',{name:'Visualizer options',exact:true}).click();
 const dialog=page.getByRole('dialog',{name:'Visualizer',exact:true}),box=await dialog.boundingBox();expect(box!.width).toBeLessThanOrEqual(354);expect(box!.height).toBeLessThan(400);expect(box!.y).toBeGreaterThanOrEqual(0);await page.screenshot({path:'out/expressive-visualizer-options.png'});
});

test('description-free album controls align with the artwork bottom',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();
 const hero=page.locator('.album-hero');await expect(hero).toBeVisible();await expect(hero.locator('.album-description')).toHaveCount(0);
 await expect.poll(async()=>{const art=await hero.locator('.album-artwork-stage').boundingBox(),actions=await hero.locator('.album-footer').boundingBox();return Math.abs(art!.y+art!.height-actions!.y-actions!.height);}).toBeLessThan(2);await page.screenshot({path:'out/expressive-album-footer.png'});
});

test('welcome setup keeps footer and identity fields usable in a square window',async({page})=>{
 await page.setViewportSize({width:480,height:480});await page.goto('/?fixture=1');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'General',exact:true}).click();await page.getByRole('button',{name:'Open welcome setup',exact:true}).click();
 const setup=page.getByRole('dialog',{name:/musicOS|welcome/i});await page.getByRole('button',{name:'Make it mine',exact:true}).click();await expect(page.getByRole('textbox',{name:'What should we call you?'})).toBeVisible();await expect.poll(()=>page.locator('.onboarding-copy').evaluate(el=>el.getAnimations().length)).toBe(0);const footer=await page.locator('.onboarding-footer').boundingBox();expect(footer!.y+footer!.height).toBeLessThanOrEqual(480);expect(await page.locator('.onboarding').evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);await page.screenshot({path:'out/expressive-welcome-square.png'});
});
