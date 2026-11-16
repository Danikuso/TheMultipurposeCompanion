import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import { defaultOverlaySettings, loadPrefs, prefs } from '../src/lib/prefs.svelte';
import { resolveOverlayRole } from '../src/overlay/overlayModel';
import OverlayApp from '../src/overlay/OverlayApp.svelte';
import TopBar from '../src/shell/TopBar.svelte';
import Settings from '../src/settings/Settings.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

const settle = async () => {
  for (let i = 0; i < 30; i++) await Promise.resolve();
  flushSync();
};

const jsonRes = (body: unknown, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => body,
});

function seedBasicTelemetry() {
  live.connected = true;
  live.topics['core.location'] = { system: 'Synuefe GT-H b43-1', star_pos: [10, 20, 30], body: 'Synuefe GT-H b43-1 4 a' };
  live.topics['core.ship'] = {
    fuel_main: 24,
    fuel_capacity_main: 32,
    hull_health: 0.92,
    cargo_count: 18,
    cargo_capacity: 128,
  };
  live.topics['navigation.route'] = {
    source: 'plan',
    current: { name: 'Synuefe GT-H b43-1', jump_ly: null, star_class: 'K', scoopable: true, special: null, fuel_after_pct: null, reachable: true, visits: 1, remote: { status: 'ok', source: 'edsm', fetched_at: null, data: null }, local_politics: null },
    next: { name: 'Eol Prou SS-T d3-128', jump_ly: 42.5, star_class: 'G', scoopable: true, special: null, fuel_after_pct: 60, reachable: true, visits: 0, remote: { status: 'pending', source: null, fetched_at: null, data: null }, local_politics: null },
    following: [],
    remaining_jumps: 4,
    total_ly: 168.4,
    off_route: false,
  };
}

beforeEach(() => {
  disconnect();
  setLocale('es');
  loadPrefs({ overlay: defaultOverlaySettings(), auto_companion: true });
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined;
  target.remove();
  vi.unstubAllGlobals();
});

test('resolveOverlayRole respects pinned role, auto detection and disabled blocks fallback', () => {
  const settings = defaultOverlaySettings();
  const baseCtx = {
    modules: [{ id: 'mining', relevance: 'active' }, { id: 'combat', relevance: 'active' }],
    hasSamplingOrBio: false,
    hasCombatActivity: true,
    hasProspector: true,
    hasColonisation: false,
    hasTrade: false,
    hasExploration: false,
  };

  // Prioridad por defecto pone combat delante de mining
  expect(resolveOverlayRole(settings, baseCtx)).toBe('combat');

  // Si desactivamos el bloque combat, cae en mining
  settings.blocks.combat = false;
  expect(resolveOverlayRole(settings, baseCtx)).toBe('mining');

  // Rol fijado manualmente sobrevive si su bloque está habilitado
  settings.blocks.combat = true;
  settings.pinned_role = 'exobiology';
  expect(resolveOverlayRole(settings, baseCtx)).toBe('exobiology');

  // Si todos los bloques están desactivados devuelve null
  for (const k of Object.keys(settings.blocks) as (keyof typeof settings.blocks)[]) {
    settings.blocks[k] = false;
  }
  expect(resolveOverlayRole(settings, baseCtx)).toBeNull();
});

test('OverlayApp renders minimal 1-line strip without tactical body', () => {
  seedBasicTelemetry();
  prefs.overlay = { ...defaultOverlaySettings(), enabled: true, mode: 'minimal', click_through: true };
  live.topics['combat.ship'] = { shields_up: false, hull_pct: 92, heat: 'normal', overheating_since: null, overheat_total_s: 0, pips: null, hardpoints: true, alerts: ['shields_down'] };

  mounted = mount(OverlayApp, { target, props: { autoConnect: false } });
  flushSync();

  expect(target.querySelector('[data-testid="overlay-dragbar"]')).toBeNull();
  expect(target.querySelector('[data-testid="overlay-tactical-body"]')).toBeNull();
  expect(target.querySelector('[data-testid="overlay-system"]')?.textContent).toBe('Synuefe GT-H b43-1');
  expect(target.querySelector('[data-testid="overlay-scoopable"]')).not.toBeNull();
  expect(target.querySelector('[data-testid="overlay-next-hop"]')?.textContent).toContain('Eol Prou SS-T d3-128');
  expect(target.querySelector('[data-testid="overlay-next-hop"]')?.textContent).toContain('4 s.');
  expect(target.querySelector('[data-testid="overlay-fuel-chip"]')?.textContent).toContain('75 %');
  expect(target.querySelector('[data-testid="overlay-hull-chip"]')?.textContent).toContain('92 %');
  expect(target.querySelector('[data-testid="overlay-shield-alert"]')).not.toBeNull();
});

