import {test,expect} from '@playwright/test';
import {chooseSection,openAlbums,setPlayerView} from './support/ui';
test('quality tooltip dismisses after pointer clicks, hover leave, outside click and Escape',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
 const quality=page.locator('.album-copy .audio-quality button'),popup=page.locator('[id^=audio-specs-album-]');
 await quality.hover();await expect(popup).toBeVisible();await quality.click();await page.locator('.album-copy h1').hover();await expect(popup).not.toBeVisible();
 await quality.hover();await expect(popup).toBeVisible();await page.locator('.album-copy h1').click();await expect(popup).not.toBeVisible();
 await quality.focus();await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');await expect(popup).toBeVisible();await page.keyboard.press('Escape');await expect(popup).not.toBeVisible();
});
test('collaboration credits wrap separately at compact widths and large interface sizes',async({page})=>{
 for(const percent of [80,100,125,150]){
  await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');await openAlbums(page);
  await page.evaluate(value=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'Henry',interface_percent:value,coverflow_colors:false})),percent);await page.evaluate(()=>sessionStorage.removeItem('musicos-navigation-v1'));await page.reload();await openAlbums(page);
  await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
  // Reproduce a collaboration in the existing header, exercising its responsive CSS.
  await page.locator('.album-artist-credits').evaluate(el=>{const first=el.querySelector('button')!;first.querySelector('.artist-credit-name')!.textContent='Travis Scott';const second=first.cloneNode(true) as HTMLElement;second.querySelector('.artist-credit-name')!.textContent='Kendrick Lamar';el.append(second);});
  const credits=page.locator('.album-artist-credits button');const a=await credits.nth(0).boundingBox(),b=await credits.nth(1).boundingBox();
  expect(a!.y+a!.height<=b!.y+.5||a!.x+a!.width<=b!.x+.5).toBe(true);
  await page.screenshot({animations:'disabled',path:`out/rebuild-qa/artist-credits-fixed-${percent}.png`});
 }
});
test('normal Settings entry retains the last category',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);await setPlayerView(page.locator('.now-playing'),'queue');await setPlayerView(page.locator('.now-playing'),'playing');await chooseSection(page,'Settings');await page.getByRole('tab',{name:'Audio',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Audio',exact:true})).toHaveAttribute('aria-selected','true');
 await chooseSection(page,'Library');
 await chooseSection(page,'Settings');
 await expect(page.getByRole('tab',{name:'Audio',exact:true})).toHaveAttribute('aria-selected','true');
});
test('only the playing album has a playback indicator, and browsing leaves no selected card panel',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Albums',exact:true}).click();
 await expect(page.locator('.collection-card.selected')).toHaveCount(0);await expect(page.locator('.card-playing')).toHaveCount(0);
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();await page.getByRole('button',{name:'Albums',exact:true}).click();
 await expect(page.locator('.collection-card.currently-playing')).toHaveCount(1);await expect(page.locator('.collection-card.currently-playing')).toContainText('Discovery');await expect(page.locator('.card-playing')).toHaveAttribute('aria-label','Now playing');
 await expect(page.getByRole('button',{name:'Search music and commands'})).toContainText('⌘ K');
 await expect(page.locator('.now-playing [data-player-cycle="playing"] [data-icon="playerQueue"]')).toHaveCount(1);
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/playing-card-indicator.png'});
});
test('grid titles cap at two lines, with full text retained for album view and tooltip',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);
 const card=page.locator('.collection-card').first();
 await card.locator('strong').evaluate(el=>el.textContent='Random Access Memories (10th Anniversary Edition)');await card.locator('small').evaluate(el=>el.textContent='Kevin Abstract; Dominic Fike & Another Contributor');
 await expect(card.locator('strong')).toHaveCSS('white-space','normal');await expect(card.locator('strong')).toHaveCSS('overflow','hidden');await expect(card.locator('strong')).toHaveCSS('-webkit-line-clamp','2');await expect(card.locator('small')).toHaveCSS('white-space','nowrap');
 expect(await card.locator('strong').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThanOrEqual(1);
 const caption=await card.locator('.card-caption').boundingBox(),subtitle=await card.locator('small').boundingBox();expect(caption!.y+caption!.height).toBeGreaterThanOrEqual(subtitle!.y+subtitle!.height-1);
 const nextRow=await page.locator('.collection-card').nth(6).boundingBox();expect(nextRow!.y).toBeGreaterThan(caption!.y+caption!.height);
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/long-card-titles.png'});
});
test('a delayed known artist portrait keeps its ordinary artwork without a completion overlay',async({page})=>{
 const held: (()=>Promise<void>)[]=[];await page.route('**/*m83*.png',route=>{held.push(()=>route.continue());});
 await page.goto('/?fixture=1');await openAlbums(page);await page.getByRole('button',{name:'Artists',exact:true}).click();
 const card=page.locator('.collection-card[data-artist-name="M83"]');await expect(card).toBeVisible();
 await expect(card.getByRole('status',{name:'Loading artist photo'})).toHaveCount(0);await expect(card.locator('.portrait-loading')).toHaveCount(0);
 await page.emulateMedia({reducedMotion:'reduce'});await expect.poll(()=>held.length).toBeGreaterThan(0);for(const release of held)await release();
 await expect(card.locator('img.cover')).toBeVisible();await expect(card.locator('.portrait-loading')).toHaveCount(0);
});
test('card artwork stays free of corner Play buttons and retains keyboard actions',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);
 const card=page.locator('.collection-card').filter({has:page.getByRole('button',{name:'Open Discovery',exact:true})});
 await card.hover();
 await expect(card.getByRole('button',{name:'Play Discovery',exact:true})).toHaveCount(0);
 await expect(card.locator('.menu-trigger')).toHaveCSS('opacity','0');
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/transparent-card-play.png'});
 await card.locator('.card-open').focus();await page.keyboard.press('Shift+F10');await expect(page.getByRole('menuitem',{name:'View Artist',exact:true})).toBeVisible();await page.keyboard.press('Escape');await expect(card.locator('.card-open')).toBeFocused();
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
 const detailPlay=page.getByRole('button',{name:'Play collection',exact:true});await detailPlay.hover();
 await expect(detailPlay).toHaveCSS('background-color','rgba(0, 0, 0, 0)');await expect(detailPlay).toHaveCSS('box-shadow','none');await expect(detailPlay).toHaveCSS('border-radius','0px');
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/transparent-play-correction.png'});
});

