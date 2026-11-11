import {test,expect,type Locator} from '@playwright/test';

async function fits(locator: Locator) {
 await expect.poll(() => locator.evaluate(node => node.scrollWidth - node.clientWidth)).toBeLessThan(2);
}

test('long labels fit narrow Home, collection details and expanded player', async ({page}) => {
 await page.setViewportSize({width:320,height:640});
 await page.goto('/?fixture=long');
 await page.evaluate(() => { document.documentElement.style.fontSize='22px'; });
 await expect(page.locator('.continue-card strong')).toContainText('UnbrokenSongTitle');
 await fits(page.locator('.phone-app'));
 await fits(page.locator('main'));
 await expect(page.locator('.continue-card .icon-button')).toBeInViewport();
 await page.getByRole('button',{name:'Library',exact:true}).click();
 await page.locator('.album-tile').filter({hasText:'AnExtremelyLongAlbumTitle'}).click();
 await expect(page.getByRole('heading',{name:/AnExtremelyLongAlbumTitle/})).toBeVisible();
 await fits(page.locator('main'));
 await page.getByRole('button',{name:'Back',exact:true}).click();
 await page.getByRole('button',{name:'Home',exact:true}).click();
 await page.getByRole('button',{name:/LongPlaylistName/}).click();
 await expect(page.getByText('4 of 5 songs offline')).toBeVisible();
 await fits(page.locator('.phone-app'));
 await fits(page.locator('main'));
 await page.getByRole('button',{name:'Play',exact:true}).click();
 await page.locator('.mini-identify').click();
 const player=page.getByRole('dialog',{name:'Player'});
 await expect(player.locator('h1')).toContainText('UnbrokenSongTitle');
 await fits(player);
 await expect(player.getByRole('button',{name:'Next song'})).toBeInViewport();
 await expect.poll(() => player.locator('.phone-playing-identity').evaluate(node => node.getBoundingClientRect().bottom <= node.closest('.phone-player')!.querySelector('.player-bottom')!.getBoundingClientRect().top + 1)).toBe(true);
 await page.setViewportSize({width:844,height:390});
 await fits(player);
 await expect(player.getByRole('button',{name:'Next song'})).toBeInViewport();
});

test('Computer reports stages and retry without polling the full library',async ({page})=>{
 await page.goto('/?fixture=sync');
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 await page.getByRole('button',{name:/Computer/}).click();
 const computer=page.getByRole('dialog',{name:'Computer'});
 await expect(computer.getByText(/72 songs offline/)).toBeVisible();
 const snapshots=() => page.evaluate(async () => {
  const {request}=await import('/bridge.ts');
  return request({action:'fixture_snapshot_count'});
 });
 const before=await snapshots();
 await computer.getByRole('button',{name:'Sync now'}).click();
 await expect(computer.getByText('Preparing songs on your Mac')).toBeVisible();
 await expect(computer.getByText('Copying music')).toBeVisible();
 await expect(computer.getByText('1 of 2 songs')).toBeVisible();
 await fits(computer);
 await computer.getByRole('button',{name:'Stop sync'}).click();
 await expect(computer.getByText('Sync stopped')).toBeVisible();
 await computer.getByRole('button',{name:'Retry sync'}).click();
 await expect(computer.getByText('2 songs copied.')).toBeVisible();
 expect(await snapshots()).toBe(before);
});
