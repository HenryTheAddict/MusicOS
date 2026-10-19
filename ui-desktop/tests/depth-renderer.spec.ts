import {expect,test} from '@playwright/test';
// Software WebGL cold starts are not a hardware frame-rate acceptance test.
test.setTimeout(60000);
test.use({launchOptions:{args:['--enable-webgl','--use-angle=swiftshader','--enable-unsafe-swiftshader']}});
async function start(page:import('@playwright/test').Page){await page.goto('/?fixture=1&depthFixture=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:"QA",interface_percent:100,coverflow_colors:false,artwork_depth:true,artwork_parallax:true})));await page.reload();await expect.poll(()=>page.locator('.depth-compositor canvas').getAttribute('data-resident')).not.toBe('0');await page.locator('.card-artwork').first().hover({position:{x:100,y:50}});await expect(page.locator('.card-artwork[data-depth-ready=true]').first()).toBeVisible({timeout:15000});}
test('depth quads stay aligned through scrolling without synthetic overscroll',async({page})=>{
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
 await expect(page.locator('.page-scroll')).toHaveCSS('scale','none');await expect(page.locator('.page-scroll')).toHaveCSS('translate','none');await expect(page.locator('.depth-compositor')).toHaveCSS('filter','none');
 await page.getByRole('button',{name:'Artists',exact:true}).click();await expect(page.locator('.depth-compositor')).toHaveCount(1);
 await page.getByRole('button',{name:'Open M83',exact:true}).click();await expect(page.locator('.depth-compositor')).toHaveCount(0);
});
test('artist detail stays still while grid depth remains available with reduced motion changes',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();
 const portrait=page.locator('.artist-portrait-button');await portrait.hover({position:{x:140,y:80}});await expect(page.locator('.depth-compositor')).toHaveCount(0);await expect(portrait.locator('.cover')).toHaveCSS('transform','none');await expect(portrait).toHaveCSS('transform','none');
 await page.getByRole('button',{name:'Back to Artists',exact:true}).click();await expect(page.locator('.depth-compositor')).toHaveCount(1);
 await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('.depth-compositor')).toHaveCount(0);await page.emulateMedia({reducedMotion:'no-preference'});await expect(page.locator('.depth-compositor')).toHaveCount(1);
});

test('album depth follows pointer while wheel edges leave content undistorted',async({page})=>{
 await start(page);const art=page.locator('.card-artwork[data-depth-ready=true]').first();const box=(await art.boundingBox())!;
 await page.mouse.move(box.x+box.width*.85,box.y+box.height*.65);await page.waitForTimeout(200);
 await expect.poll(()=>page.locator('.depth-compositor canvas').evaluate(el=>Math.max(...(el as any).depthGeometry().map((g:any)=>Math.abs(g.pointerX))))).toBeGreaterThan(2);
 expect(await art.evaluate(el=>(el as HTMLElement).style.scale)).toBe('');
 await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=0;el.dispatchEvent(new WheelEvent('wheel',{deltaY:-180,bubbles:true}));});
 await expect(page.locator('.page-scroll')).toHaveCSS('translate','none');await expect(page.locator('.page-scroll')).toHaveCSS('scale','none');await expect(art.locator('img')).toHaveCSS('filter','none');
});

test('open album stays normal and artwork expands with focus restored',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();const art=page.getByRole('button',{name:'Expand Discovery artwork',exact:true});await art.hover();const box=(await art.boundingBox())!;
 await page.waitForTimeout(200);await expect(page.locator('.album-artwork-stage .depth-compositor')).toHaveCount(1);expect((await art.boundingBox())!.width).toBeCloseTo(box.width,1);
 await art.click();await expect(page.getByRole('dialog',{name:'Discovery artwork'})).toBeVisible();await expect(page.getByRole('dialog').getByRole('img',{name:'Discovery',exact:true})).toBeVisible();
 await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).not.toBeVisible();await expect(art).toBeFocused();
});
test('fresh depth never flashes and GPU loss restores the ordinary image',async({page})=>{
 await page.addInitScript(()=>{const original=Element.prototype.animate;Object.assign(window,{depthRevealCount:0});Element.prototype.animate=function(frames,options){if(this instanceof HTMLElement&&this.matches('.card-artwork,.album-artwork,.viewer-artwork')&&typeof options==='object'&&options?.duration===220)(window as any).depthRevealCount++;return original.call(this,frames,options);};});
 await page.goto('/?fixture=1&depthFixture=1&freshDepth=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:100,artwork_depth:true})));await page.reload();await page.locator('.card-artwork').first().hover();await expect(page.locator('.card-artwork[data-depth-ready=true]').first()).toBeVisible();expect(await page.evaluate(()=>(window as any).depthRevealCount)).toBe(0);
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.locator('.album-artwork').hover();await expect(page.locator('.album-artwork[data-depth-ready=true]')).toBeVisible();const count=await page.evaluate(()=>(window as any).depthRevealCount);await page.getByRole('button',{name:'Expand Discovery artwork'}).click();await page.locator('.viewer-artwork').hover();await expect(page.locator('.viewer-artwork[data-depth-ready=true]')).toBeVisible();expect(await page.evaluate(()=>(window as any).depthRevealCount)).toBe(count);
 await page.locator('.viewer-depth-stage canvas').evaluate(el=>(el as HTMLCanvasElement).getContext('webgl2')!.getExtension('WEBGL_lose_context')!.loseContext());await expect(page.locator('.viewer-artwork > img')).toHaveCSS('opacity','1');await page.keyboard.press('Escape');
});

test('expanded depth waits for entrance animation and matches the final image bounds',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Expand Discovery artwork',exact:true}).evaluate(el=>{el.querySelector('img')!.dataset.artworkVersion='42';});await page.getByRole('button',{name:'Expand Discovery artwork',exact:true}).click();
 await page.locator('.viewer-artwork').hover();await expect(page.locator('.viewer-artwork[data-depth-ready=true]')).toBeVisible();await expect(page.locator('.viewer-artwork img')).toHaveAttribute('data-artwork-version','42');
 await expect.poll(()=>page.locator('.viewer-depth-stage canvas').evaluate(canvas=>{const bounds=canvas.getBoundingClientRect(),art=document.querySelector('.viewer-artwork')!.getBoundingClientRect(),g=(canvas as any).depthGeometry()[0];return g?Math.max(Math.abs(bounds.x+g.left-art.x),Math.abs(bounds.y+g.top-art.y),Math.abs(g.width-art.width),Math.abs(g.height-art.height)):999;})).toBeLessThan(1.2);
 const art=page.locator('.viewer-artwork'),box=(await art.boundingBox())!;await page.mouse.move(box.x+box.width*.9,box.y+box.height*.75);
 await expect.poll(()=>page.locator('.viewer-depth-stage canvas').evaluate(canvas=>Math.abs((canvas as any).depthGeometry()[0]?.pointerX??0))).toBeGreaterThan(2);
 await page.screenshot({path:'out/rebuild-qa/expanded-depth-aligned.png',animations:'disabled'});
});


