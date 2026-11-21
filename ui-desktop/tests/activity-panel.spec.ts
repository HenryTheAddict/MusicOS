import { test, expect } from '@playwright/test';

test('Activity refreshes progress and returns focus when dismissed', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('musicos-fixture-activity-jobs', JSON.stringify([
    { id: 'check', kind: 'rescan', title: 'Check music folders', state: 'running', completed: 2, total: 8, message: 'Checking' },
  ])));
  await page.goto('/?fixture=1');
  const opener = page.getByRole('button', { name: 'musicOS actions', exact: true });
  await opener.click();
  await page.getByRole('menuitem', { name: 'Activity', exact: true }).click();
  const panel = page.getByRole('dialog', { name: 'Activity', exact: true });
  await expect(panel.getByRole('progressbar')).toHaveAttribute('value', '2');
  await page.evaluate(() => {
    const key = 'musicos-fixture-activity-jobs';
    const [job] = JSON.parse(localStorage.getItem(key)!);
    localStorage.setItem(key, JSON.stringify([{ ...job, state: 'complete', completed: 8 }]));
  });
  await expect(panel.getByText('Complete', { exact: true })).toBeVisible();
  await expect(panel.getByRole('progressbar')).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(panel).toHaveCount(0);
  await expect(opener).toBeFocused();
});
