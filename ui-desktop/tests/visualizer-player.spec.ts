import {test,expect} from '@playwright/test';
test('presets, nested Escape, remembered lyrics and editor Paste have one owner',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();
 const player=page.locator('.expanded-player');await expect(player).toHaveAttribute('data-chrome','hidden');
 expect(await page.evaluate(()=>(window as any).__musicosCalls?.filter((c:any)=>c.command==='lyrics')??[])).toHaveLength(0);
 await page.mouse.move(400,100);await page.getByRole('button',{name:'Choose visualizer'}).click();await page.getByRole('menuitemradio',{name:'carbonated',exact:true}).click();await expect(page.getByRole('button',{name:'Choose visualizer'})).toContainText('carbonated');
 await page.getByRole('button',{name:'Visualizer options',exact:true}).click();await expect(page.getByRole('dialog',{name:'Visualizer',exact:true})).toBeVisible();
 await page.keyboard.press('Escape');await expect(page.getByRole('dialog',{name:'Visualizer',exact:true})).toHaveCount(0);await expect(player).toBeVisible();
 await page.keyboard.press('Meta+b');await expect(player).toHaveAttribute('data-view','queue');await page.keyboard.press('Meta+b');await expect(player).toHaveAttribute('data-view','lyrics');await expect(player.locator('.lyric-line')).toHaveCount(4);await expect(player.locator('.lyric-line[data-in-window=true]')).toHaveCount(2);
 await page.keyboard.press('Escape');await expect(player).toHaveCount(0);const input=page.getByRole('textbox',{name:'Paste check'});await input.focus();await page.keyboard.press('Meta+v');await expect(player).toHaveCount(0);
});
for(const size of [{width:480,height:480},{width:720,height:480},{width:1280,height:800}])for(const scale of [80,100,125,150])test(`shared controls fit ${size.width}×${size.height} at ${scale}% across themes`,async({page})=>{
 await page.setViewportSize(size);
 for(const theme of ['light','dark','blue','extreme']){
  await page.goto(`/visualizer-proof.html?lyrics=timed&scale=${scale}&theme=${theme}`);await page.getByRole('button',{name:'Open visualizer'}).click();await page.mouse.move(200,100);await page.keyboard.press('Tab');
  const player=page.locator('.expanded-player'),seek=player.getByRole('slider',{name:'Playback position'});await seek.focus();await expect.poll(()=>player.evaluate(el=>el.getAnimations().length)).toBe(0);const boxes=[];
  for(let view=0;view<3;view++){
   if(view)await page.keyboard.press('Meta+b');
   await expect(player.getByRole('button',{name:'Next track',exact:true})).toBeVisible();
   const controls=await player.locator('.expanded-transport').boundingBox(),bar=await seek.boundingBox();expect(controls!.x).toBeGreaterThanOrEqual(0);expect(controls!.x+controls!.width).toBeLessThanOrEqual(size.width+1);expect(controls!.y+controls!.height).toBeLessThanOrEqual(size.height+1);expect(bar!.x+bar!.width).toBeLessThanOrEqual(size.width+1);boxes.push(controls);
   for(const button of await player.locator('.expanded-header-actions>button').all()){const b=await button.boundingBox();expect(b!.width).toBeGreaterThanOrEqual(43.99);expect(b!.height).toBeGreaterThanOrEqual(43.99);}
  }
  for(const box of boxes.slice(1)){expect(Math.abs(box!.x-boxes[0]!.x)).toBeLessThan(1);expect(Math.abs(box!.y-boxes[0]!.y)).toBeLessThan(1);}
  expect(await player.evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);
 }
});

test('lyrics remain mounted through cues and a skip reveals a finite stationary transport',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');
 const player=page.locator('.expanded-player'),pane=page.locator('.lyrics-pane[data-variant=visualizer]');
 await expect(pane.locator('.lyric-line')).toHaveCount(4);
 const first=pane.locator('[data-line="0"]'),same=await first.elementHandle();
 expect(await first.evaluate(el=>parseFloat(getComputedStyle(el).fontSize))).toBeGreaterThan(40);
 const seek=player.getByRole('slider',{name:'Playback position'});await seek.focus();await seek.press('End');
 await expect(pane.locator('[data-line="3"]')).toHaveAttribute('data-current','true');expect(await same!.evaluate(el=>el.isConnected)).toBe(true);
 await page.waitForTimeout(280);expect(await pane.locator('.lyrics-lines').evaluate(el=>el.scrollTop)).toBeGreaterThan(0);
 const before=await player.locator('.expanded-transport').boundingBox();await player.getByRole('button',{name:'Next track',exact:true}).click();
 await expect(player.locator('.song-change-notice')).toContainText('The next song');await expect(player.locator('.song-change-notice')).toHaveAttribute('data-visible','true');
 const after=await player.locator('.expanded-transport').boundingBox();expect(after).toEqual(before);
 await page.waitForTimeout(2450);await expect(player.locator('.song-change-notice')).toBeEmpty();
});

test('retained expanded player and its backdrop do not leak through native visualizer transparency',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed&retainedPlayer=1');
 await page.getByRole('slider',{name:'Playback position'}).focus();await page.keyboard.press('Meta+v');const visualizer=page.locator('[data-visualizer=true]'),covered=page.locator('.expanded-player:not([data-visualizer=true])');
 await expect(visualizer).toBeVisible();await expect(covered).toHaveAttribute('aria-hidden','true');
 await expect(covered).toHaveCSS('visibility','hidden');await expect(covered).toHaveCSS('opacity','0');
 expect(await covered.evaluate(el=>getComputedStyle(el,'::backdrop').backdropFilter)).toBe('none');
 await page.keyboard.press('Meta+v');await expect(visualizer).toHaveCount(0);await expect(covered).toBeVisible();
});


test('keyboard focus uses a compact lyric cue instead of outlining the viewport',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');
 const pane=page.locator('.lyrics-pane[data-variant=visualizer]'),scroll=pane.locator('.lyrics-lines');await expect(scroll).toBeVisible();await page.keyboard.press('Tab');await scroll.focus();
 await expect(pane).not.toHaveAttribute('tabindex','0');expect(await scroll.evaluate(el=>getComputedStyle(el).outlineStyle)).toBe('none');
 expect(await pane.evaluate(el=>getComputedStyle(el,'::after').content)).toContain('Lyrics');
 const font=await pane.locator('.lyric-line').first().evaluate(el=>parseFloat(getComputedStyle(el).fontSize));expect(font).toBeGreaterThan(40);expect(font).toBeLessThan(60);
 const selector=page.locator('[data-player-cycle]');await page.keyboard.press('Tab');expect((await selector.boundingBox())!.width).toBe(44);
});
