import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import TargetPanel from '../src/modules/combat/TargetPanel.svelte';
import ShipPanel from '../src/modules/combat/ShipPanel.svelte';
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
  mounted = undefined; target.remove(); vi.useRealTimers();
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
  unmount(mounted!);
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
