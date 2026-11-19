import {test,expect} from '@playwright/test';
test.use({browserName:'webkit'});
async function phase(page:any,value:number){await page.evaluate((value:number)=>window.dispatchEvent(new CustomEvent('musicos-trackpad-phase',{detail:value})),value);}
test('page tracks fingers, releases once, and suppresses trailing momentum',async({page})=>{await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();const detail=page.locator('.album-detail h1');await detail.hover();await phase(page,1);await page.mouse.wheel(-80,0);await expect(page.locator('.back-page-preview')).toHaveCount(1);await expect(page.locator('.back-page-preview')).toHaveCSS('transform','none');await expect(page.locator('#library-content')).toHaveAttribute('data-swiping','true');expect(await page.locator('#library-content').evaluate(e=>getComputedStyle(e).transform)).not.toBe('none');await page.mouse.wheel(-220,0);await phase(page,2);await phase(page,4);await expect(page.locator('#library-content .collection-grid')).toBeVisible();await expect(page.locator('.app-shell .album-detail')).toHaveCount(0);await expect(page.locator('.back-page-preview')).toHaveCount(0);await page.mouse.wheel(-400,0);await expect(page.locator('#library-content .collection-grid')).toBeVisible();expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='playback').length)).toBe(0);});
test('reversal, vertical intent, modal menus and reduced motion retain behavior',async({page})=>{await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();const detail=page.locator('.album-detail h1');await detail.hover();await page.mouse.wheel(-20,100);await page.waitForTimeout(100);await expect(detail).toBeVisible();await phase(page,1);await page.mouse.wheel(-80,0);await page.mouse.wheel(70,0);await phase(page,2);await expect(page.locator('.back-page-preview')).toHaveCount(0);await expect(detail).toBeVisible();await page.getByRole('button',{name:'More library views'}).click();await page.getByRole('menuitem',{name:'Playlists',exact:true}).hover();await page.mouse.wheel(-400,0);await expect(page.getByRole('menu')).toBeVisible();await page.keyboard.press('Escape');await page.emulateMedia({reducedMotion:'reduce'});await detail.hover();await phase(page,1);await page.mouse.wheel(-400,0);await phase(page,2);await expect(page.locator('.personal-home')).toBeVisible();await expect(page.locator('.back-page-preview')).toHaveCount(0);});
test('full-distance release hands over immediately with no second transition or retained preview',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await page.locator('.album-detail h1').hover();await phase(page,1);await page.mouse.wheel(-1400,0);await phase(page,2);await expect(page.locator('#library-content .collection-grid')).toBeVisible();await expect(page.locator('.back-page-preview')).toHaveCount(0);expect(await page.locator('#library-content').evaluate(e=>e.getAnimations({subtree:true}).filter(a=>/interactive-back|content-change|page-navigation/.test(a.id)).length)).toBe(0);await phase(page,4);await page.mouse.wheel(-400,0);await expect(page.locator('#library-content .collection-grid')).toBeVisible();
});
test('a short fast flick survives the zero-distance finger-release event',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await page.locator('.album-detail h1').hover();await page.locator('#library-content').evaluate(node=>{window.dispatchEvent(new CustomEvent('musicos-trackpad-phase',{detail:1}));node.dispatchEvent(new WheelEvent('wheel',{deltaX:-90,deltaY:0,bubbles:true,cancelable:true}));node.dispatchEvent(new WheelEvent('wheel',{deltaX:0,deltaY:0,bubbles:true,cancelable:true}));window.dispatchEvent(new CustomEvent('musicos-trackpad-phase',{detail:2}));});await expect(page.locator('#library-content .collection-grid')).toBeVisible();await expect(page.locator('.back-page-preview')).toHaveCount(0);
});
test('artwork and fade follow finger distance and retract without a second transfer',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:2000});
 const id=await page.locator('.album-detail').getAttribute('data-album-id');const source=(await page.locator('.album-artwork-stage .cover').boundingBox())!;await page.locator('.album-detail h1').hover();await phase(page,1);await page.mouse.wheel(-120,0);const cover=page.locator('.swipe-cover-transfer');await expect(cover).toHaveCount(1);
 const at=await page.locator('#library-content').evaluate(n=>({distance:new DOMMatrixReadOnly(getComputedStyle(n).transform).m41,width:n.getBoundingClientRect().width,opacity:Number(getComputedStyle(n).opacity)}));
 const target=await page.locator(`.back-page-preview [data-collection-id="${id}"] .card-artwork`).boundingBox();expect(target).not.toBeNull();
 const shown=(await cover.boundingBox())!,p=at.distance/at.width;expect(Math.abs(shown.x-(source.x+(target!.x-source.x)*p))).toBeLessThan(2);expect(at.opacity).toBe(1);
 await page.mouse.wheel(-120,0);const further=(await cover.boundingBox())!;expect(Math.abs(further.x-shown.x)+Math.abs(further.y-shown.y)).toBeGreaterThan(1);await phase(page,3);await expect(cover).toHaveCount(0);await expect(page.locator('.album-detail')).toBeVisible();await expect(page.locator('#library-content')).toHaveCSS('opacity','1');
 await phase(page,1);await page.mouse.wheel(-1400,0);await phase(page,2);await expect(page.locator('#library-content .collection-grid')).toBeVisible();await expect(page.locator('.swipe-cover-transfer,.album-cover-transfer,.back-page-preview')).toHaveCount(0);expect(await page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('interactive-back')).length)).toBe(0);
});


