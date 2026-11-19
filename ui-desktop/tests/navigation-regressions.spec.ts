import {test,expect,type Page} from '@playwright/test'; import {settleAnimations} from './support/ui';
test.use({launchOptions:{args:['--enable-webgl','--use-angle=swiftshader','--enable-unsafe-swiftshader']}});
async function section(page:Page,name:string){await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name,exact:true}).click();}
test('navigation and context menus animate, while data and theme controls do not replay page entry',async({page})=>{
 await page.addInitScript(()=>{const animate=Element.prototype.animate;(window as any).navigationMotion=[];Element.prototype.animate=function(frames,options){const animation=animate.call(this,frames,options);(window as any).navigationMotion.push({animation,frames,options,target:this.className});return animation;};});
 await page.goto('/?fixture=1');await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');await settleAnimations(page);await page.evaluate(()=>{(window as any).navigationMotion=[];});await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 expect(await page.evaluate(()=>(window as any).navigationMotion.some((m:any)=>m.animation.id==='album-detail-reveal'))).toBe(true);
 expect(await page.evaluate(()=>(window as any).navigationMotion.some((m:any)=>m.animation.id==='navigation-content'&&m.target==='page-content'))).toBe(false);
 await page.getByRole('button',{name:'musicOS actions',exact:true}).click();await expect.poll(()=>page.evaluate(()=>(window as any).navigationMotion.some((m:any)=>m.animation.id==='menu-surface'))).toBe(true);await page.keyboard.press('Escape');
 await section(page,'Settings');await page.getByRole('tab',{name:'Audio',exact:true}).click();
 const count=()=>page.evaluate(()=>(window as any).navigationMotion.filter((m:any)=>m.animation.id==='content-change').length);const before=await count();expect(before).toBeGreaterThan(0);await page.getByRole('button',{name:'8 bands',exact:true}).click();expect(await count()).toBe(before);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('tab',{name:'Library',exact:true}).click();expect(await count()).toBe(before);
});
test('adaptive surfaces follow artwork and return after toggling color sources',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode:'light',source:'artwork',accent:'#0064df',saturation:100,tint:15}})));
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');const artworkSurface=await page.locator('.current-theme').evaluate(el=>getComputedStyle(el).backgroundColor);
 await section(page,'Settings');await page.getByRole('button',{name:'Theme colors',exact:true}).click();await page.getByRole('menuitemradio',{name:'Original blue',exact:true}).click();await expect(page.locator('.current-theme')).toHaveCSS('background-color','rgb(201, 239, 255)');
 await page.getByRole('button',{name:'Theme colors',exact:true}).click();await page.getByRole('menuitemradio',{name:'From artwork',exact:true}).click();await section(page,'Library');await expect(page.locator('.current-theme')).toHaveCSS('background-color',artworkSurface);
 await page.screenshot({path:'out/persistent-qa/adaptive-repaired.png',animations:'disabled'});
});
test('Settings navigation retains Library images and depth textures without hidden drawing or reupload',async({page})=>{
 await page.setViewportSize({width:1280,height:600});await page.addInitScript(()=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({artwork_depth:true}));(window as any).uploads=0;(window as any).draws=0;const upload=WebGL2RenderingContext.prototype.texImage2D,draw=WebGL2RenderingContext.prototype.drawArrays;WebGL2RenderingContext.prototype.texImage2D=function(...args:any[]){(window as any).uploads++;return (upload as any).apply(this,args);};WebGL2RenderingContext.prototype.drawArrays=function(...args:Parameters<WebGL2RenderingContext['drawArrays']>){(window as any).draws++;return draw.apply(this,args);};});
 await page.goto('/?fixture=1&depthFixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.mouse.move(1,1);const canvas=page.locator('.library-view .depth-compositor canvas');await expect.poll(async()=>Number(await canvas.getAttribute('data-resident'))).toBeGreaterThan(5);await page.waitForTimeout(300);
 await canvas.evaluate(el=>(window as any).retainedCanvas=el);await page.locator('.library-view .card-artwork img').first().evaluate(el=>(window as any).retainedImage=el);
 await section(page,'Settings');await page.waitForTimeout(200);const before=await page.evaluate(()=>({uploads:(window as any).uploads,draws:(window as any).draws}));await page.waitForTimeout(200);expect(await page.evaluate(()=>(window as any).draws)).toBe(before.draws);
 await section(page,'Library');expect(await canvas.evaluate(el=>el===(window as any).retainedCanvas)).toBe(true);expect(await page.locator('.library-view .card-artwork img').first().evaluate(el=>el===(window as any).retainedImage)).toBe(true);await page.waitForTimeout(250);expect(await page.evaluate(()=>(window as any).uploads)).toBe(before.uploads);
});
