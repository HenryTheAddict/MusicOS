import {contrast} from '../src/appearance';
import {test,expect} from '@playwright/test';
const look={mode:'dark',source:'blue',accent:'#0064df',saturation:100,tint:15};
test('saved name is used verbatim in Home, sidebar, collections and name preview',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({display_name:'Henry',interface_percent:100})));
 await page.goto('/?fixture=1');
 await expect(page.locator('.sidebar-home')).toHaveText('Henry');
 await expect(page.locator('.home-heading h1')).toHaveText('Henry');
 await expect(page.locator('.home-collections h2')).toHaveText('Henry');
 await expect(page.locator('.sidebar-playlists h2')).toHaveText('Henry');
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'General',exact:true}).click();
 await page.getByLabel('Name',{exact:true}).fill('Zoë');await expect(page.locator('.name-preview')).toHaveText('Zoë');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Library',exact:true}).click();await expect(page.locator('.sidebar-home')).toHaveText('Zoë');await expect(page.locator('.home-heading h1')).toHaveText('Zoë');
});
for(const [mode,system] of [['dark','light'],['system','dark'],['light','dark']] as const)test(`first rendered frame respects ${mode} with ${system} system appearance`,async({page})=>{
 await page.emulateMedia({colorScheme:system});
 await page.addInitScript(({look,mode})=>{
  const appearance={...look,mode};(window as any).__MUSICOS_LAUNCH_APPEARANCE__=appearance;
  localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance,display_name:'Henry'}));
  (window as any).__launchFrames=[];
  const sample=()=>{if(document.querySelector('.app-shell'))(window as any).__launchFrames.push(getComputedStyle(document.documentElement).getPropertyValue('--shell-color'));if((window as any).__launchFrames.length<100)requestAnimationFrame(sample);};requestAnimationFrame(sample);
 },{look,mode});
 await page.goto('/?fixture=1');await expect(page.locator('.sidebar-home')).toHaveText('Henry');await page.waitForTimeout(1600);
 const colors=await page.evaluate(()=>(window as any).__launchFrames as string[]);expect(colors.length).toBeGreaterThan(10);
 for(const value of colors){const rgb=value.startsWith('#')?[1,3,5].map(i=>parseInt(value.slice(i,i+2),16)):(value.match(/[\d.]+/g)??[]).slice(0,3).map(Number);if(mode==='dark'||mode==='system')expect(Math.max(...rgb)).toBeLessThan(90);else expect(Math.max(...rgb)).toBeGreaterThan(140);}
});
test('context exit stays at its pointer without flashing or stealing a replacement panel',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Songs',exact:true}).click();const rows=page.locator('.track-row');
 await rows.nth(0).click({button:'right'});await page.waitForTimeout(170);
 await page.evaluate(()=>{(window as any).__exit=[];const started=performance.now();const sample=()=>{const node=document.querySelector<HTMLElement>('.dropdown');if(node){const box=node.getBoundingClientRect();(window as any).__exit.push({x:box.x,y:box.y,opacity:Number(getComputedStyle(node).opacity),state:node.dataset.state});}if(performance.now()-started<300)requestAnimationFrame(sample);};sample();});
 await page.keyboard.press('Escape');await page.waitForTimeout(310);
 const frames=await page.evaluate(()=>(window as any).__exit.filter((f:any)=>f.state==='closed'));expect(frames.length).toBeGreaterThan(2);
 for(let i=1;i<frames.length;i++){expect(Math.abs(frames[i].x-frames[0].x)).toBeLessThan(5);expect(Math.abs(frames[i].y-frames[0].y)).toBeLessThan(5);expect(frames[i].opacity).toBeLessThanOrEqual(frames[i-1].opacity+.025);}
 await expect(page.locator('.dropdown')).toHaveCount(0);
 await rows.nth(0).click({button:'right'});await page.keyboard.press('Escape');await rows.nth(1).click({button:'right'});await page.keyboard.press('ArrowDown');await expect(page.locator('.dropdown')).toHaveCount(1);expect(await page.locator('.dropdown').evaluate(el=>el.contains(document.activeElement))).toBe(true);
 await page.keyboard.press('Escape');await page.getByRole('button',{name:'Create collection',exact:true}).click();await expect(page.getByRole('menuitem',{name:'New playlist',exact:true})).toBeVisible();
});
test('existing playlists have no M3U toolbar action; file import stays at creation',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/1',name:'Test list',kind:'manual',track_ids:[1,2],artwork_choice:'collage'}])));
 await page.goto('/?fixture=1&sparkReady=1');await page.locator('.home-collection-open').click();await expect(page.locator('.playlist-tools')).not.toContainText('M3U');await expect(page.getByRole('button',{name:'Edit details',exact:true}).locator('svg')).toBeVisible();await page.getByRole('button',{name:'Add Music',exact:true}).click();await expect(page.getByRole('dialog',{name:'Add Music',exact:true})).not.toContainText('M3U');await page.getByRole('button',{name:'Close song selector'}).click();
 await page.locator('.sidebar-home').click();await page.getByRole('button',{name:'New playlist',exact:true}).click();await expect(page.getByRole('button',{name:'Import M3U',exact:true})).toBeVisible();
});
test('Sketch collection controls keep fixed geometry and artwork-owned shading',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'audio/1',name:'Test mix',kind:'audio',track_ids:[1,2],artwork_choice:'auto/1'}])));
 await page.goto('/?fixture=1&sparkReady=1');const button=page.getByRole('button',{name:'Play Test mix',exact:true}).first();await button.scrollIntoViewIfNeeded();await expect(button.locator('[data-icon=play]')).toBeVisible();
 const before=await button.boundingBox();await button.hover();await page.waitForTimeout(170);const after=await button.boundingBox();expect(after).toEqual(before);
 const style=await button.evaluate(el=>({surface:getComputedStyle(el).backgroundColor,fill:getComputedStyle(el).getPropertyValue('--collection-transport-top').trim(),stroke:getComputedStyle(el).getPropertyValue('--collection-transport-stroke').trim(),border:getComputedStyle(el).borderRadius,gradient:el.querySelector('linearGradient')?.innerHTML}));expect(style.surface).not.toBe('rgba(0, 0, 0, 0)');expect(style.gradient).toContain('--transport-top');
 const surface='#'+(style.surface.match(/[\d.]+/g)??[]).slice(0,3).map(v=>Math.round(Number(v)).toString(16).padStart(2,'0')).join('');expect(contrast(style.fill,surface)).toBeGreaterThanOrEqual(4.49);expect(contrast(style.stroke,surface)).toBeGreaterThanOrEqual(2.99);
 await page.screenshot({path:'out/rebuild-qa/name-menu-home.png',animations:'disabled'});
 const oldSurface=style.surface;await page.evaluate(()=>{document.documentElement.style.setProperty('--surface-color','color(srgb 0.6 0.4 0.5)');});await page.waitForTimeout(50);expect(await button.evaluate(el=>getComputedStyle(el).backgroundColor)).toBe(oldSurface);await page.evaluate(()=>{document.documentElement.style.setProperty('--surface-color','color(srgb 0.1 0.12 0.2)');document.documentElement.style.setProperty('--ink','color(srgb 0.87 0.88 0.89)');});await expect.poll(()=>button.evaluate(el=>getComputedStyle(el).backgroundColor)).not.toBe(oldSurface);
 const refreshed=await button.evaluate(el=>({surface:getComputedStyle(el).backgroundColor,fill:getComputedStyle(el).getPropertyValue('--collection-transport-top').trim(),stroke:getComputedStyle(el).getPropertyValue('--collection-transport-stroke').trim()}));const nextSurface='#'+(refreshed.surface.match(/[\d.]+/g)??[]).slice(0,3).map(v=>Math.round(Number(v)).toString(16).padStart(2,'0')).join('');expect(contrast(refreshed.fill,nextSurface)).toBeGreaterThanOrEqual(4.49);expect(contrast(refreshed.stroke,nextSurface)).toBeGreaterThanOrEqual(2.99);
});
