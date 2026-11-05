import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import EngineerGuidePanel from '../src/modules/engineering/EngineerGuidePanel.svelte';
import EngineersPanel from '../src/modules/engineering/EngineersPanel.svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import fixture from './fixtures/engineering-topics.json';
import type { EngineerRow, EngineerUnlockRow } from '../src/modules/engineering/types';

let target: HTMLDivElement;
let instances: ReturnType<typeof mount>[];
let engineerRows: EngineerRow[];
let unlockRows: EngineerUnlockRow[];
let failUnlock = false;
let failSelection = false;
beforeEach(() => {
  disconnect(); setLocale('es'); live.connected = true;
  for (const [k, v] of Object.entries(structuredClone(fixture))) if (k !== 'engineering.pages') live.topics[k] = v;
  live.topics['core.cmdr'] = { name: 'Synthetic', credits: null };
  live.topics['core.ship'] = { cargo: [{ Name: 'metaalloys', Count: 3 }] };
  live.topics['engineering.engineers'] = { revision: 21, selected_id: null, next_step: null, total: 38 };
  engineerRows = structuredClone(fixture['engineering.pages'].engineers_engineers.items) as EngineerRow[];
  unlockRows = structuredClone(fixture['engineering.pages'].engineers_unlock.items) as EngineerUnlockRow[];
  failUnlock = false; failSelection = false; instances = [];
  target = document.createElement('div'); document.body.append(target);
  vi.stubGlobal('fetch', vi.fn(async (input, init) => {
    if (init?.method === 'POST') return new Response('{}', { status: failSelection ? 500 : 200 });
    const q = new URL(String(input), 'http://test').searchParams;
    const section = q.get('section');
    if (section === 'unlock' && failUnlock) return new Response('', { status: 503 });
    const id = Number(q.get('engineer_id'));
    const items = section === 'engineers' ? engineerRows : section === 'unlock' ? unlockRows.filter(r => r.engineer_id === id) : [];
    return new Response(JSON.stringify({ revision: Number(q.get('revision')), cursor: 0, section, total: items.length, next_cursor: null, items }));
  }));
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: vi.fn().mockResolvedValue(undefined) } });
});
afterEach(async () => { for (const instance of instances) await unmount(instance); target.remove(); vi.unstubAllGlobals(); });
async function settle() { await new Promise(resolve => setTimeout(resolve, 0)); flushSync(); await new Promise(resolve => setTimeout(resolve, 0)); flushSync(); }
async function show() { instances.push(mount(EngineerGuidePanel, { target })); flushSync(); await settle(); }
async function select(id: number | null, status = 'Known', rank: number | null = null, progress: number | null = null) {
  const row = engineerRows.find(r => r.id === id);
  if (row) Object.assign(row, { status, rank, rank_progress: progress });
  const current = live.topics['engineering.engineers'] as { revision: number };
  live.topics['engineering.engineers'] = { ...current, revision: current.revision + 1, selected_id: id, next_step: null, total: 38 };
  flushSync(); await settle();
}
function panel() { return target.querySelector('[data-testid="engineer-guide"]')!; }
function details() { (target.querySelector('.panel-header-toggle') as HTMLButtonElement).click(); flushSync(); }

