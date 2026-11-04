import { expect, test, type Browser, type BrowserContext, type Locator, type Page } from '@playwright/test';
import { openLayout } from './helpers';

test.describe.configure({ mode: 'serial' });

let context: BrowserContext;
let page: Page;
let baseURL: string;
let autoHeight = 0;
let savedHeight = 0;
const unexpectedDialogs: string[] = [];

test.beforeAll(async ({ browser }, info) => {
  baseURL = String(info.project.use.baseURL);
  context = await browser.newContext({ baseURL });
  page = await context.newPage();
  // Cualquier diálogo no esperado se descarta y se registra; cada test comprueba que no hubo ninguno.
  page.on('dialog', (d) => {
    unexpectedDialogs.push(`${d.type()}: ${d.message()}`);
    void d.dismiss();
  });
});

test.afterEach(() => {
  expect(unexpectedDialogs).toEqual([]);
});

/** Atiende una única confirmación esperada durante `action`: comprueba tipo y texto, y la acepta. */
async function withConfirm(message: string, action: () => Promise<void>) {
  const seen: string[] = [];
  const handler = (d: import('@playwright/test').Dialog) => {
    seen.push(`${d.type()}: ${d.message()}`);
    expect(d.type()).toBe('confirm');
    expect(d.message()).toBe(message);
    void d.accept();
  };
  page.removeAllListeners('dialog');
  page.once('dialog', handler);
  try {
    await action();
    await expect.poll(() => seen.length).toBe(1);
  } finally {
    page.removeAllListeners('dialog');
    page.on('dialog', (d) => {
      unexpectedDialogs.push(`${d.type()}: ${d.message()}`);
      void d.dismiss();
    });
  }
}

test.afterAll(async () => {
  await context.close();
});

const slot = (id: string, p: Page = page) => p.locator(`[data-layout-slot="${id}"]`);
const content = (id: string, p: Page = page) => slot(id, p).locator('.card-content');
const slotIds = (p: Page = page) =>
  p.locator('[data-layout-slot]').evaluateAll((els) => els.map((e) => e.getAttribute('data-layout-slot') as string));

async function startEdit(p: Page = page) {
  await p.getByTestId('layout-edit').click();
  await expect(p.getByTestId('edit-toolbar')).toBeVisible();
}

async function finishEdit(p: Page = page) {
  await p.getByTestId('layout-done').click();
  await expect(p.getByTestId('edit-toolbar')).toHaveCount(0);
}

async function openEngineering(p: Page = page) {
  await p.locator('button.nav-btn:not([data-module])', { has: p.locator('.nav-label', { hasText: /^Ingeniería$/ }) }).click();
  await expect(slot('engineers', p)).toBeAttached();
}

/** Columnas que ocupa un marco, deducidas de su ancho en px dentro del tablero (posición calculada). */
const spanOf = (loc: Locator) => loc.evaluate((el) => {
  const board = el.parentElement as HTMLElement;
  const gap = 0.85 * parseFloat(getComputedStyle(document.documentElement).fontSize);
  const w = window.innerWidth;
  const columns = w >= 2400 ? 4 : w >= 1600 ? 3 : w >= 900 ? 2 : 1;
  const colW = (board.clientWidth - gap * (columns - 1)) / columns;
  return Math.round(((el as HTMLElement).offsetWidth + gap) / (colW + gap));
});

// El alto fijo se aplica en línea sobre `.card-content` (el marco no lo lleva).
const heightPx = async (loc: Locator) =>
  Number.parseInt((await loc.getAttribute('style'))?.match(/height:\s*(\d+)px/)?.[1] ?? '0', 10);

test('first run shows device prompt and creates PC profile', async () => {
  await openLayout(page, 'Ingeniería');
  await expect(page.getByTestId('device-prompt')).toBeVisible();
  await page.getByTestId('device-name').fill('PC');
  await page.getByTestId('device-accept').click();
  await expect(page.getByTestId('device-prompt')).toHaveCount(0);
  const profiles = (await (await page.request.get('/api/ui/layouts')).json()) as { name: string }[];
  expect(profiles.map((p) => p.name)).toEqual(['PC']);
  await expect(slot('engineers')).toBeAttached();
});

