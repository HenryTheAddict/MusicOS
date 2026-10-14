import {expect,test} from '@playwright/test';
test.use({launchOptions:{args:['--enable-webgl','--use-angle=swiftshader','--enable-unsafe-swiftshader']}});
async function start(page:import('@playwright/test').Page){await page.goto('/?fixture=1&depthFixture=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:"QA",interface_percent:100,coverflow_colors:false,artwork_depth:true,artwork_parallax:true})));await page.reload();await expect(page.locator('.card-artwork[data-depth-ready=true]').first()).toBeVisible();}
test('depth quads stay aligned through scroll, overscroll and navigation',async({page})=>{
 await start(page);await page.mouse.move(4,4);await page.waitForTimeout(350);
 const check=()=>page.locator('.depth-compositor canvas').evaluate(canvas=>{
  const bounds=canvas.getBoundingClientRect(),geometry=(canvas as any).depthGeometry();return geometry.map((g:any)=>{
   const image=[...document.querySelectorAll<HTMLImageElement>('img[data-artwork-path]')].find(el=>el.dataset.artworkPath===g.path&&el.closest('.card-artwork'));
   const box=image!.closest('.card-artwork')!.getBoundingClientRect();return {dx:Math.abs(bounds.left+g.left-box.left),dy:Math.abs(bounds.top+g.top-box.top),motion:Math.abs(g.motion)};
  });});
 await expect.poll(async()=>{const values=await check();return values.length>=3&&values.every((v:any)=>v.dx<1.2&&v.dy<1.2&&v.motion<=14.01);}).toBe(true);
 await page.locator('.page-scroll').evaluate(el=>{el.scrollTop+=220;});await page.waitForTimeout(220);
 await expect.poll(async()=>{const values=await check();return values.length>0&&values.every((v:any)=>v.dx<1.2&&v.dy<1.2);}).toBe(true);
 await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=0;el.dispatchEvent(new WheelEvent('wheel',{deltaY:-250,bubbles:true}));});
 expect(await page.locator('.page-scroll').evaluate(el=>parseFloat((el as HTMLElement).style.getPropertyValue('--overscroll-blur')))).toBeGreaterThan(0);
 await expect.poll(()=>page.locator('.page-scroll').evaluate(el=>parseFloat((el as HTMLElement).style.getPropertyValue('--overscroll-blur')))).toBeLessThan(.1);
 await page.getByRole('button',{name:'Artists',exact:true}).click();await expect(page.locator('.depth-compositor')).toHaveCount(1);
 await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(page.locator('.depth-compositor[data-hero=true]')).toHaveCount(1);await expect(page.locator('.depth-compositor[data-hero=false]')).toHaveCount(0);
});
test('artist depth follows the pointer and reduced motion can be toggled back',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(page.locator('.artist-portrait-button[data-depth-ready=true]')).toBeVisible();
 const portrait=page.locator('.artist-portrait-button');await portrait.hover();const box=(await portrait.boundingBox())!;await portrait.hover({position:{x:box.width*.85,y:box.height*.65}});
 await expect.poll(()=>page.locator('.depth-compositor canvas').evaluate(el=>(el as any).depthGeometry()[0]?.pointerX??0)).toBeGreaterThan(2);
 await page.getByRole('heading',{name:'M83',exact:true}).hover();await expect.poll(()=>page.locator('.depth-compositor canvas').evaluate(el=>Math.abs((el as any).depthGeometry()[0]?.pointerX??0))).toBeLessThan(.2);
 await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('.depth-compositor')).toHaveCount(0);await page.emulateMedia({reducedMotion:'no-preference'});await expect(page.locator('.artist-portrait-button[data-depth-ready=true]')).toBeVisible();await expect(page.locator('.depth-compositor')).toHaveCount(1);
});

