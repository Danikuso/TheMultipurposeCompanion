import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import MissionsPanel from '../src/modules/missions/MissionsPanel.svelte';
import MassacrePanel from '../src/modules/missions/MassacrePanel.svelte';
import MissionAlerts from '../src/modules/missions/MissionAlerts.svelte';
import PassengersPanel from '../src/modules/missions/PassengersPanel.svelte';
import TourismPanel from '../src/modules/missions/TourismPanel.svelte';
import RewardsPanel from '../src/modules/missions/RewardsPanel.svelte';
import CommunityGoalsPanel from '../src/modules/missions/CommunityGoalsPanel.svelte';
import { isExpiringSoon } from '../src/modules/missions/time';

const CLOCK = '2030-01-01T12:00:00Z';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  live.topics['core.clock'] = { as_of: CLOCK };
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

function mission(extra: Record<string, unknown> = {}) {
  return {
    id: 1, kind: 'Delivery', name_localised: 'Entrega', faction: 'Lave Union', destination_system: 'Lave',
    destination_station: 'Lave Station', stops: [], accepted_at: '2029-12-30T00:00:00Z', expiry: null,
    reward: 410000, progress: { delivered: 5, collected: 0, total: 12, kills_needed: null, passengers: null },
    redirected: false, details_missing: false, target_faction: null, wing: false, commodity: 'Biowaste', ...extra,
  };
}
function group(system: string, next: boolean, missions = [mission()], extra: Record<string, unknown> = {}) {
  return { system, station: `${system} Hub`, distance_ly: 10, expiry: null, next, missions, ...extra };
}
function active(groups: unknown[]) {
  return { as_of: CLOCK, count: 2, groups, next_system: 'Lave', copy_target: null, copy_seq: 0 };
}

test('groups by destination with next stop highlighted', () => {
  mountWith(MissionsPanel, { 'missions.active': active([group('Lave', true), group('Leesti', false, [mission({ id: 2 })])]) });
  const cards = qa('missions-group');
  expect(cards.length).toBe(2);
  expect(cards[0].classList.contains('next')).toBe(true);
  expect(cards[0].textContent).toContain('Lave');
  expect(cards[1].classList.contains('next')).toBe(false);
  expect(cards[0].textContent).toContain('Entrega');
  expect(cards[0].textContent).toContain('410.000');
  expect(cards[0].textContent).toContain('5 / 12');
});

test('countdown uses anchored clock', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  mountWith(MissionsPanel, {
    'missions.active': active([group('Lave', true, [mission({ expiry: '2030-01-01T12:30:00Z' })])]),
  });
  const c = q('mission-countdown')!;
  expect(c.textContent).toContain('30:00');
  expect(c.classList.contains('expiring-soon')).toBe(true);
  expect(isExpiringSoon('2030-01-01T12:30:00Z', Date.parse(CLOCK))).toBe(true);
  expect(isExpiringSoon('2030-01-01T14:30:00Z', Date.parse(CLOCK))).toBe(false);
});

test('details missing shows localized text', () => {
  mountWith(MissionsPanel, {
    'missions.active': active([group('Lave', true, [mission({ details_missing: true, kind: 'Other' })])]),
  });
  expect(q('mission-details-missing')!.textContent).toContain('Detalles no disponibles');
});

test('massacre bars and remaining', () => {
  const issuer = (faction: string, kills: number, needed: number) => ({ faction, kills, needed, missions: 1 });
  mountWith(MassacrePanel, {
    'missions.massacre': {
      as_of: CLOCK,
      piles: [{
        target_faction: 'Pirates', issuers: [issuer('A', 10, 10), issuer('B', 8, 8), issuer("A'", 2, 5)],
        kills_remaining: 3, total_reward: 48500000, details_missing: false,
      }],
    },
  });
  expect(qa('massacre-bar').length).toBe(3);
  expect(q('massacre-remaining')!.textContent).toContain('faltan 3');
  expect(q('massacre-reward')!.textContent).toContain('48.500.000');
  expect(qa('massacre-bar')[0].querySelector('[role="progressbar"]')!.getAttribute('aria-valuenow')).toBe('10');
});

test('to route posts json action', async () => {
  const fetchMock = vi.fn(async (..._a: unknown[]) => ({ ok: true, status: 200, json: async () => ({}) }));
  vi.stubGlobal('fetch', fetchMock);
  mountWith(MissionsPanel, { 'missions.active': active([group('Lave', true)]) });
  (q('mission-to-route') as HTMLButtonElement).click();
  await Promise.resolve();
  const call = fetchMock.mock.calls.find((c) => String(c[0]).endsWith('/api/actions/missions.to_route'))!;
  expect(call).toBeTruthy();
  expect(JSON.parse((call[1] as RequestInit).body as string)).toEqual({ system: 'Lave' });
});

test('no interval without expiring missions', () => {
  vi.useFakeTimers();
  const spy = vi.spyOn(globalThis, 'setInterval');
  mountWith(MissionsPanel, { 'missions.active': active([group('Lave', true)]) });
  expect(spy).not.toHaveBeenCalled();
});

