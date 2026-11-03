import {test,expect} from '@playwright/test';
test('presets, nested Escape, remembered lyrics and editor Paste have one owner',async({page})=>{
 await page.goto('/visualizer-proof.html?lyrics=timed');await page.getByRole('button',{name:'Open visualizer'}).click();
 const player=page.locator('.expanded-player');await expect(player).toHaveAttribute('data-chrome','hidden');
 expect(await page.evaluate(()=>(window as any).__musicosCalls?.filter((c:any)=>c.command==='lyrics')??[])).toHaveLength(0);
 await page.mouse.move(400,100);await page.getByRole('button',{name:'Choose visualizer'}).click();await page.getByRole('menuitemradio',{name:'carbonated',exact:true}).click();await expect(page.getByRole('button',{name:'Choose visualizer'})).toContainText('carbonated');
 await page.getByRole('button',{name:'Visualizer options',exact:true}).click();await expect(page.getByRole('dialog',{name:'Visualizer',exact:true})).toBeVisible();
 await page.keyboard.press('Escape');await expect(page.getByRole('dialog',{name:'Visualizer',exact:true})).toHaveCount(0);await expect(player).toBeVisible();
 await page.keyboard.press('Meta+b');await expect(player).toHaveAttribute('data-view','queue');await page.keyboard.press('Meta+b');await expect(player).toHaveAttribute('data-view','lyrics');await expect(player.locator('.lyric-line')).toHaveCount(2);
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