test('edit engineering: move engineers first, width 2, height +5, remove sources, done', async () => {
  await expect(content('engineers')).toBeVisible();
  autoHeight = Math.round((await content('engineers').boundingBox())!.height);
  expect(await slotIds()).toContain('sources');
  expect((await slotIds())[0]).not.toBe('engineers');
  await startEdit();
  while ((await slotIds())[0] !== 'engineers') {
    await slot('engineers').locator('[data-op="up"]').click();
  }
  await slot('engineers').locator('[data-op="width-2"]').click();
  for (let i = 0; i < 5; i++) await slot('engineers').locator('[data-op="height-more"]').click();
  await slot('sources').locator('[data-op="remove"]').click();
  await expect(slot('sources')).toHaveCount(0);
  await finishEdit();
  const ids = await slotIds();
  expect(ids[0]).toBe('engineers');
  expect(ids).not.toContain('sources');
  savedHeight = await heightPx(content('engineers'));
  expect(savedHeight % 40).toBe(0);
  expect(savedHeight).toBeGreaterThan(autoHeight);
});

test('reload keeps order, size and removal', async () => {
  await page.reload();
  await openEngineering();
  const ids = await slotIds();
  expect(ids[0]).toBe('engineers');
  expect(ids).not.toContain('sources');
  await expect.poll(() => spanOf(slot('engineers'))).toBe(2);
  expect(await heightPx(content('engineers'))).toBe(savedHeight);
  await expect(page.getByTestId('device-prompt')).toHaveCount(0);
});

test('catalog re-adds sources at the end', async () => {
  await startEdit();
  await page.getByTestId('layout-add').click();
  await page.locator('[data-catalog-entry="sources"] input').check();
  await page.getByTestId('catalog-add').click();
  await finishEdit();
  const ids = await slotIds();
  expect(ids[ids.length - 1]).toBe('sources');
});

test('cancel discards a move', async () => {
  const before = await slotIds();
  await startEdit();
  await slot('engineers').locator('[data-op="down"]').click();
  expect(await slotIds()).not.toEqual(before);
  await page.getByTestId('layout-cancel').click();
  await expect(page.getByTestId('edit-toolbar')).toHaveCount(0);
  expect(await slotIds()).toEqual(before);
});

test('custom tab Caza AX with ax and cargo persists across reload and is not left by auto-companion', async () => {
  await page.getByTestId('custom-tab-new').click();
  await expect(page.getByTestId('custom-tab-dialog')).toBeVisible();
  await page.getByTestId('custom-tab-name').fill('Caza AX');
  await page.getByTestId('custom-tab-submit').click();
  await page.locator('[data-catalog-entry="ax"] input').check();
  await page.getByTestId('catalog-add').click();
  await page.getByTestId('layout-add').click();
  await page.locator('[data-catalog-entry="cargo"] input').check();
  await page.getByTestId('catalog-add').click();
  await expect(slot('ax')).toBeAttached();
  await expect(slot('cargo')).toBeAttached();
  await finishEdit();
  const tab = page.locator('button.nav-btn[data-custom]').filter({ hasText: 'Caza AX' });
  await expect(tab).toHaveClass(/active/);
  // El auto-compañero no abandona una pestaña elegida a mano.
  await expect(page.getByTestId('auto-companion')).toBeChecked();
  // El servidor de este proyecto no tiene control de reproducción (sin puerto falso), así que no hay
  // disparador determinista de un cambio de disposición automática: se deja pasar un margen fijo.
  // Sirve de control que el destino automático (Ingeniería, por el diario) es otra pestaña distinta.
  const autoTarget = page.locator('button.nav-btn:not([data-module])', { has: page.locator('.nav-label', { hasText: /^Ingeniería$/ }) });
  await expect(autoTarget).not.toHaveClass(/active/);
  await page.waitForTimeout(2000);
  await expect(tab).toHaveClass(/active/);
  await expect(slot('cargo')).toBeAttached();
  await page.reload();
  const again = page.locator('button.nav-btn[data-custom]').filter({ hasText: 'Caza AX' });
  await expect(again).toBeVisible();
  await again.click();
  await expect(slot('cargo')).toBeAttached();
  // `ax` aparece solo con datos de AX: fuera de edición no ocupa sitio, en edición es un hueco atenuado.
  await startEdit();
  await expect(slot('ax')).toBeAttached();
  await page.getByTestId('layout-cancel').click();
  await expect(page.getByTestId('edit-toolbar')).toHaveCount(0);
});

test('reset engineering restores factory layout', async () => {
  await openEngineering();
  await startEdit();
  await withConfirm('¿Restablecer esta pestaña a la disposición de fábrica?', () =>
    page.getByTestId('layout-reset').click());
  await finishEdit();
  // `eng-flow` solo aparece con un flujo de ingeniería activo, así que no figura entre los marcos visibles.
  expect(await slotIds()).toEqual(['materials', 'engineers', 'eng-alerts', 'eng-ship', 'shopping', 'trades', 'sources']);
  await expect(content('engineers')).not.toHaveAttribute('style', /height:\s*\d+px/);
});

