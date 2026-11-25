import {test,expect} from '@playwright/test';
import {openPlaylistCreator} from './support/ui';
async function settings(page:import('@playwright/test').Page){await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();}
test('Settings controls are rounded and flat, with no separator above integration toggles',async({page})=>{
 for(const width of [720,1280]){
  await page.setViewportSize({width,height:width===720?480:800});await page.goto('/?fixture=1');await settings(page);await page.getByRole('tab',{name:'Audio',exact:true}).click();await expect(page.getByRole('heading',{name:'Playback',exact:true})).toHaveCount(0);await expect(page.getByRole('tabpanel',{name:'Audio',exact:true}).getByText('Playback details',{exact:true})).toBeVisible();
  const buttons=page.locator('.settings button:visible');expect(await buttons.evaluateAll(nodes=>nodes.every(node=>{const s=getComputedStyle(node);return parseFloat(s.borderRadius)>0&&s.backgroundImage==='none'&&s.boxShadow==='none';}))).toBe(true);await page.screenshot({path:`out/rebuild-qa/friendly-audio-${width}.png`,animations:'disabled'});
  await page.getByRole('tab',{name:'Integrations',exact:true}).click();await expect(page.locator('.integration-heading .themed-switch').first()).toHaveCSS('border-top-width','0px');await expect(page.locator('.beta-integrations')).toHaveCSS('background-image','none');await expect(page.locator('.integration-card')).toHaveCount(2);await page.screenshot({path:`out/rebuild-qa/friendly-integrations-${width}.png`,animations:'disabled'});
  await page.getByRole('tab',{name:'Library',exact:true}).click();for(const button of await page.locator('.source-row button').all())expect(parseFloat(await button.evaluate(el=>getComputedStyle(el).borderRadius))).toBeGreaterThan(0);await page.screenshot({path:`out/rebuild-qa/friendly-folders-${width}.png`,animations:'disabled'});
 }
});
test('playlist details use a compact separate artwork chooser and retain editing',async({page})=>{
 await page.goto('/?fixture=1');await openPlaylistCreator(page);const create=page.getByRole('dialog',{name:'Create playlist',exact:true});await create.getByRole('textbox',{name:'Name',exact:true}).fill('Evening mix');await create.getByRole('button',{name:'Create playlist',exact:true}).click();
 const hero=page.locator('.playlist-hero');await hero.getByRole('button',{name:'More actions',exact:true}).click();await page.getByRole('menuitem',{name:'Edit details',exact:true}).click();
 const editor=page.getByRole('dialog',{name:'Edit playlist details'});await expect(editor.locator('.cover-options')).toHaveCount(0);await editor.getByRole('button',{name:'Change artwork'}).click();
 const artwork=page.getByRole('dialog',{name:'Choose cover artwork'});const clouds=artwork.getByRole('button',{name:'Choose Clouds cover',exact:true});await expect(clouds).toHaveAttribute('aria-pressed','false');await clouds.click();await expect(artwork).not.toBeVisible();await expect(editor.getByAltText('Evening mix cover',{exact:true})).toHaveAttribute('src',/Clouds\.png/);
 await editor.getByRole('textbox',{name:'Playlist name'}).fill('Renamed evening');await editor.getByRole('button',{name:'Done',exact:true}).click();await expect(page.getByRole('heading',{name:'Renamed evening',exact:true})).toBeVisible();await expect(hero.getByAltText('Renamed evening cover',{exact:true})).toHaveAttribute('src',/Clouds\.png/);await page.screenshot({path:'out/rebuild-qa/friendly-playlist-editor.png',animations:'disabled'});
});
test('flat Settings remain readable in dark and adaptive palettes',async({page})=>{
 for(const palette of ['dark','adaptive']){
  await page.setViewportSize({width:1280,height:800});await page.goto('/?fixture=1');
  if(palette==='adaptive'){await page.evaluate(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:"Henry",interface_percent:100,appearance:{mode:'light',source:'artwork',accent:'#0064df',saturation:100,tint:15}})));await page.reload();await page.getByRole('button',{name:'Open Clockwork Confetti',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();await expect(page.locator('.app-shell')).toHaveAttribute('data-adaptive','true');}
  await settings(page);
  if(palette==='dark')await page.locator('.app-shell').evaluate(el=>{const colors=['#202329','#f3f6ff','#303c52','#97baff','#202329','#102532'];['--surface-color','--ink','--blob-color','--accent','--shell-color','--accent-ink'].forEach((key,i)=>{(el as HTMLElement).style.setProperty(key,colors[i]);document.documentElement.style.setProperty(key,colors[i]);});el.querySelectorAll<HTMLElement>('.theme-layer').forEach(layer=>{layer.style.backgroundColor=colors[0];layer.style.backgroundImage='none';});});
  await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.screenshot({path:`out/rebuild-qa/friendly-audio-${palette}.png`,animations:'disabled'});
  await page.getByRole('tab',{name:'Integrations',exact:true}).click();await expect(page.locator('.integration-heading .themed-switch').first()).toHaveCSS('border-top-width','0px');await page.screenshot({path:`out/rebuild-qa/friendly-integrations-${palette}.png`,animations:'disabled'});
 }
});
