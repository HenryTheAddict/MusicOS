import {expect, test, type Page} from '@playwright/test';

async function openUpdates(page: Page) {
  await page.goto('/?fixture=1');
  await page.getByRole('button', {name: 'Choose section', exact: true}).click();
  await page.getByRole('menuitem', {name: 'Settings', exact: true}).click();
  await page.getByRole('tab', {name: 'General', exact: true}).click();
  return page.getByRole('article', {name: 'Software updates'});
}

test('desktop updates require a check and an explicit install, with progress and retry', async ({page}, testInfo) => {
  await page.route('**/src/desktopUpdateApi.ts*', route => route.fulfill({
    contentType: 'application/javascript',
    body: `
      let checks = 0, attempts = 0;
      let finish;
      window.finishUpdate = () => finish();
      export const desktopUpdateApi = {
        status: async () => ({version: '0.1.0', available: true}),
        check: async () => {
          await new Promise(resolve => { finish = resolve; });
          if (++checks === 1) throw Error('Could not check GitHub Releases.');
          return checks === 2 ? null : '0.2.0';
        },
        install: async (version, progress) => {
          if (version !== '0.2.0') throw Error('Unexpected version');
          progress({phase: 'downloading', received: 50, total: 100});
          await new Promise(resolve => { finish = resolve; });
          if (++attempts === 1) throw Error('The update could not be verified.');
          progress({phase: 'installing'});
          await new Promise(resolve => { finish = resolve; });
        },
      };
    `,
  }));
  const tile = await openUpdates(page);
  const check = tile.getByRole('button', {name: 'Check for updates'});
  const install = tile.getByRole('button', {name: 'Install & restart'});
  const capture = (name: string) => page.screenshot({path: testInfo.outputPath(name + '.png'), animations: 'disabled'});
  await expect(tile.getByText('Installed version 0.1.0')).toBeVisible();
  await expect(install).toHaveCount(0);
  await capture('musicOS-updates-idle');

  await check.click();
  await expect(tile.getByRole('status')).toHaveText('Checking GitHub Releases…');
  await expect(check).toBeDisabled();
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('alert')).toHaveText('Could not check GitHub Releases.');
  await check.click();
  await expect(tile.getByRole('status')).toHaveText('Checking GitHub Releases…');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toHaveText('You’re up to date.');
  await check.click();
  await expect(tile.getByRole('status')).toHaveText('Checking GitHub Releases…');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toContainText('Version 0.2.0 is available.');
  await expect(install).toBeEnabled();
  await capture('musicOS-updates-available');

  await install.click();
  await expect(install).toBeDisabled();
  await expect(tile.getByRole('progressbar')).toHaveAttribute('value', '50');
  await capture('musicOS-updates-downloading');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('alert')).toContainText('could not be verified');
  await expect(install).toBeEnabled();

  await install.click();
  await expect(tile.getByRole('progressbar')).toHaveAttribute('value', '50');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toHaveText('Verifying and installing the update…');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toHaveText('Restarting musicOS…');
});

test('browser-only preview does not offer desktop updates', async ({page}) => {
  await expect(await openUpdates(page)).toHaveCount(0);
});
