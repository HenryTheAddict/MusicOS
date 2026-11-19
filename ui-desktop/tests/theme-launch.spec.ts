import {test,expect} from '@playwright/test';

test('launch starts with retained artwork colors and reveals the shell only once',async({page})=>{
 await page.addInitScript(()=>{const appearance={mode:'light',source:'artwork',accent:'#0064df',saturation:100,tint:15};localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:100,appearance}));localStorage.setItem('musicos-launch-palette-v1',JSON.stringify({appearance,dark:false,palette:{background:'#e8dcee',blob:'#b299ca',accent:'#633377',text:'#12202a'}}));const animate=Element.prototype.animate;(window as any).__motion=[];Element.prototype.animate=function(frames,options){const animation=animate.call(this,frames,options);(window as any).__motion.push({animation,frames,options});return animation;};});
 await page.goto('/?fixture=1');await expect(page.locator('.personal-home')).toBeVisible();await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');const launch=()=>page.evaluate(()=>(window as any).__motion.filter((r:any)=>r.animation.id==='shell-launch').map((r:any)=>({duration:r.options.duration,frames:r.frames})));expect(await launch()).toHaveLength(1);expect((await launch())[0].duration).toBe(180);
 for(const size of[{width:720,height:480},{width:1280,height:800}]){await page.setViewportSize(size);expect(await launch()).toHaveLength(1);}expect(await page.locator('.app-shell').evaluate(n=>n.getAnimations().filter(a=>a.id==='shell-launch').length)).toBe(0);
});

test('Reduced Motion shows the final theme without a launch transition',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:100,coverflow_colors:true,appearance:{mode:"light",source:"artwork",accent:"#0064df",saturation:100,tint:15}})));
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await expect(page.locator('.app-shell')).toHaveAttribute('data-adaptive','true');
 await expect(page.locator('html')).toHaveCSS('transition-duration','0s');
 await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');expect(await page.locator('.app-shell').evaluate(n=>n.getAnimations().filter(a=>a.id==='shell-launch').length)).toBe(0);
});

test('Sparklist composition may use gradients while ordinary sheet controls stay solid',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'audio/solid',kind:'audio',name:'Solid mix',track_ids:[1,2],artwork_choice:'auto/1'}])));
 await page.goto('/?fixture=1&sparkReady=1');
 await expect(page.locator('.home-collection-grid svg linearGradient')).not.toHaveCount(0);
 await page.getByRole('button',{name:'Create collection',exact:true}).first().click();await page.getByRole('menuitem',{name:'New Sparklist',exact:true}).click();
 const dialog=page.getByRole('dialog',{name:'Create Sparklist',exact:true});await expect(dialog.getByRole('slider',{name:'Sparklist length'})).toBeVisible();
 expect(await dialog.evaluate(el=>[el,...el.querySelectorAll('input,select,textarea')].filter(n=>getComputedStyle(n).backgroundImage.includes('gradient')).length)).toBe(0);
 await page.screenshot({path:'out/rebuild-qa/theme-solid-spark.png',animations:'disabled'});
});

test('launch completes when artwork colors are enabled but no artwork is available',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:100,coverflow_colors:true,appearance:{mode:"light",source:"artwork",accent:"#0064df",saturation:100,tint:15}})));
 await page.goto('/?fixture=1');
 await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
 await expect(page.locator('.app-shell')).toHaveAttribute('data-adaptive','false');
});
