import { expect, test } from '@playwright/test';
import { openLayout } from './helpers';

test('massacre_counts_match_manual', async ({ page }) => {
  await openLayout(page, 'Misiones');
  await expect(page.getByTestId('massacre-pile').first()).toBeVisible({ timeout: 20_000 });
  const piles = page.getByTestId('massacre-pile');
  await expect(piles).toHaveCount(2);
  // Pila X: 12/15, 8/8, 10/10 (la redirigida concilia); faltan 3. Pila Y: 3/4.
  await expect(piles.nth(0)).toContainText('12 / 15');
  await expect(piles.nth(0)).toContainText('8 / 8');
  await expect(piles.nth(0)).toContainText('10 / 10');
  await expect(piles.nth(0).getByTestId('massacre-remaining')).toContainText('3');
  await expect(piles.nth(1)).toContainText('3 / 4');
});
