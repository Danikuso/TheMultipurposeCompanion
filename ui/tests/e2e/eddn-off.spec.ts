import { expect, test } from '@playwright/test';
import { openLayout, watchEddnConnected } from './helpers';

test('eddn_not_connected_when_offline', async ({ page }) => {
  const eddn = watchEddnConnected(page);
  await openLayout(page, 'Comercio');
  await expect(page.getByTestId('market-row').first()).toBeVisible({ timeout: 20_000 });
  // Con los servicios en línea desactivados el suscriptor no llega a conectar nunca.
  await page.waitForTimeout(6000);
  expect(eddn.seen).toBe(false);
  await expect(page.locator('body')).not.toContainText('Puerto Vecino');
});
