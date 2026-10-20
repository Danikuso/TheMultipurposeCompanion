import { expect, test } from '@playwright/test';
import { appendJournal, basePort, openLayout, isWindows, readClipboard, writeClipboard } from './helpers';

const FAKE = basePort + 11;

test('trade_current_market_top5_excludes_49h_sale', async ({ page }) => {
  await openLayout(page, 'Comercio');
  const rows = page.getByTestId('market-row');
  await expect(rows).toHaveCount(5, { timeout: 20_000 });
  const gold = rows.filter({ hasText: 'Oro' });
  // Compatible (L) y reciente: Ficticia Hub a 11 000; el de 49 h (15 000) y el de plataforma pequeña, fuera.
  await expect(gold.getByTestId('market-profit')).toContainText(/1\D?600/, { timeout: 20_000 });
  await expect(gold).toContainText('Ficticia Hub');
  await expect(page.getByTestId('market-station').first()).toContainText('Synthetic Port');
  await expect(page.locator('body')).not.toContainText('Puerto Antiguo');
  await expect(page.locator('body')).not.toContainText('Puerto Pequeno');
});

test('trade_cargo_average_paid_and_profit_exclude_mission_and_stolen', async ({ page }) => {
  await openLayout(page, 'Comercio');
  const lines = page.getByTestId('cargo-line');
  await expect(lines).toHaveCount(3, { timeout: 20_000 });
  const profits = page.getByTestId('cargo-profit');
  // 90 t × (11 000 − 9 500); misión y robada no suman beneficio.
  await expect(profits.first()).toContainText(/135\D?000/, { timeout: 20_000 });
  await expect(lines.first()).toContainText(/9\D?500/);
  await expect(profits.nth(1)).toHaveText('—');
  await expect(profits.nth(2)).toHaveText('—');
});

test('trade_session_profit_is_exact', async ({ page }) => {
  await openLayout(page, 'Comercio');
  await expect(page.getByTestId('session-profit')).toContainText(/90\D?000/, { timeout: 20_000 });
});

test('trade_route_advances_and_copies_next_system', async ({ page }) => {
  test.skip(!isWindows(), 'la comprobación del portapapeles del sistema solo existe en Windows');
  const previousClipboard = readClipboard() ?? '';
  try {
    await openLayout(page, 'Comercio');
    await expect(page.getByTestId('market-row').first()).toBeVisible({ timeout: 20_000 });
    await page.getByTestId('route-recalculate').click();
    await expect(page.getByTestId('route-stop')).toHaveCount(2, { timeout: 20_000 });
    await expect(page.getByTestId('route-progress')).toContainText('1');
    await expect(page.getByTestId('route-total')).toContainText(/15\D?500|1\D?475\D?000/);
    // Atraca en el destino de la primera etapa: avanza y copia el sistema siguiente.
    const now = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
    await appendJournal(FAKE, JSON.stringify({
      timestamp: now, event: 'Docked', StationName: 'Fictional Dock', StationType: 'Coriolis',
      MarketID: 900000002, StarSystem: 'Testia', SystemAddress: 9000000000002,
      StationServices: ['commodities'], LandingPads: { Small: 4, Medium: 4, Large: 2 },
    }));
    await expect(page.getByTestId('route-copied')).toContainText('Ficticia', { timeout: 15_000 });
    await expect(page.getByTestId('route-stop').nth(1)).toHaveAttribute('aria-current', 'step');
    expect(readClipboard()).toBe('Ficticia');
  } finally {
    writeClipboard(previousClipboard);
  }
});
