import {test,expect,type Page} from '@playwright/test';
async function start(page:Page){
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-analysis',JSON.stringify([{track_id:1,bpm:120}])));
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await page.locator('[data-track-id="1"]').hover();await page.locator('[data-track-id="1"] .row-play').click();
}
test('mini heart follows playing BPM, and queue retains rows/scroll across animated show/hide',async({page})=>{
 await start(page);await page.getByRole('button',{name:'MusicOS actions',exact:true}).click();await page.getByRole('menuitem',{name:'Mini Player',exact:true}).click();
 await page.setViewportSize({width:244,height:320});
 const mini=page.locator('.mini-player'),heart=mini.locator('.mini-favorite');
 await expect(heart).toBeVisible();expect(await heart.evaluate(el=>el.querySelector('svg')!.getBoundingClientRect().width<=el.getBoundingClientRect().width*.6)).toBe(true);await expect(heart).toHaveAttribute('data-pulsing','true');await expect(heart.locator('.heart-beat')).toHaveCSS('animation-duration','0.5s');
 await mini.getByRole('button',{name:'Pause',exact:true}).click();await expect(heart.locator('.heart-beat')).toHaveCSS('animation-name','none');
 await mini.getByRole('button',{name:'Play',exact:true}).click();await expect(heart).toHaveAttribute('data-pulsing','true');
 await mini.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(mini).toHaveAttribute('data-layout','queue');
 const list=mini.locator('.mini-queue-list'),scroll=list.locator('.queue-scroll');
 await scroll.evaluate(el=>{el.scrollTop=80;(window as any).retainedQueue=el;});const top=await scroll.evaluate(el=>el.scrollTop);
 await mini.getByRole('button',{name:'Close Up next',exact:true}).click();await expect(list).toHaveAttribute('inert','');await expect(list).not.toBeVisible();
 await mini.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(list).toBeVisible();
 expect(await scroll.evaluate(el=>el===(window as any).retainedQueue)).toBe(true);expect(await scroll.evaluate(el=>el.scrollTop)).toBe(top);
 await page.emulateMedia({reducedMotion:'reduce'});await expect(heart.locator('.heart-beat')).toHaveCSS('animation-name','none');
 await page.screenshot({path:'out/rebuild-qa/mini-polish-queue.png',animations:'disabled'});
});

test('main queue has a closing phase and survives a quick reopen',async({page})=>{
 await start(page);await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const panel=page.locator('.audio-panel.queue-panel');await expect(panel).toBeVisible();
 await panel.getByRole('button',{name:'Close Up next',exact:true}).click();
 await expect(panel).toHaveAttribute('data-closing','true');await expect(panel).toHaveAttribute('inert','');
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(panel).toHaveAttribute('data-closing','false');await expect(panel).toBeVisible();
 await panel.getByRole('button',{name:'Close Up next',exact:true}).click();await expect(panel).toHaveCount(0);
});

function ratio(a:string,b:string){const lum=(s:string)=>{const c=s.match(/[\d.]+/g)!.slice(0,3).map(Number).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;});return c[0]*.2126+c[1]*.7152+c[2]*.0722;};const x=lum(a),y=lum(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);}
test('section header stays rounded and stationary; portal choices have readable light/dark selected states',async({page})=>{
 await page.goto('/?fixture=1');const chooser=page.getByRole('button',{name:'Choose section',exact:true});
 const original=await chooser.boundingBox();await chooser.click();await expect(chooser).toHaveCSS('border-bottom-left-radius','0px');
 expect(await chooser.boundingBox()).toEqual(original);await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 for(const mode of ['Dark','Light']){
  await page.getByRole('button',{name:'Appearance mode',exact:true}).click();await page.getByRole('menuitemradio',{name:mode,exact:true}).click();
  await page.getByRole('button',{name:'Appearance mode',exact:true}).click();const selected=page.getByRole('menuitemradio',{name:mode,exact:true});
  const colors=await selected.evaluate(el=>({ink:getComputedStyle(el).color,surface:getComputedStyle(el).backgroundColor}));expect(ratio(colors.ink,colors.surface)).toBeGreaterThanOrEqual(4.5);
  await expect(selected).toHaveCSS('border-radius','10px');await expect(selected).toHaveCSS('font-weight','400');await page.screenshot({path:`out/rebuild-qa/choice-polish-${mode}.png`,animations:'disabled'});await page.keyboard.press('Escape');
 }
});

for(const scale of [80,100,125,150])test(`metadata modes, file-tag review and Theme spacing at compact ${scale}%`,async({page})=>{
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.evaluate(scale=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:scale,coverflow_colors:scale===125,...(scale===150?{appearance:{mode:'dark',source:'custom',accent:'#b84e32',saturation:100,tint:15}}:{})})),scale);await page.reload();
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 await page.getByRole('tab',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();await page.getByRole('tab',{name:'Edit',exact:true}).click();
 const album=page.locator('.batch-metadata .metadata-album-list>button').first();await album.click();if(await page.locator('.metadata-continue').isVisible())await page.locator('.metadata-continue').click();
 await page.getByRole('button',{name:'Artist change',exact:true}).click();await page.getByRole('menuitemradio',{name:'Set value',exact:true}).click();await page.getByRole('textbox',{name:'New artist',exact:true}).fill('Review only');
 const fields=await page.locator('.metadata-field').evaluateAll(rows=>rows.map(el=>{const box=el.getBoundingClientRect();return {width:box.width,scroll:el.scrollWidth,children:[...el.children].filter(c=>!(c as HTMLElement).hidden).map(c=>{const b=c.getBoundingClientRect();return {x:b.x,y:b.y,right:b.right,bottom:b.bottom};})};}));
 for(const field of fields){expect(field.scroll).toBeLessThanOrEqual(field.width+1);for(let i=0;i<field.children.length;i++)for(let j=i+1;j<field.children.length;j++){const a=field.children[i],b=field.children[j];expect(a.right<=b.x+1||b.right<=a.x+1||a.bottom<=b.y+1||b.bottom<=a.y+1).toBe(true);}}
 await page.getByRole('button',{name:/Review changes to \d+ songs/}).click();await expect(page.getByRole('checkbox',{name:'Also write to music files',exact:true})).not.toBeChecked();
 await page.locator('.file-write-option').scrollIntoViewIfNeeded();await page.screenshot({path:`out/rebuild-qa/metadata-polish-${scale}.png`,animations:'disabled'});
});
