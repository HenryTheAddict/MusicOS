import {expect,type Locator,type Page} from '@playwright/test';

export async function chooseSection(page:Page,name:'Library'|'Settings'|'Devices'){
 await settleAnimations(page);
 if(await page.locator('.section-name').textContent()===name)return;
 await page.getByRole('button',{name:'Choose section',exact:true}).click();
 await page.getByRole('menuitem',{name,exact:true}).click();
 await expect(page.locator('.section-name')).toHaveText(name);
 await settleAnimations(page);
}

export async function openAlbums(page:Page){
 const library=page.getByRole('navigation',{name:'Main navigation',exact:true}).getByRole('button',{name:'Library',exact:true});
 if(await library.isVisible())await library.click();
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await expect(page.locator('.collection-grid').first()).toBeVisible();
}

export async function openPlaylistCreator(page:Page){
 await settleAnimations(page);
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 await page.getByRole('menuitem',{name:'New playlist',exact:true}).click();
 await expect(page.getByRole('dialog',{name:'Create playlist',exact:true})).toBeVisible();
}

export async function openSparklistCreator(page:Page){
 await settleAnimations(page);
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 await page.getByRole('menuitem',{name:'New Sparklist',exact:true}).click();
 await expect(page.getByRole('dialog',{name:'Create Sparklist',exact:true})).toBeVisible();
}

export async function addPlaylistSongs(page:Page,trackIds:number[]){
 await page.getByRole('button',{name:'Add Music',exact:true}).click();
 const selector=page.getByRole('dialog',{name:'Add Music',exact:true});
 for(const id of trackIds){
  const song=selector.locator(`button[data-track-id="${id}"]`);
  await song.click();
  await expect(song).toBeDisabled();
 }
 await selector.getByRole('button',{name:'Done',exact:true}).click();
 await expect(selector).toHaveCount(0);
}

export async function setThemeColors(page:Page,source:'artwork'|'blue'|'custom'){
 await page.getByRole('button',{name:'Theme colors',exact:true}).click();
 await page.getByRole('menuitemradio',{name:source==='artwork'?'From artwork':source==='blue'?'Original blue':'Custom',exact:true}).click();
}

export async function openCollections(page:Page){
 await page.getByRole('button',{name:'musicOS actions',exact:true}).click();
 await page.getByRole('menuitem',{name:'Manage collections',exact:true}).click();
 await expect(page.locator('.collection-hub')).toBeVisible();
}

export async function setPlayerView(player:Locator,view:'playing'|'queue'|'lyrics'){
 const cycle=player.locator('[data-player-cycle]');
 for(let step=0;step<3;step++){
  const current=await cycle.getAttribute('data-player-cycle');
  if(current===view){await settleAnimations(player.page());return;}
  await player.hover({position:{x:8,y:8},force:true});
  await cycle.click();
  const next=current==='playing'?'queue':current==='queue'?'lyrics':'playing';
  await expect(cycle).toHaveAttribute('data-player-cycle',next);
 }
 await expect(cycle).toHaveAttribute('data-player-cycle',view);
}

export async function settleAnimations(page:Page){
 await page.evaluate(()=>document.fonts.ready.then(()=>undefined));
 await expect.poll(()=>page.evaluate(()=>document.getAnimations().filter(animation=>animation.playState==='running'&&animation.effect?.getComputedTiming().iterations!==Infinity).length)).toBe(0);
 await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
}
