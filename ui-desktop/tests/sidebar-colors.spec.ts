import {test,expect} from '@playwright/test';
test('sidebar keeps artwork, text and cover-colored controls separate across sizes',async({page})=>{
 for(const [width,height,scale] of [[720,480,150],[1280,800,100],[1280,800,150]]){
  await page.setViewportSize({width,height});await page.goto('/?fixture=1');
  await page.evaluate(percent=>{
   localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:percent,display_name:'Henry'}));
   const art=(color:string)=>'data:image/svg+xml,'+encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="${color}"/></svg>`);
   localStorage.setItem('musicos-fixture-collections',JSON.stringify([
    {id:'red',name:'A long ordinary playlist title',track_ids:[1,2],spark:null,kind:'manual',artwork:art('#e04020'),artwork_choice:'custom'},
    {id:'blue',name:'Another long ordinary playlist title',track_ids:[1,2],spark:null,kind:'manual',artwork:art('#2040e0'),artwork_choice:'custom'}]));
  },scale);await page.reload();
  const rows=page.locator('.personal-collection');await expect(rows).toHaveCount(2);
  for(const row of await rows.all()){
   await row.scrollIntoViewIfNeeded();
   await expect.poll(()=>row.evaluate(el=>el.style.getPropertyValue('--collection-tone'))).toMatch(/rgb\(/);
   const art=(await row.locator('.playlist-artwork-tone').boundingBox())!,label=(await row.locator('.collection-label').boundingBox())!,shuffle=(await row.locator('.collection-shuffle').boundingBox())!;
   expect(art.x+art.width).toBeLessThanOrEqual(label.x);expect(label.x+label.width).toBeLessThanOrEqual(shuffle.x);
  }
  const tones=await rows.evaluateAll(els=>els.map(el=>(el as HTMLElement).style.getPropertyValue('--collection-tone')));expect(tones[0]).not.toBe(tones[1]);
  const ordinary=rows.nth(0).locator('.collection-shuffle .icon');expect(await ordinary.evaluate(el=>getComputedStyle(el).getPropertyValue('--shuffle-ink').trim())).toBe(tones[0]);
  const second=rows.nth(1).locator('.collection-shuffle .icon');expect(await second.evaluate(el=>getComputedStyle(el).getPropertyValue('--shuffle-ink').trim())).toBe(tones[1]);
  await page.screenshot({path:`out/rebuild-qa/sidebar-colors-${width}-${scale}.png`,animations:'disabled'});
 }
});
test('Sparklist creation has one header icon and a reachable close action',async({page})=>{
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByRole('menuitem',{name:'New Sparklist',exact:true}).click();
 const modal=page.getByRole('dialog',{name:'Create Sparklist',exact:true});await expect(modal.locator('.spark-composer-mark > [data-icon=Sparklist]')).toHaveCount(1);
 await expect(modal.getByRole('button',{name:'Close Sparklist creator'})).toBeVisible();await modal.getByRole('button',{name:'Close Sparklist creator'}).click();await expect(modal).not.toBeVisible();
});
