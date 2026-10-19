import {test,expect,type Page} from '@playwright/test';
async function start(page:Page){
 await page.addInitScript(()=>{
  localStorage.setItem('musicos-fixture-analysis',JSON.stringify([{track_id:1,bpm:120},{track_id:2,bpm:90}]));
  localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/heart-qa',kind:'manual',name:'Heart tempo QA',track_ids:[1,2,3]}]));
 });
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
}
const heart=(page:Page,id:number)=>page.locator(`[data-track-id="${id}"] .track-favorite`);
async function playRow(page:Page,id:number){const row=page.locator(`[data-track-id="${id}"]`);await row.hover();await row.locator('.row-play').click();}
test('song hearts use their own cached BPM in albums, playlists and Songs; favorites remain clickable',async({page})=>{
 await start(page);
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await playRow(page,1);
 await expect(heart(page,1)).toHaveAttribute('data-pulsing','true');
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-duration','0.5s');
 await expect(heart(page,2).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await expect(heart(page,3).locator('.heart-beat')).toHaveCSS('animation-name','none');
 const before=await heart(page,1).getAttribute('aria-pressed');
 await heart(page,1).click();await expect(heart(page,1)).toHaveAttribute('aria-pressed',String(before!=='true'));
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-name','heart-tempo');
 await expect.poll(()=>heart(page,1).locator('.heart-beat').evaluate(el=>el.getAnimations().some(a=>a.playState==='running'))).toBe(true);
 await playRow(page,2);
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await expect(heart(page,2).locator('.heart-beat')).toHaveCSS('animation-duration','0.666667s');
 await page.getByRole('button',{name:'Heart tempo QA 3 songs',exact:false}).click();
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await expect(heart(page,2).locator('.heart-beat')).toHaveCSS('animation-duration','0.666667s');
 await page.getByRole('button',{name:'Songs',exact:true}).click();
 await expect(heart(page,2)).toHaveAttribute('data-pulsing','true');
 await expect(heart(page,2).locator('.heart-beat')).toHaveCSS('animation-duration','0.666667s');
 await page.getByRole('button',{name:'Pause',exact:true}).click();
 await expect(heart(page,2).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await page.getByRole('button',{name:'Play',exact:true}).click();
 await expect(heart(page,2)).toHaveAttribute('data-pulsing','true');
});
test('off-screen and Reduced Motion hearts stay still; refreshed or invalid BPM updates safely',async({page})=>{
 await start(page);
 await playRow(page,1);
 await expect(heart(page,1)).toHaveAttribute('data-pulsing','true');
 await page.emulateMedia({reducedMotion:'reduce'});
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await page.emulateMedia({reducedMotion:'no-preference'});
 await page.evaluate(()=>{
  localStorage.setItem('musicos-fixture-analysis',JSON.stringify([{track_id:1,bpm:150},{track_id:2,bpm:0}]));
  window.dispatchEvent(new Event('musicos-audio-rescanned'));
 });
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-duration','0.4s');
 await expect(heart(page,2).locator('.heart-beat')).toHaveCSS('animation-name','none');
 // A short nonvirtualized album keeps its off-screen row mounted.
 await heart(page,1).evaluate(el=>{(el.closest('.track-row') as HTMLElement).style.marginTop='1500px';});
 await expect(heart(page,1)).toHaveAttribute('data-pulsing','false');
 await expect(heart(page,1).locator('.heart-beat')).toHaveCSS('animation-name','none');
 await heart(page,1).scrollIntoViewIfNeeded();
 await expect(heart(page,1)).toHaveAttribute('data-pulsing','true');
});