test('Back keeps one live toolbar and restores the grid without shifting it',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 const initialTop=(await page.locator('#library-content').boundingBox())!.y;
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await expect(page.locator('.album-cover-transfer')).toHaveCount(0);
 await page.locator('.album-detail h1').hover();
 await phase(page,1);
 await page.mouse.wheel(-150,0);
 await expect(page.locator('.back-page-preview')).toHaveCount(1);
 await expect(page.locator('.back-swipe-bar')).toHaveCount(0);
 await expect(page.locator('#library-content')).toHaveCSS('opacity','1');
 expect((await page.locator('.back-page-preview .page-scroll').boundingBox())!.y).toBeCloseTo(initialTop,0);
 await page.mouse.wheel(-1400,0);
 await phase(page,2);
 await expect(page.locator('.back-page-preview,.swipe-cover-transfer')).toHaveCount(0);
 await expect(page.getByRole('button',{name:'Filter',exact:true})).toBeVisible();
 expect((await page.locator('#library-content').boundingBox())!.y).toBeCloseTo(initialTop,0);
});

test('final WebKit sample can arrive after phase end and resized pages still preview',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await page.setViewportSize({width:840,height:530});
 const target=page.locator('#library-content');
 await phase(page,1);
 await target.evaluate(node=>node.dispatchEvent(new WheelEvent('wheel',{deltaX:-35,bubbles:true,cancelable:true})));
 await expect(page.locator('.back-page-preview')).toHaveCount(1);
 await target.evaluate(node=>{
  window.dispatchEvent(new CustomEvent('musicos-trackpad-phase',{detail:2}));
  node.dispatchEvent(new WheelEvent('wheel',{deltaX:-440,bubbles:true,cancelable:true}));
 });
 await expect(target.locator('.collection-grid')).toBeVisible();
 await expect(page.locator('.back-page-preview')).toHaveCount(0);
});

test('Back holds the landed artwork while the live tile is still a placeholder',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await expect(page.locator('.album-cover-transfer')).toHaveCount(0);
 const id=await page.locator('.album-detail').getAttribute('data-album-id');
 await page.evaluate(id=>{
  const column=document.querySelector('.content-column')!;
  const observer=new MutationObserver(()=>{
   if(column.getAttribute('data-back-swipe')!=='handoff')return;
   const live=column.querySelector<HTMLImageElement>(`#library-content [data-collection-id="${CSS.escape(id!)}"] img.cover`);
   if(!live)return;
   observer.disconnect();
   live.classList.add('placeholder');
   (window as any).releaseLiveCover=()=>live.classList.remove('placeholder');
  });
  observer.observe(column,{attributes:true,attributeFilter:['data-back-swipe']});
 },id);
 await page.locator('.album-detail h1').hover();
 await phase(page,1);
 await page.mouse.wheel(-1400,0);
 await phase(page,2);
 const handoff=page.locator('.content-column[data-back-swipe="handoff"]');
 await expect(handoff).toHaveCount(1);
 const result=await page.evaluate(async()=>{
  await new Promise<void>(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve()))));
  const kept=!!document.querySelector('.swipe-cover-transfer');
  (window as any).releaseLiveCover();
  return kept;
 });
 expect(result).toBe(true);
 await expect(page.locator('.swipe-cover-transfer,.back-page-preview')).toHaveCount(0);
});
