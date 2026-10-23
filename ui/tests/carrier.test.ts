import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import CarrierOverviewPanel from '../src/modules/carrier/CarrierOverviewPanel.svelte';
import CarrierJumpPanel from '../src/modules/carrier/CarrierJumpPanel.svelte';
import CarrierFinancesPanel from '../src/modules/carrier/CarrierFinancesPanel.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}

const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);

beforeEach(() => {
  disconnect();
  setLocale('es');
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined;
  target.remove();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

test('CarrierOverviewPanel shows empty state when no carrier', () => {
  mountWith(CarrierOverviewPanel, { 'carrier.overview': null });
  expect(q('carrier-no-data')).toBeTruthy();
});

test('CarrierOverviewPanel renders callsign, system, fuel and aboard badge', () => {
  mountWith(CarrierOverviewPanel, {
    'carrier.overview': {
      carrier_id: 1234567,
      callsign: 'Q4Z-99L',
      name: 'Nautilus One',
      current_system: 'Sol',
      current_body: 'Earth',
      docking_access: 'All',
      fuel_depot: 850,
      fuel_cargo: 5000,
      fuel_total: 5850,
      max_jumps_possible: 58,
      cargo_free: 15000,
      cargo_total: 25000,
      cargo_used: 10000,
      is_aboard: true,
    },
  });

  expect(q('carrier-callsign')?.textContent).toBe('Q4Z-99L');
  expect(q('carrier-name')?.textContent).toBe('Nautilus One');
  expect(q('carrier-location')?.textContent).toBe('Sol');
  expect(q('carrier-jumps')?.textContent).toBe('58');
  expect(q('carrier-aboard')).toBeTruthy();
  expect(q('carrier-fuel-total')?.textContent).toContain('5.850');
  expect(q('carrier-cargo-total')?.textContent).toContain('25.000');
});

test('CarrierJumpPanel renders countdown and destination', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { clipboard: { writeText } });

  mountWith(CarrierJumpPanel, {
    'carrier.jump': {
      status: 'counting_down',
      destination_system: 'Colonia',
      destination_body: 'Colonia 4',
      departure_time: '2026-10-23T12:15:00Z',
      seconds_remaining: 765, // 12m 45s
      cooldown_seconds_remaining: 0,
      is_cancelled: false,
    },
  });

  expect(q('jump-status-badge')?.textContent).toContain('Cuenta atrás');
  expect(q('jump-timer')?.textContent).toBe('12:45');
  expect(q('jump-dest-system')?.textContent).toBe('Colonia');
  expect(q('jump-dest-body')?.textContent).toBe('Colonia 4');

  const btn = q('jump-copy-btn') as HTMLButtonElement;
  expect(btn).toBeTruthy();
  btn.click();
  flushSync();
  expect(writeText).toHaveBeenCalledWith('Colonia');
});

test('CarrierJumpPanel renders cooldown state', () => {
  mountWith(CarrierJumpPanel, {
    'carrier.jump': {
      status: 'cooldown',
      destination_system: null,
      destination_body: null,
      departure_time: null,
      seconds_remaining: null,
      cooldown_seconds_remaining: 185, // 03:05
      is_cancelled: false,
    },
  });

  expect(q('jump-status-badge')?.textContent).toContain('Enfriamiento');
  expect(q('cooldown-timer')?.textContent).toBe('03:05');
});

test('CarrierFinancesPanel renders health badge, weeks covered and upkeep', () => {
  mountWith(CarrierFinancesPanel, {
    'carrier.finances': {
      balance: 1500000000,
      reserve_balance: 200000000,
      available_balance: 1300000000,
      weekly_upkeep: 25000000,
      maintenance_health: 'Safe',
      weeks_covered: 60,
      next_payment_time: '2026-10-29T07:00:00Z',
    },
  });

  expect(q('carrier-health-badge')?.textContent).toContain('Saludable');
  expect(q('weeks-covered-val')?.textContent?.trim()).toBe('60');
  expect(q('carrier-balance')?.textContent).toContain('1.500.000.000');
  expect(q('carrier-upkeep')?.textContent).toContain('25.000.000');
});
