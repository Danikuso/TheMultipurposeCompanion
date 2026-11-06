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
  // Observador de tareas largas instalado antes de cualquier navegación y con referencia global
  // (si no, el recolector lo descarta y la tarea de arranque de la página daría un falso fallo).
  await page.addInitScript(() => {
    const entries: { start: number; dur: number }[] = [];
    const obs = new PerformanceObserver((list) => {
      for (const e of list.getEntries()) entries.push({ start: e.startTime, dur: e.duration });
    });
    obs.observe({ type: 'longtask', buffered: true });
    (window as unknown as { __lt: unknown }).__lt = { obs, entries };
  });
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
  expect(await slotIds()).toEqual(['materials', 'engineers', 'engineer-guide', 'eng-alerts', 'eng-ship', 'shopping', 'trades', 'sources']);
  await expect(content('engineers')).not.toHaveAttribute('style', /height:\s*\d+px/);
});

test('narrow window clamps width and restores when widened', async () => {
  await page.setViewportSize({ width: 1700, height: 900 });
  await expect(slot('engineers')).toBeAttached();
  await startEdit();
  // Los anchos que no caben desde la columna de inicio están deshabilitados: primero a la columna 0.
  const left = slot('engineers').locator('[data-op="left"]');
  for (let i = 0; i < 3 && (await left.isEnabled()); i++) await left.click();
  await slot('engineers').locator('[data-op="width-3"]').click();
  await finishEdit();
  await expect.poll(() => spanOf(slot('engineers'))).toBe(3);
  await page.setViewportSize({ width: 1000, height: 900 });
  await expect.poll(() => spanOf(slot('engineers'))).toBe(2);
  await page.setViewportSize({ width: 1700, height: 900 });
  await expect.poll(() => spanOf(slot('engineers'))).toBe(3);
});

/** Geometría del tablero y de cada marco (px de página) para comprobar columnas y gravedad. */
const boardGeometry = () => page.evaluate(() => {
  const board = document.querySelector('.card-board') as HTMLElement;
  const gap = 0.85 * parseFloat(getComputedStyle(document.documentElement).fontSize);
  const w = window.innerWidth;
  const columns = w >= 2400 ? 4 : w >= 1600 ? 3 : w >= 900 ? 2 : 1;
  const b = board.getBoundingClientRect();
  const colW = (board.clientWidth - gap * (columns - 1)) / columns;
  const frames = Array.from(board.querySelectorAll<HTMLElement>(':scope > [data-layout-slot]')).map((el) => {
    const r = el.getBoundingClientRect();
    return { id: el.getAttribute('data-layout-slot') as string, x: r.x, y: r.y, w: r.width, h: r.height };
  });
  return { gap, columns, colW, left: b.x, top: b.y, frames };
});

/** Espera a que terminen las transiciones de posición (120 ms) leyendo dos veces seguidas lo mismo. */
async function settledGeometry() {
  let prev = JSON.stringify(await boardGeometry());
  for (let i = 0; i < 20; i++) {
    await page.waitForTimeout(150);
    const cur = JSON.stringify(await boardGeometry());
    if (cur === prev) break;
    prev = cur;
  }
  return JSON.parse(prev) as Awaited<ReturnType<typeof boardGeometry>>;
}

async function dragToColumn(kind: 'mouse' | 'touch') {
  let geo = await settledGeometry();
  let step = geo.colW + geo.gap;
  // Una tarjeta de una columna que no esté ya en la última.
  let card = geo.frames.find((f) => f.w < geo.colW * 1.5 && Math.round((f.x - geo.left) / step) < geo.columns - 1);
  // With a new two-column factory card, all narrow cards may initially be on the right.
  // Establish the drag's starting condition using the normal column controls.
  if (!card) {
    const narrow = geo.frames.find((f) => f.w > 0 && f.w < geo.colW * 1.5);
    expect(narrow, 'a one-column card to prepare for dragging').toBeDefined();
    await moveToColumn(narrow!.id, 0);
    geo = await settledGeometry(); step = geo.colW + geo.gap;
    card = geo.frames.find((f) => f.id === narrow!.id);
  }
  expect(card, 'a one-column card with room on its right').toBeDefined();
  const curCol = Math.round((card!.x - geo.left) / step);
  const targetCol = geo.columns - 1;
  const handle = slot(card!.id).locator('[data-op="drag"]');
  const hb = (await handle.boundingBox())!;
  const x0 = hb.x + hb.width / 2;
  const y0 = hb.y + hb.height / 2;
  const dx = (targetCol - curCol) * step;
  if (kind === 'mouse') {
    await page.mouse.move(x0, y0);
    await page.mouse.down();
    await page.mouse.move(x0 + dx, y0, { steps: 8 });
    await page.mouse.up();
  } else {
    const init = { pointerId: 7, pointerType: 'touch', isPrimary: true, bubbles: true, button: 0 };
    await handle.dispatchEvent('pointerdown', { ...init, clientX: x0, clientY: y0, buttons: 1 });
    await handle.dispatchEvent('pointermove', { ...init, clientX: x0 + dx, clientY: y0, buttons: 1 });
    await page.waitForTimeout(100);
    await handle.dispatchEvent('pointerup', { ...init, clientX: x0 + dx, clientY: y0, buttons: 0 });
  }
  // Columna de destino (±2 px) y gravedad: queda arriba del todo o justo bajo la tarjeta de encima.
  await expect.poll(async () => {
    const g = await boardGeometry();
    const me = g.frames.find((f) => f.id === card!.id)!;
    return Math.abs(me.x - (g.left + targetCol * (g.colW + g.gap)));
  }).toBeLessThanOrEqual(2);
  await expect.poll(async () => {
    const g = await boardGeometry();
    const me = g.frames.find((f) => f.id === card!.id)!;
    const above = g.frames.filter((f) => f.id !== card!.id && f.y < me.y && f.x < me.x + me.w - 1 && me.x < f.x + f.w - 1);
    const expected = above.length === 0 ? g.top : Math.max(...above.map((f) => f.y + f.h + g.gap));
    return Math.abs(me.y - expected);
  }).toBeLessThanOrEqual(2);
  return card!.id;
}

