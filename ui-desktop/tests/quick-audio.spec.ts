import {test,expect} from '@playwright/test';
test('quality opens independent audio controls, EQ layout persists, and queue stays separate',async({page})=>{
 for(const size of [{width:720,height:480},{width:1280,height:800}]){
  await page.setViewportSize(size);await page.goto('/?fixture=1');await page.evaluate(()=>{localStorage.removeItem('musicos-fixture-preferences');sessionStorage.clear();});await page.reload();await page.getByRole('button',{name:'Open Clockwork Confetti',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();
  await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();const controls=page.getByRole('dialog',{name:'Audio controls'});await expect(controls).toBeVisible();await expect(controls.getByRole('slider',{name:'Bass gain'})).toBeVisible();await expect(controls.getByRole('switch',{name:'Link pitch & speed'})).toBeChecked();
  await controls.getByRole('slider',{name:'Playback speed'}).focus();await page.keyboard.press('End');expect(Number(await controls.getByRole('slider',{name:'Playback pitch'}).inputValue())).toBeCloseTo(12,1);
  await controls.getByRole('switch',{name:'Link pitch & speed'}).uncheck();await controls.getByRole('slider',{name:'Playback speed'}).focus();await page.keyboard.press('Home');expect(Number(await controls.getByRole('slider',{name:'Playback pitch'}).inputValue())).toBeCloseTo(12,1);
  await page.keyboard.press('Escape');await expect(controls).not.toBeVisible();await page.getByRole('button',{name:'Open Up next',exact:true}).click();const queue=page.getByRole('complementary',{name:'Up next',exact:true});await expect(queue).toBeVisible();await expect(queue.getByRole('slider')).toHaveCount(0);await expect(queue.getByRole('switch',{name:'DJ mode'})).toBeVisible();await expect(controls.getByRole('switch',{name:'DJ mode'})).toHaveCount(0);await page.keyboard.press('Escape');
  await page.getByRole('button',{name:'Choose section'}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.getByRole('button',{name:'Playback',exact:true}).click();await page.getByRole('button',{name:'8 bands',exact:true}).click();await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')??'{}').listening?.bands)).toBe(8);await page.reload();
  await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();await expect(controls.getByRole('slider')).toHaveCount(13);const bounds=(await controls.boundingBox())!;expect(bounds.x).toBeGreaterThanOrEqual(0);expect(bounds.y+bounds.height).toBeLessThanOrEqual(size.height);await page.screenshot({path:`out/rebuild-qa/quick-audio-${size.width}.png`,animations:'disabled'});
 }
});
test('expanded artwork uses a readable blurred backdrop',async({page})=>{await page.goto('/?fixture=1');await page.getByRole('button',{name:'Open Clockwork Confetti',exact:true}).click();await page.getByRole('button',{name:'Expand Clockwork Confetti artwork'}).click();const dialog=page.getByRole('dialog');await expect(dialog.locator('.viewer-backdrop')).toHaveCSS('filter','blur(32px) brightness(0.4)');await expect(dialog.locator('figcaption')).toHaveText('Clockwork Confetti');await expect(dialog).toHaveCSS('opacity','1');await page.screenshot({path:'out/rebuild-qa/blurred-artwork-viewer.png'});await page.keyboard.press('Escape');await expect(dialog).not.toBeVisible();});
test('reverb is opt-in, persists, resets, and volume has no visible thumb',async({page})=>{
 await page.goto('/?fixture=1');await page.evaluate(()=>localStorage.removeItem('musicos-fixture-preferences'));await page.reload();await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();const panel=page.getByRole('dialog',{name:'Audio controls'});
 const headroom=panel.getByRole('switch',{name:'Preserve EQ headroom',exact:true});
 await expect(headroom).not.toBeChecked();
 await headroom.check();
 await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')??'{}').listening?.eq_headroom)).toBe(true);
 await expect(panel.getByRole('switch',{name:'Reverb',exact:true})).not.toBeChecked();await panel.getByRole('switch',{name:'Reverb',exact:true}).check();await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')??'{}').listening?.reverb)).toBe(.45);
 const thumb=await panel.getByRole('slider',{name:'Volume',exact:true}).evaluate(el=>{const s=getComputedStyle(el,'::-webkit-slider-thumb');return {background:s.backgroundColor,shadow:s.boxShadow};});expect(thumb).toEqual({background:'rgba(0, 0, 0, 0)',shadow:'none'});
 await page.reload();await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();await expect(panel.getByRole('switch',{name:'Reverb',exact:true})).toBeChecked();await expect(headroom).toBeChecked();await panel.getByRole('button',{name:'Reset audio'}).click();await expect(headroom).not.toBeChecked();await expect(panel.getByRole('switch',{name:'Reverb',exact:true})).not.toBeChecked();await expect(panel.getByRole('slider',{name:'Playback speed'})).toHaveValue('1');
 const flat=await panel.locator('.eq-connections path').getAttribute('d');
 for(const [name,key,count]of [['Bass gain','ArrowUp',6],['Mid gain','ArrowDown',4],['Treble gain','ArrowUp',4]] as const){await panel.getByRole('slider',{name}).focus();for(let i=0;i<count;i++)await page.keyboard.press(key);}
 await expect(panel.locator('.eq-connections path')).not.toHaveAttribute('d',flat!);await panel.getByRole('heading',{name:'Audio',exact:true}).click();
 await page.screenshot({path:'out/rebuild-qa/quick-audio-compact-revision.png',animations:'disabled'});await panel.screenshot({path:'out/rebuild-qa/audio-panel-detail.png',animations:'disabled'});
});
test('selected Settings category keeps a readable paired surface in adaptive pink',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Choose section'}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.locator('.app-shell').evaluate(el=>{el.setAttribute('data-adaptive','true');Object.entries({'--surface-color':'#eedcdd','--ink':'#14222b','--accent':'#481515','--accent-ink':'#ffffff'}).forEach(([k,v])=>(el as HTMLElement).style.setProperty(k,v));});const selected=page.getByRole('tab',{name:'Audio',exact:true});await expect(selected).toHaveCSS('background-color','rgb(72, 21, 21)');await expect(selected).toHaveCSS('color','rgb(255, 255, 255)');
});

test('compact Audio fits scales and readable theme surfaces',async({page})=>{
 test.setTimeout(120000);
 const palettes={blue:['#cbf1ff','#111820','#0064df'],light:['#fffaf5','#161616','#765c45'],dark:['#202329','#f3f6ff','#97baff'],vivid:['#ffdfeb','#251124','#b00070'],adaptive:['#eedcdd','#14222b','#481515']};
 for(const size of [{width:720,height:480},{width:1280,height:800}])for(const scale of [80,100,125,150])for(const [name,colors]of Object.entries(palettes)){
  await page.setViewportSize(size);await page.goto('/?fixture=1');await page.evaluate(scale=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:scale})),scale);await page.reload();await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();const panel=page.getByRole('dialog',{name:'Audio controls'});await panel.evaluate((el,p)=>{['--surface-color','--ink','--accent'].forEach((key,i)=>(el as HTMLElement).style.setProperty(key,p[i]));},colors);
  const box=(await panel.boundingBox())!;expect(box.x).toBeGreaterThanOrEqual(0);expect(box.y).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(size.width);expect(box.y+box.height).toBeLessThanOrEqual(size.height);expect(await panel.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThanOrEqual(1);
  if(scale===100&&size.width===1280)await panel.screenshot({path:`out/rebuild-qa/audio-panel-${name}.png`,animations:'disabled'});
 }
});

