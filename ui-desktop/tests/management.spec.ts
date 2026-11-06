import {test,expect} from '@playwright/test';
async function metadata(page:import('@playwright/test').Page){await page.getByRole('button',{name:'Choose section',exact:true}).click();await page.getByRole('menuitem',{name:'Settings',exact:true}).click();await page.getByRole('tab',{name:'Library',exact:true}).click();await page.getByRole('button',{name:'Metadata & artwork',exact:true}).click();}
test('one manager separates saved collection types, searches and preserves Smart rules',async({page})=>{
 await page.goto('/?fixture=1');await page.evaluate(()=>localStorage.setItem('musicos-fixture-collections',JSON.stringify([
 {id:'playlist/1',name:'Manual favorites',kind:'manual',spark:null,track_ids:[1,2]},
 {id:'audio/1',name:'Dream mix',kind:'audio',spark:null,track_ids:[3,4],artwork_choice:'auto/1'},
 {id:'spark/1',name:'My smart rules',kind:'rule',spark:{name:'My smart rules',match_mode:'any',limit:15,rules:[{field:'artist',op:'contains',value:'M83'},{field:'year',op:'greater_than',value:'2000'}]},track_ids:[1]}])));await page.reload();
 await page.getByRole('button',{name:'More library views',exact:true}).click();await page.getByRole('menuitem',{name:'Playlists',exact:true}).click();await expect(page.getByRole('heading',{name:'Collections',exact:true})).toBeVisible();await expect(page.locator('.hub-collection-open')).toHaveCount(1);await page.getByRole('tab',{name:/Sparklists/}).click();await expect(page.locator('.hub-collection-open')).toContainText('Dream mix');await expect(page.getByText('Audio mixes are not available yet.',{exact:false})).toHaveCount(0);
 await page.getByRole('tab',{name:/Smart playlists/}).click();await page.getByRole('button',{name:'Edit rules',exact:true}).click();await expect(page.getByRole('button',{name:'Rule matching'})).toHaveText('Any rule');await expect(page.getByRole('spinbutton',{name:'Song limit'})).toHaveValue('15');await page.getByRole('textbox',{name:'Name',exact:true}).fill('Renamed rules');await page.getByRole('button',{name:'Save Smart playlist'}).click();
 await expect.poll(()=>page.evaluate(()=>JSON.parse(localStorage.getItem('musicos-fixture-collections')!)[2].name)).toBe('Renamed rules');await page.reload();await page.getByRole('button',{name:'More library views',exact:true}).click();await page.getByRole('menuitem',{name:'Playlists',exact:true}).click();await page.getByRole('tab',{name:/Smart playlists/}).click();await expect(page.locator('.hub-collection-open')).toContainText('Renamed rules');await page.screenshot({path:'out/rebuild-qa/collection-manager.png',animations:'disabled'});await page.getByRole('searchbox',{name:'Find collections'}).fill('missing');await expect(page.getByRole('heading',{name:'No matching collections'})).toBeVisible();
});
async function openSuggested(page:import('@playwright/test').Page){
 await page.getByRole('button',{name:'Has suggestions',exact:true}).click();
 await page.locator('.tag-albums>button').first().click();
}

test('album suggestions are reviewed field by field and Undo is available',async({page})=>{
 await page.goto('/?fixture=1');
 await metadata(page);
 await openSuggested(page);
 await page.getByRole('tab',{name:'Details',exact:true}).click();
 const field=page.getByRole('checkbox',{name:/^Apply album /}).first();
 await expect(field).not.toBeChecked();
 await field.check();
 await page.getByRole('button',{name:'Review selected suggestions',exact:true}).click();
 await expect(page.getByRole('region',{name:'Review changes'})).toBeVisible();
 await page.getByRole('button',{name:'Save in musicOS',exact:true}).click();
 await expect(page.getByRole('status').filter({hasText:'Saved details'})).toBeVisible();
 const apply=await page.evaluate(()=>(window as any).__musicosCalls.filter((c:any)=>c.command==='album_metadata'&&c.args?.request?.action==='apply').at(-1).args.request);
 expect(apply.write_files).toBe(false);
 expect(apply.rows.every((row:any)=>row.fields.length===1&&row.fields[0]==='album')).toBe(true);
 await page.getByRole('button',{name:'View history',exact:true}).click();
 await page.getByRole('button',{name:'Undo',exact:true}).first().click();
 await expect(page.getByRole('button',{name:'Undone',exact:true}).first()).toBeDisabled();
});

