import {test,expect} from '@playwright/test';
test('song actions occupy one line in Songs, albums and playlists at every scale',async({page})=>{
 test.setTimeout(60000);
 await page.goto('/?fixture=1');
 await page.evaluate(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'layout-mix',name:'Layout mix',kind:'manual',spark:null,track_ids:[1,2,3],artwork_choice:'builtin/1'}])));
 for(const size of [{width:720,height:480},{width:1280,height:800}])for(const percent of [80,100,125,150]){
  await page.setViewportSize(size);
  await page.evaluate(value=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'Henry',interface_percent:value,coverflow_colors:false}));sessionStorage.removeItem('musicos-navigation-v1');},percent);await page.reload();
  for(const route of ['songs','album','playlist']){
   if(route==='songs')await page.getByRole('button',{name:'Songs',exact:true}).click();
   if(route==='album'){await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();}
   if(route==='playlist')await page.locator('.collection-open').filter({hasText:'Layout mix'}).click();
   const row=page.locator('.track-body > .track-row').first();await expect(row).toBeVisible();
   const actions=row.locator('.track-actions');await expect(actions.locator('.track-quality')).toBeVisible();
   const layout=await row.evaluate(el=>{
    const row=el.getBoundingClientRect(),actions=el.querySelector('.track-actions')!.getBoundingClientRect(),title=el.querySelector('.track-title')!.getBoundingClientRect();
    return {row:{top:row.top,bottom:row.bottom,right:row.right},actions:{left:actions.left,right:actions.right,top:actions.top,bottom:actions.bottom},titleRight:title.right,centers:[...el.querySelector('.track-actions')!.children].map(child=>{const r=child.getBoundingClientRect();return r.y+r.height/2;})};
   });
   expect(Math.max(...layout.centers)-Math.min(...layout.centers)).toBeLessThan(1);
   expect(layout.actions.top).toBeGreaterThanOrEqual(layout.row.top);expect(layout.actions.bottom).toBeLessThanOrEqual(layout.row.bottom);
   expect(layout.actions.right).toBeLessThanOrEqual(layout.row.right);expect(layout.titleRight).toBeLessThanOrEqual(layout.actions.left+1);
   const play=row.locator('.row-play');await row.hover();await expect(play).toHaveCSS('background-color','rgba(0, 0, 0, 0)');await expect(play).toHaveCSS('box-shadow','none');await expect(play).toHaveCSS('border-radius','0px');
   const menu=actions.getByRole('button',{name:'More actions'});await menu.click();await expect(page.getByRole('menuitem',{name:'Play',exact:true})).toBeVisible();await page.keyboard.press('Escape');
   if((size.width===720&&percent===150)||(size.width===1280&&percent===100))await page.screenshot({animations:'disabled',path:`out/rebuild-qa/song-actions-${route}-${size.width}-${percent}.png`});
  }
 }
});
