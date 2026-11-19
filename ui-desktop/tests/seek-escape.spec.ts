import {test,expect} from '@playwright/test';

for(const scenario of [
 {width:1280,height:800,mode:'light',reducedMotion:'no-preference'},
 {width:720,height:480,mode:'dark',reducedMotion:'reduce'},
] as const)test(`Escape cancels a held seek before closing the player (${scenario.mode})`,async({page})=>{
 // The compact artwork button opens Now Playing directly in either layout.
 await page.setViewportSize({width:480,height:scenario.height});
 await page.emulateMedia({reducedMotion:scenario.reducedMotion});
 await page.addInitScript(mode=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode,source:'blue',saturation:100,tint:15}})),scenario.mode);
 // Seed a paused track so cancellation can be compared to a stable position.
 await page.route('**/src/fixtures.ts*',async route=>{
  const response=await route.fetch();
  await route.fulfill({response,body:(await response.text()).replace('let player = { ...emptyPlayback };','let player = {...emptyPlayback,track:tracks[0],queue:[tracks[0]],queue_index:0,queue_entry_ids:[71],position:30};')});
 });
 await page.goto('/?fixture=1');
 const opener=page.locator('.now-playing-cover');
 await opener.click();
 const player=page.getByRole('dialog',{name:'Music player',exact:true});
 await expect(player).toBeVisible();
 await page.setViewportSize({width:scenario.width,height:scenario.height});
 const slider=player.getByRole('slider',{name:'Playback position',exact:true});
 const progress=player.locator('.playback-progress');
 const seeks=()=>page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='playback'&&c.args.action.type==='seek').map((c:any)=>c.args.action));
 const original=await slider.inputValue();
 await slider.focus();
 // The revealed time label must finish changing the rail geometry first.
 await expect.poll(()=>progress.evaluate(el=>el.getAnimations({subtree:true}).filter(a=>a.playState==='running').length)).toBe(0);
 const box=(await slider.boundingBox())!;
 await slider.evaluate(el=>el.addEventListener('pointerdown',event=>{(window as any).__seekPointerId=(event as PointerEvent).pointerId;},{once:true}));
 await page.mouse.move(box.x+box.width*.2,box.y+box.height/2);
 await page.mouse.down();
 await page.mouse.move(box.x+box.width*.8,box.y+box.height/2);
 await expect(progress).toHaveAttribute('data-scrubbing','true');
 expect(Number(await slider.inputValue())).toBeGreaterThan(Number(original));
 await expect(slider).toHaveCSS('cursor','grabbing');

 await page.keyboard.press('Escape');
 await expect(player).toBeVisible();
 await expect(progress).not.toHaveAttribute('data-scrubbing');
 await expect(slider).toHaveValue(original);
 await expect(slider).toBeFocused();
 expect(await slider.evaluate(el=>el.hasPointerCapture((window as any).__seekPointerId))).toBe(false);
 await expect(slider).toHaveCSS('cursor','pointer');
 // Releasing outside the control after cancellation must not commit the draft.
 await page.mouse.move(box.x+box.width*.9,box.y-40);
 await page.mouse.up();
 expect(await seeks()).toEqual([]);

 // The same slider can start a fresh drag, retain capture outside, and commit once.
 await page.mouse.move(box.x+box.width*.2,box.y+box.height/2);
 await page.mouse.down();
 await page.mouse.move(box.x+box.width*.6,box.y-40);
 await page.mouse.up();
 await expect.poll(async()=>(await seeks()).length).toBe(1);
 await expect(progress).not.toHaveAttribute('data-scrubbing');
 await expect(slider).toHaveCSS('cursor','pointer');
 await slider.press('ArrowRight');
 await expect.poll(async()=>(await seeks()).length).toBe(2);
 await page.keyboard.press('Escape');
 await expect(player).toHaveCount(0);
 await expect(opener).toBeFocused();
});
