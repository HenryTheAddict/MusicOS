import {test,expect} from '@playwright/test';

test('iCloud folder, background setting, and explicit merge are reachable on iPhone',async({page})=>{
 await page.setViewportSize({width:390,height:700});
 await page.goto('/?fixture=cloud');
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 const settings=page.getByRole('dialog',{name:'Settings',exact:true});
 await settings.getByText('iCloud Drive',{exact:true}).click();
 await settings.getByRole('button',{name:'Choose iCloud Drive folder'}).click();
 await expect(settings.getByText('Music copies',{exact:true})).toBeVisible();
 await settings.getByRole('switch',{name:/Save in background/}).check();
 await expect(settings.getByRole('switch',{name:/Save in background/})).toBeChecked();
 await settings.getByRole('button',{name:'Save library now'}).click();
 await expect(settings.getByText('Saved to your Files folder.')).toBeVisible();
 await settings.getByRole('button',{name:'Find library copies'}).click();
 await settings.getByRole('button',{name:'Review',exact:true}).click();
 const review=settings.getByRole('region',{name:'Review iCloud library copy'});
 await expect(review).toContainText('Studio iPhone');
 await review.getByRole('button',{name:'Merge library copy'}).click();
 await expect(settings.getByText('Library copy merged. Songs are available offline.')).toBeVisible();
 await settings.getByRole('button',{name:'Disconnect folder'}).click();
 await expect(settings.getByRole('button',{name:'Choose iCloud Drive folder'})).toBeVisible();
 expect(await settings.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);
});
