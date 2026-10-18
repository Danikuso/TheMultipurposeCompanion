import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import TargetPanel from '../src/modules/combat/TargetPanel.svelte';
import ShipPanel from '../src/modules/combat/ShipPanel.svelte';
import HuntingPanel from '../src/modules/combat/HuntingPanel.svelte';
import ConflictZonePanel from '../src/modules/combat/ConflictZonePanel.svelte';
import AxPanel from '../src/modules/combat/AxPanel.svelte';
import PipsGauge from '../src/modules/combat/PipsGauge.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function targetTopic(extra: Record<string, unknown> = {}) {
  return {
    ship_key: 'python', ship_name: 'Python', pilot: 'Foo', rank: 'Deadly', faction: 'Los Pacíficos',
    badge: { kind: 'wanted', bounty: 125000 }, shield_pct: 80, hull_pct: 55,
    subsystem: { kind: 'power_plant', name: 'Planta de energía', health_pct: 42 },
    thargoid: false, lost_at: null, ...extra,
  };
}
function shipTopic(extra: Record<string, unknown> = {}) {
  return {
    shields_up: true, hull_pct: 73, heat: 'normal', overheating_since: null, overheat_total_s: 0,
    pips: { sys: 1.5, eng: 2.5, wep: 2 }, hardpoints: true, alerts: [], ...extra,
  };
}
function mountTarget(t: unknown) {
  live.topics['combat.target'] = t;
  mounted = mount(TargetPanel, { target });
  flushSync();
}
function mountShip(s: unknown) {
  live.topics['combat.ship'] = s;
  mounted = mount(ShipPanel, { target });
  flushSync();
}
beforeEach(() => {
  disconnect(); setLocale('es');
  target = document.createElement('div'); document.body.appendChild(target);
});
afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined; target.remove(); vi.useRealTimers(); vi.unstubAllGlobals();
});

test('wanted badge shows localized bounty', () => {
  mountTarget(targetTopic());
  const badge = target.querySelector('[data-testid="target-badge"]')!;
  expect(badge.textContent).toContain('BUSCADO');
  expect(badge.textContent).toContain('125.000');
  expect(badge.getAttribute('data-kind')).toBe('wanted');
  expect(target.querySelector('[data-testid="shield-value"]')!.textContent).toContain('80');
  expect(target.querySelector('[data-testid="subsystem"]')!.textContent).toContain('Planta de energía');
  expect(target.querySelector('[data-testid="subsystem"]')!.textContent).toContain('42');
});

test('wanted badge in english uses english grouping', () => {
  setLocale('en');
  mountTarget(targetTopic());
  const badge = target.querySelector('[data-testid="target-badge"]')!;
  expect(badge.textContent).toContain('WANTED');
  expect(badge.textContent).toContain('125,000');
});

test('clean badge', () => {
  mountTarget(targetTopic({ badge: { kind: 'clean' }, subsystem: null }));
  const badge = target.querySelector('[data-testid="target-badge"]')!;
  expect(badge.textContent).toContain('LIMPIO · NO DISPARAR');
  expect(badge.getAttribute('data-kind')).toBe('clean');
  expect(target.querySelector('[data-testid="subsystem"]')).toBeNull();
});

test('scanning badge', () => {
  mountTarget(targetTopic({ badge: { kind: 'scanning' } }));
  expect(target.querySelector('[data-testid="target-badge"]')!.textContent).toContain('escaneando…');
});

test('pips gauge renders half pips', () => {
  mounted = mount(PipsGauge, { target, props: { pips: { sys: 1.5, eng: 2.5, wep: 2 } } });
  flushSync();
  expect(target.querySelectorAll('[data-state="full"]').length).toBe(5);
  expect(target.querySelectorAll('[data-state="half"]').length).toBe(2);
  expect(target.querySelectorAll('[data-state="empty"]').length).toBe(5);
  expect(target.querySelectorAll('[data-col]').length).toBe(3);
  const halves = (c: string) => target.querySelectorAll(`[data-col="${c}"] [data-state="half"]`).length;
  const fulls = (c: string) => target.querySelectorAll(`[data-col="${c}"] [data-state="full"]`).length;
  expect([fulls('sys'), halves('sys')]).toEqual([1, 1]);
  expect([fulls('eng'), halves('eng')]).toEqual([2, 1]);
  expect([fulls('wep'), halves('wep')]).toEqual([2, 0]);
  expect(target.querySelector('[data-testid="pips"]')!.getAttribute('aria-label'))
    .toBe('Energía: SIS 1,5, MOT 2,5, ARM 2 de 4');
});

