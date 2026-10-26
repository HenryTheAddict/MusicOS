import {test,expect,type Page} from '@playwright/test';
async function hold(page:Page){await page.addInitScript(()=>{const original=Element.prototype.animate;Element.prototype.animate=function(frames,options){const a=original.call(this,frames,options);if(this.matches('.menu-panel-surface,.menu-panel-items,.section-control>.menu-trigger:first-child')){a.pause();a.currentTime=0;}return a;};});}
async function finish(page:Page){await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('menu-')).forEach(a=>a.finish()));}
async function time(page:Page,ms:number){await page.evaluate(ms=>document.getAnimations().filter(a=>a.id.startsWith('menu-')).forEach(a=>a.currentTime=ms),ms);}
async function shell(page:Page){return page.getByRole('button',{name:'Choose section',exact:true}).evaluate(button=>{const node=button as HTMLElement,s=getComputedStyle(node,'::after'),panel=document.querySelector('.section-dropdown')!,surface=panel.querySelector('.menu-panel-surface')!,label=node.querySelector('.section-name')!.getBoundingClientRect();return {height:parseFloat(s.height),header:node.getBoundingClientRect().height,join:surface.getBoundingClientRect().height,shadow:s.boxShadow,labelX:label.x,labelY:label.y,text:label.width,caption:node.textContent};});}
test('section collapse returns one rounded shell to its header and delays navigation until exit',async({page})=>{
 await hold(page);await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);await finish(page);const opened=await shell(page);
 await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect(page.locator('.section-dropdown')).toHaveAttribute('inert','');await expect(trigger.locator('.section-name')).toHaveText('Library');await expect(page.getByRole('heading',{name:'Appearance',exact:true})).toHaveCount(0);
 await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);
 let previous=opened.height;for(const ms of [30,70,110,135]){await time(page,ms);const state=await shell(page);expect(state.height).toBeLessThan(previous);expect(state.height).toBeGreaterThanOrEqual(state.header);expect(Math.abs(state.height-state.header-Math.max(0,state.join-1))).toBeLessThan(1);expect(state.labelX).toBe(opened.labelX);expect(state.labelY).toBe(opened.labelY);previous=state.height;if(ms===70)await page.screenshot({path:'out/persistent-qa/selector-connected-collapse.png'});}
 await finish(page);await expect(page.locator('.section-dropdown')).toHaveCount(0);await expect(trigger.locator('.section-name')).toHaveText('Settings');await expect(page.getByRole('heading',{name:'Appearance',exact:true})).toBeVisible();expect(await trigger.evaluate(el=>el.getAnimations().length)).toBe(0);
});
test('reopening or replacing an exiting section menu cancels its pending navigation',async({page})=>{
 await hold(page);await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);await finish(page);await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);await time(page,70);const current=await shell(page);await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);expect(Math.abs((await shell(page)).height-current.height)).toBeLessThan(1);await finish(page);await expect(trigger.locator('.section-name')).toHaveText('Library');
 await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('button',{name:'MusicOS actions',exact:true}).click();await finish(page);await expect(trigger.locator('.section-name')).toHaveText('Library');await page.keyboard.press('Escape');await finish(page);await expect(page.getByRole('button',{name:'MusicOS actions',exact:true})).toBeFocused();
 await page.emulateMedia({reducedMotion:'reduce'});await trigger.click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect(page.locator('.section-dropdown')).toHaveCount(0);await expect(trigger.locator('.section-name')).toHaveText('Settings');
});
// Check the painted pixel, not only the shell's geometry. The header shell and
// portaled choices can have correct bounds while artwork paints between them.
async function opaqueChoicePixel(page:Page){
 const sample=await page.locator('.section-dropdown>.menu-panel-surface').evaluate(el=>{const r=el.getBoundingClientRect();return {x:Math.ceil(r.x+18),y:Math.ceil(r.y+Math.min(20,r.height/2)),fill:getComputedStyle(el).backgroundColor};});
 const joinedFill=await page.getByRole('button',{name:'Choose section',exact:true}).evaluate(el=>getComputedStyle(el,'::after').backgroundColor);
 expect(sample.fill).toBe(joinedFill);
 const png=await page.screenshot({clip:{x:sample.x,y:sample.y,width:1,height:1},scale:'css'});
 const colors=await page.evaluate(async({png,fill})=>{
  const image=new Image();image.src=`data:image/png;base64,${png}`;await image.decode();
  const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d')!;
  ctx.drawImage(image,0,0);const actual=[...ctx.getImageData(0,0,1,1).data];ctx.clearRect(0,0,1,1);ctx.fillStyle=fill;ctx.fillRect(0,0,1,1);return {actual,expected:[...ctx.getImageData(0,0,1,1).data]};
 },{png:png.toString('base64'),fill:sample.fill});
 expect(colors.expected[3]).toBe(255);
 expect(Math.max(...colors.actual.map((value,i)=>Math.abs(value-colors.expected[i])))).toBeLessThanOrEqual(2);
}
test('section choices paint above album art throughout opening, collapse and reversal',async({page})=>{
 await hold(page);
 await page.addInitScript(()=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:125,appearance:{mode:'light',source:'artwork',accent:'#b02020',saturation:100,tint:15}}));});
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 const trigger=page.getByRole('button',{name:'Choose section',exact:true}),menu=page.locator('.section-dropdown');
 await expect(page.locator('.album-artwork-stage img').first()).toBeVisible();
 await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
 await expect.poll(()=>page.evaluate(()=>document.documentElement.getAnimations().filter(a=>a.playState==='running').length)).toBe(0);
 await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);
 for(const ms of [35,70,110]){await time(page,ms);await opaqueChoicePixel(page);}
 await finish(page);await opaqueChoicePixel(page);
 // A late palette update must color both joined paints together.
 await page.evaluate(()=>{document.documentElement.style.setProperty('--surface-color','#d39e9a');document.documentElement.style.setProperty('--blob-color','#aa7772');document.documentElement.getAnimations().forEach(a=>a.finish());});
 await opaqueChoicePixel(page);await page.screenshot({path:'out/persistent-qa/selector-opaque-over-album.png'});
 await page.keyboard.press('Escape');await expect(menu).toHaveAttribute('inert','');await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);
 await time(page,70);await opaqueChoicePixel(page);await page.screenshot({path:'out/persistent-qa/selector-opaque-collapse.png'});
 await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);await opaqueChoicePixel(page);await finish(page);await opaqueChoicePixel(page);
 await page.keyboard.press('Escape');await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);await finish(page);await expect(menu).toHaveCount(0);await expect(trigger).toBeFocused();
});
async function seedQueue(page:Page){await page.route('**/src/fixtures.ts*',async route=>{const response=await route.fetch(),body=await response.text();const seed='let player = { ...emptyPlayback };';expect(body).toContain(seed);await route.fulfill({response,body:body.replace(seed,'let player = {...emptyPlayback,track:tracks[0],queue:new URLSearchParams(location.search).has("emptyQueue")?[tracks[0]]:[tracks[0],tracks[1],tracks[1],tracks[2]],queue_index:0,queue_entry_ids:[71,72,73,74],position:30};')});});}
async function settings(page:Page){await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();}
test('Up next preview and player share fully readable rows at all preset minimums',async({page})=>{
 await seedQueue(page);await page.goto('/?fixture=1');await settings(page);const picker=page.locator('.mini-presets');await picker.getByRole('button',{name:'Up next',exact:true}).click();
 for(const [name,width,height] of [['Compact',220,120],['Wide',320,120],['Standard',244,320]] as const){await picker.getByRole('group',{name:'Mini Player size'}).getByRole('button',{name,exact:true}).click();const preview=picker.locator('.mini-preview');await picker.scrollIntoViewIfNeeded();await expect(preview.locator('.mini-queue-view .queue-row')).toHaveCount(3);const bounds=await preview.evaluate(el=>{const list=el.querySelector('.mini-queue-list')!.getBoundingClientRect(),row=el.querySelector('.mini-queue-view .queue-row')!.getBoundingClientRect(),copy=el.querySelector('.mini-queue-view .queue-track-copy')!.getBoundingClientRect(),footer=el.querySelector('.mini-player-transport')!.getBoundingClientRect();return {list:{top:list.top,bottom:list.bottom},row:{top:row.top,bottom:row.bottom},copy:{top:copy.top,bottom:copy.bottom},footer:footer.top};});expect(bounds.copy.top).toBeGreaterThanOrEqual(bounds.list.top);expect(bounds.copy.bottom).toBeLessThanOrEqual(bounds.list.bottom+1);expect(bounds.copy.top).toBeGreaterThanOrEqual(bounds.row.top);expect(bounds.copy.bottom).toBeLessThanOrEqual(bounds.row.bottom+1);if(name!=='Standard'){await expect(preview.locator('.queue-entry-label')).toHaveText('Up next');expect(bounds.row.bottom).toBeLessThanOrEqual(bounds.list.bottom+1);}
  await page.screenshot({path:`out/persistent-qa/upnext-preview-${name.toLowerCase()}.png`});
 }
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback'))).toEqual([]);
 await picker.getByRole('button',{name:'Wide',exact:true}).click();await page.getByRole('button',{name:'MusicOS actions',exact:true}).click();await page.getByRole('menuitem',{name:'Mini Player',exact:true}).click();await page.setViewportSize({width:320,height:120});const mini=page.locator('.mini-shell>.mini-player');await expect.poll(()=>mini.evaluate(el=>el.getAnimations().length)).toBe(0);const footer=await mini.locator('.mini-player-footer').boundingBox();await mini.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(mini.locator('.queue-entry-label')).toHaveText('Up next');await page.waitForTimeout(200);const row=mini.locator('.queue-row').first(),copy=row.locator('.queue-track-copy'),region=mini.locator('.mini-queue-list');expect((await copy.boundingBox())!.y+(await copy.boundingBox())!.height).toBeLessThanOrEqual((await region.boundingBox())!.y+(await region.boundingBox())!.height+1);expect(await mini.locator('.mini-player-footer').boundingBox()).toEqual(footer);
 const ids=await mini.locator('.queue-row').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('data-queue-index')));await mini.getByRole('button',{name:'Close Up next'}).click();await mini.getByRole('button',{name:'Open Up next'}).click();expect(await mini.locator('.queue-row').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('data-queue-index')))).toEqual(ids);await expect.poll(()=>mini.evaluate(el=>el.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);await page.screenshot({path:'out/persistent-qa/upnext-wide-minimum.png'});
});

