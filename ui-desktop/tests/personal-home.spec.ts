import {test,expect} from '@playwright/test';
import {setPlayerView} from './support/ui';
const collections=[{id:'playlist/1',name:'Road home',track_ids:[1,2,3],kind:'manual',spark:null,artwork_choice:'collage'},{id:'audio/1',name:'Late evening',track_ids:[13,14,15],kind:'audio',spark:null,artwork_choice:'auto/2'},{id:'spark/1',name:'Always favorites',track_ids:[1,13],kind:'rule',spark:{name:'Always favorites',rules:[]},artwork_choice:'auto/1'}];
async function seed(page:import('@playwright/test').Page){await page.addInitScript(items=>{localStorage.setItem('musicos-fixture-collections',JSON.stringify(items));localStorage.setItem('musicos-fixture-home-activity',JSON.stringify({frequent:[{id:13,plays:18},{id:1,plays:5}],recent:[13,1,2]}));},collections);}
test('personal home is the first destination and connects listening, albums and saved collections',async({page})=>{
 await seed(page);await page.goto('/?fixture=1');const home=page.locator('.personal-home');await expect(home.getByRole('heading',{name:"Henry",exact:true,level:1})).toBeVisible();await expect(home.getByRole('heading',{name:'In rotation',exact:true})).toBeVisible();await expect(home.getByRole('region',{name:'Frequently played albums'}).getByRole('button').nth(1)).toHaveAttribute('aria-label','Open Cardboard Carnivals');await expect(home.getByRole('region',{name:'Recently played'}).getByRole('button',{name:/Play/})).toHaveCount(3);await expect(home.getByRole('region',{name:'Recommended Sparklists'})).toContainText('Late evening');await page.screenshot({path:'out/rebuild-qa/personal-home-top.png',animations:'disabled'});
 const original=await page.evaluate(()=>localStorage.getItem('musicos-fixture-collections'));await home.getByRole('button',{name:'Show Sparklists',exact:true}).click();const saved=home.locator('.home-collection-grid');await expect(saved.getByRole('button',{name:/Late evening/})).toHaveCount(2);await expect(saved.getByRole('button',{name:/Road home/})).toHaveCount(0);await saved.getByRole('button',{name:/Late evening Sparklist/}).click();await expect(page.getByRole('heading',{name:'Late evening',exact:true})).toBeVisible();await page.getByRole('button',{name:'Back to Home',exact:true}).click();await expect(home.getByRole('button',{name:'Show Sparklists',exact:true})).toHaveAttribute('aria-pressed','true');expect(await page.evaluate(()=>localStorage.getItem('musicos-fixture-collections'))).toBe(original);
 await page.getByRole('navigation',{name:'Library',exact:true}).getByRole('button',{name:'Albums',exact:true}).click();await expect(page.locator('.collection-grid')).toBeVisible();await page.locator('.sidebar-home').click();await expect(home).toBeVisible();await page.screenshot({path:'out/rebuild-qa/personal-home-wide.png',animations:'disabled'});
});
test('collection play reveals beside the menu without covering the art',async({page})=>{
 await seed(page);
 await page.goto('/?fixture=1');
 const row=page.locator('.home-collection-grid article').filter({has:page.getByRole('button',{name:'Play Road home',exact:true})});
 await row.hover();
 const art=await row.locator('.playlist-artwork-tone').boundingBox();
 const play=row.getByRole('button',{name:'Play Road home'});
 await expect(play).toHaveCSS('opacity','1');
 const button=await play.boundingBox();
 const menu=await row.locator('.menu-trigger').boundingBox();
 expect(button!.x).toBeGreaterThan(art!.x+art!.width);
 expect(button!.x+button!.width).toBeLessThanOrEqual(menu!.x+1);
 expect(button!.y+button!.height/2).toBeCloseTo(menu!.y+menu!.height/2);
 const base=await play.evaluate(button=>getComputedStyle(button).backgroundColor);
 await play.hover();
 await expect.poll(()=>play.evaluate(button=>getComputedStyle(button).backgroundColor)).not.toBe(base);
});
test('listening summary reuses cached visits and refreshes when listening changes',async({page})=>{
 await seed(page);await page.goto('/?fixture=1');await expect(page.locator('.personal-home')).toBeVisible();await expect.poll(()=>page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='home_activity').length)).toBe(1);await page.waitForTimeout(1000);expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='home_activity').length)).toBe(1);
 await page.getByRole('navigation',{name:'Library',exact:true}).getByRole('button',{name:'Albums',exact:true}).click();await page.evaluate(()=>localStorage.setItem('musicos-fixture-home-activity',JSON.stringify({frequent:[{id:1,plays:100}],recent:[1]})));await page.waitForTimeout(1000);expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='home_activity').length)).toBe(1);await page.locator('.sidebar-home').click();expect(await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='home_activity').length)).toBe(1);await page.getByRole('button',{name:'Open Bubblewrap Lagoon',exact:true}).click();await page.getByRole('button',{name:'Play collection',exact:true}).click();await page.locator('.sidebar-home').click();await expect.poll(()=>page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='home_activity').length)).toBe(2);await expect(page.getByRole('region',{name:'Frequently played albums'}).getByRole('button',{name:'Open Bubblewrap Lagoon',exact:true})).toBeVisible();await expect(page.getByRole('region',{name:'Recently played'}).getByRole('button',{name:/Play/})).toHaveCount(1);
});
test('home and collection shelves fit compact/wide windows at every scale and theme',async({page})=>{
 test.setTimeout(120000);await seed(page);await page.addInitScript(()=>{const q=new URLSearchParams(location.search);localStorage.setItem('musicos-fixture-preferences',JSON.stringify({interface_percent:Number(q.get('testScale')),appearance:{mode:q.get('mode'),source:q.get('source'),accent:'#ff00ff',saturation:150,tint:30}}));});
 for(const percent of [80,100,125,150])for(const width of [720,1280])for(const [mode,source]of [['light','blue'],['light','artwork'],['dark','artwork'],['dark','blue'],['dark','custom']]){
  await page.setViewportSize({width,height:width===720?480:800});await page.goto(`/?fixture=1&testScale=${percent}&mode=${mode}&source=${source}`);await expect(page.locator('.personal-home')).toBeVisible();await expect(page.locator('.home-collection-grid article')).toHaveCount(3);expect(await page.locator('.page-scroll').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);expect(await page.locator('.home-collection-grid').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThan(2);await page.locator('.home-collection-grid').scrollIntoViewIfNeeded();const collage=page.locator('.home-collection-grid .playlist-collage').first();if(await collage.count())expect(await collage.evaluate(el=>el.scrollHeight-el.clientHeight)).toBeLessThan(2);
 }
 await page.screenshot({path:'out/rebuild-qa/personal-home-compact.png',animations:'disabled'});
});
test('home resumes the saved song once without replacing its queue or returning after later pauses',async({page})=>{
 await page.goto('/?fixture=1&lyrics=1');
 const player=page.locator('.now-playing'),resume=page.getByRole('button',{name:'Resume current song',exact:true});
 await expect(resume).toBeVisible();await expect(player.getByRole('button',{name:'Play',exact:true})).toBeVisible();
 await setPlayerView(player,'queue');const rows=page.locator('.audio-panel .queue-row strong');await expect(rows).toHaveText(['Kazoo Sunrise']);const before=await rows.allTextContents();await setPlayerView(player,'playing');
 await page.locator('.home-resume').hover();await resume.click();await expect(player.getByRole('button',{name:'Pause',exact:true})).toBeVisible();await expect(resume).toHaveCount(0);
 const calls=await page.evaluate(()=>(window as any).__musicosCalls.filter((r:any)=>r.command==='playback').map((r:any)=>r.args.action));expect(calls.map((action:any)=>action.type)).toEqual(['toggle']);
 await player.getByRole('button',{name:'Pause',exact:true}).click();await expect(player.getByRole('button',{name:'Play',exact:true})).toBeVisible();await expect(resume).toHaveCount(0);
 await setPlayerView(player,'queue');await expect(rows).toHaveText(before);
});

test('Favorites reveals Play in the index and keeps the title still',async({page})=>{
 await page.goto('/?fixture=1');
 await page.locator('.sidebar-favorites').click();
 const row=page.locator('.track-table[data-play-in-index] .track-row').first();
 const title=row.locator('.track-label');
 const x=(await title.boundingBox())!.x;
 await row.hover();
 const play=row.locator('.track-index .row-play');
 await expect(play).toHaveCSS('opacity','1');
 await expect(row.locator('.track-index>span')).toHaveCSS('opacity','0');
 expect((await title.boundingBox())!.x).toBeCloseTo(x);
 await play.click();
 await expect(play).toHaveAccessibleName(/^Pause /);
});

test('Home song drag adds to an empty manual playlist',async({page})=>{
 await page.addInitScript(()=>{
  localStorage.setItem('musicos-fixture-collections',JSON.stringify([{id:'playlist/drop',name:'Road home',kind:'manual',track_ids:[]}]));
  localStorage.setItem('musicos-fixture-home-activity',JSON.stringify({frequent:[],recent:[1]}));
 });
 await page.goto('/?fixture=1');
 const playlist=page.getByRole('button',{name:'Open playlist Road home',exact:true});
 await playlist.click();
 const dropArea=page.getByRole('region',{name:'Add songs to playlist'});
 await expect(dropArea.getByRole('button',{name:'Add Music'})).toBeVisible();
 await expect(page.locator('.playlist-detail .track-table')).toHaveCount(0);
 await expect(page.getByRole('region',{name:'Suggested additions'})).toHaveCount(0);
 await page.locator('.sidebar-home').click();
 const song=page.locator('.home-recent-copy').first();
 await song.scrollIntoViewIfNeeded();
 const source=(await song.boundingBox())!;
 const target=(await playlist.boundingBox())!;
 await page.mouse.move(source.x+12,source.y+10);
 await page.mouse.down();
 await page.mouse.move(target.x+12,target.y+10,{steps:12});
 await expect(page.locator('.song-drag-preview')).toContainText('Road home');
 await page.mouse.up();
 await expect(page.getByRole('status').filter({hasText:'Added 1 song to Road home'})).toBeVisible();
 await playlist.click();
 await expect(page.locator('.playlist-detail .track-row')).toHaveCount(1);
});


test('playlist actions reflow using the available content width',async({page})=>{
 await seed(page);
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Open playlist Road home',exact:true}).click();
 const hero=page.locator('.playlist-detail .playlist-hero');
 const artwork=hero.locator('.playlist-artwork-tone');
 const tools=hero.locator('.playlist-tools');
 await expect(tools.getByRole('button',{name:'Add Music'})).toBeVisible();
 await expect.poll(async()=> (await tools.boundingBox())!.x>(await artwork.boundingBox())!.x+(await artwork.boundingBox())!.width).toBe(true);
 await page.setViewportSize({width:520,height:720});
 await expect.poll(async()=> (await tools.boundingBox())!.y>=(await artwork.boundingBox())!.y+(await artwork.boundingBox())!.height).toBe(true);
 const bounds=(await hero.boundingBox())!;
 for(const button of await tools.getByRole('button').all()){
  const box=(await button.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(bounds.x-1);
  expect(box.x+box.width).toBeLessThanOrEqual(bounds.x+bounds.width+1);
 }
});
