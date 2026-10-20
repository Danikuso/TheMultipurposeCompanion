import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import ProspectorPanel from '../src/modules/mining/ProspectorPanel.svelte';
import MiningSessionPanel from '../src/modules/mining/MiningSessionPanel.svelte';
import SellPanel from '../src/modules/mining/SellPanel.svelte';
import { namesFrom, materialName } from '../src/modules/mining/types';
import { relativeAge, skewFrom } from '../src/lib/format';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mat(key: string, name: string | null, proportion: number, est_value: number | null, interest = false) {
  return { key, name, proportion, est_value, interest };
}
function asteroid(extra: Record<string, unknown> = {}) {
  return {
    content: 'high', motherlode: null,
    materials: [
      mat('platinum', 'Platino', 31.5, 2079000, true),
      mat('palladium', 'Paladio', 12, 600000),
    ],
    remaining: 100, verdict: { kind: 'worth', est_value: 2079000 }, est_total: 2679000, ...extra,
  };
}
function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}
const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);

beforeEach(() => {
  disconnect(); setLocale('es');
  target = document.createElement('div'); document.body.appendChild(target);
});
afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined; target.remove(); vi.useRealTimers(); vi.unstubAllGlobals();
});

test('stacked composition widths follow proportions', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid(), history: [] } });
  const segs = [...target.querySelectorAll('[data-testid="stack-segment"]')] as HTMLElement[];
  expect(segs.map((s) => s.style.width)).toEqual(['31.5%', '12%', '56.5%']);
  expect(q('stack-rest')).not.toBeNull();
});

test('worth verdict shows approx value', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid(), history: [] } });
  const v = q('verdict')!;
  expect(v.textContent).toContain('≈');
  expect(v.textContent).toContain('2.079.000');
  expect(v.getAttribute('data-kind')).toBe('worth');
});

test('no data verdict', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid({ verdict: { kind: 'no_data' } }), history: [] } });
  expect(q('verdict')!.textContent).toContain('sin datos');
});

test('history lists 8 rows', () => {
  const history = Array.from({ length: 8 }, (_, i) => asteroid({ est_total: (i + 1) * 1000 }));
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid(), history } });
  expect(target.querySelectorAll('[data-testid="history-item"]').length).toBe(8);
});

test('material table uses localized name, marks interest and core', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid({ motherlode: 'Painita' }), history: [] } });
  const rows = target.querySelectorAll('[data-testid="material-row"]');
  expect(rows.length).toBe(2);
  expect(rows[0].textContent).toContain('Platino');
  expect(rows[0].textContent).toContain('★');
  expect(rows[0].textContent).toContain('≈');
  expect(rows[0].textContent).toContain('31,5');
  expect(q('core')!.textContent).toContain('Painita');
  expect(target.querySelector('[role="progressbar"][data-testid="remaining-bar"]')).not.toBeNull();
});

test('empty topic shows no data', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: null, history: [] } });
  expect(q('prospector-empty')!.textContent).toContain('sin datos');
});

function sessionTopic(extra: Record<string, unknown> = {}) {
  return {
    as_of: '2030-01-01T00:47:00Z', started_at: '2030-01-01T00:00:00Z', refined: { platinum: 29, palladium: 7 },
    limpets_launched: { Prospector: 9, Collection: 33 }, limpets_left: 18, cargo: 38, capacity: 256,
    asteroids_prospected: 4, cracked: 3, est_credits: 9100000,
    refinery_bins: [{ key: 'platinum', tons: 3 }, { key: 'palladium', tons: 1 }], ...extra,
  };
}
function option(extra: Record<string, unknown> = {}) {
  return {
    station_name: 'Ogden Port', system_name: 'LHS 20', distance_ly: 12.3, sell_price: 220000, demand: 1500,
    pad: 'Medium', pad_ok: true, updated_at: '2030-01-01T10:00:00Z', ...extra,
  };
}
function sellTopic(options = [option()]) {
  return {
    status: 'ok', as_of: '2030-01-01T12:00:00Z', by_commodity: { platinum: options }, best_station_total: ['Ogden Port', 9400000],
    fetched_at: '2030-01-01T10:00:00Z', source: 'remote',
  };
}

test('sell panel shows price distance pad and age', () => {
  vi.useFakeTimers();
  // El reloj local va muy por delante del del diario: manda `as_of`.
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  mountWith(SellPanel, {
    'mining.sell': sellTopic(),
    'mining.prospector': { current: asteroid(), history: [] },
  });
  const row = q('sell-option')!;
  expect(row.textContent).toContain('220.000');
  expect(row.textContent).toContain('12,3 al');
  expect(row.textContent).toContain('M');
  expect(row.textContent).toContain('hace 2 h');
  expect(row.textContent).toContain('Ogden Port');
  expect(target.textContent).toContain('Platino');
  expect(q('sell-best')!.textContent).toContain('9.400.000');
  expect(q('sell-best')!.textContent).toContain('≈');
  expect(q('sell-source')!.textContent).toContain('Ardent');
});

test('unknown pad shows a warning', () => {
  mountWith(SellPanel, { 'mining.sell': sellTopic([option({ pad_ok: null, pad: null })]) });
  expect(q('pad-warning')).not.toBeNull();
});

