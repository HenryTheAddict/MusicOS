import {test,expect} from '@playwright/test';
import {contrast,defaultAppearance,themePalette} from '../src/appearance';
import {cssColorHex} from '../src/colorValue';

test('bright artwork keeps restored color while original covers remain standard brightness',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode:'light',source:'artwork',accent:'#0064df',saturation:150,tint:40}})));
 await page.route('**/src/fixtures.ts*',async route=>{
  const response=await route.fetch(),body=await response.text(),marker='case "artwork_palette":';expect(body).toContain(marker);
  await route.fulfill({response,body:body.replace(marker,`${marker} return {background:'#ff0000',blob:'#ff0000',accent:'#ff0000',text:'#12202a'};`)});
 });
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 const expected=themePalette({...defaultAppearance,mode:'light',source:'artwork',saturation:150,tint:40},{background:'#ff0000',blob:'#ff0000',accent:'#ff0000',text:'#12202a'},false)!;
 await expect.poll(()=>page.locator('html').evaluate(el=>el.style.getPropertyValue('--surface-color'))).toBe(expected.background);
 await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');await page.locator('html').evaluate(el=>el.getAnimations().forEach(a=>a.finish()));
 const actual=await page.locator('html').evaluate(el=>{const s=getComputedStyle(el);return {surface:s.getPropertyValue('--surface-color').trim(),ink:s.getPropertyValue('--ink').trim(),accent:s.getPropertyValue('--accent').trim()};});
 expect(cssColorHex(actual.surface,'#000000')).toBe(expected.background);expect(contrast(expected.text,expected.background)).toBeGreaterThanOrEqual(4.8);expect(expected.accent).not.toBe(expected.background);
 await expect(page.locator('.album-artwork .cover').first()).toHaveCSS('filter','none');
 await page.screenshot({path:'out/rebuild-qa/matte-bright-artwork.png',animations:'disabled'});
});

test('standard brightness is inherited by ordinary artwork and floating panels',async({page})=>{
 await page.goto('/?fixture=1');
 expect(await page.evaluate(()=>CSS.supports('dynamic-range-limit','standard'))).toBe(true);
 for(const selector of ['html','.app-shell','.card-artwork img'])await expect(page.locator(selector).first()).toHaveCSS('dynamic-range-limit','standard');
 await page.getByRole('button',{name:'Songs',exact:true}).click();await page.locator('.track-row').first().click({button:'right'});
 await expect(page.locator('.dropdown')).toHaveCSS('dynamic-range-limit','standard');
});

test('collection hover feedback never raises brightness or enlarges its bounds',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'audio/1',name:'Soft mix',kind:'audio',track_ids:[1,2],artwork_choice:'auto/1'}])));
 await page.goto('/?fixture=1&sparkReady=1');const button=page.getByRole('button',{name:'Play Soft mix',exact:true}).first();await button.scrollIntoViewIfNeeded();await page.evaluate(()=>document.fonts.ready);const bounds=(await button.boundingBox())!;
 await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);await page.waitForTimeout(180);await expect(button).toHaveCSS('filter','none');expect(await button.boundingBox()).toEqual(bounds);expect(await button.locator('svg linearGradient').count()).toBeGreaterThan(0);
 await page.locator('.home-collection-open').click();
 const shuffle=page.getByRole('button',{name:'Shuffle collection',exact:true});await shuffle.hover();await expect(shuffle).toHaveCSS('filter','none');
});
