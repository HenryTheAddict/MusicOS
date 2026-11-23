import {test,expect,type Page,type Locator} from '@playwright/test';
// DesignGuidelines §8: button and right-click menus stay inside the window, so
// every enabled action stays pointer-reachable in short windows.
async function expectReachable(page:Page,shot?:string){
 const menu=page.locator('.menu-panel[data-state=open]');await expect(menu).toBeVisible();await page.waitForTimeout(250);if(shot)await page.screenshot({path:`out/rebuild-qa/${shot}.png`});
 const box=(await menu.boundingBox())!,view=page.viewportSize()!;
 expect(box.y).toBeGreaterThanOrEqual(0);expect(box.y+box.height).toBeLessThanOrEqual(view.height);
 expect(box.x).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(view.width);
 const items=menu.locator('[role=menuitem]:not([data-disabled])'),labels=await items.allTextContents();expect(labels.length).toBeGreaterThan(0);
 const reached=new Set<string>();await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.wheel(0,-4000);await page.waitForTimeout(80);
 for(let step=0;step<40&&reached.size<labels.length;step++){
  for(const label of await items.evaluateAll(els=>els.filter(el=>{const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.left+r.width/2,r.top+r.height/2);return r.top>=0&&r.bottom<=innerHeight&&!!hit&&el.contains(hit);}).map(el=>el.textContent??'')))reached.add(label);
  await page.mouse.wheel(0,60);await page.waitForTimeout(40);
 }
 expect([...labels].filter(label=>!reached.has(label))).toEqual([]);
}
async function closeAndCheckFocus(page:Page,owner:Locator){await page.keyboard.press('Escape');await expect(page.locator('.menu-panel[data-state=open]')).toHaveCount(0);await expect.poll(()=>owner.evaluate(el=>el.contains(document.activeElement))).toBe(true);}
for(const percent of [100,150])for(const mode of ['light','dark'] as const)test(`song menus stay inside a 720x480 window at ${percent}% ${mode}`,async({page})=>{
 test.setTimeout(60000);await page.setViewportSize({width:720,height:480});await page.goto('/?fixture=1');
 await page.evaluate(([value,mode])=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,display_name:'QA',interface_percent:value,appearance:{mode,source:'blue',accent:'#2f6fed',saturation:1,tint:0}}));sessionStorage.removeItem('musicos-navigation-v1');},[percent,mode] as const);await page.reload();
 await expect(page.locator('html')).toHaveAttribute('data-theme-mode',mode);
 for(const route of ['songs','album']){
  if(route==='songs')await page.getByRole('button',{name:'Songs',exact:true}).click();
  else{await page.getByRole('button',{name:'Albums',exact:true}).click();await page.getByRole('button',{name:'Open Bubblewrap Lagoon',exact:true}).click();}
  const rows=page.locator('.track-body > .track-row');await expect(rows.first()).toBeVisible();await rows.first().scrollIntoViewIfNeeded();await page.waitForTimeout(300);
  const scroll=(await page.locator('.page-scroll').first().boundingBox())!;
  const visible=await rows.evaluateAll((els,bounds)=>els.map((el,index)=>({index,r:el.getBoundingClientRect()})).filter(({r})=>r.top>=bounds.y&&r.bottom<=bounds.y+bounds.height).map(({index})=>index),scroll);expect(visible.length).toBeGreaterThan(0);
  for(const index of [visible[0],visible[visible.length-1]]){
   const row=rows.nth(index),trigger=row.getByRole('button',{name:'More actions'});
   await trigger.click();await expectReachable(page,`menu-bounds-${route}-button-${index}-${percent}-${mode}`);await closeAndCheckFocus(page,trigger);
   const b=(await row.boundingBox())!;await page.mouse.click(b.x+b.width*.4,b.y+b.height/2,{button:'right'});await expectReachable(page,`menu-bounds-${route}-context-${index}-${percent}-${mode}`);await page.keyboard.press('Escape');await expect(page.locator('.menu-panel[data-state=open]')).toHaveCount(0);
   await row.focus();await page.keyboard.press('Shift+F10');await expectReachable(page);await page.keyboard.press('End');const last=page.locator('.menu-panel[data-state=open] [role=menuitem]:not([data-disabled])').last();await expect(last).toBeFocused();await expect(last).toBeInViewport({ratio:1});await page.keyboard.press('Home');await expect(page.locator('.menu-panel[data-state=open] [role=menuitem]:not([data-disabled])').first()).toBeFocused();await closeAndCheckFocus(page,row);
  }
  const bottom=(await rows.nth(visible[visible.length-1]).boundingBox())!;await page.mouse.click(scroll.x+scroll.width*.4,Math.min(bottom.y+bottom.height+4,scroll.y+scroll.height-2),{button:'right'});
  if(await page.locator('.menu-panel[data-state=open]').count()){await expectReachable(page);await page.keyboard.press('Escape');}
 }
});
