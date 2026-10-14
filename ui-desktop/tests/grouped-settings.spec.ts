import {test,expect} from '@playwright/test';
test('grouped Settings retain navigation, readable widths and independent persisted switches',async({page})=>{
 for(const size of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(size);await page.goto('/?fixture=1');
  await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
  await page.getByRole('tab',{name:'Appearance',exact:true}).click();
  const durations=page.getByRole('switch',{name:'Show song durations',exact:true});await durations.uncheck();
  await expect(page.getByRole('switch',{name:'Show Now Playing time',exact:true})).toBeChecked();await durations.check();
  await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.getByRole('button',{name:'Quality & analysis',exact:true}).click();
  await expect(page.getByRole('switch',{name:'Recheck every available song'})).not.toBeChecked();await page.getByRole('switch',{name:'Recheck every available song'}).check();
  const panel=page.locator('.settings-panel');expect(await panel.evaluate(el=>el.scrollWidth<=el.clientWidth+1)).toBe(true);
  await page.locator('.settings .page-heading').scrollIntoViewIfNeeded();
  await page.screenshot({path:`out/rebuild-qa/grouped-settings-${size.width}.png`,animations:'disabled'});
  await page.getByRole('tab',{name:'Appearance',exact:true}).click();await expect(page.getByRole('switch',{name:'Depth artwork',exact:true})).toBeVisible();
  await page.getByRole('tab',{name:'Appearance',exact:true}).focus();await page.keyboard.press('ArrowDown');await expect(page.getByRole('tab',{name:'Audio',exact:true})).toBeFocused();
 }
});
