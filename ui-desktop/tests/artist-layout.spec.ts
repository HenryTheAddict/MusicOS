import {test,expect,type Page} from '@playwright/test';
async function openArtist(page:Page,query=''){
 await page.goto(`/?fixture=1&longBiography=1${query}`);
 await page.getByRole('button',{name:'Artists',exact:true}).click();
 await page.getByRole('button',{name:'Open M83',exact:true}).click();
 await page.evaluate(()=>document.fonts.ready);
 await expect.poll(()=>page.locator('.artist-hero').evaluate(el=>el.getAnimations({subtree:true}).length)).toBe(0);
}
async function settled(page:Page){await expect.poll(()=>page.locator('.artist-hero').evaluate(el=>el.getAnimations({subtree:true}).length)).toBe(0);}
async function geometry(page:Page){return page.locator('.artist-hero').evaluate(el=>{
 const box=(s:string)=>{const r=el.querySelector(s)!.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom,right:r.right};};
 return {title:box('h1'),about:box('.artist-about'),summary:box('.artist-summary'),portrait:box('.artist-portrait-stage'),stats:box('.artist-statistics'),actions:box('.artist-playback'),content:el.closest('.artist-detail')!.getBoundingClientRect().width};
});}
test('long biography keeps the portrait, facts and playback together and collapse available',async({page})=>{
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:100})));
 await openArtist(page);const before=await geometry(page),toggle=page.getByRole('button',{name:'Read more',exact:true});
 expect(before.title.height).toBeLessThan(55);await expect(page.locator('.artist-about h2')).toHaveCSS('background-color','rgba(0, 0, 0, 0)');
 await toggle.focus();await page.keyboard.press('Enter');await settled(page);const after=await geometry(page);
 expect(after.stats.y-after.portrait.bottom).toBeLessThan(20);expect(after.actions.y-after.stats.bottom).toBeLessThan(20);
 expect(Math.abs(after.summary.y-before.summary.y)).toBeLessThan(1);expect(Math.abs((after.actions.y-after.title.y)-(before.actions.y-before.title.y))).toBeLessThan(1);
 expect(after.about.right).toBeLessThanOrEqual(after.summary.x);expect(after.about.width).toBeGreaterThan(after.summary.width*1.8);
 await expect(page.locator('.artist-biography p')).toHaveCount(2);await expect(page.locator('.artist-biography p').last()).toHaveText('This final paragraph must remain readable when expanded.');
 await expect(page.getByRole('button',{name:'Wikipedia ↗'})).toBeVisible();await expect(page.getByRole('button',{name:'Read less'})).toBeFocused();
 expect((await page.getByRole('button',{name:'Read less'}).boundingBox())!.y).toBeLessThan(350);
 await page.screenshot({path:'out/persistent-qa/artist-refined-expanded.png',animations:'disabled'});
 await page.keyboard.press('Enter');await settled(page);await expect(toggle).toBeFocused();await page.screenshot({path:'out/persistent-qa/artist-refined-collapsed.png',animations:'disabled'});
 expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='playback'))).toEqual([]);
});
test('artist layout and text contrast fit both windows at every scale and theme',async({page})=>{
 test.setTimeout(240000);
 await page.addInitScript(()=>{const q=new URLSearchParams(location.search);localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(q.get('scale')),appearance:{mode:q.get('mode'),source:q.get('source'),accent:'#ff00ff',saturation:150,tint:40}}));});
 for(const width of [720,1280])for(const scale of [80,100,125,150])for(const [mode,source] of [['light','blue'],['dark','blue'],['light','artwork'],['dark','artwork'],['dark','custom']]){
  await page.setViewportSize({width,height:width===720?480:800});await openArtist(page,`&scale=${scale}&mode=${mode}&source=${source}`);
  const before=await geometry(page);await page.getByRole('button',{name:'Read more'}).click();await settled(page);const after=await geometry(page);
  expect(after.summary.x).toBeGreaterThanOrEqual(0);expect(after.summary.right).toBeLessThanOrEqual(width+1);expect(after.about.right).toBeLessThanOrEqual(width+1);
  expect(Math.abs(after.portrait.width-after.portrait.height),JSON.stringify({width,scale,mode,source,before,after})).toBeLessThan(1);expect(Math.abs((after.actions.y-after.title.y)-(before.actions.y-before.title.y))).toBeLessThan(1);
  if(after.content<48*16*scale/100)expect(after.about.y).toBeGreaterThanOrEqual(after.summary.bottom);else expect(after.about.right).toBeLessThanOrEqual(after.summary.x);
  expect(await page.locator('.artist-biography').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);
  const ratio=await page.locator('.artist-biography').evaluate(el=>{
   const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d')!;
   const lum=(color:string)=>{ctx.fillStyle=color;ctx.fillRect(0,0,1,1);const c=[...ctx.getImageData(0,0,1,1).data].slice(0,3).map(v=>{const n=v/255;return n<=.04045?n/12.92:((n+.055)/1.055)**2.4;});return c[0]*.2126+c[1]*.7152+c[2]*.0722;};
   const colors=[lum(getComputedStyle(el).color),lum(getComputedStyle(document.querySelector('.page-frame')!).backgroundColor)].sort((a,b)=>b-a);return (colors[0]+.05)/(colors[1]+.05);
  });expect(ratio).toBeGreaterThanOrEqual(4.49);
  await page.getByRole('button',{name:'Read less'}).click();await settled(page);
 }
});
test('interrupted biography changes settle, keep focus and respect Reduced Motion',async({page})=>{
 await openArtist(page);const more=page.getByRole('button',{name:'Read more'});await more.click();await page.getByRole('button',{name:'Read less'}).click();await more.click();await page.setViewportSize({width:720,height:480});await settled(page);
 await expect(page.locator('.artist-biography p').last()).toHaveText('This final paragraph must remain readable when expanded.');
 await page.getByRole('button',{name:'Read less'}).click();await settled(page);await page.emulateMedia({reducedMotion:'reduce'});await more.click();expect(await page.locator('.artist-hero').evaluate(el=>el.getAnimations({subtree:true}).length)).toBe(0);
 await page.getByRole('button',{name:'Read less'}).press('Enter');await expect(more).toBeFocused();
});
