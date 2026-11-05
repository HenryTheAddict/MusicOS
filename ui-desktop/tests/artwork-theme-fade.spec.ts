import {test,expect,type Page} from '@playwright/test';
// Different palettes and a slow provider response exercise the real navigation
// and CSS transitions; normal fixtures intentionally share one palette.
async function coloredArtwork(page:Page,mode='light'){
 await page.addInitScript(mode=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({appearance:{mode,source:'artwork',accent:'#0064df',saturation:100,tint:15}})),mode);
 await page.route('**/src/fixtures.ts*',async route=>{
  const response=await route.fetch(),body=await response.text(),marker='case "artwork_palette":';expect(body).toContain(marker);
  await route.fulfill({response,body:body.replace(marker,`${marker}
    if(String(a.path).includes('NightVisions')){await new Promise(resolve=>setTimeout(resolve,600));return {background:'#b8cbe8',blob:'#6d8fc6',accent:'#163d6b',text:'#12202a'};}
    if(String(a.path).includes('Pier'))return {background:'#ecc4c4',blob:'#bc7777',accent:'#802525',text:'#12202a'};
  `)});
 });
 await page.goto('/?fixture=1');await expect(page.locator('html')).toHaveAttribute('data-theme-phase','ready');await page.getByRole('button',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open Junk',exact:true}).click();await expect(page.locator('html')).toHaveAttribute('data-theme-motion','artwork');
 await expect.poll(()=>page.locator('html').evaluate(el=>el.style.getPropertyValue('--surface-color'))).not.toBe('#00aeff');
 await finishColors(page);await page.getByRole('button',{name:'Play collection',exact:true}).click();await page.locator('.sidebar-back').click();
 await expect(page.getByRole('button',{name:'Open Plastic Beach',exact:true})).toBeVisible();await finishColors(page);
}
async function finishColors(page:Page){await page.locator('html').evaluate(el=>el.getAnimations().forEach(a=>a.finish()));}
async function transitions(page:Page){return page.locator('html').evaluate(el=>el.getAnimations().filter(a=>a instanceof CSSTransition).map(a=>(a as CSSTransition).transitionProperty));}
async function pauseColors(page:Page,time:number){return page.locator('html').evaluate((el,time)=>{el.getAnimations().forEach(a=>{a.pause();a.currentTime=time;});return getComputedStyle(el).getPropertyValue('--surface-color').trim();},time);}
for(const mode of ['light','dark'])test(`${mode}: artwork fades after launch and rapid return uses current displayed colors`,async({page})=>{
 await coloredArtwork(page,mode);const root=page.locator('html');const original=await root.evaluate(el=>getComputedStyle(el).getPropertyValue('--surface-color').trim());
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await expect.poll(()=>transitions(page)).toContain('--surface-color');
 const minimumContrast=await root.evaluate(el=>{
  const animations=el.getAnimations(),context=document.createElement('canvas').getContext('2d')!;
  const luminance=(color:string)=>{context.fillStyle=color;context.fillRect(0,0,1,1);const c=[...context.getImageData(0,0,1,1).data].map(v=>{const n=v/255;return n<=.04045?n/12.92:((n+.055)/1.055)**2.4;});return c[0]*.2126+c[1]*.7152+c[2]*.0722;};
  const ratio=(a:string,b:string)=>{const c=[luminance(a),luminance(b)].sort((a,b)=>b-a);return(c[0]+.05)/(c[1]+.05);};let minimum=Infinity,kind='',when=0;
  for(let time=0;time<=420;time+=20){animations.forEach(a=>{a.pause();a.currentTime=time;});const s=getComputedStyle(el),v=(key:string)=>s.getPropertyValue(key).trim();for(const [name,value] of [['page',ratio(v('--ink'),v('--surface-color'))],['shell',ratio(v('--ink'),v('--blob-color'))],['accent',ratio(v('--accent-ink'),v('--accent'))]] as const)if(value<minimum){minimum=value;kind=name;when=time;}}
  return {minimum,kind,when};
 });expect(minimumContrast.minimum,JSON.stringify(minimumContrast)).toBeGreaterThanOrEqual(4.5);
 const halfway=await pauseColors(page,160);expect(halfway).not.toBe(original);
 const destination=await root.evaluate(el=>el.style.getPropertyValue('--surface-color'));expect(halfway).not.toBe(destination);
 await page.locator('.sidebar-back').click();await expect.poll(()=>transitions(page)).toContain('--surface-color');const reversed=await pauseColors(page,0);
 expect(reversed).toBe(halfway);await finishColors(page);expect(await root.evaluate(el=>getComputedStyle(el).getPropertyValue('--surface-color').trim())).toBe(original);
 await page.screenshot({path:`out/persistent-qa/theme-fade-${mode}.png`,animations:'disabled'});
});
test('late palette never replaces the current album and pending lookup retains displayed colors',async({page})=>{
 await coloredArtwork(page);const root=page.locator('html');const red=await root.evaluate(el=>getComputedStyle(el).getPropertyValue('--surface-color').trim());
 await page.getByRole('button',{name:'Open Fantasy',exact:true}).click();expect(await root.evaluate(el=>getComputedStyle(el).getPropertyValue('--surface-color').trim())).toBe(red);
 await page.locator('.sidebar-back').click();await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await expect.poll(()=>root.evaluate(el=>el.style.getPropertyValue('--surface-color'))).toBe('#c8ded9');
 await page.waitForTimeout(750);await expect(page.locator('.current-theme')).toHaveCSS('background-color','rgb(200, 222, 217)');expect(await root.evaluate(el=>el.style.getPropertyValue('--surface-color'))).toBe('#c8ded9');
});
test('explicit appearance changes, System mode switches, and Reduced Motion remain immediate',async({page})=>{
 await coloredArtwork(page);await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();await expect.poll(()=>transitions(page)).toContain('--surface-color');
 await page.emulateMedia({reducedMotion:'reduce'});await expect(page.locator('html')).toHaveCSS('transition-duration','0s');expect(await transitions(page)).toEqual([]);
 await page.emulateMedia({reducedMotion:'no-preference'});await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();
 await page.getByRole('button',{name:'Appearance mode',exact:true}).click();await page.getByRole('menuitemradio',{name:'Dark',exact:true}).click();await expect(page.locator('html')).not.toHaveAttribute('data-theme-motion','artwork');expect(await transitions(page)).toEqual([]);
 await page.getByRole('button',{name:'Appearance mode',exact:true}).click();await page.getByRole('menuitemradio',{name:'System',exact:true}).click();await page.emulateMedia({colorScheme:'light'});await page.emulateMedia({colorScheme:'dark'});await expect(page.locator('html')).toHaveAttribute('data-theme-mode','dark');expect(await transitions(page)).toEqual([]);
});

