import {test,expect,type Page} from '@playwright/test';
async function settings(page:Page){await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();}
async function artist(page:Page){await page.getByRole('button',{name:'Artists',exact:true}).click();await page.getByRole('button',{name:'Open Moonspindle',exact:true}).click();}
test('one still preview follows the selected size and view, retaining settings without playback',async({page})=>{
 await page.goto('/?fixture=1');await settings(page);
 const picker=page.locator('.mini-presets'),preview=picker.locator('.mini-preview'),sizes=picker.getByRole('group',{name:'Mini Player size'});
 await expect(preview).toHaveCount(1);await picker.scrollIntoViewIfNeeded();
 const requests=await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>['artist_info','album_info'].includes(c.command)).length);
 for(const [label,density,width,height] of [['Standard','standard',244,320],['Compact','compact',220,120],['Wide','wide',320,120]] as const){
  const button=sizes.getByRole('button',{name:label,exact:true});await button.click();await expect(button).toHaveAttribute('aria-pressed','true');await expect(preview).toHaveAttribute('data-density',density);await expect(preview).toHaveCSS('width',`${width}px`);await expect(preview).toHaveCSS('height',`${height}px`);await expect(picker.locator('.mini-preset-description small')).toHaveText(`${width} × ${height} minimum`);
  for(const [label,view] of [['Up next','queue'],['Artwork','tall']] as const){await picker.getByRole('button',{name:label,exact:true}).click();await expect(preview.locator('.mini-player')).toHaveAttribute('data-layout',view);await expect(preview).toHaveAttribute('inert','');}
 }
 await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.getByRole('tab',{name:'Appearance',exact:true}).click();await expect(sizes.getByRole('button',{name:'Wide',exact:true})).toHaveAttribute('aria-pressed','true');
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback'))).toEqual([]);
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>['artist_info','album_info'].includes(c.command)).length)).toBe(requests);
 await picker.scrollIntoViewIfNeeded();await expect.poll(()=>preview.evaluate(el=>el.getAnimations({subtree:true}).length)).toBe(0);await page.screenshot({path:'out/persistent-qa/mini-single-preview-wide.png'});
 await page.reload();await settings(page);await expect(sizes.getByRole('button',{name:'Wide',exact:true})).toHaveAttribute('aria-pressed','true');
});
test('portrait remains clear and photo attribution is a separate keyboard-accessible disclosure',async({page})=>{
 await page.route('**/src/fixtures.ts*',async route=>{const response=await route.fetch(),body=await response.text();await route.fulfill({response,body:body.replace('artist_mbid: "fixture"','artist_mbid: "fixture", portrait_source_url:"https://commons.wikimedia.org/wiki/File:Moonspindle.jpg", portrait_credit:"Example photographer", portrait_license:"CC BY-SA 4.0"')});});
 await page.goto('/?fixture=1');await artist(page);
 const portrait=page.getByRole('button',{name:'Expand Moonspindle portrait',exact:true}),credit=page.locator('.artist-photo-credit');
 await portrait.hover();await portrait.focus();await expect(page.locator('.portrait-fact,.portrait-notes')).toHaveCount(0);await expect(portrait.locator('.cover')).toHaveCSS('transform','none');await expect(credit).not.toHaveAttribute('open','');await expect(credit.getByRole('button',{name:'CC BY-SA 4.0 ↗'})).not.toBeVisible();
 await credit.locator('summary').focus();await page.keyboard.press('Enter');await expect(credit).toHaveAttribute('open','');await expect(credit.getByText('Example photographer')).toBeVisible();await expect(credit.getByRole('button',{name:'CC BY-SA 4.0 ↗'})).toBeVisible();
 const photo=(await portrait.boundingBox())!,caption=(await credit.boundingBox())!;expect(caption.y).toBeGreaterThanOrEqual(photo.y+photo.height);await page.screenshot({path:'out/persistent-qa/artist-separate-photo-credit.png'});
 await credit.locator('summary').press('Enter');await portrait.click();await expect(page.getByRole('dialog',{name:'Moonspindle artwork'})).toBeVisible();await page.keyboard.press('Escape');await expect(portrait).toBeFocused();await expect(credit).not.toHaveAttribute('open','');
});
test('size picker and single preview fit every desktop scale and theme without enlarging the player',async({page})=>{
 test.setTimeout(180000);
 await page.addInitScript(()=>{const q=new URLSearchParams(location.search);localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(q.get('scale')),appearance:{mode:q.get('mode'),source:q.get('source'),accent:'#ff00ff',saturation:150,tint:15}}));});
 for(const width of [720,1280])for(const scale of [80,100,125,150])for(const [mode,source] of [['light','blue'],['dark','blue'],['light','artwork'],['dark','artwork'],['light','custom'],['dark','custom']]){
  await page.setViewportSize({width,height:width===720?480:800});await page.goto(`/?fixture=1&scale=${scale}&mode=${mode}&source=${source}`);await settings(page);
  const picker=page.locator('.mini-presets');
  for(const label of ['Standard','Compact','Wide']){await picker.getByRole('group',{name:'Mini Player size'}).getByRole('button',{name:label,exact:true}).click();await expect(picker.locator('.mini-preview')).toHaveCount(1);expect(await picker.evaluate(el=>el.scrollWidth-el.clientWidth),`${width}/${scale}/${mode}/${source}/${label}`).toBeLessThan(2);expect(await page.locator('.settings-panel').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);}
  if(width===1280&&scale===125&&mode==='light'&&source==='artwork'){await picker.scrollIntoViewIfNeeded();await page.screenshot({path:'out/persistent-qa/mini-single-preview-settings.png'});}
 }
});
