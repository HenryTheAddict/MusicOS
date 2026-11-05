import {test,expect} from '@playwright/test';
test('revision return shares progressive cover, fade and toolbar frames',async({page})=>{
 await page.addInitScript(()=>{const run=Element.prototype.animate;(window as any).motion=[];Element.prototype.animate=function(f,o){const a=run.call(this,f,o);(window as any).motion.push(a);return a;};});
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Junk',exact:true}).click();await expect(page.locator('.album-cover-transfer')).toHaveCount(0,{timeout:2000});
 await page.evaluate(()=>{const run=Element.prototype.animate;Element.prototype.animate=function(f,o){const a=run.call(this,f,o);a.pause();a.currentTime=0;return a;};});
 await page.locator('.sidebar-back').click();const exit=page.locator('.library-page-exit');await expect(exit).toHaveCount(1);await expect(page.locator('.album-cover-transfer')).toHaveCount(1);
 await page.evaluate(()=>{for(const a of (window as any).motion)if(a.effect?.target?.isConnected&&a.playState==='paused')a.currentTime=160;});
 const opacity=Number(await exit.evaluate(n=>getComputedStyle(n).opacity));expect(opacity).toBeGreaterThan(.01);expect(opacity).toBeLessThan(.99);expect(await exit.getAttribute('inert')).not.toBeNull();
 const bar=page.locator('.library-toolbar');expect(await bar.evaluate(n=>n.getAnimations().some(a=>a.id==='library-toolbar-return'))).toBe(true);
 await page.evaluate(()=>{for(const a of (window as any).motion)if(a.effect?.target?.isConnected&&a.playState==='paused')a.finish();});await expect(exit).toHaveCount(0,{timeout:1000});await expect(page.locator('.album-cover-transfer')).toHaveCount(0);
});
test('revision filter reverses its clipped reveal, preserves nested Escape and releases exit',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Albums',exact:true}).click();const trigger=page.getByRole('button',{name:'Filter',exact:true});await trigger.click();const panel=page.locator('#library-filter-panel');await expect(panel).toBeVisible();await expect(panel.getByRole('button',{name:'Sort',exact:true})).toBeVisible();
 await panel.getByRole('button',{name:'Sort',exact:true}).click();await page.keyboard.press('Escape');await expect(panel).toBeVisible();await expect(trigger).toHaveAttribute('aria-expanded','true');
 await trigger.click();await expect(panel).toHaveAttribute('inert','');await trigger.click();await expect(panel).not.toHaveAttribute('inert');await expect(panel).toBeVisible();
 await page.keyboard.press('Escape');await expect(panel).toHaveCount(0);await expect(trigger).toBeFocused();
 await page.emulateMedia({reducedMotion:'reduce'});await trigger.click();await expect(panel).toBeVisible();expect(await panel.evaluate(n=>n.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);await page.keyboard.press('Escape');await expect(panel).toHaveCount(0);
});
for(const mode of ['words','rtlwords'])test(`revision ${mode} keeps its light layer through word handoffs and seeks`,async({page})=>{
 await page.route('**/src/bridge.ts*',async route=>{const response=await route.fetch(),body=await response.text(),start=body.indexOf('export async function subscribe('),end=body.indexOf('\n}',start)+2;await route.fulfill({response,body:body.slice(0,start)+`export async function subscribe(event,callback){const fn=e=>callback(e.detail);window.addEventListener('qa:'+event,fn);return()=>window.removeEventListener('qa:'+event,fn);}`+body.slice(end)});});
 await page.goto(`/?fixture=1&lyrics=1&lyricsCase=${mode}`);await page.getByRole('button',{name:'Open lyrics',exact:true}).click();const player=page.locator('.expanded-player');await expect(player.locator('.lyric-word').first()).toBeVisible();
 const tick=async(position:number)=>{await page.evaluate(position=>window.dispatchEvent(new CustomEvent('qa:playback',{detail:{position,is_playing:false}})),position);};
 await tick(.25);const layer=player.locator('.lyric-glow-layer');await expect(layer).toHaveCount(1);await expect(layer.locator('.lyric-word').first()).toHaveCSS('opacity',/0\.[0-9]+/);await layer.evaluate(n=>n.dataset.continuity='retained');
 const first=player.locator('.lyrics-lines .lyric-word').first();const before=await first.evaluate(n=>n.style.getPropertyValue('--word-progress'));await tick(.4);await expect.poll(()=>first.evaluate(n=>n.style.getPropertyValue('--word-progress'))).not.toBe(before);
 await tick(mode==='words'?.7:1.95);await expect(layer).toHaveAttribute('data-continuity','retained');await tick(.1);await expect(layer).toHaveAttribute('data-continuity','retained');
 await page.keyboard.press('Escape');await expect(player).not.toBeVisible();await expect(page.locator('.lyric-glow-layer')).toHaveCount(0);expect(await page.evaluate(()=>document.getAnimations().filter(a=>a.playState==='running'&&a.id.startsWith('lyric')).length)).toBe(0);
});
test('revision metadata hides identical song cards and keeps match sources grouped',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();await page.locator('.metadata-album-list button').first().click();await expect(page.getByText('No song-detail changes',{exact:true})).toBeVisible();await expect(page.locator('.album-track-comparison article')).toHaveCount(0);
 await page.getByRole('checkbox',{name:'Show unchanged track fields',exact:true}).check();await expect(page.locator('.album-track-comparison article')).not.toHaveCount(0);const details=page.locator('.metadata-match-details').first();await details.locator('summary').click();await expect(details.getByRole('button',{name:'Ignore suggestion',exact:true})).toBeVisible();expect(await details.evaluate(n=>n.scrollWidth-n.clientWidth)).toBeLessThanOrEqual(1);expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='album_metadata'&&c.args.request.action==='apply').length)).toBe(0);
 await page.screenshot({path:'out/navigation-setup-2026-11-04/revised-metadata.png',animations:'disabled'});
});
