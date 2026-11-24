import {test,expect} from '@playwright/test';

test('Show cover also controls artwork in the visualizer lyrics view',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');
 await page.getByRole('button',{name:'Open visualizer'}).click();
 const player=page.getByRole('dialog',{name:'Music player'}),cover=player.locator('.expanded-player-art img.cover');
 await expect(cover).toBeInViewport();
 await page.keyboard.press('Meta+b');await page.keyboard.press('Meta+b');
 await expect(player).toHaveAttribute('data-view','lyrics');
 await expect(cover).toBeInViewport();
 await expect(player.locator('.lyrics-lines')).toBeInViewport();
 for(const shown of [false,true]){
  await page.mouse.move(400,100);
  await player.getByRole('button',{name:'Visualizer options'}).click();
  const options=page.getByRole('dialog',{name:'Visualizer',exact:true});
  await options.getByRole('switch',{name:'Show cover'}).setChecked(shown);
  await options.getByRole('button',{name:'Close visualizer options'}).click();
  if(shown)await expect(cover).toBeInViewport();else await expect(cover).toBeHidden();
 }
});

test('exiting fullscreen leaves the native preset open and usable',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');
 await page.getByRole('button',{name:'Open visualizer'}).click();
 await page.mouse.move(400,100);
 const player=page.getByRole('dialog',{name:'Music player'});
 await player.getByRole('button',{name:'Exit full screen'}).click();
 await expect(player).toBeVisible();
 await expect(page.locator('html')).toHaveAttribute('data-native-visualizer','true');
 await expect(player.getByRole('button',{name:'Choose visualizer'})).toContainText('boxes');
 await player.getByRole('button',{name:'Play',exact:true}).click();
 await expect(player.getByRole('button',{name:'Pause',exact:true})).toBeVisible();
 await player.getByRole('button',{name:'Full screen',exact:true}).click();
 await expect(player.getByRole('button',{name:'Exit full screen'})).toBeVisible();
});

test('windowed visualizer controls fit and work at the Compact Mini Player minimum',async({page})=>{
 await page.addInitScript(()=>Object.defineProperty(navigator,'platform',{value:'MacIntel'}));
 await page.goto('/visualizer-proof.html?lyrics=timed');
 await page.getByRole('button',{name:'Open visualizer'}).click();
 await page.mouse.move(400,100);
 const player=page.getByRole('dialog',{name:'Music player'});
 await player.getByRole('button',{name:'Exit full screen'}).click();
 await page.setViewportSize({width:220,height:120});
 await page.mouse.move(110,60);
 await expect(player).toHaveAttribute('data-native-titlebar','true');
 const headerButtons=player.locator('.expanded-player-header button');
 for(const view of ['playing','lyrics']){
  if(view==='lyrics'){
   await player.getByRole('button',{name:'Pause',exact:true}).click();
   await player.getByRole('button',{name:'Open Up next'}).click();
   await player.getByRole('button',{name:'Show Lyrics'}).click();
  }
  await expect(player).toHaveAttribute('data-view',view);
  await expect(headerButtons).toHaveCount(6);
  for(const button of await headerButtons.all())await expect(button).toBeInViewport({ratio:1});
  for(const button of await headerButtons.all())await button.click({trial:true});
  await player.getByRole('button',{name:'Play',exact:true}).click();
  await expect(player.getByRole('button',{name:'Pause',exact:true})).toBeVisible();
  await player.getByRole('slider',{name:'Playback position'}).click();
  await expect(page.locator('#shortcut-result')).toHaveText('seek');
 }
 await player.getByRole('button',{name:'Close player'}).click();
 await expect(player).toHaveCount(0);
});

test('Space pauses after a pointer click on Next but activates keyboard-focused Next',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');
 await page.getByRole('button',{name:'Open visualizer'}).click();
 await page.mouse.move(400,100);
 const player=page.getByRole('dialog',{name:'Music player'});
 const play=player.getByRole('button',{name:'Play',exact:true});
 const pause=player.getByRole('button',{name:'Pause',exact:true});
 const next=player.getByRole('button',{name:'Next track',exact:true});
 const action=page.locator('#shortcut-result');
 await play.click();
 await expect(pause).toBeVisible();
 await next.click();
 await expect(action).toHaveText('next');
 await expect(next).toBeFocused();
 await page.keyboard.press('Space');
 await expect(action).toHaveText('toggle');
 await expect(play).toBeVisible();
 await play.focus();
 await page.keyboard.press('Tab');
 await expect(next).toBeFocused();
 await page.keyboard.press('Space');
 await expect(action).toHaveText('next');
 await expect(play).toBeVisible();
});

