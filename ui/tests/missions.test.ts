import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import MissionsPanel from '../src/modules/missions/MissionsPanel.svelte';
import MassacrePanel from '../src/modules/missions/MassacrePanel.svelte';
import MissionAlerts from '../src/modules/missions/MissionAlerts.svelte';
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