test('queue previews fit every interface scale and theme, including empty queues',async({page})=>{
 test.setTimeout(120000);await seedQueue(page);
 await page.addInitScript(()=>{const q=new URLSearchParams(location.search);localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(q.get('scale')),appearance:{mode:q.get('mode'),source:q.get('source'),accent:'#ff00ff',saturation:150,tint:15}}));});
 for(const width of [720,1280])for(const scale of [80,100,125,150])for(const [mode,source] of [['light','artwork'],['dark','artwork'],['light','blue'],['dark','custom']]){
  await page.setViewportSize({width,height:width===720?480:800});await page.goto(`/?fixture=1&scale=${scale}&mode=${mode}&source=${source}${scale===150?'&emptyQueue=1':''}`);await settings(page);const picker=page.locator('.mini-presets');await picker.getByRole('button',{name:'Up next',exact:true}).click();
  for(const density of ['Compact','Wide','Standard']){await picker.getByRole('group',{name:'Mini Player size'}).getByRole('button',{name:density,exact:true}).click();await picker.scrollIntoViewIfNeeded();const preview=picker.locator('.mini-preview');expect(await picker.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);const geometry=await preview.evaluate(el=>{const region=el.querySelector('.mini-queue-list')!.getBoundingClientRect(),first=el.querySelector('.mini-queue-view .queue-track-copy,.mini-queue-view .mini-queue-empty')!.getBoundingClientRect();return {top:first.top,bottom:first.bottom,regionTop:region.top,regionBottom:region.bottom};});expect(geometry.top,`${width}/${scale}/${mode}/${source}/${density}`).toBeGreaterThanOrEqual(geometry.regionTop-.5);expect(geometry.bottom).toBeLessThanOrEqual(geometry.regionBottom+1);}
 }
});

