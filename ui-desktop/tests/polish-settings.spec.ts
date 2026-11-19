import {expect,test} from '@playwright/test'; import {settleAnimations} from './support/ui';
test('quality shortcut opens analysis and audio settings sections stay separate',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Discovery',exact:true}).click();const quality=page.locator('.album-detail').getByRole('button',{name:/Audio quality:.*Open audio/});await quality.scrollIntoViewIfNeeded();await settleAnimations(page);await quality.click();
 await expect(page.getByRole('dialog',{name:'Audio controls'})).toBeVisible();await page.keyboard.press('Escape');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.getByRole('button',{name:'Quality & analysis',exact:true}).click();await expect(page.getByRole('heading',{name:'Lossless & lossy audio'})).toBeVisible();await expect(page.getByRole('heading',{name:'Audio analysis',exact:true})).toBeVisible();
 await expect(page.getByRole('heading',{name:'Listening volume'})).not.toBeVisible();
 await page.getByRole('button',{name:'Sparklist',exact:true}).click();await expect(page.getByRole('heading',{name:'Sparklist beta'})).toBeVisible();await expect(page.getByRole('heading',{name:'Audio analysis',exact:true})).not.toBeVisible();
 await page.getByRole('button',{name:'Playback',exact:true}).click();await expect(page.getByRole('heading',{name:'Listening volume'})).toBeVisible();
});
test('album and artist image viewer fit small and wide windows',async({page})=>{
 for(const width of [720,1280]){
  await page.setViewportSize({width,height:width===720?480:800});await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await page.getByRole('button',{name:'Expand M83 portrait'}).click();
  const image=page.getByRole('dialog').getByRole('img');await expect(image).toBeVisible();const box=(await image.boundingBox())!;expect(box.x).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(width);await expect(page.getByRole('dialog')).toHaveCSS('opacity','1');await page.screenshot({path:`out/rebuild-qa/polish-artwork-${width}.png`});await page.getByRole('button',{name:'Close artwork'}).click();await expect(page.getByRole('dialog')).not.toBeVisible();
 }
});
