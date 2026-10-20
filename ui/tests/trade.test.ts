import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import MarketPanel from '../src/modules/trade/MarketPanel.svelte';
import CargoPanel from '../src/modules/trade/CargoPanel.svelte';
import RoutePanel from '../src/modules/trade/RoutePanel.svelte';
import TradeSessionPanel from '../src/modules/trade/TradeSessionPanel.svelte';
import SearchPanel from '../src/modules/trade/SearchPanel.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}
const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);
const qa = (id: string) => [...target.querySelectorAll(`[data-testid="${id}"]`)];

beforeEach(() => {
  disconnect(); setLocale('es');
  target = document.createElement('div'); document.body.appendChild(target);
});
afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined; target.remove(); vi.useRealTimers(); vi.unstubAllGlobals();
});

function sell(extra: Record<string, unknown> = {}) {
  return {
    station_name: 'Ogden Port', system_name: 'LHS 20', distance_ly: 12.3, sell_price: 11000, demand: 1500,
    pad: 'Large', pad_ok: true, updated_at: '2030-01-01T10:00:00Z', origin: 'Eddn', ...extra,
  };
}
function mrow(extra: Record<string, unknown> = {}) {
  return {
    commodity: 'gold', name_localised: 'Oro', buy_price: 9400, stock: 5000,
    best_sell: sell(), profit_per_ton: 1600, rare: null, ...extra,
  };
}
function market(rows = [mrow()]) {
  return { as_of: '2030-01-01T12:00:00Z', station: 'Synthetic Port', system: 'Synthetic System', rows, eddn_connected: true };
}
function line(extra: Record<string, unknown> = {}) {
  return {
    commodity: 'gold', name_localised: 'Oro', tons: 96, paid_avg: 41300, paid_partial: false, price_here: 53600,
    best_nearby: null, est_profit: 1200000, advice: 'SellHere', kind: 'Trade', ...extra,
  };
}
function cargo(lines = [line()], used = 180, capacity: number | null = 256) {
  return { as_of: '2030-01-01T12:00:00Z', lines, used, capacity };
}

test('market row shows profit distance pad and age', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  mountWith(MarketPanel, { 'trade.market': market() });
  const row = q('market-row')!;
  expect(row.textContent).toContain('Oro');
  expect(row.textContent).toContain('9.400');
  expect(row.textContent).toContain('1.600');
  expect(row.textContent).toContain('12,3 al');
  expect(q('pad-label')!.textContent).toBe('L');
  expect(row.textContent).toContain('Ogden Port');
  expect(row.textContent).toContain('hace 2 h');
  expect(row.textContent).toContain('EDDN');
});

test('unknown pad shows warning', () => {
  mountWith(MarketPanel, { 'trade.market': market([mrow({ best_sell: sell({ pad: null, pad_ok: null }) })]) });
  const w = q('pad-warning')!;
  expect(w).not.toBeNull();
  expect(w.getAttribute('aria-label')).toContain('plataforma sin confirmar');
});

test('rare badge with distance', () => {
  mountWith(MarketPanel, { 'trade.market': market([mrow({ best_sell: null, profit_per_ton: null, rare: 160 })]) });
  const b = q('rare-badge')!;
  expect(b.textContent).toContain('RARA');
  expect(q('market-row')!.textContent).toContain('160 al');
});

test('empty market shows no data', () => {
  mountWith(MarketPanel, { 'trade.market': { station: null, system: null, rows: [] } });
  expect(q('market-empty')!.textContent).toContain('sin datos');
});

test('stolen cargo shows black market only and no profit', () => {
  mountWith(CargoPanel, { 'trade.cargo': cargo([line({ kind: 'Stolen', est_profit: null, advice: null, paid_avg: null, price_here: null })]) });
  const row = q('cargo-line')!;
  expect(row.textContent).toContain('solo mercado negro');
  expect(row.textContent).toContain('robada');
  expect(q('cargo-profit')!.textContent).toBe('—');
});

test('cargo line shows advice and profit', () => {
  mountWith(CargoPanel, { 'trade.cargo': cargo([line(), line({ commodity: 'palladium', name_localised: 'Paladio', advice: 'Carry', kind: 'Trade', best_nearby: sell() })]) });
  const rows = qa('cargo-line');
  expect(rows[0].textContent).toContain('VENDER AQUÍ');
  expect(rows[0].textContent).toContain('1.200.000');
  expect(rows[1].textContent).toContain('LLEVAR');
  expect(rows[1].textContent).toContain('Ogden Port');
});

test('mission cargo is tagged and has no advice', () => {
  mountWith(CargoPanel, { 'trade.cargo': cargo([line({ kind: 'Mission', est_profit: null, advice: null })]) });
  expect(q('cargo-line')!.textContent).toContain('misión');
});

test('hold bar used over capacity', () => {
  mountWith(CargoPanel, { 'trade.cargo': cargo() });
  const bar = q('hold-bar')!;
  expect(bar.getAttribute('role')).toBe('progressbar');
  expect(bar.getAttribute('aria-valuenow')).toBe('180');
  expect(bar.getAttribute('aria-valuemax')).toBe('256');
  expect(q('hold')!.textContent).toContain('180');
});

