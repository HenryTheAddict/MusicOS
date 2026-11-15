import {expect,test} from '@playwright/test';

test('visualizer settings use one compact, accessible disclosure',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Choose section',exact:true}).click();
 await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 const disclosure=page.locator('.visualizer-preferences');
 const header=disclosure.locator('summary');
 await expect(header).toHaveText('Visualizer');
 await expect(header.locator('.icon')).toHaveCount(1);
 await header.focus();
 await page.keyboard.press('Enter');
 await expect(disclosure.getByRole('button',{name:'Visualizer effect'})).toBeVisible();
 await expect(disclosure.getByRole('switch',{name:'Album colors'})).toBeVisible();
 expect((await disclosure.boundingBox())!.height).toBeLessThan(300);
 await header.blur();
 await disclosure.screenshot({path:'out/rebuild-qa/visualizer-settings-compact.png',animations:'disabled'});
 await header.click();
 await expect(disclosure.getByRole('switch',{name:'Album colors'})).toHaveCount(0);
});

test('Favorites live with your collections and recent songs preview from their menu',async({page})=>{
 await page.goto('/?fixture=1&homeActivity=1');
 await expect(page.getByRole('region',{name:'Your collections'}).getByRole('button',{name:/Favorites/})).toBeVisible();
 const recent=page.getByRole('region',{name:'Recently played'});
 await expect(recent.getByRole('button',{name:/^Play /})).toHaveCount(3);
 await expect(recent.getByRole('button',{name:/Preview/})).toHaveCount(0);
 const favorite=page.locator('.sidebar-favorites');
 const row=recent.locator('[data-action-surface]').first();
 await expect(favorite).toHaveCSS('border-top-style','solid');
 await expect(row).toHaveCSS('border-top-style','solid');
 const measures=await row.evaluate(node=>{
  const middle=(el:Element)=>{const r=el.getBoundingClientRect();return r.top+r.height/2;};
  return Math.abs(middle(node.querySelector('.icon-button')!)-middle(node.querySelector('.menu-trigger')!));
 });
 expect(measures).toBeLessThan(1);

 await recent.locator('.home-recent-copy').first().click({button:'right'});
 await expect(page.getByRole('menuitem',{name:'Preview 30 seconds'})).toBeVisible();
});

test('DJ mode shows the current track waveform on the same playback slider',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await page.getByRole('button',{name:'Play collection',exact:true}).click();
 const bar=page.locator('.now-playing');
 await bar.getByRole('button',{name:'Open Up next',exact:true}).click();
 const dj=page.getByRole('switch',{name:'DJmode'});
 await page.locator('.queue-dj .themed-switch').click();
 await expect(dj).toBeChecked();
 const wave=bar.locator('.seek-wave');
 await expect(wave).toBeVisible();
 await expect.poll(async()=>(await wave.boundingBox())?.height??0).toBeGreaterThan(6);
 await expect(bar.getByRole('slider',{name:'Playback position'})).toBeEnabled();
 await expect(wave.locator('path')).toHaveCount(2);
 // Moving the playhead never rewrites the waveform SVG.
 await wave.evaluate(svg=>{
  const result={changes:0};
  window.addEventListener('musicos-test-check-wave',()=>document.documentElement.dataset.waveMutations=String(result.changes),{once:true});
  new MutationObserver(records=>result.changes+=records.length).observe(svg,{attributes:true,childList:true,subtree:true});
 });
 const waveformSeek=bar.getByRole('slider',{name:'Playback position'});
 await waveformSeek.focus();
 await waveformSeek.press('End');
 await page.evaluate(()=>window.dispatchEvent(new Event('musicos-test-check-wave')));
 await expect(page.locator('html')).toHaveAttribute('data-wave-mutations','0');

 await page.locator('.queue-dj .themed-switch').click();
 await expect(dj).not.toBeChecked();
 await expect(wave).toHaveCount(0);
});

test('a dropped image can be reviewed and saved without a provider lookup',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Choose section',exact:true}).click();
 await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 await page.getByRole('tab',{name:'Library',exact:true}).click();
 await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();
 const browser=page.locator('.tag-albums');
 await page.getByRole('searchbox',{name:'Find albums'}).fill('Discovery');
 await browser.getByRole('button',{name:/Discovery/}).first().click();
 const data=await page.evaluateHandle(()=>{
  const canvas=document.createElement('canvas');
  canvas.width=160;
  canvas.height=96;
  const context=canvas.getContext('2d')!;
  context.fillStyle='#8040c0';
  context.fillRect(0,0,160,96);
  return new Promise<DataTransfer>(resolve=>canvas.toBlob(blob=>{
   const transfer=new DataTransfer();
   transfer.items.add(new File([blob!],'cover.png',{type:'image/png'}));
   resolve(transfer);
  }));
 });
 await page.locator('[data-album-cover-drop]').dispatchEvent('drop',{dataTransfer:data});
 await expect(page.getByText(/160 × 96/).first()).toBeVisible();
 await page.getByRole('button',{name:'Review cover',exact:true}).click();
 await expect(page.getByRole('region',{name:'Review changes'})).toContainText('Discovery');
 await page.getByRole('button',{name:'Save cover only',exact:true}).click();
 await expect(page.getByRole('status').filter({hasText:'Cover updated'})).toBeVisible();
});

test('Appearance panels align and stay reachable in compact and wide settings',async({page})=>{
 for(const width of [480,720,1280]){
  await page.setViewportSize({width,height:width===1280?800:480});
  await page.goto('/?fixture=1');
  await page.getByRole('button',{name:'Choose section',exact:true}).click();
  await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
  const panel=page.getByRole('tabpanel',{name:'Appearance'});
  const cards=panel.locator('.appearance-bento .settings-tile:visible');
  await expect(cards.first()).toBeVisible();
  const bounds=await cards.evaluateAll(nodes=>nodes.map(node=>{
   const {left,right,width}=node.getBoundingClientRect();
   return {left,right,width};
  }));
  expect(bounds.length).toBeGreaterThan(3);
  for(const card of bounds){
   expect(Math.abs(card.left-bounds[0].left)).toBeLessThan(2);
   expect(Math.abs(card.width-bounds[0].width)).toBeLessThan(2);
   expect(card.right).toBeLessThanOrEqual(width);
  }
  const visualizer=panel.locator('.visualizer-preferences summary');
  await visualizer.focus();
  await expect(visualizer).toBeInViewport();
 }
});
