import {test,expect,type Page} from '@playwright/test';
async function hold(page:Page){await page.addInitScript(()=>{const original=Element.prototype.animate;(window as any).transfers=[];Element.prototype.animate=function(frames,options){const a=original.call(this,frames,options);if(this.classList.contains('album-cover-transfer')){(window as any).transfers.push(a);a.pause();a.currentTime=0;}return a;};});}
async function end(page:Page){await expect.poll(async()=>{await page.evaluate(()=>{for(const a of (window as any).transfers)if(a.effect?.target?.isConnected&&a.playState!=='finished')a.finish();});return page.locator('.album-cover-transfer').count();},{timeout:1500}).toBe(0);}
for(const kind of ['album','artist'] as const)test(`${kind} transfer follows the displayed cover and lands without a positional jump`,async({page})=>{
 await hold(page);await page.goto('/?fixture=1&contributors=1');await page.getByRole('button',{name:kind==='album'?'Albums':'Artists',exact:true}).click();
 const opener=page.getByRole('button',{name:kind==='album'?'Open Junk':'Open M83',exact:true});await expect.poll(()=>opener.locator('img.cover').first().evaluate((im:HTMLImageElement)=>im.complete&&im.naturalWidth>0)).toBe(true);const source=(await opener.locator('img.cover').first().boundingBox())!;
 await opener.click();const layer=page.locator('.album-cover-transfer');await expect(layer).toHaveCount(1);const start=(await layer.boundingBox())!;expect(Math.abs(start.x-source.x)).toBeLessThan(1);expect(Math.abs(start.y-source.y)).toBeLessThan(1);
 const target=page.locator(kind==='album'?'.album-artwork-stage':'.artist-portrait-button');await expect(target).toHaveCSS('opacity','0');const first=(await target.boundingBox())!;expect(Math.abs(first.width-first.height)).toBeLessThan(1);
 await page.evaluate(()=>{(window as any).transfers.at(-1).currentTime=160;});const middle=(await layer.boundingBox())!;expect(Math.abs(middle.x-source.x)+Math.abs(middle.y-source.y)).toBeGreaterThan(1);
 await page.evaluate(()=>{(window as any).transfers.at(-1).currentTime=319;});const last=(await layer.boundingBox())!;await end(page);const visible=(await target.boundingBox())!;expect(Math.abs(last.x-visible.x)).toBeLessThan(1);expect(Math.abs(last.y-visible.y)).toBeLessThan(1);expect(Math.abs(last.width-visible.width)).toBeLessThan(1);
 const scroll=await page.locator('.page-scroll').evaluate(n=>n.scrollTop);await page.evaluate(()=>window.dispatchEvent(new Event('musicos-artwork-invalidated')));await expect.poll(()=>page.locator('.page-scroll').evaluate(n=>n.scrollTop)).toBe(scroll);
 await page.locator('.sidebar-back').click();await expect(layer).toHaveCount(1);await end(page);const returned=(await opener.locator('img.cover').first().boundingBox())!;expect(Math.abs(returned.y-source.y)).toBeLessThan(1);expect(await page.locator('[data-album-entering]').count()).toBe(0);
});
for(const mode of ['light','dark'])test(`${mode}: Back to Home hands the cover to the card without fading it in again`,async({page})=>{
 await hold(page);
 await page.addInitScript(mode=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode,source:'artwork',accent:'#0064df',saturation:100,tint:15}})),mode);
 await page.goto('/?fixture=1');
 const opener=page.getByRole('button',{name:'Open Plastic Beach',exact:true});
 const artwork=opener.locator('.card-artwork');
 await expect.poll(()=>artwork.locator('img.cover').evaluate((im:HTMLImageElement)=>im.complete&&im.naturalWidth>0)).toBe(true);
 await opener.click();await end(page);
 const root=page.locator('html');
 await expect(root).toHaveAttribute('data-theme-mode',mode);
 const color=await root.evaluate(el=>el.style.getPropertyValue('--surface-color'));
 await page.locator('.sidebar-back').click();
 const layer=page.locator('.album-cover-transfer');
 await expect(layer).toHaveCount(1);
 await expect(artwork).toHaveAttribute('data-album-entering','true');
 await expect.poll(()=>layer.evaluate(el=>el.getAnimations().length)).toBe(1);
 const handoff=await artwork.evaluate(el=>new Promise<{opacity:number;visibility:string;ready:boolean;color:string}>(resolve=>{
  const observer=new MutationObserver(()=>{
   if(document.querySelector('.album-cover-transfer'))return;
   observer.disconnect();
   const style=getComputedStyle(el),image=el.querySelector('img');
   resolve({opacity:Number(style.opacity),visibility:style.visibility,ready:!!(image?.complete&&image.naturalWidth),color:document.documentElement.style.getPropertyValue('--surface-color')});
  });
  observer.observe(document.body,{childList:true});
  for(const animation of document.querySelector('.album-cover-transfer')!.getAnimations())animation.finish();
 }));
 expect(handoff).toEqual({opacity:1,visibility:'visible',ready:true,color});
});
test('a late album description cannot stretch the cover or scroll the page',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:1500});const stage=page.locator('.album-artwork-stage'),before=(await stage.boundingBox())!;
 await page.locator('.album-footer').evaluate(n=>{const about=document.createElement('section');about.className='album-description';about.style.height='220px';about.textContent='Late album details';n.prepend(about);});const after=(await stage.boundingBox())!;expect(after).toEqual(before);expect(await page.locator('.page-scroll').evaluate(n=>n.scrollTop)).toBe(0);
});
test('scrolled virtual album return preserves the exact saved position through delayed artwork',async({page})=>{
 await page.goto('/?fixture=1&largeAlbums=1');await page.getByRole('button',{name:'Albums',exact:true}).click();const scroller=page.locator('.page-scroll');await scroller.evaluate(n=>n.scrollTop=1800);await expect.poll(()=>scroller.evaluate(n=>n.scrollTop)).toBe(1800);await page.waitForTimeout(100);
 await page.locator('.page-scroll').evaluate(root=>{const bounds=root.getBoundingClientRect();const visible=Array.from(root.querySelectorAll<HTMLElement>('.collection-card')).find(n=>{const r=n.getBoundingClientRect();return r.top>bounds.top+30&&r.bottom<bounds.bottom-20;});if(!visible)throw Error('No fully visible album');visible.dataset.scrollTest='true';});const card=page.locator('[data-scroll-test=true]');await card.click();await expect(page.locator('.album-detail')).toBeVisible();await page.locator('.sidebar-back').click();await expect.poll(()=>scroller.evaluate(n=>n.scrollTop)).toBe(1800);await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:1500});await page.evaluate(()=>window.dispatchEvent(new Event('musicos-artwork-invalidated')));await page.waitForTimeout(100);expect(await scroller.evaluate(n=>n.scrollTop)).toBe(1800);
});
test('rapid reversal, resize, and Reduced Motion leave no transfer layers or movement',async({page})=>{
 await hold(page);await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();await page.evaluate(()=>(window as any).transfers.at(-1).currentTime=140);const layer=page.locator('.album-cover-transfer'),shown=(await layer.boundingBox())!;await page.locator('.sidebar-back').click();await expect(layer).toHaveCount(1);const reverse=(await layer.boundingBox())!;expect(Math.abs(reverse.x-shown.x)).toBeLessThan(1);expect(Math.abs(reverse.y-shown.y)).toBeLessThan(1);await page.setViewportSize({width:1200,height:760});await expect(layer).toHaveCount(0);expect(await page.locator('[data-album-entering]').count()).toBe(0);
 await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('button',{name:'Open Junk',exact:true}).click();await expect(layer).toHaveCount(0);expect(await page.locator('.album-detail').evaluate(n=>n.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);
});

test('album return into an artist strip lands at the actual unshifted cover',async({page})=>{await hold(page);await page.goto('/?fixture=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await end(page);const button=page.getByRole('button',{name:'Junk',exact:true});await button.click();await end(page);await page.locator('.sidebar-back').click();const layer=page.locator('.album-cover-transfer');await expect(layer).toHaveCount(1);await page.waitForTimeout(190);await page.evaluate(()=>(window as any).transfers.at(-1).currentTime=319);const last=(await layer.boundingBox())!;await end(page);const actual=(await button.locator('.cover').boundingBox())!;expect(Math.abs(last.x-actual.x)).toBeLessThan(1);expect(Math.abs(last.y-actual.y)).toBeLessThan(1);});

test('a changing destination retargets from the visible shape instead of snapping',async({page})=>{await hold(page);await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();const layer=page.locator('.album-cover-transfer');await expect(layer).toHaveCount(1);await page.evaluate(()=>(window as any).transfers.at(-1).currentTime=140);const shown=(await layer.boundingBox())!;await page.locator('.album-artwork-stage').evaluate(n=>n.style.width='176px');await expect.poll(()=>page.evaluate(()=>(window as any).transfers.length)).toBe(2);const next=(await layer.boundingBox())!;expect(Math.abs(shown.x-next.x)).toBeLessThan(1);expect(Math.abs(shown.y-next.y)).toBeLessThan(1);expect(Math.abs(shown.width-next.width)).toBeLessThan(1);await end(page);expect((await page.locator('.album-artwork-stage').boundingBox())!.width).toBe(176);});

test('Back preserves an expanded artist biography and its scrolled album position',async({page})=>{await page.goto('/?fixture=1&longBiography=1');await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open M83',exact:true}).click();await page.getByRole('button',{name:'Read more',exact:true}).click();await expect.poll(()=>page.locator('.artist-hero').evaluate(n=>n.getAnimations().filter(a=>a.playState==='running').length)).toBe(0);const album=page.getByRole('button',{name:'Junk',exact:true});await page.locator('.page-scroll').evaluate(n=>n.scrollTop=500);await album.scrollIntoViewIfNeeded();const scroll=await page.locator('.page-scroll').evaluate(n=>n.scrollTop);expect(scroll).toBeGreaterThan(0);await album.click();await expect(page.locator('.album-detail')).toBeVisible();await page.locator('.sidebar-back').click();await expect(page.locator('.artist-hero')).toHaveAttribute('data-biography-expanded','true');await expect.poll(()=>page.locator('.page-scroll').evaluate(n=>n.scrollTop)).toBe(scroll);await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:1500});expect(await page.locator('.page-scroll').evaluate(n=>n.scrollTop)).toBe(scroll);});