test('narrow window clamps width and restores when widened', async () => {
  await page.setViewportSize({ width: 1700, height: 900 });
  await expect(slot('engineers')).toBeAttached();
  await startEdit();
  await slot('engineers').locator('[data-op="width-3"]').click();
  await finishEdit();
  await expect.poll(() => spanOf(slot('engineers'))).toBe(3);
  await page.setViewportSize({ width: 1000, height: 900 });
  await expect.poll(() => spanOf(slot('engineers'))).toBe(2);
  await page.setViewportSize({ width: 1700, height: 900 });
  await expect.poll(() => spanOf(slot('engineers'))).toBe(3);
});

test('drag with mouse and with touch pointer events', async () => {
  await startEdit();
  const first = (await slotIds())[0];
  const from = (await slot(first).locator('[data-op="drag"]').boundingBox())!;
  const to = (await slot((await slotIds())[2]).boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
  await page.mouse.up();
  // Con la rejilla libre el orden se normaliza por posición (Tarea 20 reescribe el arrastre): basta con que se mueva.
  await expect.poll(async () => (await slotIds())[0]).not.toBe(first);

  const moved = (await slotIds())[0];
  const handle = slot(moved).locator('[data-op="drag"]');
  const tb = (await handle.boundingBox())!;
  const dest = (await slot((await slotIds())[2]).boundingBox())!;
  const init = { pointerId: 7, pointerType: 'touch', isPrimary: true, bubbles: true, button: 0 };
  const x = dest.x + dest.width / 2;
  const y = dest.y + dest.height / 2;
  await handle.dispatchEvent('pointerdown', { ...init, clientX: tb.x + 5, clientY: tb.y + 5, buttons: 1 });
  await handle.dispatchEvent('pointermove', { ...init, clientX: x, clientY: y, buttons: 1 });
  await page.waitForTimeout(100);
  await handle.dispatchEvent('pointerup', { ...init, clientX: x, clientY: y, buttons: 0 });
  await expect.poll(async () => (await slotIds())[0]).not.toBe(moved);
  await finishEdit();
});

test('second browser context copies PC layout', async ({ browser }: { browser: Browser }) => {
  const pcOrder = await slotIds();
  const second = await browser.newContext({ baseURL });
  try {
    const p2 = await second.newPage();
    await openLayout(p2, 'Ingeniería');
    await expect(p2.getByTestId('device-prompt')).toBeVisible();
    await p2.getByTestId('device-name').fill('Tablet');
    await p2.getByTestId('device-mode-copy').check();
    const select = p2.getByTestId('device-copy-select');
    const pc = await select.locator('option', { hasText: /^PC/ }).getAttribute('value');
    await select.selectOption(pc as string);
    await p2.getByTestId('device-accept').click();
    await expect(p2.getByTestId('device-prompt')).toHaveCount(0);
    await expect(slot('engineers', p2)).toBeAttached();
    expect(await slotIds(p2)).toEqual(pcOrder);
  } finally {
    await second.close();
  }
});

test('enlarging engineers card shows more engineer rows', async () => {
  // Con los detalles desplegados (fuera de edición: en edición el contenido es inerte).
  const toggle = slot('engineers').locator('.panel-header-toggle').first();
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click();
  await expect(slot('engineers').locator('[data-engineer-id]').first()).toBeVisible();
  await startEdit();
  const less = slot('engineers').locator('[data-op="height-less"]');
  const more = slot('engineers').locator('[data-op="height-more"]');
  const visibleRows = () => page.evaluate(() => {
    const box = document.querySelector('[data-layout-slot="engineers"] .card-content')!.getBoundingClientRect();
    return Array.from(document.querySelectorAll('[data-layout-slot="engineers"] [data-engineer-id]'))
      .filter((r) => { const b = r.getBoundingClientRect(); return b.height > 0 && b.top >= box.top && b.bottom <= box.bottom; }).length;
  });
  for (let i = 0; i < 60 && (await heightPx(content('engineers'))) !== 160; i++) await less.click();
  expect(await heightPx(content('engineers'))).toBe(160);
  const small = await visibleRows();
  for (let i = 0; i < 60 && (await heightPx(content('engineers'))) < 1200; i++) await more.click();
  expect(await heightPx(content('engineers'))).toBe(1200);
  const large = await visibleRows();
  expect(large).toBeGreaterThan(small);
  await page.getByTestId('layout-cancel').click();
});