test('reverb pad drags both axes, supports keyboard, and keeps a fixed square',async({page})=>{
 await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await page.evaluate(()=>localStorage.removeItem('musicos-fixture-preferences'));await page.reload();await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();const panel=page.getByRole('dialog',{name:'Audio controls'}),pad=panel.locator('.reverb-pad');const box=(await pad.boundingBox())!;expect(Math.abs(box.width-box.height)).toBeLessThan(1);
 await panel.getByRole('switch',{name:'Reverb',exact:true}).check();await pad.focus();await page.keyboard.press('ArrowUp');await page.keyboard.press('Shift+ArrowRight');
 const saved=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')??'{}').listening);
 await expect.poll(saved).toMatchObject({reverb:.55,reverb_depth:.51});
 await pad.scrollIntoViewIfNeeded();const from=(await pad.boundingBox())!;
 await page.mouse.move(from.x+from.width/2,from.y+from.height/2);await page.mouse.down();await page.mouse.move(from.x+from.width+30,from.y-30,{steps:8});await page.mouse.up();await expect.poll(saved).toMatchObject({reverb:1,reverb_depth:1});
 const after=(await pad.boundingBox())!;expect(after.width).toBe(box.width);expect(after.height).toBe(box.height);
 await pad.focus();await page.keyboard.press('ArrowDown');await expect.poll(saved).toMatchObject({reverb_depth:.99});
 await page.reload();await page.locator('.now-playing').getByRole('button',{name:/Audio quality/}).click();await expect(pad).toHaveAccessibleName('Reverb pad, 100% room, 99% depth');
 await panel.getByRole('switch',{name:'Reverb',exact:true}).uncheck();await expect.poll(saved).toMatchObject({reverb:0,reverb_depth:.99});await panel.screenshot({path:'out/rebuild-qa/reverb-pad.png',animations:'disabled'});
});