test('closed selector keeps the same paint through the final exit frames',async({page})=>{
 await hold(page);await page.goto('/?fixture=1');await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');await expect.poll(()=>page.evaluate(()=>document.documentElement.getAnimations().filter(a=>a.playState==='running').length)).toBe(0);
 const trigger=page.getByRole('button',{name:'Choose section',exact:true});
 const paint=()=>trigger.evaluate(el=>{const fill=getComputedStyle(el,'::after'),button=getComputedStyle(el);return {content:fill.content,background:fill.backgroundColor,border:fill.borderColor,height:parseFloat(fill.height),buttonFill:button.backgroundColor,transition:button.transitionProperty,headerZ:getComputedStyle(el.closest('.app-header')!).zIndex};});
 const closed=await paint();expect(closed.content).not.toBe('none');
 await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);await finish(page);
 await page.keyboard.press('Escape');await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);
 for(const ms of [110,130,139]){await time(page,ms);const p=await paint();expect(p.content).toBe(closed.content);expect(p.background).toBe(closed.background);expect(p.buttonFill).toBe(closed.buttonFill);expect(p.headerZ).toBe(closed.headerZ);expect(p.transition).toBe('none');}
 await finish(page);await expect(page.locator('.section-dropdown')).toHaveCount(0);expect(await paint()).toEqual(closed);await expect(trigger).toBeFocused();
 await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));expect(await paint()).toEqual(closed);expect(await trigger.evaluate(el=>el.getAnimations({subtree:true}).length)).toBe(0);
});