for(const mode of ['light','dark'])test(`${mode}: a Back swipe blends the shell and preview together through release`,async({page})=>{
 await coloredArtwork(page,mode);
 const colors=()=>page.evaluate(()=>{
  const root=document.documentElement;
  const canvas=document.createElement('canvas').getContext('2d')!;
  const pixel=(value:string)=>{
   canvas.fillStyle=value;
   canvas.fillRect(0,0,1,1);
   return [...canvas.getImageData(0,0,1,1).data].join(',');
  };
  const style=getComputedStyle(root);
  const preview=document.querySelector('.back-page-preview');
  return {
   shell:pixel(style.getPropertyValue('--shell-color')),
   surface:pixel(style.getPropertyValue('--page-color')),
   preview:preview?pixel(getComputedStyle(preview).backgroundColor):null,
   owned:root.hasAttribute('data-back-swipe-theme'),
  };
 });
 const gesture=(phase:number|null, fraction=0)=>page.evaluate(({phase,fraction})=>{
  const target=document.querySelector('#library-content')!;
  if(phase!==null)window.dispatchEvent(new CustomEvent('musicos-trackpad-phase',{detail:phase}));
  if(fraction)target.dispatchEvent(new WheelEvent('wheel',{deltaX:target.clientWidth*fraction,bubbles:true,cancelable:true}));
 },{phase,fraction});
 const destination=await colors();
 await page.getByRole('button',{name:'Open Plastic Beach',exact:true}).click();
 await expect.poll(()=>transitions(page)).toContain('--shell-color');
 await finishColors(page);
 const source=await colors();
 expect(source.shell).not.toBe(destination.shell);

 await gesture(1,-.25);
 const blend=await colors();
 expect(blend.owned).toBe(true);
 expect(blend.shell).not.toBe(source.shell);
 expect(blend.shell).not.toBe(destination.shell);
 expect(blend.preview).toBe(blend.surface);
 await gesture(null,-.25);
 await expect.poll(async()=>(await colors()).shell).not.toBe(blend.shell);
 await gesture(null,.25);
 await expect.poll(async()=>(await colors()).shell).toBe(blend.shell);
 await gesture(3);
 await expect.poll(colors).toEqual(source);

 await gesture(1,-.58);
 const beforeRelease=await colors();
 expect(beforeRelease.shell).not.toBe(destination.shell);
 expect(beforeRelease.preview).toBe(beforeRelease.surface);
 await gesture(2);
 await expect(page.locator('#library-content .collection-grid')).toBeVisible();
 await expect.poll(colors).toEqual(destination);
 expect(await transitions(page)).toEqual([]);
});
