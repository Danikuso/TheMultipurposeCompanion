import { expect, test } from '@playwright/test';

test('combat_layout_shows_wanted_badge_and_subsystem', async ({ page }) => {
  await page.goto('/');
  const target = page.locator('[data-layout-slot="target"]');
  await expect(target.getByTestId('target-badge')).toHaveAttribute('data-kind', 'wanted', {
    timeout: 15_000,
  });
  await expect(target.getByTestId('target-badge')).toContainText('42');
  await expect(target.getByTestId('subsystem')).toBeVisible();
  await expect(page.locator('[data-layout-slot="hunting"]').getByTestId('session-kills')).toContainText('8');
  await expect(page.locator('[data-layout-slot="hunting"]').getByTestId('vouchers-total')).toBeVisible();
});
