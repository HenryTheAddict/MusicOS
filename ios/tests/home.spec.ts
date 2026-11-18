import {test,expect} from '@playwright/test';

test('Continue listening is launch-only, including playback started away from Home',async ({page})=>{
 await page.goto('/?fixture=default');
 const resume=page.getByRole('region',{name:'Continue listening'});
 await expect(resume).toBeVisible();
 await expect(resume.getByRole('progressbar')).toHaveAttribute('value','24');
 await page.getByRole('button',{name:'Library',exact:true}).click();
 await page.locator('.phone-mini').getByRole('button',{name:'Resume',exact:true}).click();
 await page.locator('.phone-mini').getByRole('button',{name:'Pause',exact:true}).click();
 await page.getByRole('button',{name:'Home',exact:true}).click();
 await expect(resume).toHaveCount(0);
 await page.reload();
 await expect(resume).toBeVisible();
 await resume.getByRole('button',{name:'Resume',exact:true}).click();
 await expect(resume).toHaveCount(0);
 await page.locator('.phone-mini').getByRole('button',{name:'Pause',exact:true}).click();
 await expect(resume).toHaveCount(0);
});

test('Home preserves collection filtering, touch details, creation, and album browsing',async ({page})=>{
 await page.goto('/?fixture=default');
 const home=page.getByRole('article',{name:'Home',exact:true});
 await expect(home.getByRole('heading',{name:'Your library',exact:true})).toBeVisible();
 await expect(home.getByText('72 songs · 12 albums')).toBeVisible();
 await home.getByRole('button',{name:'Sparklists',exact:true}).click();
 await expect(home.getByText('Copy saved collections from your computer to listen offline.')).toBeVisible();
 await home.getByRole('button',{name:'Playlists',exact:true}).click();
 await home.getByRole('button',{name:/Weekend/}).click();
 await expect(page.getByRole('heading',{name:'Weekend',exact:true})).toBeVisible();
 await expect(page.getByRole('button',{name:'Play',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Back',exact:true}).click();
 await expect(home.getByRole('button',{name:'Playlists',exact:true})).toHaveAttribute('aria-pressed','true');
 await home.getByRole('button',{name:'New playlist',exact:true}).click();
 await expect(page.getByRole('dialog',{name:'Playlist editor',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Close playlist editor',exact:true}).click();
 await home.getByRole('button',{name:/Favorites/}).click();
 await expect(page.getByRole('heading',{name:'Favorites',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Back',exact:true}).click();
 await home.getByRole('button',{name:'All albums',exact:true}).click();
 await expect(page.getByRole('button',{name:'Library',exact:true})).toHaveAttribute('aria-current','page');
 await expect(page.locator('.album-tile')).toHaveCount(12);
});

test('All albums opens the Library root without changing ordinary tab restoration',async ({page})=>{
 await page.goto('/?fixture=default');
 await page.getByRole('button',{name:'Library',exact:true}).click();
 await page.locator('.album-tile').nth(1).click();
 await expect(page.getByRole('heading',{name:'Album 2',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Home',exact:true}).click();
 await page.getByRole('button',{name:'Library',exact:true}).click();
 await expect(page.getByRole('heading',{name:'Album 2',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Home',exact:true}).click();
 await page.getByRole('button',{name:'All albums',exact:true}).click();
 await expect(page.locator('.album-tile')).toHaveCount(12);
 await expect(page.getByRole('heading',{name:'Library',exact:true})).toBeVisible();
 await expect.poll(()=>page.locator('main').evaluate(node=>node.scrollTop)).toBe(0);
});