test('drag with mouse and with touch pointer events places the card in the target column', async () => {
  await startEdit();
  const moved = await dragToColumn('mouse');
  // Segundo arrastre (táctil): otra tarjeta de una columna a la última.
  const second = await dragToColumn('touch');
  expect([moved, second].every(Boolean)).toBe(true);
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

// ---------------------------------------------------------------------------------------------
// Enmienda A (rejilla libre, catálogo múltiple y fluidez): CA-12 … CA-16.
// ---------------------------------------------------------------------------------------------

type Geo = Awaited<ReturnType<typeof boardGeometry>>;
type Frame = Geo['frames'][number];

/** Ningún par de marcos visibles se cruza (con 1 px de tolerancia). */
function expectNoOverlap(frames: Frame[]) {
  const vis = frames.filter((f) => f.w > 0 && f.h > 0);
  for (let i = 0; i < vis.length; i++) {
    for (let j = i + 1; j < vis.length; j++) {
      const a = vis[i];
      const b = vis[j];
      const crossX = a.x < b.x + b.w - 1 && b.x < a.x + a.w - 1;
      const crossY = a.y < b.y + b.h - 1 && b.y < a.y + a.h - 1;
      expect(crossX && crossY, `${a.id} overlaps ${b.id}`).toBe(false);
    }
  }
}

const box = async (id: string) => (await slot(id).boundingBox())!;

/** Columna (0-based) que ocupa un marco según la geometría ya asentada. */
async function columnOf(id: string) {
  const g = await settledGeometry();
  const f = g.frames.find((x) => x.id === id)!;
  return Math.round((f.x - g.left) / (g.colW + g.gap));
}

/** Pulsa ← / → hasta dejar la tarjeta en la columna pedida. */
async function moveToColumn(id: string, target: number) {
  for (let i = 0; i < 6; i++) {
    const col = await columnOf(id);
    if (col === target) return;
    await slot(id).locator(col < target ? '[data-op="right"]' : '[data-op="left"]').click();
  }
  expect(await columnOf(id), `${id} column`).toBe(target);
}

/** Alto fijo mínimo (160 px) para que la geometría no dependa de los datos. */
async function fixHeight(id: string) {
  const less = slot(id).locator('[data-op="height-less"]');
  for (let i = 0; i < 60 && (await heightPx(content(id))) !== 160; i++) await less.click();
  expect(await heightPx(content(id))).toBe(160);
}

const SEVEN = ['dashboard-cmdr', 'dashboard-location', 'dashboard-status', 'dashboard-route', 'dashboard-session', 'dashboard-alerts', 'jump'];
let tabName = '';
let T: string[] = [];

async function newCustomTab(name: string) {
  await page.getByTestId('custom-tab-new').click();
  await expect(page.getByTestId('custom-tab-dialog')).toBeVisible();
  await page.getByTestId('custom-tab-name').fill(name);
  await page.getByTestId('custom-tab-submit').click();
  await expect(page.getByTestId('custom-tab-dialog')).toHaveCount(0);
}

const openCustom = async (name: string) => {
  await page.locator('button.nav-btn[data-custom]').filter({ hasText: name }).click();
};

test('CA-12 free grid: seven cards added at once and arranged as T1 T2 T3 / T4 T5 / T6 / T7, kept after reload', async () => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  tabName = 'Rejilla';
  await newCustomTab(tabName);
  // La nueva pestaña abre el catálogo en edición: se marcan las siete de una vez.
  for (const id of SEVEN) await page.locator(`[data-catalog-entry="${id}"] input`).check();
  await expect(page.getByTestId('catalog-add')).toHaveText('Añadir (7)');
  await page.getByTestId('catalog-add').click();
  await expect(page.getByTestId('catalog-add')).toHaveCount(0);
  T = await slotIds();
  expect([...T].sort()).toEqual([...SEVEN].sort());
  for (const id of T) {
    await slot(id).locator('[data-op="width-1"]').click();
    await fixHeight(id);
  }
  const cols = [0, 1, 2, 0, 1, 0, 0];
  for (let i = 0; i < T.length; i++) await moveToColumn(T[i], cols[i]);
  await finishEdit();
  await page.reload();
  await openCustom(tabName);
  await expect(slot(T[0])).toBeAttached();
  const g = await settledGeometry();
  expect(g.columns).toBe(3);
  expectNoOverlap(g.frames);
  const b = Object.fromEntries(await Promise.all(T.map(async (id, i) => [`T${i + 1}`, await box(id)])));
  const below = (lo: string, hi: string) => {
    expect(Math.abs(b[lo].x - b[hi].x), `${lo}/${hi} x`).toBeLessThanOrEqual(2);
    expect(b[lo].y, `${lo} below ${hi}`).toBeGreaterThanOrEqual(b[hi].y + b[hi].height - 1);
  };
  below('T4', 'T1');
  below('T5', 'T2');
  below('T6', 'T4');
  below('T7', 'T6');
  expect(b.T3.x).toBeGreaterThan(b.T2.x + b.T2.width - 1);
  expect(Math.abs(b.T3.y - b.T1.y)).toBeLessThanOrEqual(2);
});

test('CA-14 resize handle sits at the bottom-right corner of its own card', async () => {
  await startEdit();
  const g = await settledGeometry();
  expect(g.frames.length).toBeGreaterThanOrEqual(7);
  for (const f of g.frames) {
    const fb = await box(f.id);
    const hb = (await slot(f.id).locator('[data-op="resize"]').boundingBox())!;
    expect(hb.x, `${f.id} left`).toBeGreaterThanOrEqual(fb.x - 1);
    expect(hb.y, `${f.id} top`).toBeGreaterThanOrEqual(fb.y - 1);
    expect(hb.x + hb.width, `${f.id} right`).toBeLessThanOrEqual(fb.x + fb.width + 1);
    expect(hb.y + hb.height, `${f.id} bottom`).toBeLessThanOrEqual(fb.y + fb.height + 1);
    expect(fb.x + fb.width - (hb.x + hb.width), `${f.id} dx`).toBeLessThanOrEqual(4);
    expect(fb.y + fb.height - (hb.y + hb.height), `${f.id} dy`).toBeLessThanOrEqual(4);
  }
  await page.getByTestId('layout-cancel').click();
  await expect(page.getByTestId('edit-toolbar')).toHaveCount(0);
});

test('CA-13 resizing a card never moves the card on its left', async () => {
  // Estado propio: abre la pestaña de CA-12 (T[] viene de ese test, la suite es serial) y entra en edición.
  await openCustom(tabName);
  await startEdit();
  const before = await settledGeometry();
  const t1 = before.frames.find((f) => f.id === T[0])!;
  const t2 = before.frames.find((f) => f.id === T[1])!;
  for (let i = 0; i < 2; i++) await slot(T[1]).locator('[data-op="height-more"]').click();
  const g0 = await settledGeometry();
  const hb = (await slot(T[1]).locator('[data-op="resize"]').boundingBox())!;
  const x0 = hb.x + hb.width / 2;
  const y0 = hb.y + hb.height / 2;
  await page.mouse.move(x0, y0);
  await page.mouse.down();
  await page.mouse.move(x0 + g0.colW + g0.gap, y0, { steps: 8 });
  await page.mouse.up();
  await expect.poll(() => spanOf(slot(T[1]))).toBe(2);
  const after = await settledGeometry();
  const t1b = after.frames.find((f) => f.id === T[0])!;
  for (const k of ['x', 'y', 'w', 'h'] as const) expect(Math.abs(t1b[k] - t1[k]), `T1 ${k}`).toBeLessThanOrEqual(1);
  expect(Math.abs(after.frames.find((f) => f.id === T[1])!.h - t2.h - 80)).toBeLessThanOrEqual(2);
  expectNoOverlap(after.frames);
  await page.getByTestId('layout-cancel').click();
  await expect(page.getByTestId('edit-toolbar')).toHaveCount(0);
});

test('CA-15 catalog adds three checked cards with a single click', async () => {
  await newCustomTab('Tres');
  const ids = ['dashboard-cmdr', 'dashboard-status', 'jump'];
  for (const id of ids) await page.locator(`[data-catalog-entry="${id}"] input`).check();
  await expect(page.getByTestId('catalog-add')).toHaveText('Añadir (3)');
  await page.getByTestId('catalog-add').click();
  await expect(page.getByTestId('catalog-add')).toHaveCount(0);
  expect([...(await slotIds())].sort()).toEqual([...ids].sort());
  await finishEdit();
});

test('Focus 1 and 2: engineering with replay data has no overlapping frames outside edit', async () => {
  await openEngineering();
  await expect(content('engineers')).toBeVisible();
  const g = await settledGeometry();
  expect(g.frames.length).toBeGreaterThanOrEqual(5);
  expectNoOverlap(g.frames);
  // Desplegar los detalles hace crecer el contenido fuera de edición: se reempaqueta sin solapes.
  const toggle = slot('engineers').locator('.panel-header-toggle').first();
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click();
  expectNoOverlap((await settledGeometry()).frames);
});

/** Máximo de tareas largas cuyo inicio cae entre dos marcas de `performance.now()`. */
async function longtaskMax(step: () => Promise<void>) {
  expect(await page.evaluate(() => typeof (window as unknown as { __lt?: unknown }).__lt)).toBe('object');
  const t0 = await page.evaluate(() => performance.now());
  await step();
  await page.waitForTimeout(300);
  const t1 = await page.evaluate(() => performance.now());
  const durs = await page.evaluate(
    ([a, b]) => (window as unknown as { __lt: { entries: { start: number; dur: number }[] } }).__lt.entries
      .filter((e) => e.start >= a && e.start <= b).map((e) => e.dur),
    [t0, t1],
  );
  return Math.max(0, ...durs);
}

test('CA-16 catalog and new-tab dialog open, scroll and respond without long tasks', async () => {
  // Estado propio: pestaña nueva y vacía (su catálogo lista todas las tarjetas, así que desborda).
  await newCustomTab('Fluidez');
  await page.getByTestId('catalog-cancel').click();
  const catalogMax = await longtaskMax(async () => {
    await page.getByTestId('layout-add').click();
    const list = page.locator('.catalog-list');
    await expect(list).toBeVisible();
    expect(await list.evaluate((el) => el.scrollHeight - el.clientHeight), 'catalog list overflows').toBeGreaterThan(0);
    await list.evaluate((el) => { el.scrollTop = el.scrollHeight; });
    expect(await list.evaluate((el) => el.scrollTop), 'list actually scrolled').toBeGreaterThan(0);
    await page.waitForTimeout(100);
    await list.evaluate((el) => { el.scrollTop = 0; });
    const free = page.locator('[data-catalog-entry] input:not(:disabled):not(:checked)');
    for (let i = 0; i < 5; i++) await free.first().check();
  });
  expect(catalogMax).toBeLessThanOrEqual(100);
  await page.getByTestId('catalog-cancel').click();
  await page.getByTestId('layout-cancel').click();
  const dialogMax = await longtaskMax(async () => {
    await page.getByTestId('custom-tab-new').click();
    await expect(page.getByTestId('custom-tab-dialog')).toBeVisible();
    await page.getByTestId('custom-tab-cancel').click();
    await expect(page.getByTestId('custom-tab-dialog')).toHaveCount(0);
  });
  expect(dialogMax).toBeLessThanOrEqual(100);
});

test('M1 fixed-height cards scroll their body when the content overflows and show no bar when it fits', async () => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await openCustom(tabName); // T[] son las siete tarjetas de CA-12, todas de alto fijo 160
  await expect(slot(T[0])).toBeAttached();
  const bodies = await page.evaluate((ids) => ids.map((id) => {
    const b = document.querySelector(`[data-layout-slot="${id}"] .hud-panel-body`) as HTMLElement | null;
    return b ? { id, sh: b.scrollHeight, ch: b.clientHeight, oy: getComputedStyle(b).overflowY } : null;
  }), T);
  const found = bodies.filter((b) => b !== null);
  expect(found.length).toBeGreaterThan(0);
  const overflowing = found.filter((b) => b.sh > b.ch);
  const fitting = found.filter((b) => b.sh <= b.ch);
  expect(overflowing.length, 'at least one 160 px card overflows').toBeGreaterThan(0);
  expect(fitting.length, 'at least one 160 px card fits').toBeGreaterThan(0);
  for (const b of found) expect(b.oy, `${b.id} overflow-y`).toBe('auto');
  const target = overflowing[0];
  const body = slot(target.id).locator('.hud-panel-body');
  await body.hover();
  await page.mouse.wheel(0, 300);
  await expect.poll(() => body.evaluate((el) => el.scrollTop), `${target.id} scrolls`).toBeGreaterThan(0);
  // El marco mantiene su alto fijo: el desbordamiento se desplaza dentro, no estira la tarjeta.
  expect(await heightPx(content(target.id))).toBe(160);
});
