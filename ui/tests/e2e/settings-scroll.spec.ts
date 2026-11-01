import { expect, test } from '@playwright/test';

test('settings can reach INARA and CAPI with the wheel and return to the top', async ({ page }, testInfo) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/api/**', (route) => route.fulfill({ json: {} }));
  await page.routeWebSocket('**/ws', (socket) => {
    socket.send(JSON.stringify({ type: 'hello', locale: 'es' }));
    socket.send(JSON.stringify({ type: 'snapshot', seq: 1, topics: { 'core.modules': [] } }));
  });

  await page.goto('/');
  await page.locator('aside .sidebar-bottom button').click();
  const body = page.locator('.hud-panel-body');
  await expect(body).toHaveCSS('overflow-y', 'auto');
  const bounds = await body.boundingBox();
  expect(bounds).not.toBeNull();
  await page.mouse.move(bounds!.x + bounds!.width - 30, bounds!.y + 100);

  for (const service of ['inara', 'capi']) {
    const card = page.getByTestId(`accounts-${service}-card`);
    const delta = await card.evaluate((element) => {
      const cardRect = element.getBoundingClientRect();
      const viewport = element.closest('.hud-panel-body')!.getBoundingClientRect();
      if (cardRect.top >= viewport.top && cardRect.bottom <= viewport.bottom) return 0;
      return Math.round((cardRect.top + cardRect.bottom - viewport.top - viewport.bottom) / 2);
    });
    await page.mouse.wheel(0, delta);
    await expect.poll(() => card.evaluate((element) => {
      const cardRect = element.getBoundingClientRect();
      const viewport = element.closest('.hud-panel-body')!.getBoundingClientRect();
      return cardRect.top >= viewport.top && cardRect.bottom <= viewport.bottom;
    })).toBe(true);
  }

  expect(await body.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  expect(await page.locator('.hud-grid').evaluate((element) => element.scrollTop)).toBe(0);
  await page.screenshot({ path: testInfo.outputPath('settings-bottom.png') });
  await page.mouse.wheel(0, -10_000);
  await expect.poll(() => body.evaluate((element) => element.scrollTop)).toBe(0);
  await expect(page.getByTestId('accounts-eddn-card')).toBeVisible();
  expect(errors).toEqual([]);
});