test('heat timer only runs while overheating', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-18T12:00:30Z'));
  mountShip(shipTopic());
  expect(vi.getTimerCount()).toBe(0);
  unmount(mounted!);
  live.topics['combat.ship'] = shipTopic({ heat: 'overheating', overheating_since: '2026-10-18T12:00:00Z' });
  mounted = mount(ShipPanel, { target });
  flushSync();
  expect(vi.getTimerCount()).toBe(1);
  expect(target.querySelector('[data-testid="heat-timer"]')!.textContent).toBe('0:00');
  vi.advanceTimersByTime(5000);
  flushSync();
  expect(target.querySelector('[data-testid="heat-timer"]')!.textContent).toBe('0:05');
  unmount(mounted!); mounted = undefined;
  expect(vi.getTimerCount()).toBe(0);
});

test('ship panel shows shields, stepwise hull and alerts', () => {
  mountShip(shipTopic({ shields_up: false, hull_pct: 28, alerts: ['hull_low', 'shields_down'] }));
  expect(target.querySelector('[data-testid="ship-shields"]')!.getAttribute('data-state')).toBe('down');
  expect(target.querySelector('[data-testid="hull-value"]')!.textContent).toContain('28');
  expect(target.querySelectorAll('[data-testid="hull-step"][data-on="true"]').length).toBe(3);
  const alerts = target.querySelector('[data-testid="ship-alerts"]')!;
  expect(alerts.textContent).toContain('Casco al 28 %');
  expect(alerts.textContent).toContain('Escudos caídos');
  expect(target.querySelector('[data-testid="heat-state"]')!.textContent).toContain('normal');
});

test('target panel empties 10 s after lost_at', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-18T12:00:00Z'));
  mountTarget(targetTopic({ lost_at: '2026-10-18T12:00:00Z' }));
  expect(target.querySelector('[data-testid="target-badge"]')).not.toBeNull();
  vi.advanceTimersByTime(10000);
  flushSync();
  expect(target.querySelector('[data-testid="target-badge"]')).toBeNull();
  expect(target.querySelector('[data-testid="empty"]')!.textContent).toContain('Sin objetivo fijado');
  expect(vi.getTimerCount()).toBe(0);
});

test('heat timer is anchored to reception, not to the journal clock', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2030-01-01T00:00:00Z'));
  mountShip(shipTopic({ heat: 'overheating', overheating_since: '2026-10-18T12:00:00Z' }));
  expect(target.querySelector('[data-testid="heat-timer"]')!.textContent).toBe('0:00');
  vi.advanceTimersByTime(65000);
  flushSync();
  expect(target.querySelector('[data-testid="heat-timer"]')!.textContent).toBe('1:05');
});

test('target timeout runs from reception even with an old lost_at', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2030-01-01T00:00:00Z'));
  mountTarget(targetTopic({ lost_at: '2026-10-18T12:00:00Z' }));
  expect(target.querySelector('[data-testid="target-badge"]')).not.toBeNull();
  vi.advanceTimersByTime(9999);
  flushSync();
  expect(target.querySelector('[data-testid="target-badge"]')).not.toBeNull();
  vi.advanceTimersByTime(1);
  flushSync();
  expect(target.querySelector('[data-testid="empty"]')).not.toBeNull();
});

test('accessibility: labelled shields, bars and live badge', () => {
  mountShip(shipTopic({ shields_up: false }));
  expect(target.querySelector('[data-testid="ship-shields"]')!.getAttribute('aria-label')).toBe('Escudos caídos');
  const bar = target.querySelector('[role="progressbar"]')!;
  expect(bar.getAttribute('aria-label')).toBe('Casco');
  expect(bar.getAttribute('aria-valuenow')).toBe('73');
  unmount(mounted!); mounted = undefined;
  mountTarget(targetTopic());
  const bars = target.querySelectorAll('[role="progressbar"]');
  expect(bars.length).toBe(3);
  expect(bars[0].getAttribute('aria-label')).toBe('Escudos');
  expect(bars[0].getAttribute('aria-valuenow')).toBe('80');
  expect(target.querySelector('[data-testid="target-badge"]')!.parentElement!.getAttribute('aria-live')).toBe('polite');
});

