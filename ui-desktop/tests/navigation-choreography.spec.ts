import {test,expect,type Page} from '@playwright/test';
import {openAlbums} from './support/ui';

async function recordMotion(page:Page){
 await page.addInitScript(()=>{
  const animate=Element.prototype.animate;
  (window as any).navigationMotion=[];
  Element.prototype.animate=function(frames,options){
   const animation=animate.call(this,frames,options);
   (window as any).navigationMotion.push({animation,target:this,frames,options});
   if(this.classList.contains('library-toolbar-exit')){animation.pause();animation.currentTime=50;}
   return animation;
  };
 });
}

test('Filter leaves without reserving space or retaining its open panel, and Back restores it',async({page})=>{
 await recordMotion(page);await page.goto('/?fixture=1');await openAlbums(page);
 await page.getByRole('button',{name:'Filter',exact:true}).click();await expect(page.getByRole('region',{name:'Library filters'})).toBeVisible();
 await page.getByRole('button',{name:'Open Junk',exact:true}).click();
 const ghost=page.locator('.library-toolbar-exit');await expect(ghost).toHaveCount(1);
 await expect(ghost).toHaveAttribute('inert','');await expect(ghost).toHaveAttribute('aria-hidden','true');await expect(ghost).toHaveCSS('pointer-events','none');
 await expect(page.getByRole('region',{name:'Library filters'})).toHaveCount(0);await expect(page.getByRole('button',{name:'Filter',exact:true})).toHaveCount(0);
 const bars=await page.evaluate(()=>({frame:document.querySelector('.page-frame')!.getBoundingClientRect().top,column:document.querySelector('.content-column')!.getBoundingClientRect().top}));expect(Math.abs(bars.frame-bars.column)).toBeLessThan(1);
 await page.getByRole('button',{name:'Back to Albums',exact:true}).click();await expect(ghost).toHaveCount(0);await expect(page.getByRole('button',{name:'Filter',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Filter',exact:true}).click();await expect(page.getByRole('region',{name:'Library filters'})).toBeVisible();
 await page.keyboard.press('Escape');await expect(page.getByRole('button',{name:'Filter',exact:true})).toBeFocused();
});

test('album details arrive independently and artist names reveal only on navigation',async({page})=>{
 await recordMotion(page);await page.goto('/?fixture=1&contributors=1');await openAlbums(page);await page.getByRole('button',{name:'Open Junk',exact:true}).click();
 const credits=await page.evaluate(()=>(window as any).navigationMotion.filter((item:any)=>item.animation.id==='album-detail-reveal'));
 expect(credits.length).toBeGreaterThan(0);expect(credits.length).toBeLessThanOrEqual(8);
 await page.getByRole('button',{name:'View artist M83',exact:true}).click();await expect(page.getByRole('heading',{name:'M83',exact:true})).toBeVisible();
 await page.evaluate(()=>(window as any).navigationMotion.filter((item:any)=>/artist-(name|portrait|detail)-reveal/.test(item.animation.id)&&item.animation.playState==='running').forEach((item:any)=>{item.animation.pause();item.animation.currentTime=120;}));
 await page.screenshot({path:'out/persistent-qa/navigation-artist-mid-reveal.png'});
 await page.evaluate(()=>(window as any).navigationMotion.filter((item:any)=>item.animation.playState==='paused').forEach((item:any)=>item.animation.finish()));
 await expect.poll(()=>page.locator('.artist-name-part').evaluateAll(nodes=>nodes.every(node=>getComputedStyle(node).opacity==='1'))).toBe(true);
 const records=await page.evaluate(()=>(window as any).navigationMotion.filter((item:any)=>['artist-name-reveal','artist-detail-reveal'].includes(item.animation.id)).map((item:any)=>({id:item.animation.id,duration:item.options.duration})));
 expect(records.some((item:any)=>item.id==='artist-name-reveal')).toBe(true);expect(records.some((item:any)=>item.id==='artist-detail-reveal')).toBe(true);expect(records.every((item:any)=>item.duration<=300)).toBe(true);
 const before=records.length;await page.evaluate(()=>window.dispatchEvent(new Event('musicos-artwork-invalidated')));
 expect(await page.evaluate(()=>(window as any).navigationMotion.filter((item:any)=>['artist-name-reveal','artist-detail-reveal'].includes(item.animation.id)).length)).toBe(before);
 await page.screenshot({path:'out/persistent-qa/navigation-artist-light.png'});
});

test('Reduced Motion opens immediately with no exit copies or portrait motion',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await recordMotion(page);await page.goto('/?fixture=1');await openAlbums(page);await page.getByRole('button',{name:'Open Junk',exact:true}).click();
 await expect(page.locator('.library-toolbar')).toHaveCount(0);await page.getByRole('button',{name:'View artist M83',exact:true}).click();
 await expect(page.getByRole('heading',{name:'M83',exact:true})).toBeVisible();
 expect(await page.evaluate(()=>(window as any).navigationMotion.filter((item:any)=>/artist-name|artist-portrait|library-toolbar/.test(item.animation.id)).length)).toBe(0);
});

for(const mode of ['light','dark'])test(`${mode} Settings navigation surfaces follow artwork instead of a blue accent`,async({page})=>{
 await page.addInitScript(mode=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode,source:'artwork',accent:'#0064df',saturation:100,tint:15}})),mode);
 // This deliberately mismatched provider palette models the reported screenshot.
 await page.route('**/src/fixtures.ts*',async route=>{const response=await route.fetch();await route.fulfill({response,body:(await response.text()).replace('case "artwork_palette":','case "artwork_palette": return {background:"#c5d7c9",blob:"#91ad98",accent:"#0055ff",text:"#12202a"};')});});
 await page.goto('/?fixture=1');await openAlbums(page);await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
 await page.getByRole('button',{name:'Open Junk',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Appearance',exact:true})).toBeVisible();
 const surfaces=await page.evaluate(()=>{const root=getComputedStyle(document.documentElement),header=getComputedStyle(document.querySelector('.section-control>.menu-trigger')!),nav=getComputedStyle(document.querySelector('.settings-tabs')!);return {header:header.backgroundColor,control:root.getPropertyValue('--control-surface').trim(),nav:nav.backgroundColor,ink:root.getPropertyValue('--ink').trim(),text:header.color};});
 expect(surfaces.header).not.toBe('rgb(0, 85, 255)');expect(surfaces.nav).not.toBe('rgb(142, 209, 255)');
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await expect(page.locator('.section-dropdown')).toHaveCSS('background-color',surfaces.header);
 await page.keyboard.press('Escape');await page.screenshot({path:`out/persistent-qa/navigation-settings-${mode}.png`});
});
