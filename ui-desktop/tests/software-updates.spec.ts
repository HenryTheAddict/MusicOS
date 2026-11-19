import {expect, test, type Page} from '@playwright/test';

async function openUpdates(page: Page, startPlayback = false) {
  await page.goto('/?fixture=1');
  if (startPlayback) {
    await page.getByRole('button', {name: 'Open Discovery', exact: true}).click();
    await page.getByRole('button', {name: 'Play collection', exact: true}).click();
  }
  await page.getByRole('button', {name: 'Choose section', exact: true}).click();
  await page.getByRole('menuitem', {name: 'Settings', exact: true}).click();
  await page.getByRole('tab', {name: 'General', exact: true}).click();
  return page.getByRole('article', {name: 'Software updates'});
}

test('desktop updates download first and wait for an explicit restart', async ({page}, testInfo) => {
  await page.route('**/src/desktopUpdateApi.ts*', route => route.fulfill({
    contentType: 'application/javascript',
    body: `
      let checks = 0, attempts = 0, ready = null;
      let finish;
      window.finishUpdate = () => finish();
      window.restartCalls = 0;
      export const desktopUpdateApi = {
        status: async () => ({version: '0.1.0', available: true, ready}),
        check: async () => {
          await new Promise(resolve => { finish = resolve; });
          if (++checks === 1) throw Error("Couldn't check for updates. Check your connection and try again.");
          return checks === 2 ? null : '0.2.0';
        },
        download: async (version, progress) => {
          if (version !== '0.2.0') throw Error('Unexpected version');
          progress({phase: 'downloading', received: 50, total: 100});
          await new Promise(resolve => { finish = resolve; });
          if (++attempts === 1) throw Error('The update could not be verified.');
          progress({phase: 'verifying'});
          await new Promise(resolve => { finish = resolve; });
          ready = version;
        },
        restart: async (version, pausePlayback) => {
          if (version !== ready || pausePlayback) throw Error('Unexpected restart request');
          window.restartCalls++;
          await new Promise(resolve => { finish = resolve; });
        },
      };
    `,
  }));
  const tile = await openUpdates(page);
  const check = tile.getByRole('button', {name: 'Check for updates'});
  const download = tile.getByRole('button', {name: 'Download update'});
  const restart = tile.getByRole('button', {name: 'Restart to update'});
  const capture = (name: string) => page.screenshot({path: testInfo.outputPath(name + '.png'), animations: 'disabled'});
  await expect(tile.getByText('Installed version 0.1.0')).toBeVisible();
  await expect(restart).toHaveCount(0);
  await capture('musicOS-updates-idle');

  await check.click();
  await expect(tile.getByRole('status')).toHaveText('Checking for updates…');
  await expect(check).toBeDisabled();
  await capture('musicOS-updates-checking');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('alert')).toHaveText("Couldn't check for updates. Check your connection and try again.");
  await capture('musicOS-updates-error');
  await check.click();
  await expect(tile.getByRole('status')).toHaveText('Checking for updates…');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toHaveText('You’re up to date.');
  await check.click();
  await expect(tile.getByRole('status')).toHaveText('Checking for updates…');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toContainText('Version 0.2.0 is available.');
  await expect(download).toBeEnabled();
  await expect(tile.getByRole('link', {name: 'What’s new'}))
    .toHaveAttribute('href', 'https://github.com/HenryTheAddict/MusicOS/releases/tag/v0.2.0');
  await capture('musicOS-updates-available');

  await download.click();
  await expect(download).toHaveCount(0);
  await expect(tile.getByRole('progressbar')).toHaveAttribute('value', '50');
  await expect(tile.getByText('50%', {exact: true})).toBeVisible();
  await capture('musicOS-updates-downloading');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('alert')).toContainText('could not be verified');
  await expect(download).toBeEnabled();

  await download.click();
  await expect(tile.getByRole('progressbar')).toHaveAttribute('value', '50');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toHaveText('Verifying the update…');
  await page.evaluate('window.finishUpdate()');
  await expect(tile.getByRole('status')).toContainText('Version 0.2.0 is ready.');
  await expect(restart).toBeEnabled();
  await expect.poll(() => page.evaluate('window.restartCalls')).toBe(0);
  await capture('musicOS-updates-ready');
  await restart.click();
  await expect(tile.getByRole('status')).toHaveText('Installing and restarting musicOS…');
  await expect.poll(() => page.evaluate('window.restartCalls')).toBe(1);
});

test('during playback a ready update asks to pause and restart', async ({page}) => {
  await page.route('**/src/desktopUpdateApi.ts*', route => route.fulfill({
    contentType: 'application/javascript',
    body: `export const desktopUpdateApi = {
      status: async () => ({version: '0.1.0', available: true, ready: '0.2.0'}),
      restart: async (version, pausePlayback) => {
        window.restartRequest = {version, pausePlayback};
      },
    };`,
  }));
  const tile = await openUpdates(page, true);
  const restart = tile.getByRole('button', {name: 'Pause and restart'});
  const player = page.getByRole('region', {name: 'Now playing'});
  await expect(restart).toBeEnabled();
  await expect(tile.getByRole('status')).toContainText('update next time you open musicOS');
  await player.getByRole('button', {name: 'Pause', exact: true}).click();
  await expect(tile.getByRole('button', {name: 'Restart to update'})).toBeEnabled();
  await player.getByRole('button', {name: 'Play', exact: true}).click();
  await expect(restart).toBeEnabled();
  await restart.click();
  await expect.poll(() => page.evaluate('window.restartRequest'))
    .toEqual({version: '0.2.0', pausePlayback: true});
});

test('browser-only preview does not offer desktop updates', async ({page}) => {
  await expect(await openUpdates(page)).toHaveCount(0);
});
