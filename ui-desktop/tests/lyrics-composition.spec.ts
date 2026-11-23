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
 const cues=lines.locator('.lyric-line');
 const last=(await cues.filter({visible:true}).last().boundingBox())!;
 expect(last.y+last.height).toBeLessThanOrEqual(reading.y+reading.height);
 for(const reducedMotion of ['no-preference','reduce'] as const){
  await page.emulateMedia({reducedMotion});
  await expect(cues.nth(0)).toHaveCSS('filter','none');
  await expect(cues.nth(1)).toHaveCSS('filter','none');
  await expect(cues.nth(2)).toBeVisible();
  await expect(cues.nth(3)).toBeHidden();
  await expect(lyrics.getByRole('button',{name:/Carry the sound with you/})).toHaveCount(0);
  await expect(cues.nth(0)).toHaveCSS('opacity','0.63');
  await expect(cues.nth(2)).toHaveCSS('opacity','0.63');
 }
 await expect(cues.nth(3)).toHaveCSS('transition-property','none');

 await lines.focus();
 await page.keyboard.press('ArrowDown');
 await expect(follow).toBeVisible();
 await expect(lyrics.getByRole('button',{name:/Carry the sound with you/})).toBeVisible();
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
 await expect(cues.nth(3)).toBeHidden();
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
 await expect(inProgress).toHaveAttribute('data-weight-text','to');
 const style=await inProgress.evaluate(el=>{const s=getComputedStyle(el,'::after');return {fill:s.backgroundImage,weight:s.fontWeight,content:s.content};});
 expect(style.content).toMatch(/^"to"/);
 const {fill}=style;
 expect(fill).toContain('38%');
 expect(fill).toContain('62%');
 expect(style.weight).toBe('500');
 const weights=()=>words.evaluateAll(nodes=>nodes.map(el=>getComputedStyle(el,'::after').fontWeight));
 expect(await weights()).toEqual(['700','700','700','500','300']);
 const bounds=()=>words.evaluateAll(nodes=>nodes.map(el=>{
  const word=el.getBoundingClientRect(),cue=el.closest('.lyric-line')!.getBoundingClientRect();
  return [word.x-cue.x,word.y-cue.y,word.width,word.height];
 }));
 const before=await bounds();
 await page.emulateMedia({reducedMotion:'reduce'});
 await expect(inProgress).toHaveCSS('background-image','none');
 await expect.poll(weights).toEqual(['700','700','700','700','300']);
 expect(await inProgress.evaluate(el=>getComputedStyle(el,'::after').backgroundImage)).toBe('none');
 expect(await bounds()).toEqual(before);
 await expect(line).toHaveAttribute('aria-current','true');
});

test('line timed lyrics keep fixed word geometry as the next cue becomes bold',async({page})=>{
 await page.goto('/?fixture=1&lyrics=1');
 await page.getByRole('button',{name:'Open lyrics',exact:true}).click();
 const lyrics=page.getByRole('region',{name:'Lyrics',exact:true});
 const next=lyrics.getByRole('button',{name:/Let the evening unfold/});
 const token=next.locator('.lyric-line-ink').first();
 await expect.poll(()=>token.evaluate(el=>getComputedStyle(el,'::after').content)).toMatch(/^"Let"/);
 const weight=()=>token.evaluate(el=>getComputedStyle(el,'::after').fontWeight);
 const width=()=>token.evaluate(el=>el.getBoundingClientRect().width);
 await expect.poll(weight).toBe('300');
 const before=await width();
 expect(await token.evaluate(el=>getComputedStyle(el,'::after').transitionProperty)).toBe('font-weight');
 await next.click();
 await expect(next).toHaveAttribute('aria-current','true');
 await expect.poll(weight).toBe('700');
 expect(await width()).toBe(before);
 await page.emulateMedia({reducedMotion:'reduce'});
 const following=lyrics.getByRole('button',{name:/Carry the sound with you/});
 await following.click();
 await expect(following).toHaveAttribute('aria-current','true');
 expect(await following.locator('.lyric-line-ink').first().evaluate(el=>getComputedStyle(el,'::after').transitionProperty)).toBe('none');
 await expect.poll(weight).toBe('700');
});
