import { expect, test, type Page } from '@playwright/test';
import { appendJournal, basePort, openLayout } from './helpers';

const fakePort = basePort + 17;
test.describe.configure({ mode: 'serial' });

async function dirtyDrive(page: Page) {
  await page.request.post('/api/actions/engineering.close_flow', { data: {} });
  await openLayout(page, 'Ingeniería');
  await page.getByTestId('module-MainEngines').click();
  await page.getByTestId('blueprint').selectOption('engine_dirty');
  await page.getByTestId('experimental').selectOption('special_engine_overloaded');
  await page.getByTestId('preview').click();
  await expect(page.locator('tr[data-material="cadmium"]')).toBeVisible();
}

async function clearGoals(page: Page) {
  await openLayout(page, 'Ingeniería');
  await expect(page.getByTestId('module-MainEngines')).toBeVisible();
  while (await page.getByTestId('goal-remove-0').count()) {
    const count = await page.locator('[data-goal-id]').count();
    await page.getByTestId('goal-remove-0').click();
    await expect(page.locator('[data-goal-id]')).toHaveCount(count - 1);
  }
}

test('engineering_inventory_and_at_max', async ({ page, context }) => {
  // Keep obsolete requests deliverable so this checks revision guards, not only abort().
  await page.addInitScript(() => {
    const original = window.fetch.bind(window);
    const delivered: number[] = [];
    Object.assign(window, { engineeringDeliveredRevisions: delivered });
    window.fetch = async (input, init) => {
      if (!String(input).includes('/api/engineering/locker?')) return original(input, init);
      const response = await original(input, { ...init, signal: undefined });
      const body = await response.clone().json();
      response.json = async () => body;
      // The caller's await and Svelte updates run before this next browser task.
      setTimeout(() => delivered.push(body.revision), 0);
      return response;
    };
  });
  await openLayout(page, 'Ingeniería');
  await expect(page.locator('li[data-key="iron"]')).toContainText('117');
  await expect(page.locator('li[data-key="polonium"]')).toContainText('150');
  await expect(page.locator('li[data-key="polonium"]')).toContainText('Al máximo');
  await page.getByTestId('tab-Manufactured').click();
  await expect(page.locator('li[data-key="conductiveceramics"]')).toContainText('26');
  await page.getByTestId('tab-Encoded').click();
  await expect(page.locator('li[data-key="foo_new"]')).toContainText('Sin datos');
  await page.getByTestId('tab-locker').click();
  await expect(page.locator('li[data-key="graphene"]')).toContainText('8');
  const second = await context.newPage();
  await openLayout(second, 'Ingeniería');
  await second.getByTestId('tab-locker').click();
  await expect(second.locator('li[data-key="graphene"]')).toContainText('8');
  let release!: () => void;
  let started!: () => void;
  let finished!: () => void;
  let obsoleteRevision = 0;
  const held = new Promise<void>((resolve) => { release = resolve; });
  const captured = new Promise<void>((resolve) => { started = resolve; });
  const fulfilled = new Promise<void>((resolve) => { finished = resolve; });
  let first = true;
  await page.route('**/api/engineering/locker?**', async (route) => {
    if (!first) { await route.continue(); return; }
    first = false;
    const response = await route.fetch();
    const body = await response.body();
    expect(body.byteLength).toBeLessThan(16_384);
    obsoleteRevision = JSON.parse(body.toString()).revision;
    started();
    await held;
    await route.fulfill({ response, body });
    finished();
  });
  await page.getByTestId('tab-Raw').click();
  await page.getByTestId('tab-locker').click();
  await captured;
  await page.evaluate(() => {
    (window as unknown as { engineeringDeliveredRevisions: number[] }).engineeringDeliveredRevisions.length = 0;
  });
  try {
    await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'BackpackChange',
      Added: [{ Name: 'graphene', Type: 'Component', Count: 1 }] }));
    // Both consumers finish the new revision while an obsolete response is held.
    await expect(page.locator('li[data-key="graphene"]')).toContainText('9');
    await expect(second.locator('li[data-key="graphene"]')).toContainText('9');
  } finally { release(); }
  await fulfilled;
  await page.waitForFunction((revision) => {
    return (window as unknown as { engineeringDeliveredRevisions: number[] }).engineeringDeliveredRevisions.includes(revision);
  }, obsoleteRevision);
  await expect(page.locator('li[data-key="graphene"]')).toContainText('9');
  await expect(second.locator('li[data-key="graphene"]')).toContainText('9');
  await page.unroute('**/api/engineering/locker?**');
  await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'BackpackChange',
    Removed: [{ Name: 'graphene', Type: 'Component', Count: 1 }] }));
  await expect(second.locator('li[data-key="graphene"]')).toContainText('8');
  await second.close();
});

