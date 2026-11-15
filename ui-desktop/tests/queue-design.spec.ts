import {test,expect} from '@playwright/test';
test('queue stays compact and opens full analysis in Audio settings',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const queue=page.getByRole('complementary',{name:'Up next'});
 await expect(queue.locator('.audio-analysis')).toHaveCount(0);
 await expect(queue.getByText('Your next listen starts here')).toBeVisible();
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/queue-clean-1280.png'});
 await page.keyboard.press('Escape');await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Audio',exact:true}).click();await page.getByRole('button',{name:'Quality & analysis',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Audio',exact:true})).toHaveAttribute('aria-selected','true');
 await expect(page.getByRole('region',{name:'BPM and key analysis'})).toBeVisible();
 await expect(page.getByRole('button',{name:'Analyze library',exact:true})).toBeDisabled();
 await page.screenshot({animations:'disabled',path:'out/rebuild-qa/audio-analysis-settings-1280.png'});
});
test('empty queue fits compact windows at each interface size',async({page})=>{
 for(const percent of [80,100,125,150]){
  await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');
  await page.evaluate(value=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'Henry',interface_percent:value,coverflow_colors:false})),percent);await page.reload();
  await page.getByRole('button',{name:'Open Up next',exact:true}).click();
  const queue=page.locator('.queue-panel'),box=await queue.boundingBox();
  expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.y).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(721);expect(box!.y+box!.height).toBeLessThanOrEqual(481);
  expect(await queue.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThanOrEqual(1);
  await page.screenshot({animations:'disabled',path:`out/rebuild-qa/queue-clean-720-${percent}.png`});
 }
});

test('the open panel floats below the dock threshold and both layouts reach the workspace edge',async({page})=>{
 await page.setViewportSize({width:1280,height:560});
 await page.goto('/?fixture=1&scale=100');
 await page.getByRole('button',{name:'Open Up next',exact:true}).click();
 const panel=page.getByRole('complementary',{name:'Up next'});
 await expect(panel).toHaveClass(/audio-docked/);
 const panelBottom=()=>panel.evaluate(el=>el.getBoundingClientRect().bottom);
 const workspace=page.locator('.workspace');
 const workspaceBottom=()=>workspace.evaluate(el=>el.getBoundingClientRect().bottom);
 expect(await workspaceBottom()-await panelBottom()).toBeLessThan(7);
 await page.setViewportSize({width:840,height:480});
 await expect(panel).toHaveClass(/audio-floating/);
 expect(await workspaceBottom()-await panelBottom()).toBeLessThan(7);
 await page.getByRole('button',{name:'Show Lyrics',exact:true}).click();
 await expect(page.getByRole('complementary',{name:'Lyrics'})).toBeVisible();
 await page.getByRole('button',{name:'Close lyrics',exact:true}).click();
 await expect(page.getByRole('button',{name:'Open Up next',exact:true})).toBeVisible();
});