test('OverlayApp renders exobiology, exploration, combat, mining, colonisation and trade tactical views', () => {
  seedBasicTelemetry();
  prefs.overlay = { ...defaultOverlaySettings(), enabled: true, mode: 'tactical', click_through: true };

  // 1. Exobiología
  live.topics['core.modules'] = [{ id: 'exobiology', relevance: 'urgent' }];
  live.topics['exobiology.body'] = { body: 'Synuefe GT-H b43-1 4 a', body_id: 4, system_address: 1, kind: 'Rocky body', gravity_g: 0.14, temperature_k: 165, atmosphere: 'CarbonDioxide', bio_signals: 3, first_logged: true, dss: true, candidates: [], confirmed_genera: [], completed: 1 };
  live.topics['exobiology.sampling'] = { species: '$sp;', species_name: 'Stratum Tectonicas', genus: '$g;', genus_name: 'Stratum', body_id: 4, points: 2, complete: false, distance_m: 380, min_m: 500, remaining_m: 120, direction: { rel: 'ahead', compass: 'NE' }, unsold_after: 19010800, discarded_species: null, discarded_name: null };
  live.topics['exobiology.unsold'] = { total: 19010800, species: 1 };

  mounted = mount(OverlayApp, { target, props: { autoConnect: false } });
  flushSync();

  const exoBodyEl = target.querySelector('[data-testid="overlay-tactical-body"]')!;
  expect(exoBodyEl.getAttribute('data-role')).toBe('exobiology');
  expect(exoBodyEl.textContent).toContain('Stratum Tectonicas');
  expect(exoBodyEl.textContent).toContain('Muestra 2/3');
  expect(exoBodyEl.textContent).toContain('380 m / 500 m');

  // 2. Combate
  prefs.overlay = { ...prefs.overlay, pinned_role: 'combat' };
  live.topics['combat.target'] = { ship_key: 'anaconda', ship_name: 'Anaconda', pilot: 'Pirate', rank: 'Dangerous', faction: 'Gang', badge: { kind: 'wanted', bounty: 285000 }, shield_pct: 40, hull_pct: 72, subsystem: null, thargoid: false, lost_at: null };
  live.topics['combat.session'] = { started_at: null, kills: 5, credits: 900000, kills_by_type: {}, combat_rank: 6, combat_progress: 40, vouchers: [], vouchers_total: { bounty: 900000, combat_bond: 120000 }, nearest_if: null, if_fee_pct: 25 };
  flushSync();
  expect(target.querySelector('[data-testid="overlay-tactical-body"]')?.getAttribute('data-role')).toBe('combat');
  expect(target.textContent).toContain('Anaconda');
  expect(target.textContent).toContain('1.020.000 CR');

  // 3. Minería
  prefs.overlay = { ...prefs.overlay, pinned_role: 'mining' };
  live.topics['mining.prospector'] = { current: { content: 'high', motherlode: null, materials: [{ key: 'platinum', name: 'Platino', proportion: 58.4, est_value: 282000, interest: true }], remaining: 100, verdict: { kind: 'worth', est_value: 282000 }, est_total: 282000 }, history: [] };
  live.topics['mining.session'] = { started_at: null, refined: {}, limpets_launched: {}, limpets_left: 42, cargo: 88, capacity: 128, asteroids_prospected: 12, cracked: 0, est_credits: null, refinery_bins: [] };
  flushSync();
  expect(target.querySelector('[data-testid="overlay-tactical-body"]')?.getAttribute('data-role')).toBe('mining');
  expect(target.textContent).toContain('Platino');
  expect(target.textContent).toContain('58,4 %');
  expect(target.textContent).toContain('42 drones');

  // 4. Colonización
  prefs.overlay = { ...prefs.overlay, pinned_role: 'colonisation' };
  live.topics['colonisation.depot'] = { active_project: true, market_id: 1, station_name: 'Orbital Apollo', construction_type: 'Outpost', progress_percent: 64, total_required: 1000, total_provided: 640, total_remaining: 360, cargo_capacity: 720, total_trips_remaining: 1, is_docked: false, resources: [{ name: 'steel', name_localised: 'Acero', required: 500, provided: 140, remaining: 360, progress_percent: 28, trips_remaining: 1 }] };
  flushSync();
  expect(target.querySelector('[data-testid="overlay-tactical-body"]')?.getAttribute('data-role')).toBe('colonisation');
  expect(target.textContent).toContain('64 %');
  expect(target.textContent).toContain('Acero');

  // 5. Comercio
  prefs.overlay = { ...prefs.overlay, pinned_role: 'trade' };
  live.topics['trade.cargo'] = { used: 720, capacity: 720, lines: [{ commodity: 'silver', name_localised: 'Plata', tons: 720, paid_avg: 34800, paid_partial: false, price_here: null, best_nearby: null, est_profit: null, advice: 'Carry', kind: 'Trade' }] };
  live.topics['trade.route'] = { plan: { stops: [{ from_system: 'Sol', from_station: 'Lincoln', to_system: 'Wally Bei', to_station: 'Malerba Orbital', commodities: [], stop_profit: 10000000, distance_ly: 14.2, jumps: 1, data_updated_at: null }], total_profit: 10000000 }, current: 0, completed: false, copy_target: 'Wally Bei', copy_seq: 1, status: 'ok', blocker: null };
  flushSync();
  expect(target.querySelector('[data-testid="overlay-tactical-body"]')?.getAttribute('data-role')).toBe('trade');
  expect(target.textContent).toContain('Plata');
  expect(target.textContent).toContain('Wally Bei');
});

