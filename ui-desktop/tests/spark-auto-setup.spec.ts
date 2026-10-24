import {test,expect} from '@playwright/test';
async function playlist(page:import('@playwright/test').Page,enabled=false){
 await page.addInitScript(enabled=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({sparklist_beta:enabled,appearance:{mode:'dark',source:'artwork',accent:'#0064df',saturation:100,tint:15}}));localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/1',name:'Morning',track_ids:[1],kind:'manual',spark:null,artwork_choice:'collage'}]));},enabled);
 await page.goto('/?fixture=1&sparkSetup=1');await page.getByRole('button',{name:'Open playlist Morning',exact:true}).click();
}
test('personal headings are Henry and setup opens Sparklists, not Audio Quality',async({page})=>{
 await playlist(page);const box=page.getByRole('region',{name:'Your playlists',exact:true});await expect(box.getByRole('heading',{name:'Henry',exact:true})).toBeVisible();await expect(page.locator('.sidebar-home')).toHaveText('Henry');
 const button=page.getByRole('button',{name:'Set up audio suggestions',exact:true});await expect(button).toBeVisible();
 const geometry=await button.evaluate(el=>{const b=el.getBoundingClientRect(),i=el.querySelector('.icon')!.getBoundingClientRect();return {center:Math.abs(i.y+i.height/2-b.y-b.height/2),icon:i.height,height:b.height};});expect(geometry.center).toBeLessThan(1);expect(geometry.icon).toBeLessThan(25);expect(geometry.height).toBeGreaterThanOrEqual(32);
 await page.screenshot({path:'out/rebuild-qa/spark-auto-suggestion-button.png'});await button.click();await expect(page.getByRole('heading',{name:'Sparklist beta',exact:true})).toBeVisible();await expect(page.getByRole('heading',{name:'Lossless & lossy audio',exact:true})).not.toBeVisible();
 await page.getByRole('switch',{name:'Experimental audio Sparklist beta'}).check();await expect(page.getByText('Model installed',{exact:true})).toBeVisible({timeout:7000});
 const installs=()=>page.evaluate(()=>(window as any).__musicosCalls.filter((call:any)=>call.command==='sparklist'&&call.args.request.action==='install_staged').length);expect(await installs()).toBe(1);
 await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Library',exact:true}).click();await expect(page.getByRole('button',{name:'Prepare audio suggestions',exact:true})).toBeVisible();expect(await installs()).toBe(1);
 await page.locator('.sidebar-home').click();await expect(page.getByRole('heading',{name:'Henry',exact:true}).first()).toBeVisible();
});
test('an already enabled playlist fetches its bundled model automatically without a folder picker',async({page})=>{
 await playlist(page,true);await expect(page.getByRole('button',{name:'Prepare audio suggestions',exact:true})).toBeVisible({timeout:7000});
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((call:any)=>call.command==='sparklist'&&call.args.request.action==='install_staged').length)).toBe(1);
 const collections=await page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-collections')!));expect(collections[0].track_ids).toEqual([1]);
});
