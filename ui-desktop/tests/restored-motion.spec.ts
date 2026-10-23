import {test,expect} from '@playwright/test';
test('row Play reveal slides only title and credits, supports focus and reduced motion',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();
 const row=page.locator('.track-body > .track-row').first(),label=row.locator('.track-label'),actions=row.locator('.track-actions');
 await page.getByRole('button',{name:'Songs',exact:true}).hover();const before=await label.boundingBox(),fixed=await actions.boundingBox(),height=(await row.boundingBox())!.height;
 await row.hover();await expect(label).toHaveCSS('transform',/matrix\(1, 0, 0, 1, [1-9]/);
 await expect.poll(async()=>(await label.boundingBox())!.x-before!.x).toBeGreaterThan(15);
 expect((await actions.boundingBox())!.x).toBe(fixed!.x);expect((await row.boundingBox())!.height).toBe(height);
 await page.getByRole('button',{name:'Songs',exact:true}).hover();await page.keyboard.press('Tab');await row.locator('.row-play').focus();await expect(row.locator('.row-play')).toHaveCSS('opacity','1');
 await page.emulateMedia({reducedMotion:'reduce'});await expect(label).toHaveCSS('transition-duration','0s');
});
test('song and now-playing time preferences survive restart',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 const songs=page.getByRole('switch',{name:'Show song durations'}),playing=page.getByRole('switch',{name:'Show Now Playing time'});
 await expect(songs).toBeChecked();await expect(playing).toBeChecked();await songs.uncheck();await playing.uncheck();
 await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')!).show_now_playing_time)).toBe(false);
 await page.reload();await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect(songs).not.toBeChecked();await expect(playing).not.toBeChecked();
});
test('contributor names stay visible and the complete artist menu restores focus',async({page})=>{
 await page.goto('/?fixture=1&contributors=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();
 const artist=page.locator('.album-collaborator').first();await expect(artist.locator('.artist-credit-name')).toBeVisible();await artist.hover();await expect(page.locator('.artist-name-popover')).toHaveCount(0);
 const all=page.getByRole('button',{name:'All 17 contributing artists'});await all.click();await expect(page.getByRole('menuitem',{name:'Guest artist 17',exact:true})).toBeVisible();await page.keyboard.press('Escape');await expect(all).toBeFocused();
});
test('compact rows and playback times fit at both window sizes and all interface sizes',async({page})=>{
 for(const width of [720,1280])for(const size of [80,100,125,150]){
  await page.setViewportSize({width,height:width===720?480:800});
  await page.goto('/?fixture=1');await page.evaluate(size=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:size})),size);await page.reload();
  await page.getByRole('button',{name:'Songs',exact:true}).click();
  const row=page.locator('.track-row').first();await row.hover();await row.locator('.row-play').click();
  const index=row.locator('.track-index > span').first();await expect(index).toHaveText('1');await expect(row.locator('.track-index')).toHaveCSS('opacity','1');
  const rowBox=(await row.boundingBox())!,font=await page.locator('.app-shell').evaluate(el=>parseFloat(getComputedStyle(el).fontSize));expect(rowBox.height/font).toBeLessThanOrEqual(2.76);
  const bar=(await page.locator('.now-playing').boundingBox())!,title=(await page.locator('.now-playing-info strong').boundingBox())!,time=(await page.locator('.playback-time').boundingBox())!;
  expect(title.y).toBeGreaterThanOrEqual(bar.y);expect(time.y+time.height).toBeLessThanOrEqual(bar.y+bar.height);
 }
 await page.screenshot({path:'out/rebuild-qa/compact-song-rows.png',animations:'disabled'});
});
test('current-row Play morphs into levels and back on pause without hiding the number',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();
 const row=page.locator('.track-row').first(),control=row.locator('.row-play'),label=row.locator('.track-label');
 await row.hover();await control.click();
 await expect(control).toHaveAttribute('data-meter','true');
 await expect(control.locator('[data-icon=rowLevels] rect')).toHaveCount(3);
 await expect(row.locator('.track-index')).toHaveCSS('opacity','1');
 const height=(await row.boundingBox())!.height,x=(await label.boundingBox())!.x;
 await control.click();await expect(control).toHaveAttribute('data-meter','false');await expect(control.locator('[data-icon=play]')).toHaveCount(1);
 await expect(control.locator('[data-morphing=true]')).toHaveCount(0);
 expect((await row.boundingBox())!.height).toBe(height);expect((await label.boundingBox())!.x).toBe(x);
 await page.emulateMedia({reducedMotion:'reduce'});await control.click();await expect(control.locator('[data-icon=rowLevels] rect')).toHaveCount(3);await expect(control.locator('[data-morphing=true]')).toHaveCount(0);
});

test('album durations stay on the song row and compact About stays beside header controls',async({page})=>{
 test.setTimeout(60000);
 for(const size of [{width:720,height:480},{width:1280,height:800}])for(const percent of [80,100,125,150]){
  await page.setViewportSize(size);await page.goto('/?fixture=1&contributors=1');
  await page.evaluate(percent=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,show_song_durations:true}));sessionStorage.removeItem('musicos-navigation-v1');},percent);await page.reload();
  await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();
  const description=page.getByRole('region',{name:'About this album'});await expect(description).toBeVisible();
  const header=page.locator('.album-hero');const a=(await header.boundingBox())!,b=(await description.boundingBox())!;expect(b.y).toBeGreaterThanOrEqual(a.y);expect(b.y+b.height).toBeLessThanOrEqual(a.y+a.height+1);await expect(page.locator('.album-footer>.album-description')).toBeVisible();
  const row=page.locator('.track-row').first();await row.scrollIntoViewIfNeeded();await expect(row.locator('.time')).toBeVisible();
  const box=(await row.boundingBox())!,time=(await row.locator('.time').boundingBox())!,actions=(await row.locator('.track-actions').boundingBox())!;expect(time.y).toBeGreaterThanOrEqual(box.y);expect(time.y+time.height).toBeLessThanOrEqual(box.y+box.height+1);expect(time.x+time.width).toBeLessThanOrEqual(actions.x+1);
  if(percent===100)await page.screenshot({path:`out/rebuild-qa/album-duration-${size.width}.png`,animations:'disabled'});
 }
});
