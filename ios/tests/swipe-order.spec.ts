import {test,expect,type Locator} from '@playwright/test';

// Exercise native touch listeners in WebKit, including a vertical scroll cancellation.
// This checks recognition/actions; tactile feel still needs the physical iPhone.
async function swipe(target:Locator,dx:number,dy=0,edge=false){
 await target.evaluate(async(node,{dx,dy,edge})=>{
  const box=node.getBoundingClientRect(),x=edge?25:box.x+box.width/2,y=box.y+Math.min(28,box.height/2);
  const touch=(px:number,py:number)=>({identifier:7,target:node,clientX:px,clientY:py,pageX:px,pageY:py});
  const event=(name:string,touches:any[],changedTouches:any[])=>{const e=new Event(name,{bubbles:true,cancelable:true});Object.defineProperties(e,{touches:{value:{...touches,length:touches.length}},changedTouches:{value:{...changedTouches,length:changedTouches.length}}});return e;};
  node.dispatchEvent(event('touchstart',[touch(x,y)],[touch(x,y)]));
  for(let step=1;step<=5;step++){
   const t=touch(x+dx*step/5,y+dy*step/5);
   node.dispatchEvent(event('touchmove',[t],[t]));
   await new Promise(requestAnimationFrame);
  }
  const t=touch(x+dx,y+dy);
  node.dispatchEvent(event('touchend',[],[t]));
 },{dx,dy,edge});
}
async function touchTap(target:Locator){
 await target.evaluate(node=>{const b=node.getBoundingClientRect(),t={identifier:8,target:node,clientX:b.x+b.width/2,clientY:b.y+b.height/2};for(const name of ['touchstart','touchend']){const e=new Event(name,{bubbles:true,cancelable:true});Object.defineProperties(e,{touches:{value:name==='touchstart'?[t]:[]},changedTouches:{value:[t]}});node.dispatchEvent(e);}(node as HTMLElement).click();});
}
test('library sorts albums naturally and album playback follows discs/tracks, not copy order',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Library',exact:true}).click();
 const albums=page.locator('.album-tile');await expect(albums.nth(0)).toContainText('Album 1');await expect(albums.nth(1)).toContainText('Album 2');await expect(albums.nth(9)).toContainText('Album 10');
 await albums.first().click();
 await expect(page.locator('.song-open strong')).toHaveText(['Second light','Soft signal','Transit','Silver morning','Second light 5','Soft signal 6']);
 await page.getByRole('button',{name:'Play',exact:true}).click();await page.getByRole('button',{name:'Up next',exact:true}).click();
 await expect(page.locator('.queue-current')).toContainText('Second light');
 await expect(page.locator('.phone-reorder-row strong')).toHaveText(['Soft signal','Transit','Silver morning','Second light 5','Soft signal 6']);
});
test('song catalogue sorts titles while saved playlist duplicates retain their exact order',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Songs',exact:true}).click();
 const titles=await page.locator('.song-open strong').allTextContents();
 expect(titles[0]).toBe('Second light');expect(titles[1]).toBe('Second light 5');expect(titles[2]).toBe('Second light 9');
 await page.getByRole('button',{name:'Home',exact:true}).click();await page.getByRole('button',{name:'Weekend 4 songs',exact:true}).click();
 await expect(page.locator('.song-open strong')).toHaveText(['Second light','Transit','Second light','Transit 7']);
});
test('song swipe follows the finger, reveals Play next, and leaves vertical list gestures alone',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Songs',exact:true}).click();
 const row=page.locator('.phone-swipe-song').first();await swipe(row,0,-90);await expect(row).not.toHaveAttribute('data-open');
 await swipe(row,-85);await expect(row).toHaveAttribute('data-open');
 await touchTap(row.getByRole('button',{name:'Play next',exact:true}));await expect(row).not.toHaveAttribute('data-open');
 await expect(page.getByRole('button',{name:'Resume',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Up next',exact:true}).click();await expect(page.locator('.phone-reorder-row')).toHaveCount(6);
});
test('mini-player swipes skip once, expand, and header swipe dismisses',async({page})=>{
 await page.goto('/?fixture=1');const mini=page.locator('.phone-mini');await swipe(mini,-100);
 await expect(mini).toContainText('Soft signal');await swipe(mini,100);await expect(mini).toContainText('Second light');
 await swipe(mini,0,-95);await expect(page.getByRole('dialog',{name:'Player',exact:true})).toBeVisible();
 await swipe(page.locator('.player-top'),0,100);await expect(page.getByRole('dialog',{name:'Player',exact:true})).toHaveCount(0);
 await expect(page.locator('[data-swiping]')).toHaveCount(0);
});
test('edge swipe over a detail song returns to retained library without queueing or playing',async({page})=>{
 await page.goto('/?fixture=1');await page.getByRole('button',{name:'Library',exact:true}).click();await page.locator('.album-tile').first().click();
 await page.locator('main').evaluate(n=>n.scrollTop=280);
 await swipe(page.locator('.phone-swipe-song').first(),100,0,true);
 await expect(page.locator('.album-tile').first()).toBeVisible();
 await expect(page.getByRole('button',{name:'Resume',exact:true})).toBeVisible();await expect(page.locator('[data-swiping]')).toHaveCount(0);
});