test('alerts show localized text and dismiss', async () => {
  const fetchMock = vi.fn(async (..._a: unknown[]) => ({ ok: true, status: 200, json: async () => ({}) }));
  vi.stubGlobal('fetch', fetchMock);
  mountWith(MissionAlerts, {
    'missions.alerts': {
      as_of: CLOCK,
      alerts: [
        { mission_id: 7, kind: 'ExpiringSoon', at: CLOCK, bucket: 0, name: 'Entrega X', mission_kind: 'Delivery' },
        { mission_id: 8, kind: { CargoMissing: { commodity: 'gold', missing: 14 } }, at: CLOCK, bucket: 14, name: null, mission_kind: 'Collect' },
      ],
    },
  });
  const items = qa('missions-alert');
  expect(items.length).toBe(2);
  expect(items[0].textContent).toContain('Entrega X');
  expect(items[1].textContent).toContain('14');
  (items[0].querySelector('button') as HTMLButtonElement).click();
  await Promise.resolve();
  const call = fetchMock.mock.calls.find((c) => String(c[0]).endsWith('/api/actions/missions.dismiss_alert'))!;
  expect(JSON.parse((call[1] as RequestInit).body as string)).toEqual({ mission_id: 7 });
});

function pax(extra: Record<string, unknown> = {}) {
  return {
    as_of: CLOCK,
    cabins: {
      by_class: { Economy: { occupied: 12, seats: 16 }, Business: { occupied: null, seats: 22 } },
      occupied_total: 12, seats_total: 38,
    },
    groups: [
      { destination: 'Sol', count: 12, vip: false, wanted: true, mission_id: 1 },
      { destination: null, count: 4, vip: true, wanted: false, mission_id: 2 },
    ],
    wanted: true, ...extra,
  };
}

test('wanted passenger shows warning', () => {
  mountWith(PassengersPanel, { 'missions.passengers': pax() });
  expect(q('missions-wanted-warning')!.textContent).toContain('Pasajero buscado a bordo');
  expect(qa('passenger-group').length).toBe(2);
  expect(qa('passenger-group')[0].textContent).toContain('BUSCADO');
  expect(qa('passenger-group')[1].textContent).toContain('VIP');
  expect(qa('passenger-group')[1].textContent).toContain('Destino desconocido');
  expect(q('cabin-Economy')!.textContent).toContain('12');
  expect(q('cabin-Business')!.textContent).toContain('?');
});

test('no wanted warning when clean', () => {
  mountWith(PassengersPanel, { 'missions.passengers': pax({ wanted: false }) });
  expect(q('missions-wanted-warning')).toBeNull();
});

function plan(extra: Record<string, unknown> = {}) {
  return {
    mission_id: 3, order: ['Sol', 'Alpha Centauri'], board_order_ly: 51, optimal_ly: 32, jumps: 4,
    cr_per_ly: 100000, cr_per_hour: 6000000, estimates_verified: false, reference_cr_ly: 150000,
    reference_is_own: true, verdict: 'Profitable', can_abandon_now: false, ...extra,
  };
}
function tourism(status: string, p: unknown) {
  return { as_of: CLOCK, trips: [{ mission_id: 3, name: 'Tour', stops: ['Alpha Centauri', 'Sol'], status, plan: p }] };
}

test('tourism not worth shows abandon hint', () => {
  mountWith(TourismPanel, {
    'missions.tourism': tourism('ok', plan({ verdict: 'NotWorth', can_abandon_now: true })),
  });
  expect(q('missions-tourism-abandon')!.textContent).toContain('abandonarla');
  expect(q('missions-tourism-verdict')!.textContent).toContain('NO COMPENSA');
});

test('tourism profitable has no abandon hint', () => {
  mountWith(TourismPanel, { 'missions.tourism': tourism('ok', plan()) });
  expect(q('missions-tourism-abandon')).toBeNull();
  expect(q('missions-tourism-verdict')!.textContent).toContain('RENTABLE');
});

test('tourism shows both distances and crly', () => {
  mountWith(TourismPanel, { 'missions.tourism': tourism('ok', plan()) });
  const txt = q('missions-tourism-plan')!.textContent!;
  expect(txt).toContain('51 al');
  expect(txt).toContain('32 al');
  expect(txt).toContain('100.000');
  expect(txt).toContain('≈');
});

test('stops unavailable message', () => {
  mountWith(TourismPanel, { 'missions.tourism': tourism('stops_unavailable', null) });
  expect(q('missions-tourism-no-stops')!.textContent).toContain('Paradas no disponibles');
});

test('community goal remaining time', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2031-06-01T00:00:00Z'));
  mountWith(CommunityGoalsPanel, {
    'missions.community_goals': {
      as_of: CLOCK,
      goals: [{
        cgid: 1, title: 'Goal', system: 'Sol', station: null, contribution: 5000, percentile: 25, tier: 'Tier 2',
        expiry: '2030-01-03T12:00:00Z', bonus: 1000000, complete: false,
      }],
    },
  });
  expect(q('missions-cg-remaining')!.textContent).toContain('2 d');
  expect(q('cg-goal')!.textContent).toContain('1.000.000');
});

test('rewards show signed faction scores', () => {
  mountWith(RewardsPanel, {
    'missions.rewards': {
      as_of: CLOCK, credits: 6300000, completed: 7, failed: 1, abandoned: 2,
      materials: { 'Modular Terminals': 3 },
      factions: { 'Liberty Party': { reputation: 'UpGood', influence: 'Down', reputation_score: 2, influence_score: -1 } },
      since: CLOCK,
    },
  });
  const t = q('rewards-factions')!.textContent!;
  expect(t).toContain('Liberty Party');
  expect(t).toContain('+2');
  expect(t).toContain('-1');
  expect(q('rewards-summary')!.textContent).toContain('6.300.000');
  expect(q('rewards-materials')!.textContent).toContain('Modular Terminals');
});
