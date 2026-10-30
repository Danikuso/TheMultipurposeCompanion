import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import App from '../src/App.svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { prefs } from '../src/lib/prefs.svelte';
import { layouts } from '../src/shell/layouts';
import { modules } from '../src/modules/registry';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

describe('test navigation errors', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  const errors: any[] = [];

  beforeEach(() => {
    errors.length = 0;
    const origError = console.error;
    console.error = (...args: any[]) => {
      errors.push(args.join(' '));
      origError(...args);
    };
    window.onerror = (msg) => {
      errors.push(String(msg));
    };

    disconnect();
    live.topics['core.location'] = {
      system: 'Forld Pri YZ-W d2-5635',
      system_address: 123456,
      body: 'Planet A 1',
      mode: 'NormalSpace',
    };
    live.topics['core.ship'] = {
      ship_type: 'krait_mkii',
      ship_type_localised: 'Krait Mk II',
      name: 'Millennium',
      ident: 'DAN-01',
      hull_health: 0.93,
      fuel_main: 27.5,
      fuel_capacity_main: 32,
      cargo_count: 16,
      cargo_capacity: 16,
      rebuy: 5000000,
      modules: [],
    };
    live.topics['core.cmdr'] = {
      name: 'Danikuso',
      credits: 61627444,
      ranks: { combat: 3, trade: 4, explore: 5 },
    };
    live.topics['core.status'] = { flags: 0, pips: [4, 4, 4] };
    live.topics['core.modules'] = modules.map((m) => ({ id: m.id, relevance: 'normal' }));
    live.topics['navigation.route'] = {
      current: { name: 'Forld Pri YZ-W d2-5635', star_class: 'K' },
      next: { name: 'Sol', star_class: 'G' },
      remaining_jumps: 12,
      total_jumps: 15,
      source: 'game',
      hops: [],
    };
    live.topics['navigation.jump'] = {
      source: 'game',
      range_ly: 45.2,
      range_full_ly: 42.1,
      fuel_main: 27.5,
      fuel_capacity: 32,
      max_fuel_per_jump: 5,
    };
    live.topics['navigation.plan'] = { status: 'idle' };
    live.topics['exploration.system'] = {
      system: 'Forld Pri YZ-W d2-5635',
      system_address: 123456,
      star_classes: ['K'],
      bodies_total: 10,
      bodies_scanned: 5,
      bodies: [],
    };
    live.topics['exobiology.body'] = {
      system_address: 123456,
      body_id: 1,
      body: 'Planet A 1',
      species: [],
    };
    live.topics['combat.ship'] = { shields_up: true, hull_pct: 0.93 };
    live.topics['mining.session'] = { minerals: [] };
    live.topics['trade.market'] = { commodities: [] };
    live.topics['missions.board'] = { missions: [] };
    live.topics['carrier.summary'] = { name: 'TMC Carrier', callsign: 'ABC-123' };
    live.topics['colonisation.overview'] = { systems: [] };
    live.topics['powerplay.overview'] = { power: 'Aisling Duval' };
    live.topics['bgs.system'] = { system: 'Forld Pri YZ-W d2-5635', factions: [] };
    live.topics['crime.overview'] = { bounties: [], fines: [] };
    live.topics['ship.outfitting_catalog'] = {
      ships: [{ symbol: 'krait_mkii', name: 'Krait Mk II' }],
      modules: [],
    };

    target = document.createElement('div');
    document.body.appendChild(target);
    prefs.loaded = true;
    prefs.autoCompanion = true;
    instance = mount(App, { target });
    flushSync();
  });

  afterEach(() => {
    unmount(instance);
    target.remove();
    disconnect();
  });

  test('navigate from each layout to dashboard without any console/runtime errors', () => {
    const layoutBtns = Array.from(target.querySelectorAll('aside .sidebar-top > button'));
    for (let i = 0; i < layouts.length; i++) {
      const lBtn = layoutBtns[i] as HTMLButtonElement;
      lBtn.click();
      flushSync();

      const dashBtn = target.querySelector('[data-module="dashboard"]') as HTMLButtonElement;
      dashBtn.click();
      flushSync();

      if (errors.length > 0) {
        console.log(`Error after navigating from layout ${layouts[i].id} to dashboard:`, errors);
      }
      expect(errors).toEqual([]);
    }
  });

  test('navigate from each module to dashboard without any console/runtime errors', () => {
    for (const mod of modules) {
      if (mod.id === 'dashboard' || mod.id === 'accounts') continue;
      const modBtn = target.querySelector(`[data-module="${mod.id}"]`) as HTMLButtonElement;
      if (!modBtn) continue;
      modBtn.click();
      flushSync();

      const dashBtn = target.querySelector('[data-module="dashboard"]') as HTMLButtonElement;
      dashBtn.click();
      flushSync();

      if (errors.length > 0) {
        console.log(`Error after navigating from module ${mod.id} to dashboard:`, errors);
      }
      expect(errors).toEqual([]);
    }
  });
});
