import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount, type Component } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale, t } from '../src/lib/i18n';
import CmdrPanel from '../src/modules/dashboard/CmdrPanel.svelte';
import ShipPanel from '../src/modules/dashboard/ShipPanel.svelte';
import LocationPanel from '../src/modules/dashboard/LocationPanel.svelte';
import StatusPanel from '../src/modules/dashboard/StatusPanel.svelte';
import RoutePanel from '../src/modules/dashboard/RoutePanel.svelte';
import SessionPanel from '../src/modules/dashboard/SessionPanel.svelte';
import AlertsPanel from '../src/modules/dashboard/AlertsPanel.svelte';
import Settings from '../src/settings/Settings.svelte';

describe('dashboard panels', () => {
  let target: HTMLElement;
  let mounted: Record<string, unknown>[] = [];

  function renderPanel(Comp: Component) {
    const inst = mount(Comp, { target });
    flushSync();
    mounted.push(inst);
    return inst;
  }

  beforeEach(() => {
    disconnect();
    setLocale('es');
    target = document.createElement('div');
    document.body.appendChild(target);
    mounted = [];
  });

  afterEach(() => {
    for (const inst of mounted) {
      unmount(inst);
    }
    target.remove();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  test('ship panel shows fuel bar and hull percent', () => {
    live.topics['core.ship'] = {
      ship_type: 'python',
      ship_type_localised: 'Python',
      name: 'TEST HAULER',
      ident: 'TS-01A',
      hull_health: 0.87,
      fuel_main: 16.0,
      fuel_reservoir: 0.5,
      fuel_capacity_main: 32.0,
      fuel_capacity_reserve: 1.0,
      cargo_count: 12,
      cargo_capacity: 64,
      rebuy: 500000,
      max_jump_range: 18.25,
      damaged_modules: [],
    };

    renderPanel(ShipPanel);
    expect(target.textContent).toContain('87 %');
    const fuelFill = target.querySelector('[data-testid="fuel-main-fill"]') as HTMLElement | null;
    expect(fuelFill).not.toBeNull();
    expect(fuelFill?.style.width).toBe('50%');
  });

  test('settings save online services, map threshold and efficiency', async () => {
    const config = { language: 'es', auto_copy_next_system: false, online_services: true,
      map_value_threshold: 600000, default_neutron_efficiency: 65,
      server: { lan: false, port: 47320, lan_token: 'synthetic-token' }, alerts_enabled: false };
    const fetchMock = vi.fn().mockImplementation(async (_url, options) => ({ ok: true,
      json: async () => options?.method === 'PUT' ? JSON.parse(options.body) : config }));
    vi.stubGlobal('fetch', fetchMock);
    renderPanel(Settings);
    await vi.waitFor(() => {
      flushSync();
      expect((target.querySelector('[data-testid="map-threshold"]') as HTMLInputElement)?.value).toBe('600000');
    });
    await Promise.resolve(); flushSync();
    const online = target.querySelector('[data-testid="online-services"]') as HTMLInputElement;
    const threshold = target.querySelector('[data-testid="map-threshold"]') as HTMLInputElement;
    const efficiency = target.querySelector('[data-testid="planner-efficiency"]') as HTMLInputElement;
    expect(online.checked).toBe(true);
    expect(threshold.value).toBe('600000');
    expect(efficiency.value).toBe('65');
    online.click();
    threshold.value = '750000'; threshold.dispatchEvent(new Event('input', { bubbles: true }));
    threshold.dispatchEvent(new Event('change', { bubbles: true }));
    efficiency.value = '75'; efficiency.dispatchEvent(new Event('input', { bubbles: true }));
    efficiency.dispatchEvent(new Event('change', { bubbles: true }));
    await vi.waitFor(() => {
      const requests = fetchMock.mock.calls.filter(([, options]) => options?.method === 'PUT');
      expect(requests.length).toBeGreaterThan(0);
      expect(JSON.parse(requests.at(-1)![1].body)).toMatchObject({ ...config, online_services: false,
        map_value_threshold: 750000, default_neutron_efficiency: 75 });
    });
  });

  test('settings accept efficiency zero and one hundred and reject invalid numbers', async () => {
    const config = { language: 'es', online_services: true, map_value_threshold: 500000,
      default_neutron_efficiency: 60, server: { lan: false } };
    const fetchMock = vi.fn().mockImplementation(async (_url, options) => ({ ok: true,
      json: async () => options?.method === 'PUT' ? JSON.parse(options.body) : config }));
    vi.stubGlobal('fetch', fetchMock);
    renderPanel(Settings);
    await vi.waitFor(() => expect(target.querySelector('[data-testid="planner-efficiency"]')).not.toBeNull());
    await Promise.resolve(); flushSync();
    const efficiency = target.querySelector('[data-testid="planner-efficiency"]') as HTMLInputElement;
    const threshold = target.querySelector('[data-testid="map-threshold"]') as HTMLInputElement;
    const change = (input: HTMLInputElement, value: string) => {
      input.value = value;
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
    };
    const puts = () => fetchMock.mock.calls.filter(([, options]) => options?.method === 'PUT');
    for (const value of ['0', '100']) {
      change(efficiency, value);
      await vi.waitFor(() => expect(JSON.parse(puts().at(-1)![1].body).default_neutron_efficiency).toBe(Number(value)));
    }
    const count = puts().length;
    for (const value of ['101', '-1', '', '1.5']) change(efficiency, value);
    expect(puts()).toHaveLength(count);
    efficiency.value = '60'; efficiency.dispatchEvent(new Event('input', { bubbles: true }));
    for (const value of ['-1', '', '0.5']) change(threshold, value);
    expect(puts()).toHaveLength(count);
  });

  test.each(['http', 'network'])('failed settings save rolls back opt out and shows an error (%s)', async (failure) => {
    const config = { language: 'es', online_services: true, map_value_threshold: 600000,
      default_neutron_efficiency: 65, server: { lan: false } };
    const fetchMock = vi.fn().mockImplementation(async (_url, options) => {
      if (options?.method === 'PUT') {
        if (failure === 'network') throw new Error('offline');
        return { ok: false, status: 500 };
      }
      return { ok: true, json: async () => config };
    });
    vi.stubGlobal('fetch', fetchMock);
    renderPanel(Settings);
    await vi.waitFor(() => { flushSync(); expect((target.querySelector('[data-testid="map-threshold"]') as HTMLInputElement)?.value).toBe('600000'); });
    const online = target.querySelector('[data-testid="online-services"]') as HTMLInputElement;
    online.click();
    await vi.waitFor(() => {
      flushSync();
      expect(online.checked).toBe(true);
      expect(target.querySelector('[role="alert"]')?.textContent).toBe(t('settings-save-error'));
    });
    expect((target.querySelector('[data-testid="map-threshold"]') as HTMLInputElement).value).toBe('600000');
    expect((target.querySelector('[data-testid="planner-efficiency"]') as HTMLInputElement).value).toBe('65');
  });

  test('route panel copy button calls action', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ system: 'Exampla' }),
    });
    vi.stubGlobal('fetch', fetchMock);

    live.topics['core.route'] = {
      next_system: 'Exampla',
      remaining_jumps: 2,
      hops: [
        { system: 'Testia', star_class: 'G' },
        { system: 'Exampla', star_class: 'K' },
        { system: 'Sampleton', star_class: 'M' },
      ],
    };

    renderPanel(RoutePanel);
    const btn = target.querySelector('[data-testid="copy-next-system"]') as HTMLButtonElement | null;
    expect(btn).not.toBeNull();
    btn!.click();
    await Promise.resolve();

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/actions/clipboard.copy-next-system',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  test('panels show empty state without data', () => {
    for (const Comp of [
      CmdrPanel,
      ShipPanel,
      LocationPanel,
      StatusPanel,
      RoutePanel,
      SessionPanel,
      AlertsPanel,
    ]) {
      renderPanel(Comp);
    }
    const emptyMsg = t('empty-no-data');
    const occurrences = target.textContent?.split(emptyMsg).length ?? 1;
    expect(occurrences - 1).toBeGreaterThanOrEqual(7);
  });

  test('session duration does not re-render faster than 30 s', () => {
    vi.useFakeTimers();
    const baseTime = new Date('2026-10-17T18:00:00Z');
    vi.setSystemTime(baseTime);

    live.topics['core.session'] = {
      started_at: '2026-10-17T18:00:00Z',
      jumps: 3,
      distance_ly: 42.5,
      credits_delta: 125000,
    };

    renderPanel(SessionPanel);
    const durEl = () => target.querySelector('[data-testid="session-duration"]')?.textContent?.trim();
    expect(durEl()).toBe('0m');

    // After 29 seconds (even past 1 minute if we started at -35s): let's check with started_at 50s before baseTime
    unmount(mounted.pop()!);
    live.topics['core.session'] = {
      started_at: new Date(baseTime.getTime() - 50_000).toISOString(),
      jumps: 3,
      distance_ly: 42.5,
      credits_delta: 125000,
    };
    renderPanel(SessionPanel);
    expect(durEl()).toBe('0m');

    // Advance 15s -> total elapsed is 65s (1m), but interval hasn't fired yet (< 30s)
    vi.advanceTimersByTime(15_000);
    flushSync();
    expect(durEl()).toBe('0m');

    // Advance another 15s -> 30s interval fires, now shows 1m
    vi.advanceTimersByTime(15_000);
    flushSync();
    expect(durEl()).toBe('1m');
  });
});
