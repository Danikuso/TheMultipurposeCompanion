import { expect, test } from '@playwright/test';

test('loads dashboard and updates TopBar to final system from replay without console errors', async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });
  page.on('pageerror', (err) => {
    consoleErrors.push(err.message);
  });

  await page.goto('/');

  await expect(page.getByTestId('topbar-system')).toHaveText('Sampleton', {
    timeout: 15_000,
  });
  expect(consoleErrors).toEqual([]);
});
