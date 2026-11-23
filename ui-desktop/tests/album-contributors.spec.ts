import {test,expect} from '@playwright/test';
test('guest-heavy album headers remain compact and reveal clickable artist names',async({page})=>{
 test.setTimeout(60000);
 for(const width of [720,1280])for(const scale of [80,100,125,150]){
  await page.setViewportSize({width,height:width===720?480:800});
  await page.goto('/?fixture=1&contributors=1');
  await page.evaluate(value=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'Henry',interface_percent:value,coverflow_colors:true}));sessionStorage.removeItem('musicos-navigation-v1');},scale);await page.reload();
  await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Teacup Parade',exact:true}).click();
  const credits=page.locator('.album-artist-credits');await expect(credits).toHaveCSS('display','flex');await expect(credits.locator('.album-primary-artist')).toHaveText('Moonspindle');await expect(credits).not.toContainText('Various artists');
  const portraits=credits.locator('.artist-link');await expect(portraits).toHaveCount(3);
  const more=credits.getByRole('button',{name:'15 more contributing artists',exact:true});await more.click();await expect(page.getByRole('menuitem')).toHaveCount(15);await page.keyboard.press('Escape');await expect(more).toBeFocused();
  const first=portraits.first();await expect(first).toHaveAttribute('aria-label','View artist Moonspindle');
  await first.hover();await expect(first.locator('.artist-credit-name')).toHaveCSS('opacity','1');
  await portraits.nth(1).focus();await expect(portraits.nth(1).locator('.artist-credit-name')).toHaveCSS('opacity','1');
  const bounds=await credits.evaluate(el=>({height:el.getBoundingClientRect().height,rem:parseFloat(getComputedStyle(document.documentElement).fontSize),overflow:el.scrollWidth>el.clientWidth+1}));expect(bounds.overflow).toBe(false);expect(bounds.height/bounds.rem).toBeLessThan(10);
  const layout=await page.locator('.album-copy').evaluate(el=>{const credits=el.querySelector('.album-artist-credits')!.getBoundingClientRect(),quality=el.querySelector('.audio-quality')!.getBoundingClientRect(),actions=el.querySelector('.collection-actions')!.getBoundingClientRect();return {creditsBottom:credits.bottom,qualityTop:quality.top,qualityBottom:quality.bottom,actionsTop:actions.top};});expect(layout.qualityTop).toBeGreaterThanOrEqual(layout.creditsBottom);expect(layout.actionsTop).toBeGreaterThanOrEqual(layout.qualityBottom);
  const description=page.getByRole('region',{name:'About this album'});await expect(description).toBeVisible();
  const about=page.getByRole('dialog',{name:'About this album',exact:true});const preview=description.locator(':scope > p');const text=await preview.textContent();
  await description.getByRole('button',{name:'Read more'}).click();await expect(about).toBeVisible();await expect(about.locator('p')).toHaveText(text!);
  await about.getByRole('button',{name:'Close album description',exact:true}).click();await expect(about).toHaveCount(0);await expect(preview).toHaveAttribute('data-expanded','false');
  if(scale===100){await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=0;});await page.screenshot({path:`out/rebuild-qa/compact-contributors-${width}.png`,animations:'disabled'});}
  await first.click();await expect(page.locator('.artist-detail')).toBeVisible();
 }
});
