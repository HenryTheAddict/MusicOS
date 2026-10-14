import {test,expect} from '@playwright/test';
test('collections live under personal heading, persist cover selection, and have separate shuffle',async({page})=>{
 await page.goto('/?fixture=1');await expect(page.getByRole('heading',{name:"Henry's stuff"})).toBeVisible();
 const nav=page.getByRole('navigation',{name:'Library',exact:true});
 await expect(nav.getByRole('button')).toHaveCount(4);await expect(nav.getByRole('button',{name:'Playlists',exact:true})).toHaveCount(0);
 await expect(page.getByRole('button',{name:'Add music',exact:true})).toHaveCount(0);
 await page.getByRole('button',{name:'Create playlist',exact:true}).click();
 const dialog=page.getByRole('dialog',{name:'Create playlist',exact:true});await expect(dialog).toBeVisible();
 await expect(dialog.locator('.cover-options button')).toHaveCount(32);await dialog.getByRole('textbox',{name:'Name',exact:true}).fill('Designed mix');
 await dialog.getByRole('button',{name:'Choose Lake cover',exact:true}).click();await dialog.getByRole('button',{name:'Create playlist',exact:true}).click();
 await expect(dialog).not.toBeVisible();await expect(page.getByRole('heading',{name:'Designed mix',exact:true})).toBeVisible();
 const row=page.locator('.personal-collection').filter({hasText:'Designed mix'});await expect(row.getByRole('button',{name:'Shuffle Designed mix',exact:true})).toBeVisible();await expect(row.locator('img')).toHaveAttribute('src',/Lake/);
 await page.reload();await expect(page.locator('.personal-collection').filter({hasText:'Designed mix'}).locator('img')).toHaveAttribute('src',/Lake/);
});
test('Sparklist creation uses seed search, six moods, automatic covers and an honest model gate',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Create Sparklist',exact:true}).click();
 const dialog=page.getByRole('dialog',{name:'Create Sparklist',exact:true});await expect(dialog).toBeVisible();
 await expect(dialog.getByRole('searchbox',{name:'Search Sparklist seed songs'})).not.toBeVisible();await dialog.getByRole('button',{name:'Choose a starting song'}).click();await expect(dialog.getByRole('searchbox',{name:'Search Sparklist seed songs'})).toBeVisible();await expect(dialog.locator('.spark-moods button')).toHaveCount(6);
 await expect(dialog.getByRole('status')).toContainText('Beta off');await dialog.getByText('Model, storage & details',{exact:true}).click();await expect(dialog.locator('.spark-setup details')).toContainText('Experimental candidate');await expect(dialog.getByRole('button',{name:'Preview Sparklist'})).toBeDisabled();
 await page.keyboard.press('Escape');await expect(dialog).not.toBeVisible();await expect(page.getByRole('button',{name:'Create Sparklist',exact:true})).toBeFocused();
});
test('designed creation stays inside compact windows at all interface sizes',async({page})=>{
 for(const percent of [80,100,125,150]){
  await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');
  await page.evaluate(value=>{const prefs={version:1,display_name:'Henry',interface_percent:value,coverflow_colors:false};localStorage.setItem('musicos-fixture-preferences',JSON.stringify(prefs));},percent);await page.reload();
  await page.getByRole('button',{name:'Create playlist',exact:true}).click();const dialog=page.getByRole('dialog',{name:'Create playlist',exact:true});await expect(dialog).toBeVisible();
  const box=await dialog.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.y).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(720);expect(box!.y+box!.height).toBeLessThanOrEqual(480);
  await page.keyboard.press('Escape');
 }
});
