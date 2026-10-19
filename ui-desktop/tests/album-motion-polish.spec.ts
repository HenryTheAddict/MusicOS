import {test,expect,type Page} from '@playwright/test';
async function observeMotion(page:Page){
 await page.addInitScript(()=>{
  const original=Element.prototype.animate;(window as any).detailMotion=[];(window as any).holdCover=false;
  Element.prototype.animate=function(frames,options){const animation=original.call(this,frames,options);(window as any).detailMotion.push({animation,target:this,frames,options});if(this.classList.contains('album-cover-transfer')&&(window as any).holdCover){animation.pause();animation.currentTime=0;}return animation;};
 });
}
test('album cover moves from its clicked grid position and copy reveals in order',async({page})=>{
 await observeMotion(page);await page.goto('/?fixture=1');
 const card=page.getByRole('button',{name:'Open Junk',exact:true});const start=(await card.locator('.card-artwork').boundingBox())!;
 await page.evaluate(()=>(window as any).holdCover=true);await card.click();
 const transfer=page.locator('.album-cover-transfer');await expect(transfer).toHaveCount(1);const initial=(await transfer.boundingBox())!;
 expect(Math.abs(initial.x-start.x)).toBeLessThan(2);expect(Math.abs(initial.y-start.y)).toBeLessThan(2);expect(Math.abs(initial.width-start.width)).toBeLessThan(2);
 await expect(transfer).toHaveCSS('pointer-events','none');await expect(page.locator('.album-artwork-stage')).toHaveCSS('opacity','0');
 const reveal=await page.evaluate(()=>(window as any).detailMotion.filter((item:any)=>item.animation.id==='album-detail-reveal').map((item:any)=>({delay:item.options.delay,duration:item.options.duration,frames:item.frames})));
 expect(reveal.length).toBeGreaterThanOrEqual(5);expect(reveal.length).toBeLessThanOrEqual(7);expect(reveal.slice(0,4).map((item:any)=>item.delay)).toEqual([30,78,102,126]);expect(reveal.every((item:any)=>item.duration<=220)).toBe(true);
 await page.evaluate(()=>{const item=(window as any).detailMotion.find((item:any)=>item.animation.id==='album-cover-transfer');item.animation.finish();});
 await expect(transfer).toHaveCount(0);await expect(page.locator('.album-artwork-stage')).toHaveCSS('opacity','1');
 const before=await page.evaluate(()=>(window as any).detailMotion.filter((item:any)=>item.animation.id==='album-detail-reveal').length);
 await page.evaluate(()=>window.dispatchEvent(new Event('musicos-artwork-invalidated')));
 expect(await page.evaluate(()=>(window as any).detailMotion.filter((item:any)=>item.animation.id==='album-detail-reveal').length)).toBe(before);
 await page.screenshot({path:'out/persistent-qa/album-motion-light.png',animations:'disabled'});
});
test('interrupted return starts from the visible cover and resize removes transient layers',async({page})=>{
 await observeMotion(page);await page.goto('/?fixture=1');await page.evaluate(()=>(window as any).holdCover=true);
 await page.getByRole('button',{name:'Open Junk',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(1);
 await page.evaluate(()=>{const item=(window as any).detailMotion.find((item:any)=>item.animation.id==='album-cover-transfer');item.animation.currentTime=140;});
 const halfway=(await page.locator('.album-cover-transfer').boundingBox())!;
 await page.getByRole('button',{name:'Back to Albums',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(1);
 const returned=(await page.locator('.album-cover-transfer').boundingBox())!;expect(Math.abs(returned.x-halfway.x)).toBeLessThan(2);expect(Math.abs(returned.y-halfway.y)).toBeLessThan(2);
 await page.setViewportSize({width:1000,height:700});await expect(page.locator('.album-cover-transfer')).toHaveCount(0);expect(await page.locator('[data-album-entering=true]').count()).toBe(0);
});
test('keyboard opening works and Reduced Motion cancels all spatial detail motion',async({page})=>{
 await observeMotion(page);await page.goto('/?fixture=1');await page.evaluate(()=>(window as any).holdCover=true);
 await page.getByRole('button',{name:'Open Junk',exact:true}).focus();await page.keyboard.press('Enter');await expect(page.locator('.album-cover-transfer')).toHaveCount(1);
 await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('.album-cover-transfer')).toHaveCount(0);await expect(page.locator('.album-artwork-stage')).toHaveCSS('opacity','1');
 const before=await page.evaluate(()=>(window as any).detailMotion.length);await page.getByRole('button',{name:'Back to Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();expect(await page.evaluate(()=>(window as any).detailMotion.length)).toBe(before);
});
test('artist portrait stays flat and theme controls keep readable neutral ink',async({page})=>{
 await page.addInitScript(()=>{const q=new URLSearchParams(location.search);localStorage.setItem('musicos-fixture-preferences',JSON.stringify({artwork_depth:true,appearance:{mode:q.get('polishMode'),source:'artwork',accent:'#0064df',saturation:Number(q.get('polishSaturation')),tint:15}}));});
 for(const mode of ['light','dark'])for(const saturation of [100,150]){
  await page.goto(`/?fixture=1&depthFixture=1&polishMode=${mode}&polishSaturation=${saturation}`);await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();
  const portrait=page.locator('.artist-portrait-button');await portrait.hover({position:{x:120,y:65}});await expect(portrait).toHaveCSS('transform','none');await expect(portrait.locator('img')).toHaveCSS('transform','none');await expect(page.locator('.artist-detail .depth-compositor')).toHaveCount(0);
  await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
  const colors=await page.evaluate(()=>{const root=getComputedStyle(document.documentElement),play=document.querySelector('.artist-statistics .play-button .asset-icon path[fill]')!,shuffle=document.querySelector('.artist-statistics .shuffle-button .asset-icon use')!;return {ink:root.getPropertyValue('--ink').trim(),play:getComputedStyle(play).fill,shuffle:getComputedStyle(shuffle).fill,surface:getComputedStyle(document.querySelector('.artist-statistics .menu-trigger')!).backgroundColor};});
  expect(colors.play).toBe(mode==='dark'?'rgb(222, 223, 227)':'rgb(18, 32, 42)');expect(colors.shuffle).toBe(colors.surface);
  const ratios=await page.evaluate(()=>{
   const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const context=canvas.getContext('2d')!;
   const lum=(color:string)=>{context.clearRect(0,0,1,1);context.fillStyle=color;context.fillRect(0,0,1,1);const [r,g,b]=[...context.getImageData(0,0,1,1).data].map(value=>{const n=value/255;return n<=.04045?n/12.92:((n+.055)/1.055)**2.4;});return r*.2126+g*.7152+b*.0722;};
   const contrast=(a:string,b:string)=>{const values=[lum(a),lum(b)].sort((a,b)=>b-a);return (values[0]+.05)/(values[1]+.05);};
   const header=getComputedStyle(document.querySelector('.track-head')!),background=getComputedStyle(document.querySelector('.current-theme')!),back=getComputedStyle(document.querySelector('.sidebar-back')!);
   return {labels:contrast(header.color,background.backgroundColor),back:contrast(back.color,back.backgroundColor)};
  });expect(ratios.labels).toBeGreaterThanOrEqual(4.5);expect(ratios.back).toBeGreaterThanOrEqual(4.5);
  await page.screenshot({path:`out/persistent-qa/artist-polish-${mode}-${saturation}.png`,animations:'disabled'});
 }
});
test('artist album-strip return completes promptly with no lingering cover layer',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();
 await page.getByRole('button',{name:'Junk',exact:true}).click();await expect(page.locator('.album-detail')).toBeVisible();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:800});
 await page.locator('.sidebar-back').click();await expect(page.locator('.artist-detail')).toBeVisible();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:800});
});
test('album opening fits compact windows at every interface scale',async({page})=>{
 for(const percent of [80,100,125,150]){
  await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.evaluate(percent=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent}));sessionStorage.removeItem('musicos-navigation-v1');},percent);await page.reload();
  await page.getByRole('button',{name:'Open Junk',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:800});
  const cover=(await page.locator('.album-artwork-stage').boundingBox())!,title=(await page.locator('.album-copy h1').boundingBox())!;
  expect(cover.x).toBeGreaterThanOrEqual(0);expect(cover.x+cover.width).toBeLessThanOrEqual(720);expect(title.x+title.width).toBeLessThanOrEqual(721);
  await expect(page.getByRole('button',{name:'Expand Junk artwork',exact:true})).toBeEnabled();
  if(percent===100)await page.screenshot({path:'out/persistent-qa/album-motion-compact.png',animations:'disabled'});
 }
});
test('artist portrait connects to its header, reverses from current bounds, and stays steady afterward',async({page})=>{
 await observeMotion(page);await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();
 const card=page.getByRole('button',{name:'Open M83',exact:true});await expect.poll(()=>card.locator('img.cover:not(.placeholder)').evaluate((img:HTMLImageElement)=>img.complete&&img.naturalWidth>0)).toBe(true);
 const start=(await card.locator('.card-artwork').boundingBox())!;const source=await card.locator('img.cover').evaluate((img:HTMLImageElement)=>img.currentSrc||img.src);
 await page.evaluate(()=>(window as any).holdCover=true);await card.focus();await page.keyboard.press('Enter');
 const transfer=page.locator('.album-cover-transfer[data-cover-kind=artist]');await expect(transfer).toHaveCount(1);expect(await transfer.getAttribute('src')).toBe(source);
 const initial=(await transfer.boundingBox())!;expect(Math.abs(initial.x-start.x)).toBeLessThan(2);expect(Math.abs(initial.y-start.y)).toBeLessThan(2);expect(Math.abs(initial.width-start.width)).toBeLessThan(2);
 await expect(page.locator('.artist-portrait-button')).toHaveCSS('opacity','0');await expect(transfer).toHaveCSS('pointer-events','none');
 const reveal=await page.evaluate(()=>(window as any).detailMotion.filter((item:any)=>item.animation.id==='artist-detail-reveal').map((item:any)=>item.options.delay));expect(reveal).toEqual([85,120,155,180]);
 await page.evaluate(()=>{const item=(window as any).detailMotion.find((item:any)=>item.animation.id==='artist-cover-transfer');item.animation.currentTime=140;});const halfway=(await transfer.boundingBox())!;
 await page.locator('.sidebar-back').click();await expect(transfer).toHaveCount(1);const returned=(await transfer.boundingBox())!;expect(Math.abs(returned.x-halfway.x)).toBeLessThan(2);expect(Math.abs(returned.y-halfway.y)).toBeLessThan(2);
 await page.evaluate(()=>{const items=(window as any).detailMotion.filter((item:any)=>item.animation.id==='artist-cover-transfer');items.at(-1).animation.finish();});await expect(transfer).toHaveCount(0);
 await page.evaluate(()=>(window as any).holdCover=false);await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(transfer).toHaveCount(0,{timeout:800});await expect(page.locator('.artist-portrait-button')).toHaveCSS('opacity','1');
 await page.locator('.artist-portrait-button').hover();await expect(page.locator('.artist-portrait-button')).toHaveCSS('transform','none');
 const before=await page.evaluate(()=>(window as any).detailMotion.length);await page.evaluate(()=>window.dispatchEvent(new Event('musicos-artwork-invalidated')));expect(await page.evaluate(()=>(window as any).detailMotion.length)).toBe(before);
});
test('artist transfer cancels cleanly on resize and Reduced Motion',async({page})=>{
 await observeMotion(page);await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.evaluate(()=>(window as any).holdCover=true);
 await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(1);await page.setViewportSize({width:1100,height:750});await expect(page.locator('.album-cover-transfer')).toHaveCount(0);await expect(page.locator('.artist-portrait-button')).toHaveCSS('opacity','1');
 await page.locator('.sidebar-back').click();await expect(page.locator('.album-cover-transfer')).toHaveCount(1);await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('.album-cover-transfer')).toHaveCount(0);expect(await page.locator('[data-album-entering=true]').count()).toBe(0);
 const before=await page.evaluate(()=>(window as any).detailMotion.length);await page.getByRole('button',{name:'Open M83',exact:true}).click();expect(await page.evaluate(()=>(window as any).detailMotion.length)).toBe(before);
});
test('dark artwork hue reaches player and portal menus without pale surfaces',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode:'dark',source:'artwork',accent:'#0064df',saturation:150,tint:20}})));
 await page.goto('/?fixture=1');await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
 const albumColor=await page.locator('html').evaluate(el=>getComputedStyle(el).getPropertyValue('--surface-color').trim());expect(albumColor).not.toBe('#202124');
 await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:800});
 await expect.poll(()=>page.locator('html').evaluate(el=>getComputedStyle(el).getPropertyValue('--surface-color').trim())).not.toBe(albumColor);
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await expect(page.locator('.section-dropdown')).toBeVisible();
 const colors=await page.evaluate(()=>{const root=getComputedStyle(document.documentElement);return {control:root.getPropertyValue('--control-surface').trim(),player:getComputedStyle(document.querySelector('.now-playing')!).backgroundColor,menu:getComputedStyle(document.querySelector('.section-dropdown')!).backgroundColor};});expect(colors.player).toBe(colors.menu);expect(colors.player).not.toBe('rgb(41, 43, 48)');
 await page.screenshot({path:'out/persistent-qa/artist-color-dark-menu.png',animations:'disabled'});await page.keyboard.press('Escape');await page.screenshot({path:'out/persistent-qa/artist-color-dark.png',animations:'disabled'});
});
test('artist portrait opening fits compact windows at every interface scale',async({page})=>{
 for(const percent of [80,100,125,150]){
  await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.evaluate(percent=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,appearance:{mode:'dark',source:'artwork',accent:'#0064df',saturation:150,tint:20}}));sessionStorage.removeItem('musicos-navigation-v1');},percent);await page.reload();
  await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:800});
  const portrait=(await page.locator('.artist-portrait-button').boundingBox())!,title=(await page.locator('.artist-hero > h1').boundingBox())!;
  expect(portrait.x).toBeGreaterThanOrEqual(0);expect(portrait.x+portrait.width).toBeLessThanOrEqual(720);expect(title.x+title.width).toBeLessThanOrEqual(721);await expect(page.getByRole('button',{name:'Expand M83 portrait',exact:true})).toBeEnabled();
  if(percent===100)await page.screenshot({path:'out/persistent-qa/artist-color-dark-compact.png',animations:'disabled'});
 }
});
