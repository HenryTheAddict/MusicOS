import{test,expect}from'@playwright/test';
test.use({browserName:'webkit'});
test('full lyrics stay sharp and use contrast to show the current line',async({page})=>{
 await page.goto('/?fixture=1&lyrics=1&lyricsCase=words');
 await page.getByRole('button',{name:'Open lyrics',exact:true}).click();
 const pane=page.locator('.lyrics-pane[data-variant=full]');
 const lines=pane.locator('.lyric-line');
 const current=pane.locator('.lyric-line[data-current=true]').first();
 await expect(lines).toHaveCount(4);
 await page.mouse.move(10,10);
 await expect(current).toHaveCSS('opacity','1');
 await expect(current).toHaveCSS('font-weight','680');
 await expect(pane.locator('.lyric-line:not([data-current=true],[data-past=true])').first()).toHaveCSS('font-weight','400');
 expect(await current.evaluate(el=>getComputedStyle(el,'::before').content)).toBe('none');
 const surrounding=pane.locator('.lyric-line:not([data-current=true])');
 expect(await surrounding.evaluateAll(rows=>rows.every(row=>{
  const {filter,opacity}=getComputedStyle(row);
  return filter==='none'&&Number(opacity)<1;
 }))).toBe(true);
 await pane.locator('.lyrics-lines').hover();
 await page.mouse.wheel(0,100);
 await expect(pane).toHaveAttribute('data-following','false');
 await expect(surrounding.first()).toHaveCSS('opacity','1');
});
test('native titlebar selector stays beside lights at narrow widths and all scales',async({page})=>{
 for(const scale of[80,100,125,150]){await page.setViewportSize({width:1280,height:800});await page.goto('/?fixture=1&lyrics=1');await page.evaluate(s=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:s})),scale);await page.reload();await page.getByRole('button',{name:'Open lyrics',exact:true}).click();await page.setViewportSize({width:480,height:480});await page.locator('.expanded-player').evaluate(el=>{el.setAttribute('data-native-titlebar','true');(el as HTMLElement).style.setProperty('--player-titlebar-top','12px');});await page.mouse.move(200,20);await page.keyboard.press('Tab');await page.locator('.lyrics-fullscreen').focus();const header=page.locator('.expanded-player-header'),tabs=header.locator('[data-player-cycle]');const b=await header.boundingBox(),t=await tabs.boundingBox(),actions=await header.locator('.expanded-header-actions').boundingBox();expect(b!.x).toBeGreaterThanOrEqual(88);expect(Math.abs(t!.y+t!.height/2-b!.y-b!.height/2)).toBeLessThan(1);expect(t!.x+t!.width).toBeLessThanOrEqual(actions!.x+1);expect(actions!.x+actions!.width).toBeLessThanOrEqual(480);await page.screenshot({path:`out/lyric-blur-titlebar-2026-10-31/titlebar-${scale}.png`});}
});

test('idle narrow lyrics retain their own space beneath song identification',async({page})=>{await page.goto('/?fixture=1&lyrics=1');await page.getByRole('button',{name:'Open lyrics',exact:true}).click();await page.setViewportSize({width:480,height:480});await page.mouse.move(10,10);await expect(page.locator('.expanded-player')).toHaveAttribute('data-chrome','hidden',{timeout:4000});const song=await page.locator('.expanded-playing-view').boundingBox(),lyrics=await page.locator('.expanded-lyrics-view').boundingBox();expect(lyrics!.y).toBeGreaterThanOrEqual(song!.y+song!.height);});