test('engineering_engineers_status_distance', async ({ page }) => {
  await openLayout(page, 'Ingeniería');
  const farseer = page.locator('[data-engineer-id="300100"]');
  await expect(farseer).toContainText('Desbloqueado');
  await expect(farseer).toContainText(/12,3/);
  await expect(farseer).toContainText(/Rango: 5/);
  await expect(page.locator('[data-engineer-id="300160"]')).toContainText('Desbloqueado');
  await expect(page.locator('[data-engineer-id="300260"]')).toContainText('Conocido');
});

test('engineering_flow_dirty_drive_g5', async ({ page }) => {
  await dirtyDrive(page);
  // Each G1–G4 once, G5 four times; experimental charged once.
  for (const [key, count] of Object.entries({
    legacyfirmware: 3, mechanicalequipment: 1, chromium: 1, mechanicalcomponents: 1,
    consumerfirmware: 1, selenium: 1, configurablecomponents: 1, industrialfirmware: 4,
    cadmium: 4, pharmaceuticalisolators: 4, iron: 5, hybridcapacitors: 3, securityfirmware: 1,
  })) {
    await expect(page.locator(`tr[data-material="${key}"] td`).nth(1)).toHaveText(String(count));
  }
  await expect(page.getByTestId('cost-grade-5')).toContainText('4');
  await expect(page.getByTestId('flow-engineer')).toContainText('Professor Palin');
});

test('engineering_trade_skips_reserved', async ({ page }) => {
  await clearGoals(page);
  const responses: { total: number; items: { missing: Record<string, number>; trades: unknown[] }[] }[] = [];
  page.on('response', async (response) => {
    const url = new URL(response.url());
    if (url.pathname === '/api/engineering/goals' && url.searchParams.get('section') === 'goals' && response.ok()) {
      const body = await response.body();
      expect(body.byteLength).toBeLessThan(16_384);
      responses.push(JSON.parse(body.toString()));
    }
  });
  // Two cadmium can buy one chromium (different raw families: G3 -> G2 with cross-family penalty gives 2:1).
  // Reserving four for G5 leaves one, below the two-unit exchange lot.
  await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'Materials',
    Raw: Object.entries({ cadmium: 5, selenium: 2, iron: 10 }).map(([Name, Count]) => ({ Name, Count })),
    Manufactured: Object.entries({ mechanicalequipment: 5, mechanicalcomponents: 5, configurablecomponents: 5,
      pharmaceuticalisolators: 10, hybridcapacitors: 10 }).map(([Name, Count]) => ({ Name, Count })),
    Encoded: Object.entries({ legacyfirmware: 10, consumerfirmware: 5, industrialfirmware: 10,
      securityfirmware: 2 }).map(([Name, Count]) => ({ Name, Count })) }));
  await dirtyDrive(page);
  await expect(page.locator('tr[data-material="cadmium"] td').nth(2)).toHaveText('5');
  // Positive control: without a G5 reservation, the same stock covers G4 by trading.
  await page.getByTestId('target-grade').selectOption('4');
  await page.getByTestId('experimental').selectOption('');
  await page.getByTestId('preview').click();
  await expect(page.locator('tr[data-material="chromium"]')).toContainText('Da 2 de Cadmium y recibe 1 Chromium');
  await page.getByTestId('target-grade').selectOption('5');
  await page.getByTestId('experimental').selectOption('special_engine_overloaded');
  await page.getByTestId('preview').click();
  await page.getByTestId('pin-goal').click();
  await expect(page.locator('[data-goal-id]')).toHaveCount(1);
  await page.getByTestId('target-grade').selectOption('4');
  await page.getByTestId('experimental').selectOption('');
  await page.getByTestId('preview').click();
  await page.getByTestId('pin-goal').click();
  await expect(page.locator('[data-goal-id]')).toHaveCount(2);
  await expect(page.locator('[data-section="surplus"]')).toContainText('Cadmium: 1');
  const goals = [...responses].reverse().find((response) => response.total === 2)?.items;
  expect(goals).toHaveLength(2);
  expect(goals!.map((goal) => ({ missing: goal.missing, trades: goal.trades }))).toEqual([
    { missing: { chromium: 1 }, trades: [] },
    { missing: { chromium: 1 }, trades: [] },
  ]);
});

