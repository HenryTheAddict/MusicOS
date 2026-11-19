import {test,expect,type Page} from '@playwright/test';
import {openSparklistCreator} from './support/ui';
async function openCreator(page:Page){await openSparklistCreator(page);return page.getByRole('dialog',{name:'Create Sparklist',exact:true});}
test('Sparklist creation, preview, and explicit save retain their workflow',async({page})=>{await page.goto('/?fixture=1&sparkReady=1');const dialog=await openCreator(page);await expect(dialog.getByRole('heading',{name:'Create Sparklist',exact:true})).toBeVisible();await page.screenshot({path:'out/sparklist-after-start.png',animations:'disabled'});await dialog.getByRole('button',{name:'Choose for me',exact:true}).click();await expect(dialog.getByRole('button',{name:'Listen to preview',exact:true})).toBeVisible();await page.screenshot({path:'out/sparklist-after-preview.png',animations:'disabled'});await dialog.getByRole('button',{name:'Name & save',exact:true}).click();await dialog.getByRole('textbox',{name:'Name',exact:true}).fill('Evening mixtape');await page.screenshot({path:'out/sparklist-after-save.png',animations:'disabled'});expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback').length)).toBe(0);await dialog.getByRole('button',{name:'Save Sparklist',exact:true}).click();await expect(dialog).not.toBeVisible();await expect(page.getByRole('heading',{name:'Evening mixtape',exact:true})).toBeVisible();await page.screenshot({path:'out/sparklist-after-detail.png',animations:'disabled'});});

test('Sparklist edits and destructive actions stay explicit',async({page})=>{
 await page.goto('/?fixture=1&sparkReady=1');const dialog=await openCreator(page);await dialog.getByRole('button',{name:'Choose for me',exact:true}).click();
 await dialog.getByRole('button',{name:/^Refine /}).first().click();await page.getByRole('menuitem',{name:'Less like this',exact:true}).click();
 await expect.poll(()=>page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='sparklist'&&c.args.request.action==='preview').length)).toBe(2);
 await dialog.getByRole('button',{name:'Name & save',exact:true}).click();const name=dialog.getByRole('textbox',{name:'Name',exact:true});await expect(name).toBeFocused();await name.fill('After hours');
 await dialog.getByRole('button',{name:'Back to preview',exact:true}).click();await dialog.getByRole('button',{name:'Name & save',exact:true}).click();await expect(name).toHaveValue('After hours');await dialog.getByRole('button',{name:'Save Sparklist',exact:true}).click();
 const detail=page.locator('.audio-spark-detail');await detail.getByRole('button',{name:'Edit details',exact:true}).click();const editor=detail.getByRole('region',{name:'Sparklist details'});const nameBox=(await editor.getByRole('textbox',{name:'Name',exact:true}).boundingBox())!,descriptionBox=(await editor.getByRole('textbox',{name:'Description',exact:true}).boundingBox())!;expect(descriptionBox.y).toBeGreaterThan(nameBox.y+nameBox.height);expect(Math.abs(descriptionBox.x-nameBox.x)).toBeLessThan(1);await editor.getByRole('textbox',{name:'Description',exact:true}).fill('A late-night mix');await editor.getByRole('button',{name:'Save details',exact:true}).click();await expect(editor).not.toBeVisible();await expect(detail).toContainText('A late-night mix');
 await detail.locator('.playlist-tools').getByRole('button',{name:'More actions',exact:true}).click();await page.getByRole('menuitem',{name:'Delete Sparklist',exact:true}).click();await expect(detail.getByRole('button',{name:'Confirm delete',exact:true})).toBeVisible();await detail.getByRole('button',{name:'Cancel',exact:true}).click();await expect(detail.getByRole('button',{name:'Confirm delete',exact:true})).not.toBeVisible();
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback').length)).toBe(0);
 await page.screenshot({path:'out/sparklist-after-edited.png',animations:'disabled'});
});

