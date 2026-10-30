import type { Component } from 'svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import { modules } from '../src/modules/registry';
import { layouts } from '../src/shell/layouts';
import MiningTargetsPanel from '../src/modules/mining/MiningTargetsPanel.svelte';
import type {
  MiningTargetsTopic,
  MiningLoopTopic,
  MiningCargoSellTopic,
} from '../src/modules/mining/miningTargetsTypes';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;
let fetchMock: ReturnType<typeof vi.fn>;

function sampleTargetsTopic(): MiningTargetsTopic {
  return {
    status: 'ok',
    commodity: 'Platinum',
    query: {
      reference_system: 'Sol',
      reference_coords: [0, 0, 0],
      commodity: 'Platinum',
      min_hotspots: 1,
      require_res_combo: true,
      populated_only: false,
      min_pad: 'Large',
    },
    targets: [
      {
        system_name: 'Delkar',
        distance_ly: 83.1,
        body_name: 'Delkar 7',
        ring_name: 'Delkar 7 A Ring',
        ring_type: 'Metallic',
        arrival_distance_ls: 2802,
        hotspot_count: 2,
        res_intensity: 'High',
        comment: 'Doble Platino + HIGH-RES',
        other_hotspots: ['Painite:2'],
      },
      {
        system_name: 'Paesia',
        distance_ly: 112.5,
        body_name: 'Paesia 2',
        ring_name: 'Paesia 2 A Ring',
        ring_type: 'Metallic',
        arrival_distance_ls: 821,
        hotspot_count: 1,
        res_intensity: 'Hazardous',
        comment: 'Combo Plat + HAZ-RES (+100 % fragmentos)',
        other_hotspots: [],
      },
    ],
  };
}

function sampleLoopTopic(): MiningLoopTopic {
  return {
    status: 'ok',
    plan: {
      origin_system: 'Sol',
      ring_target: {
        system_name: 'Delkar',
        distance_ly: 83.1,
        body_name: 'Delkar 7',
        ring_name: 'Delkar 7 A Ring',
        ring_type: 'Metallic',
        arrival_distance_ls: 2802,
        hotspot_count: 2,
        res_intensity: 'High',
        comment: 'Doble Platino + HIGH-RES',
        other_hotspots: [],
      },
      sell_station: {
        system_name: 'Tofana',
        station_name: 'Matthews Dock',
        station_type: 'Coriolis',
        distance_from_ring_ly: 14.2,
        distance_from_ref_ly: 75.1,
        arrival_distance_ls: 340,
        pad: 'Large',
        price_cr: 285_400,
        demand: 100,
        demand_tax_risk: true,
        updated_at: '2026-10-30T10:00:00Z',
      },
      total_distance_ly: 97.3,
    },
  };
}

function sampleCargoSellTopic(): MiningCargoSellTopic {
  return {
    status: 'ok',
    commodities_in_cargo: [
      { name: 'Platinum', count: 64 },
      { name: 'Osmium', count: 16 },
    ],
    best_multi_stations: [
      {
        station_name: 'Matthews Dock',
        system_name: 'Tofana',
        distance_ly: 14.2,
        pad: 'Large',
        item_prices: [
          { label: 'Platinum', price_cr: 285_400 },
          { label: 'Osmium', price_cr: 140_000 },
        ],
        total_revenue_cr: 20_505_600,
        updated_at: '2026-10-30T10:00:00Z',
      },
    ],
  };
}

function mountPanel(topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(MiningTargetsPanel as Component, { target });
  flushSync();
}

beforeEach(() => {
  disconnect();
  setLocale('es');
  fetchMock = vi.fn(async () => ({ ok: true, status: 200, json: async () => ({}) }));
  vi.stubGlobal('fetch', fetchMock);
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

describe('MiningTargetsPanel', () => {
  test('is registered in both Minería module and Minería layout', () => {
    const miningMod = modules.find((m) => m.id === 'mining');
    expect(miningMod).toBeDefined();
    expect(miningMod?.panels).toContain(MiningTargetsPanel);

    const miningLayout = layouts.find((l) => l.id === 'mining');
    expect(miningLayout).toBeDefined();
    expect(
      miningLayout?.slots.some(
        (s) => s.id === 'mining-targets' && s.panel === MiningTargetsPanel && s.span === 2,
      ),
    ).toBe(true);
  });

  test('renders profile tabs, preset chips, and triangular loop plan with demand tax warning', () => {
    mountPanel({
      'mining.targets': sampleTargetsTopic(),
      'mining.loop': sampleLoopTopic(),
      'mining.cargo_sell': sampleCargoSellTopic(),
      'dashboard.location': { system: 'Sol' },
    });

    // Pestañas
    const tabs = target.querySelectorAll('.tab-btn');
    expect(tabs.length).toBe(3);

    // Tarjeta del bucle
    const loopCard = target.querySelector('[data-testid="loop-plan-card"]');
    expect(loopCard).not.toBeNull();
    expect(loopCard?.textContent).toContain('Sol');
    expect(loopCard?.textContent).toContain('Delkar');
    expect(loopCard?.textContent).toContain('Matthews Dock');
    expect(loopCard?.textContent).toMatch(/97[.,]3 al/);
    expect(loopCard?.textContent).toContain('penalización demanda');

    // Tabla de objetivos
    const table = target.querySelector('[data-testid="mining-targets-table"]');
    expect(table).not.toBeNull();
    expect(table?.textContent).toContain('Delkar');
    expect(table?.textContent).toContain('Paesia');
    expect(table?.textContent).toMatch(/83[.,]1 al/);
    expect(table?.textContent).toContain('High-RES');
    expect(table?.textContent).toContain('Haz-RES');
  });

  test('switches tabs between Laser, Core and Missions', async () => {
    mountPanel({
      'mining.targets': sampleTargetsTopic(),
      'mining.loop': sampleLoopTopic(),
      'dashboard.location': { system: 'Sol' },
    });

    const coreTab = Array.from(target.querySelectorAll('.tab-btn')).find((b) =>
      b.textContent?.includes('Núcleo Profundo'),
    ) as HTMLButtonElement;
    expect(coreTab).toBeDefined();

    coreTab.click();
    flushSync();

    // Ahora deben verse los chips de núcleo (Musgravite, Monazite, etc.)
    const chips = Array.from(target.querySelectorAll('.preset-name')).map((c) => c.textContent);
    expect(chips).toContain('Musgravite');
    expect(chips).toContain('Monazite');
    expect(chips).toContain('Alexandrite');
  });

  test('renders cargo minerals strip and multi-sell section', () => {
    mountPanel({
      'mining.targets': sampleTargetsTopic(),
      'mining.loop': sampleLoopTopic(),
      'mining.cargo_sell': sampleCargoSellTopic(),
    });

    const strip = target.querySelector('.cargo-sell-strip');
    expect(strip).not.toBeNull();
    expect(strip?.textContent).toContain('Platinum: 64 t');
    expect(strip?.textContent).toContain('Osmium: 16 t');

    const multiSell = target.querySelector('.multi-sell-section');
    expect(multiSell).not.toBeNull();
    expect(multiSell?.textContent).toContain('Matthews Dock');
    expect(multiSell?.textContent).toContain('Tofana');
  });
});
