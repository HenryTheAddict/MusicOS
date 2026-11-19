import {expect,type Locator,type Page} from '@playwright/test';

export async function openAlbums(page:Page){
 const library=page.getByRole('navigation',{name:'Main navigation',exact:true}).getByRole('button',{name:'Library',exact:true});
 if(await library.isVisible())await library.click();
 await page.getByRole('button',{name:'Albums',exact:true}).click();
 await expect(page.locator('.collection-grid').first()).toBeVisible();
}

export async function openPlaylistCreator(page:Page){
 await page.getByRole('button',{name:'Create collection',exact:true}).click();
 await page.getByRole('menuitem',{name:'New playlist',exact:true}).click();
 await expect(page.getByRole('dialog',{name:'Create playlist',exact:true})).toBeVisible();
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
 await page.evaluate(async()=>{
  await document.fonts.ready;
  await Promise.allSettled(document.getAnimations().filter(animation=>animation.playState==='running'&&animation.effect?.getComputedTiming().iterations!==Infinity).map(animation=>animation.finished));
  await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
 });
}
