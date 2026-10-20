import { expect, test } from '@playwright/test';
import { openLayout, watchEddnConnected } from './helpers';

test('eddn_markets_appear_and_corrupt_ignored', async ({ page }) => {
  test.setTimeout(75_000);
  const eddn = watchEddnConnected(page);
  await openLayout(page, 'Comercio');
  await expect.poll(() => eddn.seen, { timeout: 30_000 }).toBe(true);
  const gold = page.getByTestId('market-row').filter({ hasText: 'Oro' });
  // El comprador de EDDN (12 500) sustituye al de Ardent (11 000); los mensajes corruptos no rompen nada.
  await expect(gold).toContainText('Puerto Vecino', { timeout: 45_000 });
  await expect(gold.getByTestId('market-profit')).toContainText(/3\D?100/);
  await expect(gold.getByTestId('market-origin')).toContainText(/EDDN/i);
  await expect(page.getByTestId('market-row')).toHaveCount(5);
  expect(eddn.seen).toBe(true);
});
