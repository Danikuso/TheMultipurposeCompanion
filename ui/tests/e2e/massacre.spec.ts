import { expect, test } from '@playwright/test';
import { openLayout } from './helpers';

test('massacre_counts_match_manual', async ({ page }) => {
  await openLayout(page, 'Misiones');
  await expect(page.getByTestId('massacre-pile').first()).toBeVisible({ timeout: 20_000 });
  const piles = page.getByTestId('massacre-pile');
  await expect(piles).toHaveCount(2);
  // Las pilas se localizan por su contenido, no por su posición.
  const pileX = piles.filter({ hasText: '12 / 15' });
  const pileY = piles.filter({ hasText: '3 / 4' });
  await expect(pileX).toHaveCount(1);
  await expect(pileY).toHaveCount(1);
  // Pila X: 12/15, 8/8, 10/10 (la redirigida concilia); faltan 3. Pila Y: 3/4.
  await expect(pileX).toContainText('12 / 15');
  await expect(pileX).toContainText('8 / 8');
  await expect(pileX).toContainText('10 / 10');
  await expect(pileX.getByTestId('massacre-remaining')).toContainText('3');
  await expect(pileY).toContainText('3 / 4');
});
