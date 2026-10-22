import {test,expect} from '@playwright/test';
test('guest-heavy album headers remain compact and reveal clickable artist names',async({page})=>{
 test.setTimeout(60000);
 for(const width of [720,1280])for(const scale of [80,100,125,150]){
  await page.setViewportSize({width,height:width===720?480:800});
  await page.goto('/?fixture=1&contributors=1');
  await page.evaluate(value=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'Henry',interface_percent:value,coverflow_colors:true}));sessionStorage.removeItem('musicos-navigation-v1');},scale);await page.reload();
  await page.getByRole('button',{name:'Open Junk',exact:true}).click();
  const credits=page.locator('.album-artist-credits');await expect(credits).toHaveAttribute('data-compact','true');await expect(credits.locator('.album-primary-artist')).toHaveText('M83');await expect(credits).not.toContainText('Various artists');
  const portraits=credits.locator('.artist-link');await expect(portraits).toHaveCount(18);
  const first=portraits.first();await expect(first).toHaveAttribute('aria-label','View artist M83');
  await first.hover();await expect(first.locator('.artist-credit-name')).toHaveCSS('opacity','1');
  await portraits.nth(1).focus();await expect(portraits.nth(1).locator('.artist-credit-name')).toHaveCSS('opacity','1');
  const bounds=await credits.evaluate(el=>({height:el.getBoundingClientRect().height,rem:parseFloat(getComputedStyle(document.documentElement).fontSize),overflow:el.scrollWidth>el.clientWidth+1}));expect(bounds.overflow).toBe(false);expect(bounds.height/bounds.rem).toBeLessThan(10);
  const layout=await page.locator('.album-copy').evaluate(el=>{const credits=el.querySelector('.album-artist-credits')!.getBoundingClientRect(),quality=el.querySelector('.audio-quality')!.getBoundingClientRect(),actions=el.querySelector('.collection-actions')!.getBoundingClientRect();return {creditsBottom:credits.bottom,qualityTop:quality.top,qualityBottom:quality.bottom,actionsTop:actions.top};});expect(layout.qualityTop).toBeGreaterThanOrEqual(layout.creditsBottom);expect(layout.actionsTop).toBeGreaterThanOrEqual(layout.qualityBottom);
  const description=page.getByRole('region',{name:'About this album'});await expect(description).toBeVisible();
  await description.getByRole('button',{name:'Read more'}).click();await expect(description.locator('p')).toHaveAttribute('data-expanded','true');
  await description.getByRole('button',{name:'Read less'}).click();await expect(description.locator('p')).toHaveAttribute('data-expanded','false');
  if(scale===100){await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=0;});await page.screenshot({path:`out/rebuild-qa/compact-contributors-${width}.png`,animations:'disabled'});}
  await first.click();await expect(page.locator('.artist-detail')).toBeVisible();
 }
});