function stop(extra: Record<string, unknown> = {}) {
  return {
    from_system: 'LHS 20', from_station: 'Ogden Port', to_system: 'Wolf 359', to_station: 'Wolf 359 Hub',
    commodities: [{ commodity: 'palladium', name_localised: 'Paladio', amount: 400, buy_price: 48210, sell_price: 61080, profit_per_ton: 12870 }],
    stop_profit: 1000000, distance_ly: 14, jumps: 2, data_updated_at: '2030-01-01T10:00:00Z', ...extra,
  };
}
function route(extra: Record<string, unknown> = {}) {
  return {
    as_of: '2030-01-01T12:00:00Z', status: 'ok', blocker: null, current: 1, completed: false, copy_target: null, copy_seq: 0,
    plan: { stops: [stop(), stop({ to_station: 'Nakamura Dock' }), stop({ to_station: 'Ogden Port' })], total_profit: 3200000 }, ...extra,
  };
}

test('route highlights current stop and total', () => {
  mountWith(RoutePanel, { 'trade.route': route() });
  const stops = qa('route-stop');
  expect(stops.length).toBe(3);
  expect(stops.map((s) => s.classList.contains('current'))).toEqual([false, true, false]);
  expect(q('route-total')!.textContent).toContain('3.200.000');
  expect(stops[1].textContent).toContain('Paladio');
  expect(stops[1].textContent).toContain('14 al');
  expect(stops[1].textContent).toContain('Saltos: 2');
  expect(stops.map((s) => s.getAttribute('aria-current'))).toEqual([null, 'step', null]);
});

test('recalculate posts json action', () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
  vi.stubGlobal('fetch', fetchMock);
  mountWith(RoutePanel, { 'trade.route': route() });
  (q('route-recalculate') as HTMLButtonElement).click();
  expect(fetchMock).toHaveBeenCalledTimes(1);
  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe('/api/actions/trade.route_request');
  expect(init.method).toBe('POST');
  expect(init.headers['content-type']).toBe('application/json');
});

test('blocked route shows dock hint', () => {
  mountWith(RoutePanel, { 'trade.route': route({ status: 'blocked', blocker: 'NotDocked', plan: null }) });
  expect(q('route-dock-first')!.textContent).toContain('Atraca');
});

test('route failure shows alert', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({}) }));
  mountWith(RoutePanel, { 'trade.route': route() });
  (q('route-recalculate') as HTMLButtonElement).click();
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('[role="alert"]')).not.toBeNull(); });
});

function session(extra: Record<string, unknown> = {}) {
  return {
    as_of: '2030-01-01T01:05:00Z', started_at: '2030-01-01T00:00:00Z', profit: 14600000, revenue: 20000000, cost_of_sold: 5400000,
    tons_bought: 1216, tons_sold: 1120, operations: 6, best_trade: ['Paladio', 5100000], unknown_cost_tons: 0, rank: 8, progress: 42, ...extra,
  };
}

test('session shows english trade rank', () => {
  mountWith(TradeSessionPanel, { 'trade.session': session() });
  expect(q('session-rank')!.textContent).toContain('Elite');
  expect(q('session-rank')!.textContent).toContain('42');
  expect(q('rank-bar')!.getAttribute('aria-valuenow')).toBe('42');
});

test('session rates use the journal clock', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  mountWith(TradeSessionPanel, { 'core.clock': { as_of: '2030-01-01T01:00:00Z' }, 'trade.session': session({ profit: 13500000 }) });
  expect(q('session-profit')!.textContent).toContain('13.500.000');
  expect(q('session-rate')!.textContent).toContain('13.500.000');
  expect(q('session-tons')!.textContent).toContain('1.216');
  expect(q('session-best')!.textContent).toContain('Paladio');
});

test('session rate is dash under 60 s', () => {
  mountWith(TradeSessionPanel, { 'trade.session': session({ as_of: '2030-01-01T00:00:30Z' }) });
  expect(q('session-rate')!.textContent).toContain('—');
});

test('session reset posts json action', () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
  vi.stubGlobal('fetch', fetchMock);
  mountWith(TradeSessionPanel, { 'trade.session': session() });
  (q('session-reset') as HTMLButtonElement).click();
  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe('/api/actions/trade.reset_session');
  expect(init.headers['content-type']).toBe('application/json');
});

function buy(extra: Record<string, unknown> = {}) {
  return {
    station_name: 'Hutton Orbital', system_name: 'Alpha Centauri', distance_ly: 7, buy_price: 42100, stock: 1900,
    pad: 'Large', pad_ok: true, updated_at: '2030-01-01T10:00:00Z', origin: 'Ardent', ...extra,
  };
}

test('search autocompletes known commodities', () => {
  mountWith(SearchPanel, {
    'trade.commodities': { items: [['tritium', 'Tritio'], ['gold', null], ['palladium', 'Paladio']] },
    'trade.search': { commodity: null, status: 'idle', rows: [] },
  });
  expect(target.querySelectorAll('datalist option').length).toBe(3);
});

