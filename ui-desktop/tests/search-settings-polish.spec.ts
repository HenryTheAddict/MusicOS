import {test,expect,type Page} from '@playwright/test';
import {chooseSection,openAlbums} from './support/ui';
async function settings(page:Page){await chooseSection(page,'Settings');}
test('Albums search animates both directions, reverses in place, and restores focus',async({page})=>{
 await page.goto('/?fixture=1');await openAlbums(page);const opener=page.getByRole('button',{name:'Search music and commands',exact:true});await opener.click();const dialog=page.getByRole('dialog',{name:'SuperCommand',exact:true});await expect(dialog).toHaveAttribute('data-open','true');
 const box=page.getByRole('combobox',{name:'SuperCommand search'});await box.fill('Bubblewrap Lagoon');await expect(page.locator('.command-results [role=option]').first()).toContainText('Bubblewrap Lagoon');
 await expect.poll(()=>dialog.evaluate(el=>+getComputedStyle(el).opacity)).toBe(1);
 // Freeze a real closing animation partway through to check reversal without a remount.
 await page.keyboard.press('ControlOrMeta+k');await dialog.evaluate(el=>{const a=el.getAnimations().find(a=>a.id==='command-presence')!;a.pause();a.currentTime=60;el.setAttribute('data-reversal-marker','same-dialog');});
 const closing=await dialog.evaluate(el=>+getComputedStyle(el).opacity);expect(closing).toBeGreaterThan(0);expect(closing).toBeLessThan(1);
 await page.keyboard.press('ControlOrMeta+k');await expect(dialog).toHaveAttribute('data-open','true');await expect(dialog).toHaveAttribute('data-reversal-marker','same-dialog');await expect(box).toHaveValue('Bubblewrap Lagoon');await expect.poll(()=>dialog.evaluate(el=>+getComputedStyle(el).opacity)).toBe(1);
 await page.keyboard.press('Escape');await expect(dialog).toHaveCount(0);await expect(opener).toBeFocused();
 await page.getByRole('button',{name:'Open Bubblewrap Lagoon',exact:true}).click();await expect(page.getByRole('button',{name:'Open SuperCommand',exact:true}).locator('svg')).toHaveCount(1);
});
test('Reduced Motion switches search states without movement',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});await page.goto('/?fixture=1');await page.getByRole('button',{name:'Search music and commands',exact:true}).click();const dialog=page.locator('.supercommand');await expect(dialog).toHaveCSS('transform','none');expect(await dialog.evaluate(el=>el.getAnimations().filter(a=>a.playState==='running').length)).toBe(0);await page.keyboard.press('Escape');await expect(dialog).toHaveCount(0);
});
for(const percent of [80,100,125,150])for(const width of [720,1280])test(`Settings and folder management fit ${width}px at ${percent}%`,async({page})=>{
 await page.setViewportSize({width,height:width===720?480:800});await page.addInitScript(({percent})=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:percent,appearance:{mode:percent===80||percent===125?'dark':'light',source:percent===150?'custom':'artwork',accent:'#ffff00',saturation:150,tint:20}})),{percent});await page.goto('/?fixture=1&sparkReady=1');await settings(page);await page.getByRole('tab',{name:'Library',exact:true}).click();await expect(page.locator('.source-row')).toHaveCount(1);
 const inspect=async()=>{const overflow=await page.locator('.settings-panel').evaluate(el=>el.scrollWidth-el.clientWidth);expect(overflow).toBeLessThan(2);const overlaps=await page.locator('.source-row').evaluateAll(rows=>rows.map(row=>{const copy=row.querySelector('.source-copy')!.getBoundingClientRect(),actions=row.querySelector('.source-actions')!.getBoundingClientRect();return copy.right>actions.left+1&&copy.bottom>actions.top+1&&actions.bottom>copy.top+1;}));expect(overlaps).not.toContain(true);};await inspect();
 await page.getByRole('button',{name:'Remove from library',exact:true}).click();await expect(page.getByRole('button',{name:'Restore folder',exact:true})).toBeVisible();await inspect();await page.getByRole('button',{name:'Restore folder',exact:true}).click();await expect(page.getByRole('button',{name:'Remove from library',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();await expect(page.getByRole('button',{name:'All albums',exact:true})).toBeVisible();await page.getByRole('button',{name:'More metadata tools'}).click();await page.getByRole('menuitem',{name:'Bulk edit',exact:true}).click();await expect(page.locator('.metadata-workspace')).toBeVisible();expect(await page.locator('.settings-panel').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);
 await chooseSection(page,'Library');await settings(page);await expect(page.getByRole('tab',{name:'Library',exact:true})).toHaveAttribute('aria-selected','true');await expect(page.locator('.batch-metadata')).toBeVisible();
 await page.getByRole('button',{name:'Music folders',exact:true}).click();await page.locator('.settings-category-heading').scrollIntoViewIfNeeded();await page.screenshot({path:`out/rebuild-qa/search-settings-${width}-${percent}.png`,animations:'disabled'});
});
