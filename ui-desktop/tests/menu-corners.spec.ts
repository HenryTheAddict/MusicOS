import {test,expect} from '@playwright/test';

async function corner(page:import('@playwright/test').Page,selector:string){
 const box=await page.locator(selector).boundingBox();const png=await page.screenshot({clip:{x:Math.ceil(box!.x+1),y:Math.ceil(box!.y+1),width:1,height:1},scale:'css'});
 return page.evaluate(async png=>{const img=new Image();img.src='data:image/png;base64,'+png;await img.decode();const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d')!;ctx.drawImage(img,0,0);return [...ctx.getImageData(0,0,1,1).data];},png.toString('base64'));
}
function sameCorner(a:number[],b:number[]){expect(Math.max(...a.map((v,i)=>Math.abs(v-b[i])))).toBeLessThanOrEqual(2);}

test('opening menus preserves each trigger contour instead of inheriting a square parent',async({page})=>{
 await page.goto('/?fixture=1');const more=page.getByRole('button',{name:'More library views',exact:true});await expect(more).toBeVisible();
 const appearance=(selector:string)=>page.locator(selector).evaluate(el=>{const s=getComputedStyle(el);return {radius:s.borderRadius,fill:s.backgroundColor};});
 const moreCorner=await corner(page,'.sidebar-more>button');const moreClosed=await appearance('.sidebar-more>button');expect(parseFloat(moreClosed.radius)).toBeGreaterThan(0);
 await more.click();await expect(page.getByRole('menuitem',{name:'Playlists',exact:true})).toBeVisible();expect((await appearance('.sidebar-more>button')).radius).toBe(moreClosed.radius);sameCorner(await corner(page,'.sidebar-more>button'),moreCorner);await page.screenshot({path:'out/persistent-qa/more-rounded-corners.png'});await page.keyboard.press('Escape');await expect(page.getByRole('menu')).toHaveCount(0);
 const actions=page.getByRole('button',{name:'musicOS actions',exact:true});const actionCorner=await corner(page,'.section-actions-trigger');const closed=await appearance('.section-actions-trigger');expect(closed.fill).toBe('rgba(0, 0, 0, 0)');
 await actions.click();await expect(page.getByRole('menuitem',{name:'Mini Player',exact:true})).toBeVisible();expect(await appearance('.section-actions-trigger')).toEqual(closed);sameCorner(await corner(page,'.section-actions-trigger'),actionCorner);
 await page.screenshot({path:'out/persistent-qa/header-rounded-corners.png'});await page.keyboard.press('Escape');await expect(page.getByRole('menu')).toHaveCount(0);expect(await appearance('.section-actions-trigger')).toEqual(closed);sameCorner(await corner(page,'.section-actions-trigger'),actionCorner);
});