test('compact renderer errors leave the header, playback and Retry clickable',async({page})=>{
 await page.addInitScript(()=>Object.defineProperty(navigator,'platform',{value:'MacIntel'}));
 await page.goto('/visualizer-proof.html?lyrics=timed&failRenderer');
 await page.getByRole('button',{name:'Open visualizer'}).click();
 await page.mouse.move(400,100);
 const player=page.getByRole('dialog',{name:'Music player'});
 const message='The native visualizer is currently available on macOS desktop.';
 const error=player.getByRole('alert',{name:message});
 await expect(error).toBeVisible();
 await expect(error).toHaveAttribute('title',message);
 await player.getByRole('button',{name:'Exit full screen'}).click();
 await page.setViewportSize({width:220,height:120});
 await page.mouse.move(110,60);
 const withinStage=()=>error.evaluate(card=>{
  const player=card.closest('.expanded-player')!;
  const header=player.querySelector('.expanded-player-header')!.getBoundingClientRect();
  const dock=player.querySelector('.expanded-player-dock')!.getBoundingClientRect();
  const strip=card.getBoundingClientRect();
  return strip.top>=header.bottom&&strip.bottom<=dock.top;
 });
 await expect.poll(withinStage).toBe(true);
 const headerButtons=player.locator('.expanded-player-header button');
 await expect(headerButtons).toHaveCount(6);
 for(const button of await headerButtons.all()){
  await expect(button).toBeInViewport({ratio:1});
  await button.click({trial:true});
 }
 await player.getByRole('button',{name:'Choose visualizer'}).click();
 const menu=player.getByRole('menu');
 await expect(menu).toBeVisible();
 await page.keyboard.press('Escape');
 await expect(menu).toBeHidden();
 await expect(player.getByRole('button',{name:'Choose visualizer'})).toBeFocused();
 const details=error.getByRole('button',{name:'Details'});
 const close=player.getByRole('button',{name:'Close player'});
 const fullMessage=error.getByRole('region',{name:'Visualizer error details'});
 await expect(details).toHaveAttribute('aria-expanded','false');
 await expect(fullMessage).toBeHidden();
 await close.focus();
 await page.keyboard.press('Tab');
 await expect(details).toBeFocused();
 await page.keyboard.press('Enter');
 await expect(details).toHaveAttribute('aria-expanded','true');
 await expect(fullMessage).toBeVisible();
 await expect(fullMessage).toHaveText(message);
 await expect.poll(withinStage).toBe(true);
 await page.keyboard.press('Space');
 await expect(details).toHaveAttribute('aria-expanded','false');
 await page.keyboard.press('Space');
 await expect(details).toHaveAttribute('aria-expanded','true');
 await expect(page.locator('#shortcut-result')).toHaveText('No playback commands');
 await page.keyboard.press('Tab');
 await expect(fullMessage).toBeFocused();
 const messageScroll=()=>fullMessage.evaluate(node=>node.scrollTop);
 const beforeSpace=await messageScroll();
 await page.keyboard.press('Space');
 await expect.poll(messageScroll).toBeGreaterThan(beforeSpace);
 await expect(page.locator('#shortcut-result')).toHaveText('No playback commands');
 await expect(player.getByRole('button',{name:'Play',exact:true})).toBeVisible();
 await page.keyboard.press('End');
 await expect.poll(()=>fullMessage.evaluate(node=>node.scrollTop)).toBeGreaterThan(0);
 await details.click();
 await expect(details).toHaveAttribute('aria-expanded','false');
 await details.click();
 await expect(details).toHaveAttribute('aria-expanded','true');
 await details.focus();
 await page.keyboard.press('Space');
 await expect(details).toHaveAttribute('aria-expanded','false');
 await expect(page.locator('#shortcut-result')).toHaveText('No playback commands');
 await details.click();
 await player.getByRole('button',{name:'Play',exact:true}).click();
 await expect(player.getByRole('button',{name:'Pause',exact:true})).toBeVisible();
 await player.getByRole('slider',{name:'Playback position'}).click();
 await expect(page.locator('#shortcut-result')).toHaveText('seek');
 await close.click({trial:true});
 await expect(error).toBeVisible();
 await error.getByRole('button',{name:'Retry'}).click();
 await expect(error).toHaveCount(0);
 await expect(player).toBeVisible();
 await expect(player.getByRole('button',{name:'Full screen',exact:true})).toBeVisible();
});

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
   // Fixed hit areas must contain and center the glyphs at every interface scale.
   const geometry=await player.locator('.expanded-transport .icon-button').evaluateAll(buttons=>buttons.map(button=>{const b=button.getBoundingClientRect(),g=button.querySelector('.icon')!.getBoundingClientRect();return {dx:g.x+g.width/2-b.x-b.width/2,dy:g.y+g.height/2-b.y-b.height/2,inset:Math.min(g.x-b.x,g.y-b.y,b.right-g.right,b.bottom-g.bottom)};}));
   for(const g of geometry){expect(Math.abs(g.dx)).toBeLessThan(.1);expect(Math.abs(g.dy)).toBeLessThan(.1);expect(g.inset).toBeGreaterThanOrEqual(5.99);}
   const dock=await player.locator('.expanded-player-dock').boundingBox();expect(controls!.y+controls!.height).toBeLessThanOrEqual(dock!.y+dock!.height-7.99);
  }
  for(const box of boxes.slice(1)){expect(Math.abs(box!.x-boxes[0]!.x)).toBeLessThan(1);expect(Math.abs(box!.y-boxes[0]!.y)).toBeLessThan(1);}
  expect(await player.evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);
 }
});

test('Play and Pause keep the same centered footprint through the morph',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed&scale=150');await page.getByRole('button',{name:'Open visualizer'}).click();await page.mouse.move(400,100);await page.getByRole('slider',{name:'Playback position'}).focus();
 await expect.poll(()=>page.locator('.expanded-player').evaluate(el=>el.getAnimations().length)).toBe(0);
 const control=page.locator('.expanded-primary-play>.icon-button'),glyph=control.locator('.icon'),before=await control.boundingBox(),icon=await glyph.boundingBox();
 await control.click();await expect(control).toHaveAttribute('aria-label','Pause');await expect(glyph).toHaveAttribute('data-morphing','true');expect(await control.boundingBox()).toEqual(before);expect(await glyph.boundingBox()).toEqual(icon);
 await control.click();await expect(control).toHaveAttribute('aria-label','Play');await expect(glyph).not.toHaveAttribute('data-morphing','true');expect(await control.boundingBox()).toEqual(before);expect(await glyph.boundingBox()).toEqual(icon);
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
