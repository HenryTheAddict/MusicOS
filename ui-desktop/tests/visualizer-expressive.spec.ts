import {test,expect} from '@playwright/test';

test('real word timing drives a continuous fill with room for its glow',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed&lyricsCase=words');await page.getByRole('button',{name:'Open visualizer'}).click();await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');
 await page.mouse.move(300,120);const player=page.locator('.expanded-player');await expect(player.locator('.lyric-word')).not.toHaveCount(0);
 await player.getByRole('button',{name:'Play',exact:true}).click();const word=player.locator('.lyric-word[data-word-active=true]').first();await expect(word).toBeVisible();
 await expect.poll(async()=>parseFloat(await word.evaluate(el=>el.style.getPropertyValue('--word-progress')))).toBeGreaterThan(5);
 const layer=player.locator('.lyric-glow-layer');await expect(layer).toHaveAttribute('aria-hidden','true');expect(await layer.evaluate(el=>(el as HTMLElement).inert&&!el.closest('.lyrics-lines,.lyrics-pane'))).toBe(true);await expect(layer).toHaveCSS('overflow','visible');await expect(layer).toHaveCSS('pointer-events','none');
 expect(await word.locator('..').evaluate(el=>getComputedStyle(el).overflow)).toBe('visible');
 await player.getByRole('button',{name:'Pause',exact:true}).click();const stopped=await word.evaluate(el=>el.style.getPropertyValue('--word-progress'));await page.waitForTimeout(160);expect(await word.evaluate(el=>el.style.getPropertyValue('--word-progress'))).toBe(stopped);
 await page.screenshot({path:'out/expressive-player-word-glow.png'});
});

test('acknowledged song notices include bounded artwork and release their handoff',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();await page.mouse.move(300,120);await page.getByRole('button',{name:'Next track',exact:true}).click();
 const notice=page.locator('.song-change-notice');await expect(notice).toContainText('Now playing');await expect(notice).toContainText('The next song');await expect(notice.locator('img')).toBeVisible();
 const box=await notice.locator('img').boundingBox();expect(box!.width).toBeCloseTo(44,2);expect(box!.height).toBeCloseTo(44,2);await page.waitForTimeout(250);expect(await notice.evaluate(el=>el.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);
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
 const setup=page.getByRole('dialog',{name:'Appearance'});
 await expect(setup.getByRole('textbox',{name:/Your name/})).toBeInViewport();
 await expect(setup.getByRole('button',{name:'Continue',exact:true})).toBeInViewport();
 await setup.getByRole('button',{name:'Continue',exact:true}).click();
 await expect(page.getByRole('dialog',{name:'Music folders'}).getByRole('button',{name:'Open my library',exact:true})).toBeInViewport();
 expect(await page.locator('.onboarding').evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);
 await page.screenshot({path:'out/expressive-welcome-square.png'});
});


test('a real word end clears its highlight and glow during a vocal gap',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed&lyricsCase=wordgap');await page.getByRole('button',{name:'Open visualizer'}).click();await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');await page.mouse.move(300,120);
 const player=page.locator('.expanded-player');await expect(player.locator('.lyrics-lines .lyric-word')).not.toHaveCount(0);await player.getByRole('button',{name:'Play',exact:true}).click();
 await page.waitForTimeout(260);await expect(player.locator('.lyrics-lines .lyric-word[data-word-active=true]')).toHaveCount(0);
 await expect.poll(()=>player.locator('.lyric-glow-clone .lyric-word').evaluateAll(words=>words.some(word=>Number(getComputedStyle(word).opacity)>0))).toBe(false);
 await player.getByRole('button',{name:'Pause',exact:true}).click();await page.keyboard.press('Escape');await expect(page.locator('.lyric-glow-layer')).toHaveCount(0);
});

test('RTL glyphs and detached glow share exact wrapping, then clean up in Reduced Motion',async({page})=>{
 await page.setViewportSize({width:480,height:480});await page.goto('/visualizer-proof.html?lyrics=timed&lyricsCase=rtlwords');await page.getByRole('button',{name:'Open visualizer'}).click();await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');await page.mouse.move(300,120);
 const player=page.locator('.expanded-player');await expect(player.locator('.lyrics-lines .lyric-word')).not.toHaveCount(0);await player.getByRole('button',{name:'Play',exact:true}).click();await page.waitForTimeout(80);await player.getByRole('button',{name:'Pause',exact:true}).click();
 const line=player.locator('.lyrics-lines .lyric-line[data-current=true]'),clone=player.locator('.lyric-glow-clone');await expect(clone).toHaveCount(1);expect(await line.textContent()).toBe('مرحباً  بالعالم — هنا');expect(await clone.textContent()).toBe(await line.textContent());
 await expect.poll(()=>player.evaluate(el=>{const a=el.querySelector('.lyrics-lines .lyric-line[data-current=true]')!.getBoundingClientRect(),b=el.querySelector('.lyric-glow-clone')!.getBoundingClientRect();return Math.max(Math.abs(a.x-b.x),Math.abs(a.y-b.y),Math.abs(a.height-b.height));})).toBeLessThan(.15);
 expect(await line.evaluate(el=>getComputedStyle(el).fontWeight)).toBe('600');expect(await line.evaluate(el=>getComputedStyle(el).webkitTextStrokeWidth)).toBe('0px');
 const viewport=player.locator('.lyrics-lines');await viewport.evaluate(el=>{el.scrollTop+=25;});await page.waitForTimeout(50);const moved=await line.boundingBox(),glow=await clone.boundingBox();expect(Math.abs(moved!.y-glow!.y)).toBeLessThan(.15);
 await page.addStyleTag({content:':root[data-native-visualizer=true] body{background:#101820!important}.app-shell{visibility:hidden}'});await page.screenshot({path:'out/lyric-glow-rtl.png'});await page.emulateMedia({reducedMotion:'reduce'});await expect(clone).toHaveCount(0);await page.keyboard.press('Escape');await expect(page.locator('.lyric-glow-layer')).toHaveCount(0);
});


test('visualizer chrome picks up an artwork color without a gradient behind the lyrics',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');
 await page.getByRole('button',{name:'Open visualizer',exact:true}).click();
 await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');await page.mouse.move(300,120);
 const player=page.locator('.expanded-player[data-visualizer=true]');
 await expect(player).toHaveCSS('--visualizer-source','rgb(222, 167, 43)');
 await expect(player.locator('.expanded-player-dock')).toHaveCSS('background-image','none');
 await expect(player.locator('.lyrics-pane[data-variant=visualizer]')).toHaveCSS('background-image','none');
});
