import { expect, test } from '@playwright/test';
import { openLayout } from './helpers';

test('tourism_verdict_on_accept', async ({ page }) => {
  await openLayout(page, 'Misiones');
  await expect(page.getByTestId('missions-tourism-plan')).toBeVisible({ timeout: 30_000 });
  const plan = page.getByTestId('missions-tourism-plan');
  await expect(plan).toContainText('Distancia total: 31');
  await expect(plan).toContainText('Distancia total: 51');
  await expect(plan).toContainText(/10\D?000 cr\/al/);
  await expect(page.getByTestId('missions-tourism-verdict')).toContainText('NO COMPENSA');
});
