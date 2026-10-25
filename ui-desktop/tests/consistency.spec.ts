import {test,expect} from '@playwright/test';
test('filters persist and Singles & EPs are separate only in main Albums',async({page})=>{
 await page.goto('/?fixture=1&consistency=1');await page.getByRole('button',{name:'Filter',exact:true}).click();
 const panel=page.getByRole('region',{name:'Library filters'});await expect(panel).toHaveCSS('opacity','1');await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(a=>a.playState==='running').length)).toBe(0);await page.screenshot({path:'out/rebuild-qa/filter-menu-polished.png'});await expect(panel.getByRole('button',{name:'Reset filters'})).toHaveCSS('border-radius','8px');await page.keyboard.press('Escape');await page.setViewportSize({width:720,height:480});await page.getByRole('button',{name:'Filter',exact:true}).click();await expect(panel).toHaveCSS('opacity','1');const bounds=await panel.boundingBox();expect(bounds!.x).toBeGreaterThanOrEqual(0);expect(bounds!.y+bounds!.height).toBeLessThanOrEqual(480);await page.screenshot({path:'out/rebuild-qa/filter-menu-small.png'});await panel.getByRole('button',{name:'Sort',exact:true}).click();await page.getByRole('menuitemradio',{name:'Album Name',exact:true}).click();await panel.getByRole('switch',{name:'Separate Singles & EPs'}).check();await page.keyboard.press('Escape');
 await expect(page.getByRole('heading',{name:'Singles & EPs',exact:true})).toBeVisible();await page.reload();await expect(page.getByRole('heading',{name:'Singles & EPs',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Filter',exact:true}).click();await expect(panel.getByRole('button',{name:'Sort',exact:true})).toHaveText('Album Name');await page.keyboard.press('Escape');
 await page.getByRole('button',{name:'Artists',exact:true}).click();await page.locator('.card-open').first().click();await expect(page.locator('.album-strip')).toBeVisible();await expect(page.getByRole('heading',{name:'Singles & EPs',exact:true})).toHaveCount(0);
});
test('conflicting embedded artwork never changes the album image in Now Playing or Audio',async({page})=>{
 await page.goto('/?fixture=1&consistency=1');const cover=await page.locator('.card-open').first().locator('img').getAttribute('src');await page.locator('.card-open').first().click();
 await page.locator('.track-row').nth(1).hover();await page.locator('.track-row').nth(1).locator('.row-play').click();await expect(page.locator('.now-playing-cover img')).toHaveAttribute('src',cover!);
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();await expect(page.locator('.queue-row img').first()).toHaveAttribute('src',cover!);
});
test('Settings switches and artist album strip fit every window and scale',async({page})=>{
 test.setTimeout(90000);
 for(const size of [{width:720,height:480},{width:1280,height:800}])for(const scale of [80,100,125,150]){
  await page.setViewportSize(size);await page.goto('/?fixture=1');await page.evaluate(percent=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'Henry',interface_percent:percent,coverflow_colors:false})),scale);await page.reload();
  await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
  const labels=page.locator('.timing-tile .themed-switch');const a=await labels.nth(0).boundingBox(),b=await labels.nth(1).boundingBox();expect(b!.y).toBeGreaterThanOrEqual(a!.y+a!.height);await expect(labels.nth(0).locator('.switch-track')).toBeVisible();
  if(size.width===1280&&scale===100)await page.screenshot({path:'out/rebuild-qa/consistency-settings.png',animations:'disabled'});
  for(const palette of [{surface:'#cbf1ff',ink:'#111820',blob:'#54b9ff',accent:'#0064df'},{surface:'#fffaf5',ink:'#161616',blob:'#e9dfd5',accent:'#765c45'},{surface:'#202329',ink:'#f3f6ff',blob:'#303c52',accent:'#97baff'},{surface:'#ffdfeb',ink:'#251124',blob:'#ff96ca',accent:'#b00070'}]){
   await page.locator('.app-shell').evaluate((el,p)=>{const s=(el as HTMLElement).style;s.setProperty('--surface-color',p.surface);s.setProperty('--ink',p.ink);s.setProperty('--blob-color',p.blob);s.setProperty('--accent',p.accent);},palette);
   expect(await labels.nth(0).locator('.switch-track').evaluate(el=>getComputedStyle(el).backgroundColor)).not.toBe('rgba(0, 0, 0, 0)');
   await page.screenshot({path:`out/rebuild-qa/polish-matrix/settings-${size.width}x${size.height}-${scale}-${palette.accent.slice(1)}.png`,animations:'disabled'});
  }
  await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Artists',exact:true}).click();await page.locator('.card-open').first().click();
  const strip=page.locator('.album-strip');await page.locator('.app-shell').evaluate(el=>el.setAttribute('data-adaptive','true'));await expect(strip).toHaveCSS('background-color','rgba(0, 0, 0, 0)');expect(await strip.evaluate(el=>el.scrollWidth<=el.clientWidth+1||getComputedStyle(el).overflowX!=='visible')).toBe(true);
  await page.screenshot({path:`out/rebuild-qa/polish-matrix/artist-${size.width}x${size.height}-${scale}.png`,animations:'disabled'});
  await expect(page.locator('.artist-playback .play-button')).toHaveCSS('background-color','rgba(0, 0, 0, 0)');
 }
 await page.screenshot({path:'out/rebuild-qa/consistency-artist-strip.png',animations:'disabled'});
});
test('cover parallax moves artwork while captions remain fixed and reduced motion disables it',async({page})=>{
 await page.goto('/?fixture=1');await expect(page.locator('.card-artwork').first()).toBeVisible();
 const index=await page.locator('.card-artwork').evaluateAll(arts=>{const viewport=document.querySelector('.page-scroll')!.getBoundingClientRect();return arts.map((el,index)=>({index,r:el.getBoundingClientRect()})).filter(({r})=>r.bottom>viewport.top+100&&r.top<viewport.bottom-100).sort((a,b)=>Math.abs((a.r.top+a.r.bottom)/2-(viewport.top+viewport.bottom)/2)-Math.abs((b.r.top+b.r.bottom)/2-(viewport.top+viewport.bottom)/2))[0]?.index??-1;});expect(index).toBeGreaterThanOrEqual(0);
 const art=page.locator('.card-artwork').nth(index);await expect.poll(()=>art.evaluate(el=>Math.abs(parseFloat((el as HTMLElement).style.getPropertyValue('--parallax-y'))||0))).toBeGreaterThan(.1);
 const before=await art.evaluate(el=>parseFloat((el as HTMLElement).style.getPropertyValue('--parallax-y')));
 await page.locator('.page-scroll').evaluate(el=>el.scrollTop+=100);
 await expect.poll(()=>art.evaluate(el=>parseFloat((el as HTMLElement).style.getPropertyValue('--parallax-y')))).toBeLessThan(before-1);
 expect(await art.evaluate(el=>Math.abs(parseFloat((el as HTMLElement).style.getPropertyValue('--parallax-y'))))).toBeLessThanOrEqual(14);
 await page.emulateMedia({reducedMotion:'reduce'});await expect(art.locator('.cover')).toHaveCSS('translate','none');
});
test('metadata status offers bounded recovery without exposing provider errors',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();
 const status=page.getByRole('region',{name:'Artist metadata status'});await expect(status).toContainText('8 cached · 0 checking · 2 unavailable');await status.getByRole('button',{name:'Retry available lookups'}).click();await expect(status).toContainText('8 cached · 2 checking · 2 unavailable');await expect(status.getByRole('button',{name:'Waiting before retry'})).toBeDisabled();
});

