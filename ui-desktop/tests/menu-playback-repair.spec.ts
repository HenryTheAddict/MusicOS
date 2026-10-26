import {test,expect,type Page} from '@playwright/test';
async function track(page:Page){await page.route('**/src/fixtures.ts*',async route=>{const response=await route.fetch();await route.fulfill({response,body:(await response.text()).replace('let player = { ...emptyPlayback };','let player = {...emptyPlayback,track:tracks[0],queue:[tracks[0]],queue_index:0,position:30};')});});}
async function closed(page:Page){await expect(page.locator('.section-dropdown')).toHaveCount(0);}
const playback=(page:Page)=>page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback').length);
test('pointer menu return preserves Space playback, while Tab retains button activation',async({page})=>{
 await track(page);await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});
 await trigger.click();await page.keyboard.press('Escape');await closed(page);await expect(trigger).toBeFocused();
 expect(await trigger.evaluate(el=>getComputedStyle(el).outlineStyle)).toBe('none');
 const before=await playback(page);await page.keyboard.press('Space');await expect.poll(()=>playback(page)).toBe(before+1);await closed(page);
 // Pointer selection also returns focus without turning the next Space into a menu command.
 await trigger.click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await closed(page);await expect(trigger.locator('.section-name')).toHaveText('Settings');await page.keyboard.press('Space');await expect.poll(()=>playback(page)).toBe(before+2);await closed(page);
 // A deliberate Tab visit is a real control focus and retains native activation.
 await page.keyboard.press('Tab');await trigger.focus();await expect(trigger).toBeFocused();
 expect(await trigger.evaluate(el=>getComputedStyle(el).outlineStyle)).toBe('solid');
 await page.keyboard.press('Space');await expect(page.locator('.section-dropdown')).toBeVisible();expect(await playback(page)).toBe(before+2);await page.keyboard.press('Escape');await closed(page);
});
test('collapse reverses the same timeline and outside scroll cannot truncate its exit',async({page})=>{
 await page.addInitScript(()=>{const play=Animation.prototype.play;Animation.prototype.play=function(){if(this.id.startsWith('menu-')){this.pause();return;}play.call(this);};});
 await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});await trigger.click();
 await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);
 const height=await page.evaluate(()=>{const a=document.getAnimations().find(a=>a.id==='menu-join')!;(window as any).originalMenuAnimation=a;document.getAnimations().filter(a=>a.id.startsWith('menu-')).forEach(a=>a.currentTime=80);return getComputedStyle(document.querySelector('.section-control>.menu-trigger')!).getPropertyValue('--menu-join-height');});
 await trigger.click();
 expect(await page.evaluate(()=>document.getAnimations().find(a=>a.id==='menu-join')===(window as any).originalMenuAnimation)).toBe(true);
 expect(await trigger.evaluate(el=>getComputedStyle(el).getPropertyValue('--menu-join-height'))).toBe(height);
 expect(await page.evaluate(()=>(window as any).originalMenuAnimation.playbackRate)).toBe(-1);
 await page.evaluate(()=>window.dispatchEvent(new Event('scroll')));await expect(page.locator('.section-dropdown')).toHaveCount(1);
 await trigger.click();expect(await page.evaluate(()=>(window as any).originalMenuAnimation.playbackRate)).toBe(1);
 await trigger.click();await page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('menu-')).forEach(a=>a.finish()));await closed(page);
 expect(await trigger.evaluate(el=>el.getAnimations().length)).toBe(0);
});
test('real-time collapse completes repeatedly without leftover layers or reopening',async({page})=>{
 await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});
 for(let i=0;i<6;i++){await trigger.click();await expect(page.getByRole('menuitem',{name:'Settings',exact:true})).toBeVisible();await page.waitForTimeout(155);await trigger.click();await closed(page);}
 await expect(trigger).toHaveAttribute('data-state','closed');expect(await page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('menu-')).length)).toBe(0);
});
test('a late focus return from the previous selector cannot dismiss its new session',async({page})=>{
 await page.addInitScript(()=>{const add=HTMLElement.prototype.addEventListener;HTMLElement.prototype.addEventListener=function(this:HTMLElement,type:string,listener:EventListenerOrEventListenerObject,options?:boolean|AddEventListenerOptions){if(type==='focusScope.autoFocusOnUnmount'&&typeof listener==='function'){const delayed=function(this:HTMLElement,event:Event){event.preventDefault();setTimeout(()=>listener.call(this,event),100);};return add.call(this,type,delayed,options);}return add.call(this,type,listener,options);};});
 await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});await trigger.click();await page.keyboard.press('Escape');await closed(page);await trigger.click();await page.waitForTimeout(180);await expect(trigger).toHaveAttribute('data-state','open');await expect(page.getByRole('menuitem',{name:'Settings',exact:true})).toBeVisible();await expect(trigger).not.toBeFocused();await page.keyboard.press('Escape');await closed(page);
});
