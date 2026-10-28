import type { Component } from 'svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import { modules } from '../src/modules/registry';
import { layouts } from '../src/shell/layouts';
import OutfittingSearchPanel from '../src/modules/dashboard/OutfittingSearchPanel.svelte';
import type {
  OutfittingCatalogTopic,
  OutfittingSearchTopic,
} from '../src/modules/dashboard/outfittingTypes';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;
let fetchMock: ReturnType<typeof vi.fn>;

function sampleCatalog(overrides?: Partial<OutfittingCatalogTopic['commander']>): OutfittingCatalogTopic {
  return {
    ships: [
      { name: 'Cobra MkV', symbol: 'cobramkv', pad: 'S' },
      { name: 'Mandalay', symbol: 'mandalay', pad: 'M' },
      { name: 'Python Mk II', symbol: 'python_nx', pad: 'M' },
    ],
    modules: [
      {
        name: 'Frame Shift Drive (SCO)',
        category: 'core_internal',
        has_weapon_mode: false,
        is_armour: false,
      },
      {
        name: 'Multi-Cannon',
        category: 'hardpoint',
        has_weapon_mode: true,
        is_armour: false,
      },
      {
        name: 'Military Grade Composite',
        category: 'armour',
        has_weapon_mode: false,
        is_armour: true,
      },
    ],
    classes: [0, 1, 2, 3, 4, 5, 6, 7, 8],
    ratings: ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I'],
    permit_systems: [
      { system: 'Shinrarta Dezhra', permit_key: 'permit-founders', requires_elite: true },
    ],
    commander: {
      current_system: 'Cubeo',
      current_ship_pad: 'Medium',
      pledged_power: 'Aisling Duval',
      has_elite_rank: false,
      credits: 45_000_000,
      rebuy: 2_500_000,
      ...overrides,
    },
  };
}

function idleSearch(): OutfittingSearchTopic {
  return {
    status: 'idle',
    pad_mode: 'auto',
    exclude_permits: true,
    hidden_permit_count: 0,
    params: null,
    stations: [],
    fallback: [],
    fetched_at: null,
    source: null,
    as_of: null,
  };
}

function mountPanel(topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(OutfittingSearchPanel as Component, { target });
  flushSync();
}

const q = <T extends Element = HTMLElement>(id: string) =>
  target.querySelector<T>(`[data-testid="${id}"]`);

beforeEach(() => {
  disconnect();
  setLocale('es');
  fetchMock = vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ items: [] }) }));
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