test('engineering_nearest_trader_fake_ardent', async ({ page }) => {
  await dirtyDrive(page);
  // Typed traders use Spansh through the same local transport substitution.
  await expect(page.locator('[data-trader="Raw"]')).toContainText('Mercader Ficticio', { timeout: 20_000 });
  await expect(page.locator('[data-trader="Raw"]')).toContainText(/5,2/);
  await page.getByTestId('module-FrameShiftDrive').click();
  await page.getByTestId('blueprint').selectOption('fsd_longrange');
  await page.getByTestId('preview').click();
  await expect(page.locator('[data-trader="Encoded"]')).toContainText('Base de Prueba');
});

test('engineering_no_services_no_data', async ({ page, request }) => {
  const current = await (await request.get('/api/config')).json();
  expect((await request.put('/api/config', { data: { ...current, online_services: false } })).ok()).toBeTruthy();
  try {
    await dirtyDrive(page);
    await expect(page.locator('[data-trader="Raw"]')).toContainText('Sin datos');
    await expect(page.locator('tr[data-material="cadmium"] td').nth(1)).toHaveText('4');
  } finally {
    expect((await request.put('/api/config', { data: current })).ok()).toBeTruthy();
  }
});

test('engineering_goal_persists_after_restart', async ({ page, request }) => {
  await clearGoals(page);
  await dirtyDrive(page);
  await page.getByTestId('pin-goal').click();
  await expect(page.locator('[data-goal-id]')).toHaveCount(1);
  const before = await page.locator('[data-goal-id]').evaluateAll((rows) => rows.map((row) => row.getAttribute('data-goal-id')));
  expect((await fetch(`http://127.0.0.1:${fakePort}/__restart`, { method: 'POST' })).ok).toBeTruthy();
  await expect.poll(async () => {
    try { return (await request.get('/api/health')).ok(); } catch { return false; }
  }).toBe(true);
  // Startup rebuilds the journal as replay; the first live event loads saved goals.
  await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'MaterialCollected',
    Category: 'Raw', Name: 'iron', Count: 0 }));
  await openLayout(page, 'Ingeniería');
  await expect(page.locator('[data-goal-id]')).toHaveCount(1);
  expect(await page.locator('[data-goal-id]').evaluateAll((rows) => rows.map((row) => row.getAttribute('data-goal-id')))).toEqual(before);
  // Live inventory change makes the objective completable and survives a reconnect.
  await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'EngineerProgress',
    Engineer: 'Professor Palin', EngineerID: 300220, Progress: 'Unlocked', Rank: 5, RankProgress: 100 }));
  await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'Materials',
    Raw: Object.entries({ chromium: 1, selenium: 1, cadmium: 4, iron: 5 }).map(([Name, Count]) => ({ Name, Count })),
    Manufactured: Object.entries({ mechanicalequipment: 1, mechanicalcomponents: 1, configurablecomponents: 1,
      pharmaceuticalisolators: 4, hybridcapacitors: 2 }).map(([Name, Count]) => ({ Name, Count })),
    Encoded: Object.entries({ legacyfirmware: 3, consumerfirmware: 1, industrialfirmware: 4,
      securityfirmware: 1 }).map(([Name, Count]) => ({ Name, Count })) }));
  await expect(page.locator('[data-goal-id]').first()).toContainText('Hybrid Capacitors');
  await appendJournal(fakePort, JSON.stringify({ timestamp: new Date().toISOString(), event: 'MaterialCollected',
    Category: 'Manufactured', Name: 'hybridcapacitors', Count: 1 }));
  await expect(page.locator('[data-goal-id]').first()).toContainText('Ya puedes', { timeout: 15_000 });
  await expect(page.locator('[data-goal-id]').first()).toContainText('Professor Palin');
  await expect(page.locator('[data-alert-id]').filter({ hasText: 'Ya puedes completar' })).toContainText('Professor Palin');
});
