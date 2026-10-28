import {test,expect,type Page} from '@playwright/test';
async function paintedText(page:Page,label:string){
 const span=page.getByRole('menuitem',{name:label,exact:true}).locator('span').last();const box=(await span.boundingBox())!;
 const ink=await span.evaluate(el=>getComputedStyle(el).color);const png=await page.screenshot({clip:{x:Math.ceil(box.x),y:Math.ceil(box.y),width:Math.max(1,Math.floor(box.width)-1),height:Math.max(1,Math.floor(box.height)-1)},scale:'css'});
 return page.evaluate(async({png,ink})=>{const img=new Image();img.src='data:image/png;base64,'+png;await img.decode();const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d')!;ctx.fillStyle=ink;ctx.fillRect(0,0,1,1);const target=[...ctx.getImageData(0,0,1,1).data];ctx.clearRect(0,0,c.width,c.height);ctx.drawImage(img,0,0);const pixels=ctx.getImageData(0,0,c.width,c.height).data;let painted=0;for(let i=0;i<pixels.length;i+=4){if(Math.max(...target.slice(0,3).map((v,k)=>Math.abs(v-pixels[i+k])))<30)painted++;}return painted;},{png:png.toString('base64'),ink});
}
test('selector choices actually paint over the expanded header, including scaled layouts',async({page})=>{
 await page.addInitScript(()=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(new URLSearchParams(location.search).get('scale')??100),appearance:{mode:'light',source:'blue'}}));});
 for(const scale of [80,100,125,150]){
  await page.goto(`/?fixture=1&scale=${scale}`);await page.getByRole('button',{name:'Choose section',exact:true}).click();await expect(page.getByRole('menuitem',{name:'Devices',exact:true})).toBeVisible();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('menu-')&&a.playState==='running').length)).toBe(0);
  for(const label of ['Devices','Settings'])expect(await paintedText(page,label),`${scale}% ${label} text is painted`).toBeGreaterThan(15);
  await page.screenshot({path:`out/persistent-qa/selector-painted-${scale}.png`});await page.keyboard.press('Escape');await expect(page.getByRole('menu')).toHaveCount(0);
 }
});

test('choice text remains painted through intermediate opening, collapse and reversal',async({page})=>{
 await page.addInitScript(()=>{const play=Animation.prototype.play;Animation.prototype.play=function(){if(this.id.startsWith('menu-')){this.pause();return;}play.call(this);};const original=Element.prototype.animate;Element.prototype.animate=function(frames,options){const a=original.call(this,frames,options);if(this.matches('.menu-panel-surface,.menu-panel-items,.section-control>.menu-trigger:first-child')){a.pause();a.currentTime=0;}return a;};});
 await page.goto('/?fixture=1');const trigger=page.getByRole('button',{name:'Choose section',exact:true});await trigger.click();await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id==='menu-join').length)).toBe(1);
 const at=()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('menu-')).forEach(a=>a.currentTime=70));
 const finish=()=>page.evaluate(()=>document.getAnimations().filter(a=>a.id.startsWith('menu-')).forEach(a=>a.finish()));
 await at();expect(await paintedText(page,'Devices')).toBeGreaterThan(15);await finish();await page.keyboard.press('Escape');await at();expect(await paintedText(page,'Devices')).toBeGreaterThan(15);
 await trigger.click();await at();expect(await paintedText(page,'Devices')).toBeGreaterThan(15);await finish();await page.screenshot({path:'out/persistent-qa/selector-painted-reversal.png'});await page.keyboard.press('Escape');await finish();await expect(page.getByRole('menu')).toHaveCount(0);
});
