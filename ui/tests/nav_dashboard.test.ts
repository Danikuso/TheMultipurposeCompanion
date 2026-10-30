import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import App from '../src/App.svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { loadPrefs, prefs } from '../src/lib/prefs.svelte';
import { layouts } from '../src/shell/layouts';
import { modules } from '../src/modules/registry';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

describe('sequential navigation to dashboard', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  const errors: string[] = [];

  beforeEach(() => {
    errors.length = 0;
    window.addEventListener('error', (e) => errors.push(e.message));
    disconnect();
    live.topics['core.location'] = { system: 'Forld Pri YZ-W d2-5635', system_address: 123456, body: 'Planet A 1', mode: 'NormalSpace' };
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
    live.topics['core.cmdr'] = { name: 'Danikuso', credits: 61627444, ranks: { combat: 3, trade: 4, explore: 5 } };
    live.topics['core.status'] = { flags: 0, pips: [4, 4, 4] };
    live.topics['core.modules'] = modules.map((m) => ({ id: m.id, relevance: 'normal' }));
    live.topics['accounts.status'] = {
      eddn: { enabled: true, connected: true, uploader_id: 'TMC-Tester' },
      edsm: { enabled: true, has_key: true, commander_name: 'Danikuso' },
      inara: { enabled: true, has_key: true, commander_name: 'Danikuso' },
      capi: { connected: true, commander_name: 'Danikuso' },
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

  test('sequential navigation from each layout to dashboard and back', () => {
    const layoutBtns = Array.from(target.querySelectorAll('aside .sidebar-top > button'));
    for (let i = 0; i < layouts.length; i++) {
      const layout = layouts[i];
      const lBtn = layoutBtns[i] as HTMLButtonElement;
      lBtn.click();
      flushSync();

      // Now click dashboard
      const dashBtn = target.querySelector('[data-module="dashboard"]') as HTMLButtonElement;
      dashBtn.click();
      flushSync();

      const cmdr = target.querySelector('.cmdr-head, [data-testid="rank-group-career"]');
      expect(cmdr).not.toBeNull();
      const grid = target.querySelector('.hud-grid') as HTMLElement;
      expect(grid.scrollTop).toBe(0);
    }
  });

  test('sequential navigation from each module to dashboard and back', () => {
    for (const mod of modules) {
      if (mod.id === 'dashboard' || mod.id === 'accounts') continue;
      const modBtn = target.querySelector(`[data-module="${mod.id}"]`) as HTMLButtonElement;
      if (!modBtn) continue;
      modBtn.click();
      flushSync();

      // Now click dashboard
      const dashBtn = target.querySelector('[data-module="dashboard"]') as HTMLButtonElement;
      dashBtn.click();
      flushSync();

      const cmdr = target.querySelector('.cmdr-head, [data-testid="rank-group-career"]');
      expect(cmdr).not.toBeNull();
      const grid = target.querySelector('.hud-grid') as HTMLElement;
      expect(grid.scrollTop).toBe(0);
    }
  });

  test('sequential navigation between settings (with simulated scroll), layouts and dashboard', () => {
    const settingsBtn = target.querySelector('aside .sidebar-bottom button') as HTMLButtonElement;
    settingsBtn.click();
    flushSync();

    const grid = target.querySelector('.hud-grid') as HTMLElement;
    expect(grid).not.toBeNull();
    // Simulate user scrolling down in settings
    grid.scrollTop = 850;

    // Navigate to dashboard
    const dashBtn = target.querySelector('[data-module="dashboard"]') as HTMLButtonElement;
    dashBtn.click();
    flushSync();

    // Verify grid scrollTop reset to 0
    expect(grid.scrollTop).toBe(0);

    const cmdr = target.querySelector('.cmdr-head, [data-testid="rank-group-career"]');
    expect(cmdr).not.toBeNull();
  });
});
