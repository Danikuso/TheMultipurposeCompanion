import { expect, test } from '@playwright/test';
import { openLayout } from './helpers';

test('missions_grouped_with_alerts', async ({ page }) => {
  await openLayout(page, 'Misiones');
  await expect(page.getByTestId('missions-group')).toHaveCount(2, { timeout: 20_000 });
  await expect(page.getByTestId('mission-row')).toHaveCount(3);
  // La misión de té caduca en menos de una hora.
  await expect(page.getByTestId('missions-alert').first()).toBeVisible();
  await expect(page.getByTestId('missions-alert').first()).toContainText(/hora/);
});
