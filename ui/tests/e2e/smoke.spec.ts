import { expect, test } from '@playwright/test';

test('default exploration layout shows route and bodies from real replay without console errors', async ({
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

  await expect(page.getByTestId('topbar-system')).toHaveText('Crucis Sector GH-V b2-6', {
    timeout: 15_000,
  });
  const route = page.locator('[data-layout-slot="route"]');
  const bodies = page.locator('[data-layout-slot="context-bodies"]');
  await expect(route).toBeVisible();
  await expect(bodies).toBeVisible();
  await expect(bodies.getByRole('table')).toBeVisible();
  await expect(route).toHaveCSS('grid-column-start', '1');
  await expect(route).toHaveCSS('grid-column-end', '-1');
  // El contexto ocupa dos de las cuatro columnas en el monitor de referencia.
  await page.setViewportSize({ width: 2560, height: 1080 });
  const wide = await bodies.boundingBox();
  const grid = await page.locator('main').boundingBox();
  expect(wide!.width / grid!.width).toBeGreaterThan(0.4);
  expect(wide!.width / grid!.width).toBeLessThan(0.6);
  // En una sola columna no aparecen columnas implícitas por el span 2.
  await page.setViewportSize({ width: 800, height: 900 });
  await expect(bodies).toHaveCSS('grid-column-start', '1');
  await expect(bodies).toHaveCSS('grid-column-end', '-1');
  expect(consoleErrors).toEqual([]);
});