test('pending artist lookup keeps missing portraits sharp and never blurs an album fallback',async({page})=>{
 await page.goto('/?fixture=1&profiles=pending');await page.getByRole('button',{name:'Artists',exact:true}).click();
 const missing=page.locator('.collection-card[data-artist-name="Gorillaz"]');
 await expect(missing.locator('.portrait-loading[data-photo=false]')).toBeVisible();
 await expect(missing.locator('.portrait-loading > img')).toHaveCount(0);
 await expect(missing.locator('.cover')).toHaveCSS('object-fit','contain');await expect(missing.locator('.cover')).toHaveCSS('filter','none');
 const photo=page.locator('.collection-card[data-artist-name="M83"]');await expect(photo.locator('.cover')).not.toHaveClass(/placeholder/);await expect(photo.getByRole('status',{name:'Loading artist photo'})).toHaveCount(0);
 const cover=photo.locator('.cover');const original=await cover.getAttribute('src');
 await photo.locator('.card-open').focus();await page.keyboard.press('Shift+F10');await page.getByRole('menuitem',{name:'Play',exact:true}).click();await expect(page.locator('.now-playing-info')).toContainText('Intro');
 await expect(cover).toHaveAttribute('src',original!);await expect(photo.getByRole('status',{name:'Loading artist photo'})).toHaveCount(0);
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/sharp-placeholder-loading.png'});
});
