import {expect,test} from '@playwright/test';

test('view and track changes animate independently and keep the transport stationary',async ({page})=>{
 await page.addInitScript(()=>{
  const calls:string[]=[];
  Object.assign(window,{phoneMotionCalls:calls});
  const animate=Element.prototype.animate;
  Element.prototype.animate=function(...args:Parameters<typeof animate>){
   const animation=animate.apply(this,args);
   queueMicrotask(()=>{if(animation.id)calls.push(animation.id);});
   return animation;
  };
 });
 const calls=()=>page.evaluate(()=>(window as unknown as {phoneMotionCalls:string[]}).phoneMotionCalls);
 await page.goto('/?fixture=1');
 const mini=page.locator('.mini-identify');
 await mini.click();
 const player=page.getByRole('dialog',{name:'Player'});
 const next=player.getByRole('button',{name:'Next song'});
 await expect.poll(()=>calls()).toContain('phone-player-entrance');
 await expect.poll(()=>player.evaluate(node=>node.getAnimations({subtree:true}).filter(a=>a.id.startsWith('phone-player')).length)).toBe(0);
 const transport=await next.boundingBox();
 await player.getByRole('button',{name:'Show Up next'}).click();
 await expect(player.getByRole('heading',{name:'Up next'})).toBeVisible();
 await expect.poll(async()=>((await next.boundingBox())?.y)).toBe(transport?.y);
 await player.getByRole('button',{name:'Show Lyrics'}).click();
 await player.getByRole('button',{name:'Show Now playing'}).click();
 await expect(player.getByRole('heading',{name:'Second light'})).toBeVisible();
 await expect.poll(()=>player.evaluate(node=>node.getAnimations({subtree:true}).filter(a=>a.id==='phone-player-view').length)).toBe(0);
 const viewCount=(await calls()).filter(id=>id==='phone-player-view').length;
 await next.click();
 await expect(player.getByRole('heading',{name:'Soft signal'})).toBeVisible();
 expect((await calls()).filter(id=>id==='phone-player-view')).toHaveLength(viewCount);
 expect(await calls()).toContain('phone-song-handoff');
 await player.getByRole('button',{name:'Close player'}).click();
 await expect(page.locator('.phone-player')).toHaveCount(0);
 await expect(mini).toBeFocused();
 expect(await calls()).toContain('phone-player-exit');
});

test('turning off animations also skips player glyph morphs', async ({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 await page.getByRole('switch',{name:/Animations/}).uncheck();
 await page.getByRole('button',{name:'Close Settings'}).click();
 await page.locator('.mini-identify').click();
 const player=page.getByRole('dialog',{name:'Player'});
 await expect(player).toBeVisible();
 expect(await player.evaluate(node=>node.getAnimations({subtree:true}).length)).toBe(0);
 await player.getByRole('button',{name:'Play',exact:true}).click();
 await expect(player.getByRole('button',{name:'Pause',exact:true})).toBeVisible();
 await expect(player.locator('[data-morphing]')).toHaveCount(0);
 await player.getByRole('button',{name:'Close player'}).click();
 await expect(page.locator('.phone-player')).toHaveCount(0);
});
