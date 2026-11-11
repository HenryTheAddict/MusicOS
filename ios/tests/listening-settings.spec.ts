import {expect,test} from '@playwright/test';

test('Last.fm history and Discord presence are independent and opt in',async ({page})=>{
 await page.goto('/?fixture=listening');
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 const panel=page.locator('.phone-listening');
 await expect(panel.getByText('Connected to your Mac.')).toBeVisible();
 const lastfm=panel.getByRole('switch',{name:/Last.fm listening history/});
 const discord=panel.getByRole('switch',{name:/Discord presence/});
 await expect(lastfm).not.toBeChecked();
 await expect(discord).not.toBeChecked();
 await lastfm.check();
 await expect(panel.getByText('2 listens waiting to send to your Mac')).toBeVisible();
 await expect(discord).not.toBeChecked();
 await discord.check();
 await lastfm.uncheck();
 await expect(discord).toBeChecked();
 await expect(panel.getByText(/listens waiting/)).toHaveCount(0);
});