test('OverlayApp unlocked drag bar calls overlay.start_drag, anchor update and click-through lock', async () => {
  seedBasicTelemetry();
  prefs.overlay = { ...defaultOverlaySettings(), enabled: true, mode: 'tactical', click_through: false };

  const calls: { url: string; body: unknown }[] = [];
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    const parsed = init?.body ? JSON.parse(init.body as string) : null;
    calls.push({ url, body: parsed });
    if (url === '/api/actions/overlay.update') return jsonRes({ ...prefs.overlay, ...parsed });
    if (url === '/api/actions/overlay.set_click_through') return jsonRes({ ...prefs.overlay, ...parsed });
    return jsonRes({ ok: true });
  }));

  mounted = mount(OverlayApp, { target, props: { autoConnect: false } });
  flushSync();

  expect(target.querySelector('[data-testid="overlay-dragbar"]')).not.toBeNull();

  // Cambiar esquina a bottom_left
  const cornerBtn = target.querySelector('[data-testid="overlay-anchor-bottom_left"]') as HTMLButtonElement;
  cornerBtn.click();
  await settle();
  expect(calls.some((c) => c.url === '/api/actions/overlay.update' && (c.body as Record<string, unknown>).anchor === 'bottom_left')).toBe(true);

  // Arrastre nativo + Fijar
  const dragHandle = target.querySelector('[data-testid="overlay-drag-handle"]') as HTMLButtonElement;
  dragHandle.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
  await settle();
  expect(calls.some((c) => c.url === '/api/actions/overlay.start_drag')).toBe(true);

  const lockBtn = target.querySelector('[data-testid="overlay-lock-btn"]') as HTMLButtonElement;
  lockBtn.click();
  await settle();
  const lockCall = calls.find((c) => c.url === '/api/actions/overlay.set_click_through');
  expect(lockCall).toBeDefined();
  expect((lockCall!.body as Record<string, unknown>).click_through).toBe(true);
});

test('TopBar and Settings manage overlay actions cleanly', async () => {
  const calls: { url: string; body: unknown }[] = [];
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    const parsed = init?.body ? JSON.parse(init.body as string) : null;
    calls.push({ url, body: parsed });
    if (url === '/api/actions/overlay.toggle') {
      const next = { ...prefs.overlay, enabled: Boolean((parsed as Record<string, unknown>)?.enabled) };
      return jsonRes(next);
    }
    if (url === '/api/actions/overlay.set_click_through') {
      return jsonRes({ ...prefs.overlay, ...(parsed as Record<string, unknown>) });
    }
    if (url === '/api/actions/overlay.update') {
      return jsonRes({ ...prefs.overlay, ...(parsed as Record<string, unknown>) });
    }
    if (url.startsWith('/api/tts/voices')) return jsonRes({ voices: [] });
    if (url === '/api/voice/voiceattack') return jsonRes({ detected: false, running: false });
    return jsonRes({
      language: 'es',
      overlay: defaultOverlaySettings(),
      server: { port: 47320, lan: false },
    });
  }));

  // TopBar toggle + lock
  mounted = mount(TopBar, { target });
  flushSync();
  expect(target.querySelector('[data-testid="topbar-overlay-lock"]')).toBeNull();

  const topToggle = target.querySelector('[data-testid="topbar-overlay-toggle"]') as HTMLButtonElement;
  topToggle.click();
  await settle();
  expect(prefs.overlay.enabled).toBe(true);
  expect(target.querySelector('[data-testid="topbar-overlay-lock"]')).not.toBeNull();

  unmount(mounted);
  target.innerHTML = '';

  // Settings card
  mounted = mount(Settings, { target });
  await settle();
  const modeSelect = target.querySelector('[data-testid="overlay-mode"]') as HTMLSelectElement;
  expect(modeSelect).not.toBeNull();
  modeSelect.value = 'minimal';
  modeSelect.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();

  expect(calls.some((c) => c.url === '/api/actions/overlay.update' && (c.body as Record<string, unknown>).mode === 'minimal')).toBe(true);
});
