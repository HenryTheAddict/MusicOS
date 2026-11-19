import {test,expect} from '@playwright/test';
test('compact creation keeps square artwork and readable controls across scales and palettes',async({page})=>{
 test.setTimeout(180000);
 await page.addInitScript(()=>localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(new URLSearchParams(location.search).get('scale')),coverflow_colors:false})));
 const palettes:Record<string,string[]>={blue:['#cbf1ff','#111820','#54b9ff','#0064df'],light:['#fffaf5','#161616','#e9dfd5','#765c45'],dark:['#202329','#f3f6ff','#303c52','#97baff'],vivid:['#ffdfeb','#251124','#ff96ca','#b00070'],adaptive:['#fae5be','#12202a','#eac887','#624a12']};
 for(const size of [{width:720,height:480},{width:1280,height:800}])for(const scale of [80,100,125,150])for(const [name,colors]of Object.entries(palettes)){
  await page.setViewportSize(size);await page.goto(`/?fixture=1&scale=${scale}`);await page.getByRole('button',{name:'Create collection',exact:true}).click();await page.getByRole('menuitem',{name:'New playlist',exact:true}).click();
  await page.evaluate(colors=>{['--surface-color','--ink','--blob-color','--accent'].forEach((key,i)=>{document.documentElement.style.setProperty(key,colors[i]);(document.querySelector('.app-shell') as HTMLElement).style.setProperty(key,colors[i]);});document.documentElement.style.setProperty('--accent-ink',colors[0]==='#202329'?'#12202a':'#fff');(document.querySelector('.app-shell') as HTMLElement).style.setProperty('--accent-ink',colors[0]==='#202329'?'#12202a':'#fff');},colors);
  const dialog=page.getByRole('dialog',{name:'Create playlist',exact:true}),box=(await dialog.boundingBox())!,cover=(await dialog.locator('.cover-choice img').boundingBox())!;expect(box.x).toBeGreaterThanOrEqual(0);expect(box.y).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(size.width);expect(box.y+box.height).toBeLessThanOrEqual(size.height);expect(Math.abs(cover.width-cover.height)).toBeLessThan(1);await expect(dialog.getByRole('button',{name:'Change artwork',exact:true})).toBeVisible();
  if(scale===100)await page.screenshot({path:`out/rebuild-qa/create-${size.width}-${name}.png`,animations:'disabled'});
  await page.keyboard.press('Escape');
 }
});
