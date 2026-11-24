import { expect, test } from '@playwright/test';
import { settleAnimations } from './support/ui';

const title = 'A Midnight Detour Across the Bubblewrap Lagoon and Beyond: the Very Long Live Recording';

test.beforeEach(async ({ page }) => {
  await page.route('**/src/fixtures.ts*', async route => {
    const response = await route.fetch();
    const body = (await response.text()).replace(
      'let player = { ...emptyPlayback };',
      `tracks[0].title = ${JSON.stringify(title)}; let player = {...emptyPlayback, track:tracks[0], queue:tracks.slice(0, 2), queue_index:0, position:30, is_playing:true};`,
    );
    await route.fulfill({ response, body });
  });
});

test('wide secondary controls reveal without shifting transport and are usable from the keyboard', async ({ page }) => {
  await page.goto('/?fixture=1');
  const bar = page.getByRole('region', { name: 'Now playing', exact: true });
  const pause = bar.getByRole('button', { name: 'Pause', exact: true });
  const queue = bar.getByRole('button', { name: 'Open Up next', exact: true });
  const shuffle = bar.getByRole('button', { name: 'Shuffle', exact: true });
  const repeat = bar.getByRole('button', { name: 'Repeat', exact: true });
  await expect(pause).toBeEnabled();
  await settleAnimations(page);
  await expect(queue).toHaveCSS('opacity', '1');
  await expect(shuffle).toHaveCSS('opacity', '0');
  const bounds = await pause.boundingBox();
  await pause.hover();
  await expect(shuffle).toHaveCSS('opacity', '1');
  expect(await pause.boundingBox()).toEqual(bounds);

  await page.mouse.move(0, 0);
  await expect(shuffle).toHaveCSS('opacity', '0');
  await bar.getByRole('button', { name: 'Next track', exact: true }).focus();
  for (const button of [
    queue,
    bar.getByRole('button', { name: /^Audio quality:/ }),
    shuffle,
    repeat,
  ]) {
    await page.keyboard.press('Tab');
    await expect(button).toBeFocused();
  }
  await expect(repeat).toHaveCSS('outline-style', 'solid');
  await page.keyboard.press('Enter');
  await expect(repeat).toHaveAttribute('aria-pressed', 'true');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(repeat).toHaveCSS('transition-duration', '0s');
});

test('the compact bar fits one control row and keeps the full truncated title accessible', async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 480 });
  await page.goto('/?fixture=1');
  const bar = page.getByRole('region', { name: 'Now playing', exact: true });
  const lyrics = bar.getByRole('button', { name: 'Open lyrics', exact: true });
  await expect(lyrics).toHaveAccessibleDescription(title);
  await expect(lyrics.locator('.overflow-label')).toHaveAttribute('data-overflow', 'true');

  const controls = bar.locator('.transport > button:visible, .mini-controls button');
  const rects = await controls.evaluateAll(buttons => buttons.map(button => {
    const { x, y, width, height } = button.getBoundingClientRect();
    return { x, y, width, height };
  }));
  const bounds = (await bar.boundingBox())!;
  expect(rects).toHaveLength(6);
  for (const rect of rects) {
    expect(rect.width).toBeGreaterThanOrEqual(44);
    expect(rect.height).toBeGreaterThanOrEqual(44);
    expect(rect.y).toBe(rects[0].y);
    expect(rect.x + rect.width).toBeLessThan(bounds.x + bounds.width);
  }
  await expect(bar.getByRole('button', { name: 'Shuffle', exact: true })).toHaveCSS('opacity', '1');
});

test('at 375 px the secondary row leaves room for the title and artist', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 480 });
  await page.goto('/?fixture=1');
  const bar = page.getByRole('region', { name: 'Now playing', exact: true });
  const lyrics = bar.getByRole('button', { name: 'Open lyrics', exact: true });
  const artist = bar.getByRole('button', { name: 'Velvet Static', exact: true });
  await expect(lyrics).toHaveAccessibleDescription(title);
  expect((await lyrics.boundingBox())!.width).toBeGreaterThanOrEqual(120);
  expect((await artist.boundingBox())!.width).toBeGreaterThan(0);

  const transport = (await bar.getByRole('button', { name: 'Pause', exact: true }).boundingBox())!;
  for (const button of await bar.locator('.mini-controls button').all()) {
    const bounds = (await button.boundingBox())!;
    expect(bounds.y).toBeGreaterThanOrEqual(transport.y + transport.height);
    expect(bounds.width).toBeGreaterThanOrEqual(44);
    expect(bounds.height).toBeGreaterThanOrEqual(44);
  }
});

test('compact credits prioritize the artist at 150% interface size', async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 480 });
  await page.addInitScript(() => {
    localStorage.setItem('musicos-fixture-preferences', JSON.stringify({ interface_percent: 150 }));
  });
  await page.goto('/?fixture=1');
  await expect(page.locator('.adaptive-shell')).toHaveAttribute('data-size', '150');
  const bar = page.getByRole('region', { name: 'Now playing', exact: true });
  const artist = bar.getByRole('button', { name: 'Velvet Static', exact: true });
  const album = bar.getByRole('button', { name: 'Bubblewrap Lagoon', exact: true });
  await expect(artist).toBeVisible();
  const artistWidth = (await artist.boundingBox())!.width;
  expect(artistWidth).toBeGreaterThan(45);
  expect(artistWidth).toBeGreaterThan((await album.boundingBox())!.width);
});

test('keyboard focus reveals the truncated title without losing the accessible description', async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 480 });
  await page.goto('/?fixture=1');
  const bar = page.getByRole('region', { name: 'Now playing', exact: true });
  const lyrics = bar.getByRole('button', { name: 'Open lyrics', exact: true });
  const label = lyrics.locator('.overflow-label');
  const content = label.locator('span');
  await expect(label).toHaveAttribute('data-overflow', 'true');
  const position = await content.evaluate(el => getComputedStyle(el).transform);
  await bar.getByRole('button', { name: 'Open player', exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(lyrics).toBeFocused();
  await expect(lyrics).toHaveAccessibleDescription(title);
  await expect(label).toHaveCSS('mask-image', 'none');
  await expect(content).not.toHaveCSS('transform', position);
});

test('keyboard focus on playback position draws a ring on the visible rail', async ({ page }) => {
  await page.goto('/?fixture=1');
  const bar = page.getByRole('region', { name: 'Now playing', exact: true });
  const seek = bar.getByRole('slider', { name: 'Playback position', exact: true });
  const rail = bar.locator('.progress-rail');
  await expect(rail).toHaveCSS('outline-style', 'none');
  await bar.getByRole('button', { name: 'Bubblewrap Lagoon', exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(seek).toBeFocused();
  await expect(rail).toHaveCSS('outline-style', 'solid');
  await expect(rail).toHaveCSS('outline-width', '2px');
  await page.keyboard.press('Tab');
  await expect(rail).toHaveCSS('outline-style', 'none');
});
