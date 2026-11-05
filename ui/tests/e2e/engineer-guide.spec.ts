import { expect, test, type Page } from '@playwright/test';
import { openLayout } from './helpers';

const port = Number(process.env.TMC_GUIDE_QA_PORT ?? 47690);
const slot = (page: Page, id = 'engineer-guide') => page.locator(`[data-layout-slot="${id}"]`);
const device = 'dddd1111-1111-4111-8111-111111111111';
test.describe.configure({ mode: 'serial' });
test.afterEach(async ({ request }) => { await request.delete(`/api/ui/layouts/${device}`); });

async function choose(page: Page, id: string) {
  await page.getByTestId('guide-select').selectOption(id);
  await expect(page.getByTestId('guide-select')).toHaveValue(id);
  await expect(slot(page).locator('.stepper')).toBeVisible();
}
async function journal(page: Page, event: Record<string, unknown>) {
  const response = await page.request.post(`http://127.0.0.1:${port + 1}/__journal`, {
    data: JSON.stringify({ timestamp: new Date().toISOString(), ...event }),
    headers: { 'content-type': 'text/plain' },
  });
  expect(response.ok()).toBe(true);
}

test('factory guide, all engineers, selection shared with the original card and copy feedback', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: async (text: string) => { document.documentElement.dataset.guideCopied = text; },
    } });
  });
  await openLayout(page, 'Ingeniería');
  // Wait for the synthetic replay's final event before testing selection and copy.
  await expect(page.getByRole('banner')).toContainText('Synthetic Neighbour');
  await expect(page.getByTestId('guide-select').locator('option')).toHaveCount(39);
  await choose(page, '300100');
  await expect(slot(page, 'engineers')).toContainText('Felicity Farseer');
  await page.getByTestId('guide-copy').click();
  await expect(slot(page)).toContainText('Sistema copiado');
  await expect(page.locator('html')).toHaveAttribute('data-guide-copied', 'Deciat');
  const engineers = slot(page, 'engineers');
  await engineers.locator('.panel-header-toggle').click();
  await engineers.locator('[data-engineer-id="300160"] .engineer-button').click();
  await expect(page.getByTestId('guide-select')).toHaveValue('300160');
  await expect(slot(page).locator('.identity')).toContainText('Elvira Martuuk');
  await choose(page, '300160'); // Re-selecting a select option must not toggle off the shared id.
  await expect(slot(page).locator('.identity')).toContainText('Elvira Martuuk');
  expect(errors).toEqual([]);
});

test('journal updates move from invited to rank progression without treating stock as a delivery', async ({ page }) => {
  await openLayout(page, 'Ingeniería'); await choose(page, '300100');
  await journal(page, { event: 'EngineerProgress', Engineer: 'Felicity Farseer', EngineerID: 300100, Progress: 'Invited' });
  await journal(page, { event: 'Cargo', Vessel: 'Ship', Count: 3, Inventory: [{ Name: 'metaalloys', Count: 3, Stolen: 0 }] });
  await expect(page.getByTestId('guide-stock')).toContainText('3 / 1');
  await expect(slot(page).locator('[data-stage="unlock"]')).toHaveAttribute('data-state', 'current');
  await journal(page, { event: 'EngineerProgress', Engineer: 'Felicity Farseer', EngineerID: 300100, Progress: 'Unlocked', Rank: 3, RankProgress: 42 });
  await expect(page.getByTestId('guide-rank')).toContainText('3 / 5');
  await expect(page.getByTestId('guide-rank')).toContainText('42');
  await journal(page, { event: 'EngineerProgress', Engineer: 'Felicity Farseer', EngineerID: 300100, Progress: 'Unlocked', Rank: 5, RankProgress: 0 });
  await expect(page.getByTestId('guide-rank')).toContainText('Máximo');
  await journal(page, { event: 'EngineerProgress', Engineer: 'Domino Green', EngineerID: 400002, Progress: 'Unlocked' });
  await choose(page, '400002');
  await expect(slot(page)).toContainText('Odyssey no usa rangos');
  await expect(page.getByTestId('guide-rank')).toHaveCount(0);
});

test('Yi Shen preserves all three referral conditions and navigates to the right previous contact', async ({ page }) => {
  await openLayout(page, 'Ingeniería'); await choose(page, '400013');
  await expect(page.getByTestId('guide-next')).toContainText('10 Faction Associates');
  await expect(page.getByTestId('guide-next')).toContainText('10 Digital Designs');
  await expect(page.getByTestId('guide-next')).toContainText('10 Manufacturing Instructions');
  await expect(page.getByTestId('guide-stock')).toHaveCount(0);
  await slot(page).locator('.panel-header-toggle').click();
  await expect(slot(page).locator('[data-prerequisite-id]')).toHaveCount(3);
  await slot(page).locator('[data-prerequisite-id="400010"]').click();
  await expect(page.getByTestId('guide-select')).toHaveValue('400010');
  await expect(slot(page).locator('.identity')).toContainText('Baltanos');
});

