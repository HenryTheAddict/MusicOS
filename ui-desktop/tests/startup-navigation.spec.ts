import {test,expect} from '@playwright/test';

test('deferred screens and dialogs open during playback without losing navigation',async({page})=>{
 const errors:string[]=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto('/?fixture=1&sparkReady=1');
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
 await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await page.getByRole('button',{name:'Choose section',exact:true}).click();
 await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Library',exact:true})).toBeVisible();
 await page.getByRole('tab',{name:'Library',exact:true}).click();
 await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Review',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Choose section',exact:true}).click();
 await page.getByRole('menuitem',{name:'Devices',exact:true}).click();
 await expect(page.locator('.devices')).toBeVisible();
 await page.getByRole('button',{name:'Choose section',exact:true}).click();
 await page.getByRole('menuitem',{name:'Library',exact:true}).click();
 await expect(page.locator('.section-name')).toHaveText('Library');await page.getByRole('button',{name:'musicOS actions',exact:true}).click();await page.getByRole('menuitem',{name:'Manage collections',exact:true}).click();
 await expect(page.getByRole('heading',{name:'Collections',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByRole('menuitem',{name:'New Sparklist',exact:true}).click();
 await expect(page.getByRole('slider',{name:'Sparklist length'})).toBeVisible();
 await page.keyboard.press('Escape');
 await page.keyboard.press('ControlOrMeta+k');
 await expect(page.getByRole('dialog',{name:'SuperCommand'})).toBeVisible();
 await page.keyboard.press('Escape');
 expect(errors).toEqual([]);
});