test('album depth follows pointer without box rotation and elastic edges settle',async({page})=>{
 await start(page);const art=page.locator('.card-artwork[data-depth-ready=true]').first();const box=(await art.boundingBox())!;
 await page.mouse.move(box.x+box.width*.85,box.y+box.height*.65);await page.waitForTimeout(200);
 await expect.poll(()=>page.locator('.depth-compositor canvas').evaluate(el=>Math.max(...(el as any).depthGeometry().map((g:any)=>Math.abs(g.pointerX))))).toBeGreaterThan(2);
 expect(await art.evaluate(el=>(el as HTMLElement).style.scale)).toBe('');
 await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=0;el.dispatchEvent(new WheelEvent('wheel',{deltaY:-180,bubbles:true}));});
 await expect.poll(()=>page.locator('.page-scroll').evaluate(el=>parseFloat((el as HTMLElement).style.getPropertyValue('--overscroll-y')))).toBeGreaterThan(.5);
 await expect.poll(()=>page.locator('.page-scroll').evaluate(el=>Math.abs(parseFloat((el as HTMLElement).style.getPropertyValue('--overscroll-y'))))).toBeLessThan(.1);
});

test('open album stays normal and artwork expands with focus restored',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();const art=page.getByRole('button',{name:'Expand Discovery artwork',exact:true});await art.hover();const box=(await art.boundingBox())!;
 await page.waitForTimeout(200);await expect(page.locator('.album-artwork-stage .depth-compositor')).toHaveCount(1);expect((await art.boundingBox())!.width).toBeCloseTo(box.width,1);
 await art.click();await expect(page.getByRole('dialog',{name:'Discovery artwork'})).toBeVisible();await expect(page.getByRole('dialog').getByRole('img',{name:'Discovery',exact:true})).toBeVisible();
 await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).not.toBeVisible();await expect(art).toBeFocused();
});
test('new depth reveals once and GPU loss restores the ordinary image',async({page})=>{
 await page.addInitScript(()=>{const original=Element.prototype.animate;Object.assign(window,{depthRevealCount:0});Element.prototype.animate=function(frames,options){if(this instanceof HTMLElement&&this.matches('.card-artwork,.album-artwork,.viewer-artwork')&&typeof options==='object'&&options?.duration===220)(window as any).depthRevealCount++;return original.call(this,frames,options);};});
 await page.goto('/?fixture=1&depthFixture=1&freshDepth=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:100,artwork_depth:true})));await page.reload();await expect(page.locator('.card-artwork[data-depth-ready=true]').first()).toBeVisible();await expect.poll(()=>page.evaluate(()=>(window as any).depthRevealCount)).toBeGreaterThan(0);
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await expect(page.locator('.album-artwork[data-depth-ready=true]')).toBeVisible();const count=await page.evaluate(()=>(window as any).depthRevealCount);await page.getByRole('button',{name:'Expand Discovery artwork'}).click();await expect(page.locator('.viewer-artwork[data-depth-ready=true]')).toBeVisible();expect(await page.evaluate(()=>(window as any).depthRevealCount)).toBe(count);
 await page.locator('.viewer-depth-stage canvas').evaluate(el=>(el as HTMLCanvasElement).getContext('webgl2')!.getExtension('WEBGL_lose_context')!.loseContext());await expect(page.locator('.viewer-artwork > img')).toHaveCSS('opacity','1');await page.keyboard.press('Escape');
});

test('expanded depth waits for entrance animation and matches the final image bounds',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Expand Discovery artwork',exact:true}).evaluate(el=>{el.querySelector('img')!.dataset.artworkVersion='42';});await page.getByRole('button',{name:'Expand Discovery artwork',exact:true}).click();
 await expect(page.locator('.viewer-artwork[data-depth-ready=true]')).toBeVisible();await expect(page.locator('.viewer-artwork img')).toHaveAttribute('data-artwork-version','42');
 await expect.poll(()=>page.locator('.viewer-depth-stage canvas').evaluate(canvas=>{const bounds=canvas.getBoundingClientRect(),art=document.querySelector('.viewer-artwork')!.getBoundingClientRect(),g=(canvas as any).depthGeometry()[0];return g?Math.max(Math.abs(bounds.x+g.left-art.x),Math.abs(bounds.y+g.top-art.y),Math.abs(g.width-art.width),Math.abs(g.height-art.height)):999;})).toBeLessThan(1.2);
 const art=page.locator('.viewer-artwork'),box=(await art.boundingBox())!;await page.mouse.move(box.x+box.width*.9,box.y+box.height*.75);
 await expect.poll(()=>page.locator('.viewer-depth-stage canvas').evaluate(canvas=>Math.abs((canvas as any).depthGeometry()[0]?.pointerX??0))).toBeGreaterThan(2);
 await page.screenshot({path:'out/rebuild-qa/expanded-depth-aligned.png',animations:'disabled'});
});
