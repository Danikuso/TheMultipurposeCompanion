import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount, type Component } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale, t } from '../src/lib/i18n';
import RouteStrip from '../src/modules/navigation/RouteStrip.svelte';
import JumpPanel from '../src/modules/navigation/JumpPanel.svelte';

function remoteOk(extra: Record<string, unknown> = {}, source = 'Spansh') {
  return {
    status: 'ok',
    source,
    fetched_at: new Date(Date.now() - 125_000).toISOString(),
    data: {
      id64: 1,
      name: 'X',
      discovered: true,
      body_count: 7,
      primary_star: 'K',
      allegiance: 'Federation',
      government: 'Democracy',
      controlling_faction: 'Test Faction',
      faction_state: 'Boom',
      economy: 'Industrial',
      second_economy: 'Refinery',
      security: 'High',
      population: 1234567,
      powers: ['Test Power'],
      powerplay_state: 'Fortified',
      ...extra,
    },
  };
}

function card(name: string, over: Record<string, unknown> = {}) {
  return {
    name,
    id64: 100,
    jump_ly: 12.34,
    star_class: 'K',
    scoopable: true,
    special: null,
    fuel_after_pct: 81,
    reachable: true,
    visits: 3,
    remote: remoteOk(),
    local_politics: null,
    ...over,
  };
}

function routeTopic(over: Record<string, unknown> = {}) {
  return {
    source: 'plan',
    current: card('Currentia', { jump_ly: null }),
    next: card('Nextara'),
    following: [card('Followa'), card('Followb', { special: 'neutron', star_class: 'N' }), card('Followc')],
    remaining_jumps: 9,
    total_ly: 321.5,
    off_route: false,
    copy_target: 'Nextara',
    copy_seq: 0,
    ...over,
  };
}

describe('navigation panels', () => {
  let target: HTMLElement;
  let mounted: Record<string, unknown>[] = [];

  function render(Comp: Component) {
    const inst = mount(Comp, { target });
    flushSync();
    mounted.push(inst);
  }

  beforeEach(() => {
    disconnect();
    setLocale('es');
    target = document.createElement('div');
    document.body.appendChild(target);
    mounted = [];
  });

  afterEach(() => {
    for (const m of mounted) unmount(m);
    target.remove();
    vi.unstubAllGlobals();
  });

  test('route strip renders next star card and three following', () => {
    live.topics['navigation.route'] = routeTopic();
    render(RouteStrip);
    expect(target.querySelectorAll('[data-testid="next-card"], [data-testid="small-card"]').length).toBe(4);
    expect(target.querySelector('[data-testid="next-card"]')?.textContent).toContain('Nextara');
    const small = [...target.querySelectorAll('[data-testid="small-card"]')].map((e) => e.textContent ?? '');
    expect(small.length).toBe(3);
    expect(small[0]).toContain('Followa');
    expect(small[1]).toContain('Followb');
    expect(small[1]).toContain(t('strip-neutron'));
    expect(target.textContent).toContain('12.34');
    expect(target.textContent).toContain('Federation');
    expect(target.querySelector('section')?.style.gridColumn).toBe('1 / -1');
  });

  test('remote missing shows no-data without hiding local fields', () => {
    live.topics['navigation.route'] = routeTopic({
      next: card('Nextara', { remote: { status: 'failed', source: null, fetched_at: null, data: null } }),
    });
    render(RouteStrip);
    const next = target.querySelector('[data-testid="next-card"]')?.textContent ?? '';
    expect(next).toContain(t('empty-no-data'));
    expect(next).toContain('12.34');
    expect(next).toContain('Nextara');
  });

  test('war and outbreak states are highlighted', () => {
    live.topics['navigation.route'] = routeTopic({
      next: card('Nextara', { remote: remoteOk({ faction_state: 'War' }) }),
      following: [
        card('Followa', { remote: remoteOk({ faction_state: 'Outbreak' }) }),
        card('Followb', { remote: remoteOk({ faction_state: 'CivilUnrest' }) }),
        card('Followc', { remote: remoteOk({ faction_state: 'Boom' }) }),
      ],
    });
    render(RouteStrip);
    const danger = [...target.querySelectorAll('[data-testid="faction-state"]')].map((e) => ({
      text: e.textContent?.trim(),
      danger: e.classList.contains('danger'),
    }));
    expect(danger).toEqual([
      { text: 'War', danger: true },
      { text: 'Outbreak', danger: true },
      { text: 'CivilUnrest', danger: true },
      { text: 'Boom', danger: false },
    ]);
  });

  test('cache age is shown only for cached remote', () => {
    live.topics['navigation.route'] = routeTopic({
      next: card('Nextara', { remote: remoteOk({}, 'Cache') }),
    });
    render(RouteStrip);
    expect(target.querySelector('[data-testid="next-card"]')?.textContent).toContain('2 min');
    expect(target.querySelectorAll('[data-testid="cache-age"]').length).toBe(1);
  });

  test('off-route badge shows recalculate button that calls action', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
    vi.stubGlobal('fetch', fetchMock);
    live.topics['navigation.route'] = routeTopic({ off_route: true });
    render(RouteStrip);
    expect(target.querySelector('[data-testid="off-route"]')?.textContent).toContain(t('strip-off-route'));
    const btn = target.querySelector('[data-testid="recalculate"]') as HTMLButtonElement;
    expect(btn.textContent).toContain(t('strip-recalculate'));
    btn.click();
    await Promise.resolve();
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/actions/navigation.recalculate',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  test('no off-route badge when on route', () => {
    live.topics['navigation.route'] = routeTopic();
    render(RouteStrip);
    expect(target.querySelector('[data-testid="off-route"]')).toBeNull();
  });

  test('jump panel shows range, fuel and supercharge', () => {
    live.topics['navigation.jump'] = {
      range_ly: 24.6,
      range_full_ly: 31.2,
      source: 'formula',
      fuel_main: 12.5,
      fuel_capacity: 32,
      max_fuel_per_jump: 5.2,
      supercharged: 4,
    };
    render(JumpPanel);
    const txt = target.textContent ?? '';
    expect(txt).toContain('24.6');
    expect(txt).toContain('31.2');
    expect(txt).toContain('12.5');
    expect(txt).toContain('5.2');
    expect(txt).toContain('×4');
    expect(txt).not.toContain(t('empty-no-data'));
  });

  test('jump panel with unknown source shows no-data', () => {
    live.topics['navigation.jump'] = {
      range_ly: null,
      range_full_ly: null,
      source: 'unknown',
      fuel_main: null,
      fuel_capacity: null,
      max_fuel_per_jump: null,
      supercharged: null,
    };
    render(JumpPanel);
    expect(target.textContent).toContain(t('empty-no-data'));
    expect(target.textContent).not.toContain('×');
  });
});
