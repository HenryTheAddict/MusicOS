import { expect, test } from '@playwright/test';

for (const [width, height] of [[480, 480], [1280, 800]]) {
  test(`file organization keeps actions visible and moves only explicitly at ${width}px`, async ({ page }) => {
    const folder = `/fixture/downloads/${'longfoldername'.repeat(10)}`;
    const moves = Array.from({ length: 24 }, (_, id) => ({
      id, from: `${folder}/${id}.flac`, to: `/fixture/Artist/Album/${id}.flac`,
    }));
    await page.route('**/src/fixtures.ts*', async route => {
      const response = await route.fetch();
      const body = await response.text();
      const command = /case ['"]organize_files['"]:\s*\{[^]*?(?=case ['"]library_source['"]:)/;
      expect(body).toMatch(command);
      await route.fulfill({ response, body: body.replace(command, () => `
        case 'organize_files': {
          if (a.action === 'preview') {
            const applied = sessionStorage.getItem('organizer-applied');
            return {token: 'preview', moves: applied ? [] : ${JSON.stringify(moves)}, skipped: 2, undo_available: applied ? 24 : 2};
          }
          sessionStorage.setItem('organizer-applied', 'true');
          return {count: 24};
        }
      `) });
    });
    await page.setViewportSize({ width, height });
    await page.goto('/?fixture=1');
    await page.getByRole('button', { name: 'Choose section', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Settings', exact: true }).click();
    await page.getByRole('tab', { name: 'Library', exact: true }).click();
    const opener = page.getByRole('button', { name: 'Smartly organize my files', exact: true });
    await opener.click();

    const dialog = page.getByRole('dialog', { name: 'Smartly organize my files' });
    const move = dialog.getByRole('button', { name: 'Move 24 files' });
    const review = dialog.getByRole('region', { name: 'File move preview' });
    await expect(move).toBeEnabled();
    await expect(move).toBeInViewport();
    await expect(dialog.getByText('24 files organized.', { exact: true })).toHaveCount(0);
    const box = (await dialog.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.y).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    expect(box.y + box.height).toBeLessThanOrEqual(height);

    const topPath = review.getByText(folder, { exact: true }).first();
    const lastFile = review.getByText('23.flac', { exact: true });
    await expect(topPath).toBeInViewport({ ratio: 1 });
    await expect(lastFile).not.toBeInViewport();
    await review.focus();
    await page.keyboard.press('End');
    await expect(lastFile).toBeInViewport();
    await expect(move).toBeInViewport();
    expect(await review.evaluate(node => node.scrollWidth)).toBe(await review.evaluate(node => node.clientWidth));

    await page.keyboard.press('Escape');
    await expect(dialog).not.toBeVisible();
    await expect(opener).toBeFocused();
    await opener.click();
    await expect(move).toBeInViewport();
    await move.click();
    await expect(dialog.getByText('24 files organized.', { exact: true })).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Undo organization (24)' })).toBeVisible();
  });
}
