import {expect,test} from '@playwright/test';

for(const width of [480,1280])test(`lyrics keep their reading space when follow is paused at ${width}px`,async({page})=>{
 await page.setViewportSize({width,height:width===480?480:800});
 await page.goto('/?fixture=1&lyrics=1&lyricsCase=words');
 await page.getByRole('button',{name:'Open lyrics',exact:true}).click();

 const player=page.getByRole('dialog',{name:'Music player'});
 const lyrics=player.getByRole('region',{name:'Lyrics',exact:true});
 const heading=lyrics.getByRole('heading',{name:'Lyrics',exact:true});
 const lines=lyrics.getByLabel('Song lyrics',{exact:true});
 const follow=lyrics.getByRole('button',{name:'Follow lyrics',exact:true});
 await expect(heading).toBeVisible();
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
 expect(action.y+action.height).toBeLessThanOrEqual(reading.y);
 expect(await lines.boundingBox()).toEqual(reading);
 expect(await player.locator('.expanded-player-dock').boundingBox()).toEqual(dock);

 await follow.focus();
 await page.keyboard.press('Enter');
 await expect(lyrics).toHaveAttribute('data-following','true');
 await expect(follow).toHaveCount(0);
 await expect(lines).toBeFocused();
});
