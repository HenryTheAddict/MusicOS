import {test,expect,type Page} from '@playwright/test';

async function seedPlayer(page:Page){await page.route('**/src/fixtures.ts*',async route=>{const response=await route.fetch();const body=(await response.text()).replace('let player = { ...emptyPlayback };','let player = {...emptyPlayback,track:tracks[0],queue:[tracks[0],tracks[1]],queue_index:0,queue_entry_ids:[71,72],position:30};');await route.fulfill({response,body});});}

test('Sketch category becomes supplied dots without moving the label; actions and sections stay independent',async({page})=>{
 await seedPlayer(page);await page.goto('/?fixture=1');
 await expect(page.locator('.header-brand>span')).toHaveText('musicOS');
 await expect(page).toHaveTitle(/musicOS/);
 const actions=page.getByRole('button',{name:'musicOS actions',exact:true});
 const section=page.getByRole('button',{name:'Choose section',exact:true});
 const label=section.locator('.section-name');const bounds=await label.boundingBox();
 const category=actions.locator('.section-category-glyph'),dots=actions.locator('.section-dots-glyph');
 await expect(category).toHaveCSS('opacity','1');await expect(dots).toHaveCSS('opacity','0');
 await actions.hover();await expect(dots).toHaveCSS('opacity','1');await expect(category).toHaveCSS('opacity','0');expect(await label.boundingBox()).toEqual(bounds);
 await actions.click();await expect(page.getByRole('menuitem',{name:'Mini Player',exact:true})).toBeVisible();await expect(page.locator('.section-dropdown')).toHaveCount(0);
 await page.keyboard.press('Escape');await expect(page.getByRole('menu')).toHaveCount(0);await expect(actions).toBeFocused();expect(await label.boundingBox()).toEqual(bounds);
 await section.click();await page.getByRole('menuitem',{name:'Devices',exact:true}).click();await expect(label).toHaveText('Devices');await expect(page.getByRole('heading',{name:'Devices',exact:true})).toBeVisible();
 await page.screenshot({path:'out/persistent-qa/header-musicos-devices.png'});
 await page.emulateMedia({reducedMotion:'reduce'});await actions.focus();await expect(dots).toHaveCSS('opacity','1');expect(await dots.evaluate(el=>el.getAnimations().length)).toBe(0);
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback'))).toEqual([]);
});

test('header reserves fixed icon targets and gives remaining width to the player at every scale',async({page})=>{
 await seedPlayer(page);await page.addInitScript(()=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(new URLSearchParams(location.search).get('scale')),appearance:{mode:'light',source:'blue'}}));});
 for(const scale of [80,100,125,150])for(const width of [480,720,1280]){
  await page.setViewportSize({width,height:width===1280?800:480});await page.goto(`/?fixture=1&scale=${scale}`);
  const shell=page.locator('.adaptive-shell');await expect(shell).toHaveAttribute('data-size',String(scale));
  const r=await page.evaluate(()=>{const rect=(s:string)=>{const r=document.querySelector(s)!.getBoundingClientRect();return {x:r.x,right:r.right,y:r.y,bottom:r.bottom,width:r.width,height:r.height};};return {actions:rect('.section-actions-trigger'),label:rect('.section-name'),section:rect('.section-control'),player:rect('.now-playing'),layout:document.querySelector('.adaptive-shell')!.getAttribute('data-layout'),overflow:document.documentElement.scrollWidth-innerWidth};});
  expect(r.actions.width).toBeGreaterThanOrEqual(44);expect(r.actions.height).toBeGreaterThanOrEqual(44);expect(r.actions.right).toBeLessThanOrEqual(r.label.x+1);expect(r.overflow).toBeLessThan(2);
  if(r.layout==='wide'){expect(r.player.x-r.section.right).toBeLessThanOrEqual(8*scale/100+1);expect(r.player.width).toBeGreaterThan(width-460*scale/100);}
 }
 await page.setViewportSize({width:1280,height:800});await page.goto('/?fixture=1&scale=100');await page.getByRole('button',{name:'Albums',exact:true}).click();await expect(page.getByRole('button',{name:'Open Plastic Beach',exact:true})).toBeVisible();await expect(page.locator('.now-playing-cover img')).toBeVisible();await page.screenshot({path:'out/persistent-qa/header-musicos-1280.png'});
});
