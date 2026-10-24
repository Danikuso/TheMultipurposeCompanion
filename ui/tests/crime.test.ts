import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import CrimePanel from '../src/modules/crime/CrimePanel.svelte';

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

test('CrimePanel renders clean state correctly', () => {
  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'Clean',
      notoriety: 0,
      notoriety_decay_seconds: 0,
      total_fines: 0,
      total_bounties: 0,
      factions: [],
      stolen_items: [],
      nearest_interstellar_factor: null,
      nearest_black_market: null,
      system_security: 'high',
      police_response_estimate_seconds: 20,
    },
  });

  expect(q('crime-panel')).toBeTruthy();
  expect(q('legal-state-badge')?.textContent?.trim()).toBe('Limpio');
  expect(q('notoriety-level')?.textContent).toContain('Limpia (0)');
  expect(q('guide-clean')).toBeTruthy();
  expect(q('guide-blocked')).toBeNull();
  expect(q('guide-ready')).toBeNull();
  expect(q('security-badge')?.textContent?.trim()).toBe('Alta Seguridad');
  expect(q('police-response')?.textContent).toContain('inmediata');
});

test('CrimePanel renders blocked guide when notoriety > 0 and shows decay timer', () => {
  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'Wanted',
      notoriety: 3,
      notoriety_decay_seconds: 21600, // 6h
      total_fines: 1000,
      total_bounties: 50000,
      factions: [],
      stolen_items: [],
      nearest_interstellar_factor: null,
      nearest_black_market: null,
      system_security: 'medium',
      police_response_estimate_seconds: 60,
    },
    'core.location': { mode: 'Supercruise' },
  });

  expect(q('legal-state-badge')?.textContent?.trim()).toBe('En Búsqueda');
  expect(q('notoriety-level')?.textContent).toContain('Nivel 3 / 10');
  expect(q('guide-blocked')).toBeTruthy();
  expect(q('guide-ready')).toBeNull();
  expect(q('decay-timer')?.textContent).toContain('6h 0m');
});

test('CrimePanel indicates decay paused when docked in station', () => {
  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'Wanted',
      notoriety: 2,
      notoriety_decay_seconds: 14400,
      total_fines: 500,
      total_bounties: 20000,
      factions: [],
      stolen_items: [],
      nearest_interstellar_factor: null,
      nearest_black_market: null,
      system_security: 'high',
      police_response_estimate_seconds: 20,
    },
    'core.location': { mode: 'Docked' },
  });

  expect(q('decay-timer')?.textContent).toContain('pausado');
});

test('CrimePanel renders ready guide with nearest Interstellar Factor when notoriety 0 and bounties exist', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { clipboard: { writeText } });

  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'Wanted',
      notoriety: 0,
      notoriety_decay_seconds: 0,
      total_fines: 500,
      total_bounties: 25000,
      factions: [],
      stolen_items: [],
      nearest_interstellar_factor: {
        station_name: 'Baird Gateway',
        system_name: 'LTT 1873',
        distance_ly: 8.42,
        pad_size: 'L',
        is_planetary: false,
      },
      nearest_black_market: null,
      system_security: 'low',
      police_response_estimate_seconds: 180,
    },
  });

  expect(q('guide-ready')).toBeTruthy();
  expect(q('nearest-factor-card')?.textContent).toContain('Baird Gateway');
  expect(q('nearest-factor-card')?.textContent).toContain('LTT 1873');

  const btn = q('copy-interstellar-factor-btn') as HTMLButtonElement;
  expect(btn).toBeTruthy();
  btn.click();
  flushSync();

  expect(writeText).toHaveBeenCalledWith('LTT 1873');
});

test('CrimePanel renders stolen cargo list, nearest Black Market with copy button and stealth advice', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { clipboard: { writeText } });

  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'IllegalCargo',
      notoriety: 0,
      notoriety_decay_seconds: 0,
      total_fines: 0,
      total_bounties: 0,
      factions: [],
      stolen_items: [
        {
          name: 'gold',
          name_localised: 'Oro',
          count: 12,
          stolen: 6,
          is_mission: false,
        },
      ],
      nearest_interstellar_factor: null,
      nearest_black_market: {
        station_name: 'Black Hide',
        system_name: 'Wyrd',
        distance_ly: 12.5,
        pad_size: 'M',
        is_planetary: true,
      },
      system_security: 'medium',
      police_response_estimate_seconds: 60,
    },
  });

  expect(q('legal-state-badge')?.textContent?.trim()).toBe('Carga Ilícita');
  expect(q('stolen-items-list')?.textContent).toContain('Oro');
  expect(q('stolen-items-list')?.textContent).toContain('6 unidades');
  expect(q('nearest-market-card')?.textContent).toContain('Black Hide');

  const btn = q('copy-black-market-btn') as HTMLButtonElement;
  expect(btn).toBeTruthy();
  btn.click();
  flushSync();

  expect(writeText).toHaveBeenCalledWith('Wyrd');
});

test('CrimePanel renders anarchy system security and police response', () => {
  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'Clean',
      notoriety: 0,
      notoriety_decay_seconds: 0,
      total_fines: 0,
      total_bounties: 0,
      factions: [],
      stolen_items: [],
      nearest_interstellar_factor: null,
      nearest_black_market: null,
      system_security: 'anarchy',
      police_response_estimate_seconds: null,
    },
  });

  expect(q('security-badge')?.textContent?.trim()).toBe('Anarquía (Sin Ley)');
  expect(q('police-response')?.textContent).toContain('Sin presencia policial');
});

test('CrimePanel renders factions fines and bounties list', () => {
  mountWith(CrimePanel, {
    'crime.status': {
      legal_state: 'Wanted',
      notoriety: 0,
      notoriety_decay_seconds: 0,
      total_fines: 1500,
      total_bounties: 10000,
      factions: [
        {
          faction: 'Sirius Corp',
          system: 'Procyon',
          fines: 1500,
          bounties: 0,
        },
        {
          faction: 'Federal Security',
          fines: 0,
          bounties: 10000,
        },
      ],
      stolen_items: [],
      nearest_interstellar_factor: null,
      nearest_black_market: null,
      system_security: 'high',
      police_response_estimate_seconds: 20,
    },
  });

  expect(q('total-fines')?.textContent).toContain('1.500');
  expect(q('total-bounties')?.textContent).toContain('10.000');
  expect(q('factions-list')?.textContent).toContain('Sirius Corp');
  expect(q('factions-list')?.textContent).toContain('Federal Security');
});