test('disabled status shows no data without error', () => {
  const err = vi.spyOn(console, 'error').mockImplementation(() => {});
  mountWith(SellPanel, { 'mining.sell': { status: 'disabled' } });
  expect(q('sell-empty')!.textContent).toContain('sin datos');
  expect(err).not.toHaveBeenCalled();
  err.mockRestore();
});

test('loading status shows loading text', () => {
  mountWith(SellPanel, { 'mining.sell': { status: 'loading' } });
  expect(q('sell-empty')!.textContent).toContain('consultando');
});

test('session shows rates computed from the journal clock', () => {
  // 36 t y 9,1 M cr en 1 h de diario (core.clock manda sobre el as_of del topic).
  mountWith(MiningSessionPanel, { 'core.clock': { as_of: '2030-01-01T01:00:00Z' }, 'mining.session': sessionTopic() });
  expect(q('tons-hour')!.textContent).toContain('36 t/h');
  expect(q('credits-total')!.textContent).toContain('≈');
  expect(q('credits-total')!.textContent).toContain('9.100.000');
  expect(q('credits-hour')!.textContent).toContain('9.100.000');
  expect(target.querySelectorAll('[data-testid="refined-row"]').length).toBe(2);
  expect(q('limpets-left')!.textContent).toContain('18');
  expect(q('limpets-launched')!.textContent).toContain('42');
  expect(q('cargo')!.textContent).toContain('38');
  expect(q('cargo')!.textContent).toContain('256');
  expect(target.querySelector('[role="progressbar"][data-testid="cargo-bar"]')).not.toBeNull();
  expect(target.querySelectorAll('[data-testid="bin"]').length).toBeGreaterThan(0);
  expect(q('refinery')!.textContent).toContain('estimado');
});

test('session without rates shows dashes', () => {
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic({ est_credits: null }) });
  expect(q('credits-hour')!.textContent).toContain('—');
});

test('rates are dashes under 60 s of session', () => {
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic({ as_of: '2030-01-01T00:00:30Z' }) });
  expect(q('tons-hour')!.textContent).toContain('—');
  expect(q('credits-hour')!.textContent).toContain('—');
});

test('reset posts json action', async () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
  vi.stubGlobal('fetch', fetchMock);
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic() });
  (q('reset-session') as HTMLButtonElement).click();
  expect(fetchMock).toHaveBeenCalledTimes(1);
  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe('/api/actions/mining.reset_session');
  expect(init.method).toBe('POST');
  expect(init.headers['content-type']).toBe('application/json');
});

test('reset failure shows alert', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({}) }));
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic() });
  (q('reset-session') as HTMLButtonElement).click();
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('[role="alert"]')).not.toBeNull(); });
});

test('age advances with the timer on the journal clock', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  mountWith(SellPanel, { 'mining.sell': sellTopic([option({ updated_at: '2030-01-01T11:59:20Z' })]) });
  expect(q('sell-option')!.textContent).toContain('ahora');
  vi.advanceTimersByTime(30_000);
  flushSync();
  expect(q('sell-option')!.textContent).toContain('hace 1 min');
});

test('without as_of ages and durations are hidden', () => {
  mountWith(SellPanel, { 'mining.sell': { ...sellTopic(), as_of: '1970-01-01T00:00:00Z' } });
  expect(q('sell-option')!.textContent).not.toContain('hace');
  expect(q('sell-option')!.textContent).toContain('—');
  unmount(mounted!); mounted = undefined; target.innerHTML = '';
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic({ as_of: undefined }) });
  expect(q('session-time')).toBeNull();
});

test('session duration follows the journal clock', () => {
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic() });
  expect(q('session-time')!.textContent).toContain('47m');
});

test('session duration uses core.clock over a stale topic as_of', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  // Cliente tardío: el as_of del topic es viejo (00:47) pero core.clock es actual (01:30).
  mountWith(MiningSessionPanel, {
    'core.clock': { as_of: '2030-01-01T01:30:00Z' },
    'mining.session': sessionTopic(),
  });
  expect(q('session-time')!.textContent).toContain('1h 30m');
});

test('relativeAge: now, minutes and days', () => {
  const base = Date.parse('2030-01-10T00:00:00Z');
  expect(relativeAge('2030-01-09T23:59:30Z', base)).toBe('ahora');
  expect(relativeAge('2030-01-09T23:55:00Z', base)).toContain('5 min');
  expect(relativeAge('2030-01-07T00:00:00Z', base)).toContain('3');
  expect(relativeAge('2030-01-09T23:59:30Z', null)).toBe('—');
  expect(skewFrom('1970-01-01T00:00:00Z')).toBeNull();
  expect(skewFrom(undefined)).toBeNull();
});

test('names: backend first, then prospector, then capitalized key', () => {
  const p = { current: asteroid(), history: [] } as never;
  const names = namesFrom(p, { platinum: 'Platino (diario)', gold: 'Oro' });
  expect(materialName('platinum', names)).toBe('Platino (diario)');
  expect(materialName('palladium', names)).toBe('Paladio');
  expect(materialName('gold', names)).toBe('Oro');
  expect(materialName('bauxite', names)).toBe('Bauxite');
});

test('session uses backend names and full bin labels', () => {
  mountWith(MiningSessionPanel, { 'mining.session': sessionTopic({ names: { platinum: 'Platino', palladium: 'Paladio' } }) });
  const bins = [...target.querySelectorAll('[data-testid="bin"]')];
  expect(bins[0].textContent).toContain('Platino');
  expect(bins[1].textContent).toContain('Paladio');
});