test('no initial selection sends no action and keeps the full compact selector usable', async () => {
  await show();
  expect(target.querySelectorAll('[data-testid="guide-select"] option')).toHaveLength(39);
  expect(vi.mocked(fetch).mock.calls.some(([, init]) => init?.method === 'POST')).toBe(false);
  expect(panel().textContent).toContain('Selecciona');
});
test('selection posts the existing action and waits for confirmation from the topic', async () => {
  await show();
  const input = target.querySelector('[data-testid="guide-select"]') as HTMLSelectElement;
  input.value = '300100'; input.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.select_engineer', expect.objectContaining({ body: JSON.stringify({ id: 300100 }), method: 'POST' }));
  expect(target.querySelector('[data-testid="guide-next"]')).toBeNull();
  await select(300100);
  expect(target.querySelector('[data-testid="guide-next"]')?.textContent).toContain('Scout');
});
test('reselecting the current engineer does not toggle off the shared selection', async () => {
  await show(); await select(300100);
  const input = target.querySelector('[data-testid="guide-select"]') as HTMLSelectElement;
  input.value = '300100'; input.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  expect(vi.mocked(fetch).mock.calls.filter(([, init]) => init?.method === 'POST')).toHaveLength(0);
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('Scout');
});
test('shared selection from the other card updates the guide too', async () => {
  await show();
  const second = document.createElement('div'); target.append(second);
  instances.push(mount(EngineersPanel, { target: second })); flushSync(); await settle();
  await select(300100);
  expect(panel().querySelector('[data-testid="guide-select"]')).toHaveProperty('value', '300100');
  expect(panel().textContent).toContain('Felicity Farseer');
  expect(second.textContent).toContain('Felicity Farseer');
  await select(300160);
  expect(panel().textContent).toContain('Elvira Martuuk');
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('300 AL');
});
test('complete stock is shown as availability and never confirms the unlock', async () => {
  await show(); await select(300100, 'Invited');
  expect(panel().querySelector('[data-testid="guide-stock"]')?.textContent).toContain('3 / 1');
  expect(panel().querySelector('[data-testid="guide-stock"]')?.textContent).toContain('0');
  expect(panel().querySelector('[data-stage="unlock"]')?.getAttribute('data-state')).toBe('current');
  expect(panel().querySelector('[role="progressbar"]')?.getAttribute('aria-valuenow')).toBe('100');
});
test('missing inventory stays unknown instead of becoming zero', async () => {
  delete live.topics['core.ship']; await show(); await select(300100, 'Invited');
  expect(panel().querySelector('[data-testid="guide-stock"]')?.textContent).toContain('Sin datos');
  expect(panel().querySelector('[data-testid="guide-stock"] [role="progressbar"]')).toBeNull();
});
test('three available out of ten leaves seven missing and thirty percent, without confirming delivery', async () => {
  const delivery = unlockRows.find(row => row.engineer_id === 300100 && row.field === 'contribution');
  if (delivery?.field === 'contribution') delivery.amount = 10;
  await show(); await select(300100, 'Invited');
  const stock = panel().querySelector('[data-testid="guide-stock"]')!;
  expect(stock.textContent).toContain('3 / 10');
  expect(stock.textContent).toContain('Faltan: 7');
  expect(stock.querySelector('[role="progressbar"]')?.getAttribute('aria-valuenow')).toBe('30');
  expect(panel().querySelector('[data-stage="unlock"]')?.getAttribute('data-state')).toBe('current');
});
test('copy is disabled when the system is unknown', async () => {
  const row = engineerRows.find(row => row.id === 300100)!; row.system = null;
  await show(); await select(300100);
  expect(target.querySelector('[data-testid="guide-copy"]')).toHaveProperty('disabled', true);
});
test('rank 3 and 42 percent, maximum rank, and Odyssey use distinct summaries', async () => {
  await show(); await select(300100, 'Unlocked', 3, 42);
  expect(panel().querySelector('[data-testid="guide-rank"]')?.textContent).toContain('3 / 5');
  expect(panel().querySelector('[data-testid="guide-rank"]')?.textContent).toContain('42');
  await select(300100, 'Unlocked', 5);
  expect(panel().querySelector('[data-testid="guide-rank"]')?.textContent).toContain('Máximo');
  await select(400002, 'Unlocked');
  expect(panel().querySelector('[data-testid="guide-rank"]')).toBeNull();
  expect(panel().textContent).toContain('Odyssey no usa rangos');
  expect(panel().textContent).toContain('Kit Fowler');
});
test('prerequisite can be selected and requirements stay accessible in folded details', async () => {
  await show(); await select(300000);
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('Merchant');
  expect(target.querySelector('.hud-panel-details')?.classList.contains('collapsed')).toBe(true);
  details();
  (target.querySelector('[data-prerequisite-id="300210"]') as HTMLButtonElement).click(); await settle();
  expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.select_engineer', expect.objectContaining({ body: JSON.stringify({ id: 300210 }) }));
});
test('copying a system confirms success and reports failure without crashing', async () => {
  await show(); await select(300100);
  (target.querySelector('[data-testid="guide-copy"]') as HTMLButtonElement).click(); await settle();
  expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Deciat');
  expect(panel().textContent).toContain('Sistema copiado');
  vi.mocked(navigator.clipboard.writeText).mockRejectedValueOnce(new Error('denied'));
  (target.querySelector('[data-testid="guide-copy"]') as HTMLButtonElement).click(); await settle();
  expect(panel().textContent).toContain('No se pudo copiar');
});
test('failed selection does not show another engineer requirements', async () => {
  await show(); await select(300100); failSelection = true;
  const input = target.querySelector('[data-testid="guide-select"]') as HTMLSelectElement;
  input.value = '300160'; input.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  expect(panel().textContent).toContain('No se pudo seleccionar');
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('Scout');
  expect(input.value).toBe('300100');
});
test('failed guide requests retry only on an explicit click', async () => {
  failUnlock = true; await show(); await select(300100);
  expect(panel().querySelector('[data-testid="guide-next"]')).toBeNull();
  const calls = vi.mocked(fetch).mock.calls.length; await settle(); expect(fetch).toHaveBeenCalledTimes(calls);
  failUnlock = false;
  (target.querySelector('[data-testid="guide-retry"]') as HTMLButtonElement).click(); await settle();
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('Scout');
});
test('disconnect and commander changes immediately discard the visible instructions', async () => {
  await show(); await select(300100);
  live.connected = false; flushSync();
  expect(panel().querySelector('[data-testid="guide-next"]')).toBeNull();
  live.connected = true; flushSync(); await settle();
  live.topics['core.cmdr'] = { name: 'OtherSynthetic' }; flushSync();
  expect(panel().querySelector('[data-testid="guide-next"]')).toBeNull();
});
test('late response for a previous engineer cannot replace the current guide', async () => {
  await show();
  const original = fetch;
  let finish!: (response: Response) => void;
  vi.stubGlobal('fetch', vi.fn((input, init) => {
    if (String(input).includes('section=unlock') && String(input).includes('engineer_id=300100')) {
      return new Promise<Response>(resolve => { finish = resolve; });
    }
    return original(input, init);
  }));
  await select(300100);
  expect(panel().querySelector('[data-testid="guide-next"]')).toBeNull();
  await select(300160);
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('300 AL');
  const items = unlockRows.filter(row => row.engineer_id === 300100);
  finish(new Response(JSON.stringify({ revision: 22, section: 'unlock', cursor: 0, total: items.length, next_cursor: null, items })));
  await settle();
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('300 AL');
  expect(panel().querySelector('[data-testid="guide-select"]')).toHaveProperty('value', '300160');
});
test('missing commander balance is unknown for a credit donation', async () => {
  await show(); await select(300180, 'Invited');
  const stock = panel().querySelector('[data-testid="guide-stock"]')!;
  expect(stock.textContent).toContain('Sin datos');
  expect(stock.querySelector('[role="progressbar"]')).toBeNull();
});
test('barred and acquainted states do not invent confirmed milestones', async () => {
  await show(); await select(300100, 'Barred');
  expect(panel().textContent).toContain('vetado');
  expect(panel().querySelectorAll('[data-state="confirmed"]')).toHaveLength(0);
  await select(300100, 'Acquainted');
  expect(panel().querySelectorAll('[data-state="confirmed"]')).toHaveLength(0);
  expect(panel().querySelector('[data-testid="guide-next"]')?.textContent).toContain('Meta-Alloys');
});