for(const [width,height] of [[480,480],[720,480],[1280,800]])for(const percent of [80,100,125,150])test(`Sparklist stages fit ${width}×${height} at ${percent}% across themes`,async({page})=>{
 await page.setViewportSize({width,height});
 for(const [mode,source,accent] of [['light','artwork','#b54035'],['dark','artwork','#b54035'],['light','blue','#22aaff'],['light','custom','#ffff00'],['dark','custom','#ff00ff']]){
  await page.goto('/?fixture=1&sparkReady=1');await page.evaluate(p=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify(p)),{version:1,interface_percent:percent,appearance:{mode,source,accent,saturation:150,tint:15}});await page.reload();
  const dialog=await openCreator(page);await expect(dialog.getByRole('heading',{name:'Create Sparklist',exact:true})).toBeVisible();
  for(const stage of [1,2,3]){
   if(stage===2)await dialog.getByRole('button',{name:'Choose for me',exact:true}).click();
   if(stage===3)await dialog.getByRole('button',{name:'Name & save',exact:true}).click();
   await expect(dialog.locator('.spark-create-content')).toHaveAttribute('data-stage',String(stage));
   const metrics=await dialog.evaluate(el=>{const b=el.getBoundingClientRect();return {x:b.x,y:b.y,right:b.right,bottom:b.bottom,overflow:el.scrollWidth-el.clientWidth};});expect(metrics.x).toBeGreaterThanOrEqual(-1);expect(metrics.y).toBeGreaterThanOrEqual(-1);expect(metrics.right).toBeLessThanOrEqual(width+1);expect(metrics.bottom).toBeLessThanOrEqual(height+1);expect(metrics.overflow).toBeLessThanOrEqual(1);
   const footer=dialog.locator('.spark-step-footer:visible');await footer.scrollIntoViewIfNeeded();const actions=await footer.locator('button').all();for(const action of actions)await expect(action).toBeInViewport();
   if(width===480&&percent===150&&mode==='dark'&&source==='artwork')await page.screenshot({path:`out/sparklist-small-stage-${stage}.png`,animations:'disabled'});
  }
  await page.keyboard.press('Escape');await expect(dialog).not.toBeVisible();
 }
});

test('starting-song selection keeps the model gate and nested Escape behavior',async({page})=>{
 await page.goto('/?fixture=1');const dialog=await openCreator(page);await dialog.getByRole('button',{name:'Select song',exact:true}).click();const picker=page.getByRole('dialog',{name:'Select starting song',exact:true});await picker.getByRole('searchbox',{name:'Filter songs'}).fill('Opening');await picker.locator('.song-selector-list button').first().click();await expect(dialog.getByRole('button',{name:'Add starting song',exact:true})).toBeFocused();await expect(dialog).toContainText('Starting from');await expect(dialog.getByRole('button',{name:'Preview Sparklist',exact:true})).toBeDisabled();await expect(dialog.locator('.spark-setup').getByRole('status')).toContainText('Beta off');await dialog.getByRole('button',{name:'Add starting song',exact:true}).click();await page.keyboard.press('Escape');await expect(picker).not.toBeVisible();await expect(dialog).toBeVisible();await page.keyboard.press('Escape');await expect(dialog).not.toBeVisible();
});

test('Reduced Motion finishes stage changes without leftover work',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await page.goto('/?fixture=1&sparkReady=1');const dialog=await openCreator(page);await dialog.getByRole('button',{name:'Choose for me',exact:true}).click();await dialog.getByRole('button',{name:'Name & save',exact:true}).click();await expect(dialog.getByRole('textbox',{name:'Name',exact:true})).toBeFocused();expect(await dialog.evaluate(el=>el.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);await dialog.getByRole('button',{name:'Close Sparklist creator',exact:true}).click();await expect(dialog).not.toBeVisible();
});

test('saved Sparklist editor stays usable in small windows and preserves sequence conversion',async({page})=>{
 for(const percent of [80,100,125,150]){
  await page.setViewportSize({width:480,height:480});await page.goto('/?fixture=1&sparkReady=1');await page.evaluate(percent=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:percent})),percent);await page.reload();
  const dialog=await openCreator(page);await dialog.getByRole('button',{name:'Choose for me',exact:true}).click();await dialog.getByRole('button',{name:'Name & save',exact:true}).click();await dialog.getByRole('textbox',{name:'Name',exact:true}).fill('Small-window mix');await dialog.getByRole('button',{name:'Save Sparklist',exact:true}).click();
  const detail=page.locator('.audio-spark-detail');await detail.getByRole('button',{name:'Edit details',exact:true}).click();const editor=detail.getByRole('region',{name:'Sparklist details'});await expect(editor.getByRole('textbox',{name:'Name',exact:true})).toBeFocused();expect(await editor.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThanOrEqual(1);await editor.getByRole('button',{name:'Cancel',exact:true}).click();await expect(editor).not.toBeVisible();
  await detail.getByRole('button',{name:'Refresh mix',exact:true}).click();await expect(detail.getByRole('status')).toContainText('Your playing queue stays unchanged');
  await detail.locator('.playlist-tools').getByRole('button',{name:'More actions',exact:true}).click();await page.getByRole('menuitem',{name:'Save sequence as playlist',exact:true}).click();await expect(detail.getByRole('status')).toContainText('exact song order');
  expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback').length)).toBe(0);
 }
});
