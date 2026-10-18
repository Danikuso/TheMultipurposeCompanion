import { expect, test } from '@playwright/test';

test('mining_sell_lists_fake_markets_with_pad_and_age', async ({ page }) => {
  await page.goto('/');
  const sell = page.locator('[data-layout-slot="sell"]');
  const options = sell.getByTestId('sell-option');
  await expect(options).toHaveCount(2, { timeout: 20_000 });
  // Mayor precio primero; la plataforma pequeña queda fuera para la nave (python, M).
  await expect(options.nth(0)).toContainText('Ficticia Hub');
  await expect(options.nth(1)).toContainText('Prueba Outpost');
  const cells0 = options.nth(0).locator('td');
  await expect(cells0.nth(4)).toContainText('L');
  await expect(cells0.nth(5)).not.toHaveText('—');
  await expect(cells0.nth(5)).toHaveText(/\d/);
  await expect(options.nth(1).locator('td').nth(4)).toContainText('M');
  await expect(sell.getByTestId('sell-source')).toContainText('Ardent');
});

test('prospector_shows_composition_core_and_verdict', async ({ page }) => {
  await page.goto('/');
  const p = page.locator('[data-layout-slot="prospector"]');
  await expect(p.getByTestId('core')).toContainText('Monazite', { timeout: 15_000 });
  await expect(p.getByTestId('material-row').first()).toContainText('Bauxite');
  await expect(p.getByTestId('stack-segment').first()).toBeVisible();
  await expect(p.getByTestId('verdict')).toHaveAttribute('data-kind', 'worth');
});