test('cover-only action leaves checked album details untouched',async({page})=>{
 await page.goto('/?fixture=1');
 await metadata(page);
 await openSuggested(page);
 await page.getByRole('tab',{name:'Details',exact:true}).click();
 await page.getByRole('checkbox',{name:/^Apply album /}).first().check();
 await page.getByRole('tab',{name:'Cover',exact:true}).click();
 await page.getByRole('button',{name:'Update cover only',exact:true}).click();
 await expect(page.getByRole('heading',{name:'Review cover update'})).toBeVisible();
 await page.getByRole('button',{name:'Save in musicOS',exact:true}).click();
 await expect(page.getByRole('status').filter({hasText:'Cover updated'})).toBeVisible();
 const result=await page.evaluate(async()=>{const {fixtureCall}=await import('/src/fixtures.ts' as string);return (await fixtureCall('metadata_workspace',{request:{action:'history'}})).history[0];});
 expect(result.changes).toHaveLength(12);
 for(const change of result.changes)for(const key of ['title','artist','album','year','track_number','disc_number'])expect(change.after[key]).toBe(change.before[key]);
 const request=await page.evaluate(()=>(window as any).__musicosCalls.findLast((c:any)=>c.command==='album_metadata'&&c.args?.request?.action==='apply').args.request);
 expect(request.rows).toHaveLength(1);
 expect(request.rows[0].fields).toEqual(['artwork']);
 expect(request.write_files).toBe(false);
 await expect(page.getByRole('tab',{name:'Cover',exact:true})).toHaveAttribute('aria-selected','true');
});

test('narrow album browser restores search, scroll, and focus; replacing a draft asks first',async({page})=>{
 await page.setViewportSize({width:480,height:480});
 await page.goto('/?fixture=1');
 await metadata(page);
 const query=page.getByRole('searchbox',{name:'Find albums',exact:true});
 await query.fill('a');
 const list=page.locator('.tag-albums');
 const row=list.locator('button').nth(4);
 const release=await row.getAttribute('data-release-id');
 await row.scrollIntoViewIfNeeded();
 const y=await list.evaluate(node=>node.scrollTop);
 await row.click();
 await page.getByRole('tab',{name:'Details',exact:true}).click();
 await page.getByRole('textbox',{name:'Album',exact:true}).fill('Draft release');
 await page.getByRole('tab',{name:'Cover',exact:true}).click();
 await page.getByRole('tab',{name:'Details',exact:true}).click();
 await expect(page.getByRole('textbox',{name:'Album',exact:true})).toHaveValue('Draft release');
 await page.getByRole('button',{name:'Back to albums',exact:true}).click();
 await expect(query).toHaveValue('a');
 await expect(list.locator(`[data-release-id="${release}"]`)).toBeFocused();
 expect(await list.evaluate(node=>node.scrollTop)).toBe(y);
 await list.locator('button').first().click();
 const dialog=page.getByRole('dialog',{name:'Discard metadata draft?'});
 await expect(dialog).toBeVisible();
 await dialog.getByRole('button',{name:'Keep editing'}).click();
 await row.click();
 await page.getByRole('tab',{name:'Details',exact:true}).click();
 await expect(page.getByRole('textbox',{name:'Album',exact:true})).toHaveValue('Draft release');
 await page.getByRole('button',{name:'Back to albums',exact:true}).click();
 await list.locator('button').first().click();
 await dialog.getByRole('button',{name:'Discard draft'}).click();
 await expect(page.getByRole('tab',{name:'Cover',exact:true})).toHaveAttribute('aria-selected','true');
});

test('cover review fits supported window sizes, scales, and modes',async({page})=>{
 test.setTimeout(240000);
 for(const [width,height] of [[480,480],[720,480],[1280,800]])for(const scale of [80,100,125,150])for(const mode of ['light','dark']){
  await page.setViewportSize({width,height});
  await page.goto('/?fixture=1');
  await page.evaluate(({scale,mode})=>{localStorage.setItem('musicos-fixture-preferences',JSON.stringify({version:1,interface_percent:scale,appearance:{mode,source:'artwork',accent:'#0064df',saturation:100,tint:15}}));sessionStorage.clear();},{scale,mode});
  await page.reload();
  await metadata(page);
  await openSuggested(page);
  await page.getByRole('button',{name:'Update cover only',exact:true}).click();
  const save=page.getByRole('button',{name:'Save in musicOS',exact:true});
  await expect(save).toBeVisible();
  await save.scrollIntoViewIfNeeded();
  await expect(save).toBeInViewport();
  expect(await page.locator('.release-workspace').evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThanOrEqual(1);
  if(scale===100)await page.screenshot({path:`out/rebuild-qa/metadata-review-${width}-${mode}.png`,animations:'disabled'});
 }
});

test('album and song menus open the matching editor',async({page})=>{
 await page.goto('/?fixture=1');
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await page.getByRole('button',{name:'Open Discovery',exact:true}).click();
 await page.locator('.album-detail').getByRole('button',{name:'More actions',exact:true}).first().click();
 await page.getByRole('menuitem',{name:'Edit metadata',exact:true}).click();
 await expect(page.getByRole('tab',{name:'Details',exact:true})).toHaveAttribute('aria-selected','true');
 await expect(page.getByRole('textbox',{name:'Album',exact:true})).toHaveValue('Discovery');
 await page.getByRole('tab',{name:'Songs',exact:true}).click();
 await page.locator('.tag-songs button').nth(1).click();
 await expect(page.getByRole('textbox',{name:'Title',exact:true})).toHaveValue('Second Light');
 await page.getByRole('button',{name:'Lyrics',exact:true}).click();
 await expect(page.getByRole('region',{name:'Edit lyrics',exact:true})).toBeVisible();
 await expect.poll(()=>page.evaluate(()=>(window as any).__musicosCalls.filter((call:any)=>call.command==='lyrics'&&call.args.request.action==='load').at(-1)?.args.request.trackId)).toBeTruthy();
});
