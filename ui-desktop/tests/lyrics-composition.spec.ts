import {expect,test} from '@playwright/test';

for(const width of [480,1280])test(`lyrics keep their reading space when follow is paused at ${width}px`,async({page})=>{
 await page.setViewportSize({width,height:width===480?480:800});
 await page.goto('/?fixture=1&lyrics=1&lyricsCase=words');
 await page.getByRole('button',{name:'Open lyrics',exact:true}).click();

 const player=page.getByRole('dialog',{name:'Music player'});
 const lyrics=player.getByRole('region',{name:'Lyrics',exact:true});
 const lines=lyrics.getByLabel('Song lyrics',{exact:true});
 const follow=lyrics.getByRole('button',{name:'Follow lyrics',exact:true});
 await expect(lyrics.getByText('Lyrics',{exact:true})).toHaveCount(0);
 await expect(lyrics.locator('.lyric-line[aria-current=true]')).toHaveCount(1);
 await expect(follow).toHaveCount(0);
 await expect.poll(()=>player.evaluate(el=>el.getAnimations().filter(animation=>animation.playState==='running').length)).toBe(0);
 const art=(await player.locator('.expanded-player-art').boundingBox())!;
 const reading=(await lines.boundingBox())!;
 const dock=await player.locator('.expanded-player-dock').boundingBox();
 if(width===480)expect(art.y+art.height).toBeLessThan(reading.y);
 else expect(art.x+art.width).toBeLessThan(reading.x);

 await lines.focus();
 await page.keyboard.press('ArrowDown');
 await expect(follow).toBeVisible();
 const action=(await follow.boundingBox())!;
 expect(action.height).toBeGreaterThanOrEqual(44);
 expect(action.y).toBeGreaterThanOrEqual(reading.y+reading.height);
 expect(await lines.boundingBox()).toEqual(reading);
 expect(await player.locator('.expanded-player-dock').boundingBox()).toEqual(dock);

 await follow.focus();
 await page.keyboard.press('Enter');
 await expect(lyrics).toHaveAttribute('data-following','true');
 await expect(follow).toHaveCount(0);
 await expect(lines).toBeFocused();
});

test('timed lyrics show sung, in-progress and upcoming words without changing their layout',async({page})=>{
 await page.route('**/src/fixtures.ts*',async route=>{
  const response=await route.fetch();
  const body=await response.text();
  const original='let player = { ...emptyPlayback };';
  expect(body).toContain(original);
  await route.fulfill({response,body:body.replace(original,'let player = { ...emptyPlayback, track: tracks[0], position: 6.75 };')});
 });
 await page.goto('/?fixture=1&lyricsCase=words');
 await page.getByRole('button',{name:'Open lyrics',exact:true}).click();
 const lyrics=page.getByRole('region',{name:'Lyrics',exact:true});
 const line=lyrics.locator('.lyric-line[aria-current=true]');
 await expect(line).toHaveText('A little closer to home');
 const words=line.locator('.lyric-word');
 await expect.poll(()=>words.evaluateAll(nodes=>nodes.map(node=>(node as HTMLElement).style.getPropertyValue('--word-progress')))).toEqual(['100%','100%','100%','50%','0%']);
 const inProgress=line.locator('.lyric-word[data-word-active=true]');
 await expect(inProgress).toHaveText('to');
 const fill=await inProgress.evaluate(el=>getComputedStyle(el).backgroundImage);
 expect(fill).toContain('42.5%');
 expect(fill).toContain('57.5%');
 await page.emulateMedia({reducedMotion:'reduce'});
 await expect(inProgress).toHaveCSS('background-image','none');
 await expect(line).toHaveAttribute('aria-current','true');
});