test('search posts the commodity key and shows results', () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
  vi.stubGlobal('fetch', fetchMock);
  mountWith(SearchPanel, {
    'trade.commodities': { items: [['tritium', 'Tritio']] },
    'trade.search': { as_of: '2030-01-01T12:00:00Z', commodity: 'tritium', status: 'ok', rows: [buy(), buy({ pad_ok: null, pad: null })] },
  });
  expect(qa('search-row').length).toBe(2);
  expect(qa('search-row')[0].textContent).toContain('Hutton Orbital');
  expect(qa('search-row')[0].textContent).toContain('42.100');
  expect(qa('search-row')[0].textContent).toContain('Ardent');
  expect(qa('pad-warning').length).toBe(1);
  const input = q('search-input') as HTMLInputElement;
  input.value = 'Tritio';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  (q('search-go') as HTMLButtonElement).click();
  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe('/api/actions/trade.search');
  expect(JSON.parse(init.body)).toEqual({ commodity: 'tritium' });
});

test('search loading and no data states', () => {
  mountWith(SearchPanel, { 'trade.search': { commodity: 'gold', status: 'loading', rows: [] } });
  expect(q('search-state')!.textContent).toContain('Consultando');
  unmount(mounted!); mounted = undefined; target.innerHTML = '';
  mountWith(SearchPanel, { 'trade.search': { commodity: 'gold', status: 'no_data', rows: [] } });
  expect(q('search-state')!.textContent).toContain('sin datos');
});

test('rare row does not repeat the distance', () => {
  mountWith(MarketPanel, { 'trade.market': market([mrow({ best_sell: null, profit_per_ton: null, rare: 160 })]) });
  expect(q('market-row')!.textContent!.match(/160/g)!.length).toBe(1);
});

test('route statuses: no_data, idle, loading and completed', () => {
  mountWith(RoutePanel, { 'trade.route': route({ status: 'no_data', plan: null }) });
  expect(q('route-state')!.textContent).toContain('No hay ninguna ruta');
  unmount(mounted!); mounted = undefined; target.innerHTML = '';
  mountWith(RoutePanel, { 'trade.route': route({ status: 'idle', plan: null }) });
  expect(q('route-state')!.textContent).toContain('Recalcular');
  expect((q('route-recalculate') as HTMLButtonElement).disabled).toBe(false);
  unmount(mounted!); mounted = undefined; target.innerHTML = '';
  mountWith(RoutePanel, { 'trade.route': route({ status: 'loading', plan: null }) });
  expect(q('route-state')!.textContent).toContain('Calculando');
  expect((q('route-recalculate') as HTMLButtonElement).disabled).toBe(true);
  unmount(mounted!); mounted = undefined; target.innerHTML = '';
  mountWith(RoutePanel, { 'trade.route': route({ completed: true, current: 3 }) });
  expect(q('route-progress')!.textContent).toContain('completada');
  expect(qa('route-stop').some((s) => s.classList.contains('current'))).toBe(false);
});

test('copied badge appears when copy_seq increments and hides after 5 s', () => {
  vi.useFakeTimers();
  mountWith(RoutePanel, { 'trade.route': route({ copy_seq: 1, copy_target: 'Wolf 359' }) });
  expect(q('route-copied')).toBeNull();
  live.topics['trade.route'] = route({ copy_seq: 2, copy_target: 'Nakamura' });
  flushSync();
  expect(q('route-copied')!.textContent).toContain('Nakamura');
  vi.advanceTimersByTime(5000);
  flushSync();
  expect(q('route-copied')).toBeNull();
});

test('no timers run without data', () => {
  vi.useFakeTimers();
  mountWith(MarketPanel, { 'trade.market': { station: null, system: null, rows: [] } });
  unmount(mounted!); mounted = undefined;
  mountWith(RoutePanel, { 'trade.route': route({ status: 'idle', plan: null }) });
  unmount(mounted!); mounted = undefined;
  mountWith(SearchPanel, { 'trade.search': { commodity: null, status: 'idle', rows: [] } });
  expect(vi.getTimerCount()).toBe(0);
});

test('session without rank or progress hides the rank bar', () => {
  mountWith(TradeSessionPanel, { 'trade.session': session({ rank: null, progress: null }) });
  expect(q('session-rank')).toBeNull();
  expect(q('rank-bar')).toBeNull();
  unmount(mounted!); mounted = undefined; target.innerHTML = '';
  mountWith(TradeSessionPanel, { 'trade.session': session({ rank: 4, progress: null }) });
  expect(q('session-rank')!.textContent).toContain('Merchant');
  expect(q('rank-bar')).toBeNull();
});

test('session reset failure uses the trade message', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({}) }));
  mountWith(TradeSessionPanel, { 'trade.session': session() });
  (q('session-reset') as HTMLButtonElement).click();
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('[role="alert"]')!.textContent).toContain('sesión'); });
});