test('a failed guide page is retried explicitly and never exposes stale instructions', async ({ page }) => {
  let failures = 0;
  await page.route('**/api/engineering/engineers?**', async route => {
    const query = new URL(route.request().url()).searchParams;
    if (query.get('section') === 'unlock' && query.get('engineer_id') === '300010' && failures++ === 0) await route.fulfill({ status: 503, body: '{}' });
    else await route.continue();
  });
  await openLayout(page, 'Ingeniería');
  await page.getByTestId('guide-select').selectOption('300010');
  await expect(page.getByTestId('guide-retry')).toBeVisible();
  await expect(page.getByTestId('guide-next')).toHaveCount(0);
  await page.getByTestId('guide-retry').click();
  await expect(page.getByTestId('guide-next')).toContainText('Selene Jean');
});

test('a pre-existing customized layout stays intact and can add, resize and persist the guide', async ({ page }) => {
  const cards = [
    { id: 'materials', w: 2, h: 320, x: 0 }, { id: 'engineers', w: 1, h: 400, x: 0 },
    { id: 'eng-alerts', w: 1, h: null }, { id: 'eng-ship', w: 1, h: null },
    { id: 'eng-flow', w: 2, h: null }, { id: 'shopping', w: 2, h: null },
    { id: 'trades', w: 1, h: null }, { id: 'sources', w: 1, h: null },
  ];
  const doc = { version: 1, tabs: { 'layout:engineering': { cards, removed: [] } }, custom: [], customOrder: [] };
  const response = await page.request.put(`/api/ui/layouts/${device}`, { data: { name: 'Guide QA synthetic', doc } });
  expect(response.ok()).toBe(true);
  await page.addInitScript(id => localStorage.setItem('tmc.ui.device', id), device);
  await openLayout(page, 'Ingeniería');
  await expect(slot(page)).toHaveCount(0);
  await page.getByTestId('layout-edit').click();
  await page.getByTestId('layout-add').click();
  await page.locator('[data-catalog-entry="engineer-guide"] input').check();
  await page.getByTestId('catalog-add').click();
  await expect(slot(page)).toHaveCount(1);
  await slot(page).locator('[data-op="height-more"]').click();
  await page.getByTestId('layout-done').click();
  await page.reload();
  await page.locator('button.nav-btn:not([data-module])', { has: page.locator('.nav-label', { hasText: /^Ingeniería$/ }) }).click();
  await expect(slot(page)).toHaveCount(1);
  const profile = await (await page.request.get(`/api/ui/layouts/${device}`)).json();
  expect(profile.doc.tabs['layout:engineering'].cards.slice(0, cards.length)).toMatchObject(cards);
  const guide = profile.doc.tabs['layout:engineering'].cards.find((card: { id: string }) => card.id === 'engineer-guide');
  expect(guide.w).toBe(2); expect(guide.h).toBeGreaterThanOrEqual(160);
});

test('a fixed-height guide scrolls its body while its header stays framed', async ({ page }, info) => {
  const doc = { version: 1, tabs: { 'layout:engineering': {
    cards: [{ id: 'engineer-guide', w: 1, h: 160, x: 0 }],
    removed: ['materials', 'engineers', 'eng-alerts', 'eng-ship', 'eng-flow', 'shopping', 'trades', 'sources'],
  } }, custom: [], customOrder: [] };
  expect((await page.request.put(`/api/ui/layouts/${device}`, { data: { name: 'Guide QA synthetic', doc } })).ok()).toBe(true);
  await page.addInitScript(id => localStorage.setItem('tmc.ui.device', id), device);
  await openLayout(page, 'Ingeniería'); await choose(page, '300010');
  const root = slot(page); await root.locator('.panel-header-toggle').click();
  const body = root.locator('.hud-panel-body');
  expect(await body.evaluate(el => el.scrollHeight > el.clientHeight)).toBe(true);
  const header = await root.locator('.hud-panel-header').boundingBox();
  const bounds = (await body.boundingBox())!;
  await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height - 8);
  await page.mouse.wheel(0, 500);
  await expect.poll(() => body.evaluate(el => el.scrollTop)).toBeGreaterThan(0);
  expect(await root.locator('.hud-panel-header').boundingBox()).toEqual(header);
  const screenshot = await root.screenshot({ path: process.env.TMC_GUIDE_QA_DIR
    ? `${process.env.TMC_GUIDE_QA_DIR}/guide-fixed-height.png` : info.outputPath('guide-fixed-height.png') });
  await info.attach('guide-fixed-height', { body: screenshot, contentType: 'image/png' });
});
