import {expect,test} from '@playwright/test';
import {openAlbums,settleAnimations} from './support/ui';
test('artist facts stay visible beside the portrait and Escape dismisses its artwork viewer',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();
 const portrait=page.getByRole('button',{name:'Expand M83 portrait',exact:true}),facts=page.locator('.artist-statistics');
 await expect(facts).toBeVisible();await portrait.hover();await expect(facts).toBeVisible();await portrait.click();await expect(page.getByRole('dialog')).toBeVisible();await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).not.toBeVisible();await expect(facts).toBeVisible();
 await portrait.focus();await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');await expect(portrait).toBeFocused();await portrait.press('Enter');await expect(page.getByRole('dialog')).toBeVisible();await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).not.toBeVisible();await expect(portrait).toBeFocused();
 const artwork=await portrait.boundingBox(),statistics=await facts.boundingBox();expect(statistics!.x>=artwork!.x+artwork!.width-1||statistics!.y>=artwork!.y+artwork!.height-1).toBe(true);
});
test('parallax neutralizes on hover and the soft spring settles without moving the caption',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);await settleAnimations(page);const card=page.getByRole('button',{name:'Open Plastic Beach',exact:true}),art=card.locator('.card-artwork');await expect(card).toBeVisible();
 await card.scrollIntoViewIfNeeded();await page.waitForTimeout(300);const caption=card.locator('.card-caption'),before=await caption.boundingBox();await card.hover();await page.waitForTimeout(190);
 expect(Math.abs(await art.evaluate(el=>parseFloat((el as HTMLElement).style.getPropertyValue('--parallax-y'))||0))).toBeLessThan(.2);
 expect((await caption.boundingBox())!.y).toBeCloseTo(before!.y,0);await page.waitForTimeout(260);await expect(art).not.toHaveCSS('scale',/1\.0[1-9]/);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Albums',exact:true}).hover();await card.hover();expect(await art.evaluate(el=>(el as HTMLElement).style.scale)).toBe('');
});
test('depth setup is optional and time switches stay independent at every interface size',async({page})=>{
 for(const width of [720,1280])for(const interface_percent of [80,100,125,150]){
  await page.setViewportSize({width,height:width===720?480:800});await page.goto('/?fixture=1');await page.evaluate(size=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:size})),interface_percent);await page.reload();
  await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Appearance',exact:true}).click();
  await expect(page.getByRole('switch',{name:'Depth artwork',exact:true})).not.toBeChecked();await expect(page.getByRole('switch',{name:'Show song durations',exact:true})).toBeChecked();await expect(page.getByRole('switch',{name:'Show Now Playing time',exact:true})).toBeChecked();
  const controls=await page.locator('.timing-tile .themed-switch').evaluateAll(elements=>elements.map(el=>{const r=el.getBoundingClientRect();return {top:r.top,bottom:r.bottom};}));expect(controls[1].top).toBeGreaterThanOrEqual(controls[0].bottom-1);
 }
});