test('depth retains its compositor and reconciles quads after wide resize, reflow and scrolling near the end',async({page})=>{
 await start(page);await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=el.scrollHeight;});await page.mouse.move(4,4);
 for(const size of [{width:1275,height:908},{width:1800,height:1230},{width:720,height:480},{width:1440,height:900}]){
  await page.locator('.depth-compositor canvas').evaluate(el=>el.setAttribute('data-old-surface','true'));
  await page.setViewportSize(size);await expect(page.locator('canvas[data-old-surface]')).toHaveCount(1);
  await expect.poll(()=>page.locator('.depth-compositor canvas').evaluate(canvas=>{
   const bounds=canvas.getBoundingClientRect(),values=(canvas as any).depthGeometry();
   return values.length>0&&values.every((g:any)=>{
    const art=[...document.querySelectorAll<HTMLImageElement>('img[data-artwork-path]')].find(i=>i.dataset.artworkPath===g.path)?.closest('.card-artwork');if(!art)return false;const r=art.getBoundingClientRect();return Math.max(Math.abs(bounds.left+g.left-r.left),Math.abs(bounds.top+g.top-r.top),Math.abs(g.width-r.width),Math.abs(g.height-r.height))<1.2;
   })&&getComputedStyle(canvas.parentElement!).visibility==='visible';
  }),{timeout:15000}).toBe(true).catch(async error=>{console.log(await page.locator('.depth-compositor canvas').evaluate(canvas=>({visibility:getComputedStyle(canvas.parentElement!).visibility,lost:(canvas as HTMLCanvasElement).getContext('webgl2')?.isContextLost(),geometry:(canvas as any).depthGeometry(),bounds:canvas.getBoundingClientRect().toJSON(),arts:[...document.querySelectorAll('.card-artwork')].map(el=>({path:el.querySelector('img')?.getAttribute('data-artwork-path'),box:el.getBoundingClientRect().toJSON()}))})));throw error;});
  await expect(page.locator('.depth-compositor')).toHaveCSS('translate','none');await expect(page.locator('.depth-compositor')).toHaveCSS('scale','none');
 }
 await page.screenshot({path:'out/rebuild-qa/depth-wide-resize-repaired.png',animations:'disabled'});
});

test('large grids reuse prepared images across reversals and recover a lost context',async({page})=>{
 await page.goto('/?fixture=1&depthFixture=1&largeAlbums=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:100,artwork_depth:true})));await page.reload();const canvas=page.locator('.depth-compositor canvas');await page.locator('.card-artwork').first().hover();await expect(page.locator('.card-artwork[data-depth-ready=true]').first()).toBeVisible({timeout:15000});await canvas.evaluate(el=>el.setAttribute('data-persistent','true'));
 for(const offset of [650,1400,2500,1400,650,0]){await page.locator('.page-scroll').evaluate((el,y)=>{el.scrollTop=y;},offset);await page.waitForTimeout(100);}
 await expect(page.locator('canvas[data-persistent]')).toHaveCount(1);await expect.poll(()=>canvas.getAttribute('data-cache-hits')).not.toBe('0');expect(Number(await canvas.getAttribute('data-ram-bytes'))).toBeLessThanOrEqual(256*1024*1024);expect(Number(await canvas.getAttribute('data-texture-bytes'))).toBeLessThanOrEqual(128*1024*1024);
 await canvas.evaluate(el=>(el as HTMLCanvasElement).getContext('webgl2')!.getExtension('WEBGL_lose_context')!.loseContext());await expect(page.locator('.card-artwork > img').first()).toHaveCSS('opacity','1');await page.locator('.card-artwork').first().hover();await expect(page.locator('.card-artwork[data-depth-ready=true]').first()).toBeVisible({timeout:15000});await expect(page.locator('.depth-compositor')).toHaveCount(1);
});