test('lawless badge has a label', () => {
  mountTarget(targetTopic({ badge: { kind: 'lawless' } }));
  expect(target.querySelector('[data-testid="target-badge"]')!.textContent).toBe('SIN LEY');
});

function sessionTopic(extra: Record<string, unknown> = {}) {
  return {
    as_of: new Date().toISOString(), started_at: '2026-10-18T12:00:00Z', kills: 4, credits: 120000, kills_by_type: { python: 3, viper: 1 },
    combat_rank: 5, combat_progress: 40,
    vouchers: [
      { kind: 'Bounty', faction: 'Los Pacíficos', amount: 50000, systems: ['Sol', 'Alpha Centauri'], ax: false, other: false, count: 1 },
      { kind: 'CombatBond', faction: '$faction_Federation;', amount: 30000, systems: ['Sol'], ax: false, other: false, count: 1 },
      { kind: 'CombatBond', faction: '', amount: 1000, systems: [], ax: false, other: false, count: 1 },
    ],
    vouchers_total: { bounty: 50000, combat_bond: 31000 },
    nearest_if: {
      station_name: 'Hutton Orbital', system_name: 'Alpha Centauri', distance_ly: 4.38,
      distance_to_arrival_ls: 100, max_pad: 'Large', updated_at: new Date(Date.now() - 7200_000).toISOString(),
    },
    if_fee_pct: 25, ...extra,
  };
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function mountWith(comp: Component<any>, name: string, v: unknown) {
  live.topics[name] = v;
  mounted = mount(comp, { target });
  flushSync();
}

test('vouchers grouped by faction and kind with IF fee', () => {
  mountWith(HuntingPanel, 'combat.session', sessionTopic());
  const rows = target.querySelectorAll('[data-testid="voucher-row"]');
  expect(rows.length).toBe(3);
  expect(rows[0].textContent).toContain('Los Pacíficos');
  expect(rows[0].textContent).toContain('50.000');
  expect(rows[0].textContent).toContain('Sol, Alpha Centauri');
  expect(rows[0].querySelector('[data-testid="voucher-fee"]')!.textContent).toContain('12.500');
  expect(rows[0].getAttribute('data-kind')).toBe('Bounty');
  expect(rows[1].textContent).toContain('Federation');
  expect(rows[1].textContent).not.toContain('$faction');
  expect(rows[1].querySelector('[data-testid="voucher-fee"]')!.textContent).toContain('7500');
  expect(rows[2].textContent).toContain('facción desconocida');
  expect(target.querySelector('[data-testid="if-station"]')!.textContent).toContain('Hutton Orbital');
  expect(target.querySelector('[data-testid="if-age"]')!.textContent).toContain('2h');
  expect(target.querySelector('[data-testid="session-credits"]')!.textContent).toContain('120.000');
  expect(target.querySelector('[data-testid="rank-progress"]')!.textContent).toBe('Maestro · 40 %');
});

test('credits per hour is computed locally from started_at', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-18T13:00:00Z'));
  mountWith(HuntingPanel, 'combat.session', sessionTopic());
  expect(target.querySelector('[data-testid="credits-hour"]')!.textContent).toContain('120.000');
});

test('reset session posts json action', async () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
  vi.stubGlobal('fetch', fetchMock);
  mountWith(HuntingPanel, 'combat.session', sessionTopic());
  (target.querySelector('[data-testid="reset-session"]') as HTMLButtonElement).click();
  expect(fetchMock).toHaveBeenCalledTimes(1);
  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe('/api/actions/combat.reset_session');
  expect(init.method).toBe('POST');
  expect(init.headers['content-type']).toBe('application/json');
  vi.unstubAllGlobals();
});

test('no IF data shows no-data label', () => {
  mountWith(HuntingPanel, 'combat.session', sessionTopic({ nearest_if: null }));
  expect(target.querySelector('[data-testid="if-none"]')!.textContent).toBe('sin datos');
  expect(target.querySelector('[data-testid="voucher-fee"]')).toBeNull();
});

