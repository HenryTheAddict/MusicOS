import {test,expect} from '@playwright/test';

test('launch blends solid theme colors and resizing does not restart it',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:100,coverflow_colors:true})));
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await expect(page.locator('.app-shell')).toHaveAttribute('data-adaptive','true');
 const samples=await page.evaluate(async()=>{const values:string[]=[];const start=performance.now();while(performance.now()-start<1650){values.push(getComputedStyle(document.documentElement).getPropertyValue('--shell-color'));await new Promise(requestAnimationFrame);}return values;});
 expect(new Set(samples).size).toBeGreaterThan(8);
 await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
 const background=await page.locator('.current-theme').evaluate(el=>getComputedStyle(el).backgroundColor);
 for(const size of [{width:720,height:480},{width:1600,height:900}]){await page.setViewportSize(size);await expect(page.locator('.current-theme')).toHaveCSS('background-color',background);await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');}
 const gradients=await page.locator('.app-shell *').evaluateAll(nodes=>nodes.filter(n=>getComputedStyle(n).backgroundImage.includes('gradient')).map(n=>n.className));expect(gradients).toEqual([]);
 await page.screenshot({path:'out/rebuild-qa/theme-solid-wide.png',animations:'disabled'});
});

test('Reduced Motion shows the final theme without a launch transition',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:100,coverflow_colors:true})));
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await expect(page.locator('.app-shell')).toHaveAttribute('data-adaptive','true');
 await expect(page.locator('html')).toHaveCSS('transition-duration','0s');
 await expect(page.locator('.current-theme')).toHaveCSS('background-color','rgb(200, 222, 217)');
});

test('only collection controls have gradients; Sparklist sheets retain solid surfaces',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'audio/solid',kind:'audio',name:'Solid mix',track_ids:[1,2],artwork_choice:'auto/1'}])));
 await page.goto('/?fixture=1&sparkReady=1');
 await expect(page.locator('.personal-collection svg linearGradient')).not.toHaveCount(0);
 await page.getByRole('button',{name:'Create Sparklist',exact:true}).click();
 const dialog=page.getByRole('dialog',{name:'Create Sparklist',exact:true});await expect(dialog.getByRole('slider',{name:'Sparklist length'})).toBeVisible();
 expect(await dialog.evaluate(el=>[el,...el.querySelectorAll('*')].filter(n=>getComputedStyle(n).backgroundImage.includes('gradient')).length)).toBe(0);
 await page.screenshot({path:'out/rebuild-qa/theme-solid-spark.png',animations:'disabled'});
});

test('launch completes when artwork colors are enabled but no artwork is available',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:100,coverflow_colors:true})));
 await page.goto('/?fixture=1');
 await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');
 await expect(page.locator('.app-shell')).toHaveAttribute('data-adaptive','false');
});
