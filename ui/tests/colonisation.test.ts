import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import ColonisationProgressPanel from '../src/modules/colonisation/ColonisationProgressPanel.svelte';
import ColonisationSupplyPanel from '../src/modules/colonisation/ColonisationSupplyPanel.svelte';

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

test('ColonisationProgressPanel shows empty state when no active project', () => {
  mountWith(ColonisationProgressPanel, { 'colonisation.depot': null });
  expect(q('depot-no-data')).toBeTruthy();
});

test('ColonisationProgressPanel renders depot progress, metrics and resource rows', () => {
  mountWith(ColonisationProgressPanel, {
    'colonisation.depot': {
      active_project: true,
      market_id: 3224567,
      station_name: 'Outpost Orion',
      construction_type: 'Orbital Station Stage 1',
      progress_percent: 65.0,
      total_required: 1000,
      total_provided: 650,
      total_remaining: 350,
      cargo_capacity: 752,
      total_trips_remaining: 1,
      is_docked: true,
      resources: [
        {
          name: 'Steel',
          name_localised: 'Acero',
          required: 500,
          provided: 500,
          remaining: 0,
          progress_percent: 100.0,
          trips_remaining: 0,
        },
        {
          name: 'Titanium',
          name_localised: 'Titanio',
          required: 500,
          provided: 150,
          remaining: 350,
          progress_percent: 30.0,
          trips_remaining: 1,
        },
      ],
    },
  });

  expect(q('depot-station-name')?.textContent).toBe('Outpost Orion');
  expect(q('depot-docked')).toBeTruthy();
  expect(q('depot-progress-val')?.textContent).toContain('65');
  expect(q('depot-provided')?.textContent).toContain('650');
  expect(q('depot-remaining')?.textContent).toContain('350');
  expect(q('depot-trips')?.textContent).toContain('~1');

  const rows = qa('resource-row');
  expect(rows.length).toBe(2);
  expect(rows[0].textContent).toContain('Acero');
  expect(rows[1].textContent).toContain('Titanio');
});

test('ColonisationSupplyPanel renders pending supplies and allows copying', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { clipboard: { writeText } });

  mountWith(ColonisationSupplyPanel, {
    'colonisation.depot': {
      active_project: true,
      market_id: 3224567,
      station_name: 'Outpost Orion',
      construction_type: 'Orbital Station Stage 1',
      progress_percent: 65.0,
      total_required: 1000,
      total_provided: 650,
      total_remaining: 350,
      cargo_capacity: 752,
      total_trips_remaining: 1,
      is_docked: true,
      resources: [
        {
          name: 'Steel',
          name_localised: 'Acero',
          required: 500,
          provided: 500,
          remaining: 0,
          progress_percent: 100.0,
          trips_remaining: 0,
        },
        {
          name: 'Titanium',
          name_localised: 'Titanio',
          required: 500,
          provided: 150,
          remaining: 350,
          progress_percent: 30.0,
          trips_remaining: 1,
        },
      ],
    },
  });

  const cards = qa('supply-card');
  expect(cards.length).toBe(1); // Only Titanium is pending
  expect(q('supply-name')?.textContent?.trim()).toBe('Titanio');

  const btn = q('copy-resource-btn') as HTMLButtonElement;
  expect(btn).toBeTruthy();
  btn.click();
  flushSync();
  expect(writeText).toHaveBeenCalledWith('Titanio');
});

test('ColonisationSupplyPanel shows completion card when all resources provided', () => {
  mountWith(ColonisationSupplyPanel, {
    'colonisation.depot': {
      active_project: true,
      market_id: 3224567,
      station_name: 'Outpost Orion',
      construction_type: 'Orbital Station Stage 1',
      progress_percent: 100.0,
      total_required: 1000,
      total_provided: 1000,
      total_remaining: 0,
      cargo_capacity: 752,
      total_trips_remaining: 0,
      is_docked: true,
      resources: [
        {
          name: 'Steel',
          name_localised: 'Acero',
          required: 1000,
          provided: 1000,
          remaining: 0,
          progress_percent: 100.0,
          trips_remaining: 0,
        },
      ],
    },
  });

  expect(q('supply-all-complete')).toBeTruthy();
});

test('ColonisationSupplyPanel triggers trade.search on pending supply and shows nearby market results', async () => {
  const fetchMock = vi.fn().mockResolvedValue(new Response('{}'));
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('fetch', fetchMock);
  vi.stubGlobal('navigator', { clipboard: { writeText } });

  mountWith(ColonisationSupplyPanel, {
    'colonisation.depot': {
      active_project: true,
      market_id: 3224567,
      station_name: 'Outpost Orion',
      construction_type: 'Orbital Station Stage 1',
      progress_percent: 40.0,
      total_required: 1000,
      total_provided: 400,
      total_remaining: 600,
      cargo_capacity: 752,
      total_trips_remaining: 1,
      is_docked: true,
      resources: [
        {
          name: '$titanium_name;',
          name_localised: 'Titanio',
          required: 1000,
          provided: 400,
          remaining: 600,
          progress_percent: 40.0,
          trips_remaining: 1,
        },
      ],
    },
    'trade.search': {
      commodity: 'titanium',
      status: 'ok',
      rows: [
        {
          station_name: 'Ray Gateway',
          system_name: 'Diaguandri',
          distance_ly: 14.2,
          buy_price: 4200,
          stock: 8500,
          pad: 'Large',
          pad_ok: true,
          updated_at: null,
          origin: 'Eddn',
        },
      ],
    },
  });

  const findBtn = q('find-supply-market-btn') as HTMLButtonElement | null;
  expect(findBtn).not.toBeNull();
  findBtn!.click();
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
  expect(fetchMock).toHaveBeenCalledWith(
    '/api/actions/trade.search',
    expect.objectContaining({ body: JSON.stringify({ commodity: 'titanium' }) }),
  );

  const results = q('colonisation-market-results');
  expect(results).not.toBeNull();
  expect(results!.textContent).toContain('Ray Gateway');
  expect(results!.textContent).toContain('Diaguandri');

  const copySysBtn = q('copy-market-system-btn') as HTMLButtonElement | null;
  expect(copySysBtn).not.toBeNull();
  copySysBtn!.click();
  await new Promise((r) => setTimeout(r, 0));
  expect(writeText).toHaveBeenCalledWith('Diaguandri');
});