describe('OutfittingSearchPanel', () => {
  test('is registered in both Nave module and Nave layout', () => {
    const shipMod = modules.find((m) => m.id === 'ship');
    expect(shipMod).toBeDefined();
    expect(shipMod?.panels).toContain(OutfittingSearchPanel);

    const shipLayout = layouts.find((l) => l.id === 'ship');
    expect(shipLayout).toBeDefined();
    expect(
      shipLayout?.slots.some(
        (s) => s.id === 'ship-outfitting-search' && s.panel === OutfittingSearchPanel && s.span === 2,
      ),
    ).toBe(true);
  });

  test('prefills current system, builds ship + component basket, toggles own-power Stronghold and submits action', async () => {
    mountPanel({
      'ship.outfitting_catalog': sampleCatalog(),
      'ship.outfitting_search': idleSearch(),
    });

    const originInput = q<HTMLInputElement>('outfitting-origin-input')!;
    expect(originInput.value).toBe('Cubeo');

    // Cambiar manualmente y pulsar "Usar actual" restaura Cubeo
    originInput.value = 'Diaguandri';
    originInput.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(originInput.value).toBe('Diaguandri');

    q<HTMLButtonElement>('outfitting-use-current-btn')!.click();
    flushSync();
    expect(originInput.value).toBe('Cubeo');

    // Seleccionar nave Mandalay
    const shipSelect = q<HTMLSelectElement>('outfitting-ship-select')!;
    shipSelect.value = 'Mandalay';
    shipSelect.dispatchEvent(new Event('change', { bubbles: true }));
    flushSync();

    // Seleccionar componente Multi-Cannon 2F Cardán y añadirlo a la cesta
    const modSelect = q<HTMLSelectElement>('outfitting-module-select')!;
    modSelect.value = 'Multi-Cannon';
    modSelect.dispatchEvent(new Event('change', { bubbles: true }));
    flushSync();

    const classSelect = q<HTMLSelectElement>('outfitting-class-select')!;
    classSelect.value = '2';
    classSelect.dispatchEvent(new Event('change', { bubbles: true }));

    const ratingSelect = q<HTMLSelectElement>('outfitting-rating-select')!;
    ratingSelect.value = 'F';
    ratingSelect.dispatchEvent(new Event('change', { bubbles: true }));

    const mountSelect = q<HTMLSelectElement>('outfitting-mount-select')!;
    expect(mountSelect).toBeTruthy();
    mountSelect.value = 'Gimbal';
    mountSelect.dispatchEvent(new Event('change', { bubbles: true }));
    flushSync();

    q<HTMLButtonElement>('outfitting-add-module-btn')!.click();
    flushSync();

    expect(q('outfitting-basket')?.textContent).toContain('Mandalay');
    expect(q('outfitting-basket')?.textContent).toContain('2F Multi-Cannon');

    // Activar Stronghold de la potencia del jugador ("Aisling Duval") y -15 % Li Yong-Rui
    const strongholdBtn = q<HTMLButtonElement>('outfitting-switch-stronghold')!;
    expect(strongholdBtn.disabled).toBe(false);
    expect(strongholdBtn.textContent).toContain('Aisling Duval');
    strongholdBtn.click();

    const lyrBtn = q<HTMLButtonElement>('outfitting-switch-lyr')!;
    lyrBtn.click();
    flushSync();

    // Enviar búsqueda
    q<HTMLButtonElement>('outfitting-submit-btn')!.click();
    flushSync();

    const actionCall = fetchMock.mock.calls.find((c) =>
      String(c[0]).endsWith('/api/actions/ship.outfitting_search'),
    );
    expect(actionCall).toBeDefined();
    const payload = JSON.parse(String((actionCall![1] as RequestInit).body));
    expect(payload).toMatchObject({
      reference_system: 'Cubeo',
      ship: 'Mandalay',
      modules: [
        {
          name: 'Multi-Cannon',
          class: 2,
          rating: 'F',
          weapon_mode: 'Gimbal',
        },
      ],
      only_lyr_discount: true,
      include_own_stronghold: true,
      exclude_permits: true,
    });
  });

  test('disables Stronghold switch when commander is not pledged to any Powerplay faction', () => {
    mountPanel({
      'ship.outfitting_catalog': sampleCatalog({ pledged_power: null }),
      'ship.outfitting_search': idleSearch(),
    });

    const strongholdBtn = q<HTMLButtonElement>('outfitting-switch-stronghold')!;
    expect(strongholdBtn.disabled).toBe(true);
  });

  test('renders station hits, badges, rebuy semaphore, itemized breakdown, copy and route actions', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });

    mountPanel({
      'ship.outfitting_catalog': sampleCatalog(),
      'ship.outfitting_search': {
        status: 'ok',
        pad_mode: 'auto',
        exclude_permits: false,
        hidden_permit_count: 1,
        params: {
          reference_system: 'Cubeo',
          ship: 'Mandalay',
          modules: [{ name: 'Frame Shift Drive (SCO)', class: 5, rating: 'A' }],
        },
        stations: [
          {
            station_name: 'Ray Gateway',
            system_name: 'Diaguandri',
            station_type: 'Coriolis Starport',
            market_id: 128004608,
            distance_ly: 42.18,
            distance_to_arrival_ls: 392,
            max_pad: 'Large',
            is_planetary: false,
            is_carrier: false,
            is_stronghold_carrier: false,
            controlling_power: 'Li Yong-Rui',
            discount_tag: 'lyr_15',
            updated_at: '2026-10-28T12:00:00Z',
            item_prices: [
              { label: 'Mandalay', price_cr: 14_790_000 },
              { label: '5A Frame Shift Drive (SCO)', price_cr: 5_102_350 },
            ],
            total_price_cr: 19_892_350,
            permit_key: null,
            permit_requires_elite: false,
            affordability: 'ok',
          },
          {
            station_name: 'Stronghold Carrier V9X-11Z',
            system_name: 'Cubeo',
            station_type: 'Stronghold Carrier',
            market_id: 371000111,
            distance_ly: 0.0,
            distance_to_arrival_ls: 85,
            max_pad: 'Large',
            is_planetary: false,
            is_carrier: false,
            is_stronghold_carrier: true,
            controlling_power: 'Aisling Duval',
            discount_tag: null,
            updated_at: '2026-10-28T13:00:00Z',
            item_prices: [
              { label: 'Mandalay', price_cr: 17_400_000 },
              { label: '5A Frame Shift Drive (SCO)', price_cr: 6_002_760 },
            ],
            total_price_cr: 43_000_000,
            permit_key: null,
            permit_requires_elite: false,
            affordability: 'tight_rebuy',
          },
        ],
        fallback: [],
        fetched_at: '2026-10-28T14:00:00Z',
        source: 'Spansh',
        as_of: '2026-10-28T14:00:00Z',
      } satisfies OutfittingSearchTopic,
    });

    expect(q('outfitting-results-list')?.textContent).toContain('Ray Gateway');
    expect(q('outfitting-results-list')?.textContent).toContain('−15 % LYR');
    expect(q('outfitting-results-list')?.textContent).toContain('Recompra OK');
    expect(q('outfitting-results-list')?.textContent).toContain('Stronghold');
    expect(q('outfitting-results-list')?.textContent).toContain('Ajusta recompra');
    expect(q('outfitting-hidden-permits')?.textContent).toContain('1');

    // Desplegar desglose de precios de la primera estación
    q<HTMLButtonElement>('outfitting-breakdown-btn-0')!.click();
    flushSync();
    expect(q('outfitting-breakdown-0')?.textContent).toContain('5A Frame Shift Drive (SCO)');

    // Copiar sistema Diaguandri
    q<HTMLButtonElement>('outfitting-copy-btn-0')!.click();
    flushSync();
    expect(writeText).toHaveBeenCalledWith('Diaguandri');

    // Trazar ruta hacia Diaguandri
    q<HTMLButtonElement>('outfitting-route-btn-0')!.click();
    flushSync();
    const routeCall = fetchMock.mock.calls.find((c) =>
      String(c[0]).endsWith('/api/actions/navigation.plan'),
    );
    expect(routeCall).toBeDefined();
    expect(JSON.parse(String((routeCall![1] as RequestInit).body))).toMatchObject({
      to: 'Diaguandri',
      mode: 'galaxy',
      use_ship: true,
    });
  });

  test('renders pedagogical per-item fallback card when no single station stocks the full combination', () => {
    mountPanel({
      'ship.outfitting_catalog': sampleCatalog(),
      'ship.outfitting_search': {
        status: 'fallback',
        pad_mode: 'auto',
        exclude_permits: true,
        hidden_permit_count: 0,
        params: {
          reference_system: 'Cubeo',
          ship: 'Caspian Explorer',
          modules: [{ name: 'Prismatic Shield Generator', class: 6, rating: 'A' }],
        },
        stations: [],
        fallback: [
          {
            item_label: 'Caspian Explorer',
            nearest_station: {
              station_name: 'Medupe City',
              system_name: 'Cubeo',
              station_type: 'Orbis Starport',
              market_id: 1,
              distance_ly: 0.0,
              distance_to_arrival_ls: 451,
              max_pad: 'Large',
              is_planetary: false,
              is_carrier: false,
              is_stronghold_carrier: false,
              controlling_power: 'Aisling Duval',
              discount_tag: null,
              updated_at: '2026-10-28T13:00:00Z',
              item_prices: [{ label: 'Caspian Explorer', price_cr: 98_500_000 }],
              total_price_cr: 98_500_000,
              permit_key: null,
              permit_requires_elite: false,
              affordability: 'insufficient',
            },
          },
          {
            item_label: '6A Prismatic Shield Generator',
            nearest_station: null,
          },
        ],
        fetched_at: '2026-10-28T14:00:00Z',
        source: 'Spansh',
        as_of: '2026-10-28T14:00:00Z',
      } satisfies OutfittingSearchTopic,
    });

    const fallbackBox = q('outfitting-fallback-card')!;
    expect(fallbackBox).toBeTruthy();
    expect(fallbackBox.textContent).toContain('Caspian Explorer');
    expect(fallbackBox.textContent).toContain('Medupe City');
    expect(fallbackBox.textContent).toContain('6A Prismatic Shield Generator');
    expect(fallbackBox.textContent).toContain('Sin stock conocido');
  });
});