test('ax panel lists kills by type and presence banner', () => {
  mountWith(AxPanel, 'combat.ax', { active: true, presence: true, kills_by_type: { scout: 3 }, bonds: 24000, target: 'scout' });
  expect(target.querySelector('[data-testid="ax-kill-scout"]')!.textContent).toContain('3');
  expect(target.querySelector('[data-testid="ax-presence"]')!.textContent).toContain('presencia thargoide');
  expect(target.querySelector('[data-testid="ax-bonds"]')!.textContent).toContain('24.000');
});

test('ax and conflict zone panels hide when not applicable', () => {
  mountWith(AxPanel, 'combat.ax', { active: false, presence: false, kills_by_type: {}, bonds: 0, target: null });
  expect(target.querySelector('[data-testid="ax-panel"]')).toBeNull();
  unmount(mounted!); mounted = undefined;
  mountWith(ConflictZonePanel, 'combat.conflict_zone', null);
  expect(target.querySelector('[data-testid="zone-panel"]')).toBeNull();
});

test('conflict zone panel shows side, kills and bonds by faction', () => {
  mountWith(ConflictZonePanel, 'combat.conflict_zone', {
    system: 'Sol', side: '$faction_Federation;', kills: 7, bonds_by_faction: { 'Los Pacíficos': 15000, '': 500 },
  });
  const z = target.querySelector('[data-testid="zone-panel"]')!;
  expect(z.textContent).toContain('Federation');
  expect(z.textContent).toContain('Sol');
  expect(target.querySelector('[data-testid="zone-kills"]')!.textContent).toContain('7 bajas');
  const bonds = target.querySelectorAll('[data-testid="zone-bond"]');
  expect(bonds.length).toBe(2);
  expect(bonds[0].textContent).toContain('15.000');
  expect(bonds[1].textContent).toContain('facción desconocida');
});

test('rank progress 1 is shown as 1 %', () => {
  mountWith(HuntingPanel, 'combat.session', sessionTopic({ combat_progress: 1 }));
  expect(target.querySelector('[data-testid="rank-progress"]')!.textContent).toBe('Maestro · 1 %');
});

test('other voucher row counts vouchers, not factions', () => {
  mountWith(HuntingPanel, 'combat.session', sessionTopic({
    vouchers: [{ kind: null, faction: '', amount: 9000, systems: [], ax: false, other: true, count: 5 }],
  }));
  const row = target.querySelector('[data-testid="voucher-row"]')!;
  expect(row.textContent).toContain('5 vales más');
  expect(row.textContent).not.toContain('desconocida');
});

test('reset failure shows alert', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({}) }));
  mountWith(HuntingPanel, 'combat.session', sessionTopic());
  (target.querySelector('[data-testid="reset-session"]') as HTMLButtonElement).click();
  await vi.waitFor(() => {
    flushSync();
    expect(target.querySelector('[role="alert"]')!.textContent).toBe('No se pudo reiniciar la sesión');
  });
});

test('invalid IF updated_at shows a dash', () => {
  mountWith(HuntingPanel, 'combat.session', sessionTopic({
    nearest_if: { station_name: 'H', system_name: 'S', distance_ly: 1, distance_to_arrival_ls: null, max_pad: null, updated_at: 'garbage' },
  }));
  expect(target.querySelector('[data-testid="if-age"]')!.textContent).toBe('—');
});

test('conflict zone side is cleaned', () => {
  mountWith(ConflictZonePanel, 'combat.conflict_zone', { system: 'Sol', side: '$faction_Federation;', kills: 1, bonds_by_faction: {} });
  expect(target.querySelector('[data-testid="zone-side"]')!.textContent).toBe('Bando: Federation');
});

test('ax types are localized with raw fallback', () => {
  mountWith(AxPanel, 'combat.ax', { active: true, presence: false, kills_by_type: { scout: 3, cyclops: 1, Weird_Thing: 2 }, bonds: 0, target: 'medusa' });
  expect(target.querySelector('[data-testid="ax-kill-scout"]')!.textContent).toContain('Explorador');
  expect(target.querySelector('[data-testid="ax-kill-cyclops"]')!.textContent).toContain('Cíclope');
  expect(target.querySelector('[data-testid="ax-kill-Weird_Thing"]')!.textContent).toContain('Weird Thing');
  expect(target.querySelector('[data-testid="ax-target"]')!.textContent).toBe('Objetivo: Medusa');
});
