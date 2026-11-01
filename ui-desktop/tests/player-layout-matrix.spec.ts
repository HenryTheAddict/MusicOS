import{test,expect}from'@playwright/test';
test.use({browserName:'webkit',reducedMotion:'reduce'});
for(const width of[480,720,1280])for(const scale of[80,100,125,150])test(`player fits ${width}px at ${scale}% across themes`,async({page})=>{
 test.setTimeout(60000);await page.setViewportSize({width,height:width===1280?800:480});
 for(const appearance of[{mode:'light',source:'artwork'},{mode:'dark',source:'artwork'},{mode:'light',source:'blue'},{mode:'light',source:'custom',accent:'#ffff00'},{mode:'dark',source:'custom',accent:'#000000'}]){
 await page.goto('/?fixture=1&lyrics=1');await page.evaluate(({scale,appearance})=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:scale,appearance:{saturation:150,tint:40,...appearance}})),{scale,appearance});await page.reload();await page.getByRole('button',{name:'Open lyrics',exact:true}).click();await page.keyboard.press('Tab');const player=page.getByRole('dialog',{name:'Music player'});await expect(player.locator('.lyric-line')).toHaveCount(4);const bounds=await player.evaluate(n=>{const r=n.getBoundingClientRect(),dock=n.querySelector('.expanded-player-dock')!.getBoundingClientRect(),header=n.querySelector('.expanded-player-header')!.getBoundingClientRect();return {overflow:n.scrollWidth-n.clientWidth,dockInside:dock.left>=r.left&&dock.right<=r.right+1&&dock.bottom<=r.bottom,headerInside:header.left>=r.left&&header.right<=r.right+1};});expect(bounds.overflow).toBeLessThan(2);expect(bounds.dockInside).toBe(true);expect(bounds.headerInside).toBe(true);await page.keyboard.press('Escape');
 }
});