test('collection shuffle retains supplied geometry with a solid surface and no particles',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/qa',name:'Soft controls',track_ids:[1,2],spark:null,kind:'manual'}])));
 await page.goto('/?fixture=1');const shuffle=page.getByRole('button',{name:'Shuffle Soft controls',exact:true});await expect(shuffle.locator('svg use')).toHaveCount(1);await expect(shuffle.locator('linearGradient')).toHaveCount(1);await shuffle.hover();await expect(shuffle).toHaveCSS('animation-name','none');await expect(page.locator('.collection-particles')).toHaveCount(0);
});

test('wide grid captions do not collide and parallax can be disabled persistently',async({page})=>{
 test.setTimeout(90000);
 for(const width of [1920,2560])for(const percent of [80,100,125,150]){
  await page.setViewportSize({width,height:1080});await page.goto('/?fixture=1&largeAlbums=1');await page.evaluate(percent=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,artwork_parallax:false}));sessionStorage.removeItem('musicos-navigation-v1');},percent);await page.reload();
  const cards=page.locator('.collection-card');await expect(cards.first()).toBeVisible();
  const geometry=await cards.evaluateAll(cards=>cards.slice(0,20).map(el=>{const card=el.getBoundingClientRect(),caption=el.querySelector('.card-caption')!.getBoundingClientRect(),art=el.querySelector('.card-artwork')!.getBoundingClientRect();return {top:card.top,bottom:card.bottom,captionTop:caption.top,captionBottom:caption.bottom,artBottom:art.bottom};}));
  for(const card of geometry){expect(card.captionTop).toBeGreaterThanOrEqual(card.artBottom-1);expect(card.captionBottom).toBeLessThanOrEqual(card.bottom+1);const next=geometry.find(other=>other.top>card.top+2);if(next)expect(next.top).toBeGreaterThanOrEqual(card.captionBottom-1);}
  await expect(page.locator('.card-artwork > .cover').first()).toHaveCSS('transform','none');
 }
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();const toggle=page.getByRole('switch',{name:'Artwork parallax',exact:true});await expect(toggle).not.toBeChecked();await toggle.check();await toggle.uncheck();await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-preferences')!).artwork_parallax)).toBe(false);await page.reload();await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await expect(toggle).not.toBeChecked();
});
