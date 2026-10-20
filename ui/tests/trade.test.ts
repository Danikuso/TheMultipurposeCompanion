import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import MarketPanel from '../src/modules/trade/MarketPanel.svelte';
import CargoPanel from '../src/modules/trade/CargoPanel.svelte';

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
  expect(row.textContent).toContain('L');
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
