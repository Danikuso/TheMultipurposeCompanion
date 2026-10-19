import { expect, test } from '@playwright/test';

test('mining_sell_no_data_when_disabled', async ({ page }) => {
  await page.goto('/');
  const p = page.locator('[data-layout-slot="prospector"]');
  await expect(p.getByTestId('core')).toBeVisible({ timeout: 15_000 });
  const sell = page.locator('[data-layout-slot="sell"]');
  await expect(sell.getByTestId('sell-empty')).toBeVisible();
  await expect(sell.getByTestId('sell-option')).toHaveCount(0);
});
