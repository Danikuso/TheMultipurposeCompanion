import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount, type Component } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale, t } from '../src/lib/i18n';
import RouteStrip from '../src/modules/navigation/RouteStrip.svelte';
import JumpPanel from '../src/modules/navigation/JumpPanel.svelte';
import PlannerPanel from '../src/modules/navigation/PlannerPanel.svelte';

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
    expect(target.textContent).toContain('12,34');
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
    expect(next).toContain('12,34');
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
    expect(target.querySelector('[data-testid="next-card"]')?.textContent).toContain('2min');
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
    expect(txt).toContain('24,6 ly');
    expect(txt).toContain('31,2 ly');
    expect(txt).toContain('12,5 / 32,0 t');
    expect(txt).toContain('5,2 t');
    expect(txt).toContain('×4');
    expect(txt).not.toContain(t('empty-no-data'));
  });

  test('jump panel formats numbers with the active locale', () => {
    setLocale('en');
    live.topics['navigation.jump'] = {
      range_ly: 24.6,
      range_full_ly: 31.2,
      source: 'formula',
      fuel_main: 12.5,
      fuel_capacity: 32,
      max_fuel_per_jump: 5.2,
      supercharged: null,
    };
    render(JumpPanel);
    expect(target.textContent).toContain('24.6 ly');
    expect(target.textContent).toContain('12.5 / 32.0 t');
    expect(target.querySelector('[data-testid="supercharged"]')).toBeNull();
  });

  test('falls back to local politics when remote is not ok', () => {
    const pol = { allegiance: 'Empire', government: 'Patronage', faction_state: 'CivilWar', population: 5000 };
    live.topics['navigation.route'] = routeTopic({
      next: card('Nextara', {
        remote: { status: 'pending', source: null, fetched_at: null, data: null },
        local_politics: pol,
      }),
    });
    render(RouteStrip);
    const next = target.querySelector('[data-testid="next-card"]')!;
    expect(next.textContent).toContain('Empire');
    expect(next.textContent).toContain('Patronage');
    const st = next.querySelector('[data-testid="faction-state"]')!;
    expect(st.textContent?.trim()).toBe('CivilWar');
    expect(st.classList.contains('danger')).toBe(true);
    expect(next.querySelector('[role="img"]')?.getAttribute('aria-label')).toBe(t('strip-state-danger'));
    // exploración remota no disponible
    expect(next.textContent).toContain(t('empty-no-data'));
  });

  test('remote disabled and pending without local politics show no-data in politics', () => {
    for (const status of ['pending', 'disabled']) {
      live.topics['navigation.route'] = routeTopic({
        next: card('Nextara', { remote: { status, source: null, fetched_at: null, data: null } }),
      });
      render(RouteStrip);
      const next = target.querySelector('[data-testid="next-card"]')!;
      // discovered, bodies, politics y sistema
      expect(next.textContent!.split(t('empty-no-data')).length - 1).toBe(4);
      expect(next.querySelector('[data-testid="faction-state"]')).toBeNull();
      unmount(mounted.pop()!);
    }
  });

  test('non-danger state has no warning cue', () => {
    live.topics['navigation.route'] = routeTopic();
    render(RouteStrip);
    expect(target.querySelectorAll('[role="img"]').length).toBe(0);
  });

  test('route without next shows strip-no-route and current card', () => {
    live.topics['navigation.route'] = routeTopic({ next: null, following: [], source: 'current' });
    render(RouteStrip);
    expect(target.textContent).toContain(t('strip-no-route'));
    expect(target.querySelector('[data-testid="next-card"]')?.textContent).toContain('Currentia');
    expect(target.querySelectorAll('[data-testid="small-card"]').length).toBe(0);
  });

  test('unreachable next star is flagged', () => {
    live.topics['navigation.route'] = routeTopic({ next: card('Nextara', { reachable: false }) });
    render(RouteStrip);
    const txt = target.querySelector('[data-testid="next-card"]')!.textContent!;
    expect(txt).toContain(t('strip-unreachable'));
    expect(txt).not.toContain(t('strip-reachable'));
  });

  test('no duplicate data-testid inside one card', () => {
    live.topics['navigation.route'] = routeTopic({
      next: card('Nextara', { special: 'neutron', remote: remoteOk({ faction_state: 'War' }, 'Cache') }),
    });
    render(RouteStrip);
    for (const sel of ['[data-testid="next-card"]', '[data-testid="small-card"]']) {
      for (const el of target.querySelectorAll(sel)) {
        const ids = [...el.querySelectorAll('[data-testid]')].map((e) => e.getAttribute('data-testid'));
        expect(new Set(ids).size).toBe(ids.length);
      }
    }
    expect(target.querySelectorAll('[data-testid="next-card"] [data-testid="special"]').length).toBe(1);
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

  describe('planner panel', () => {
    function planTopic(over: Record<string, unknown> = {}) {
      return { status: 'idle', request: null, result: null, active: null, error_key: null, ...over };
    }
    function typeIn(el: HTMLInputElement, value: string) {
      el.value = value;
      el.dispatchEvent(new Event('input', { bubbles: true }));
      flushSync();
    }
    function fetchRouter(calls: string[]) {
      return vi.fn(async (url: string, init?: RequestInit) => {
        calls.push(`${url}|${init?.body ?? ''}`);
        if (url.startsWith('/api/navigation/autocomplete')) {
          return {
            ok: true,
            json: async () => ({ items: [{ name: 'Colonia', id64: 3238296097059 }], page: 0, more: false }),
          };
        }
        return { ok: true, json: async () => ({}) };
      });
    }

    afterEach(() => {
      vi.useRealTimers();
    });

    test('autocomplete debounces and requires 3 chars', async () => {
      vi.useFakeTimers();
      const calls: string[] = [];
      vi.stubGlobal('fetch', fetchRouter(calls));
      live.topics['navigation.plan'] = planTopic();
      render(PlannerPanel);
      const input = target.querySelector('[data-testid="plan-to"]') as HTMLInputElement;
      typeIn(input, 'Co');
      await vi.advanceTimersByTimeAsync(400);
      expect(calls.length).toBe(0);
      typeIn(input, 'Col');
      await vi.advanceTimersByTimeAsync(299);
      expect(calls.length).toBe(0);
      await vi.advanceTimersByTimeAsync(1);
      expect(calls.length).toBe(1);
      expect(calls[0]).toContain('/api/navigation/autocomplete?q=Col&page=0');
      flushSync();
      const opts = [...target.querySelectorAll('[data-testid="suggestion"]')];
      expect(opts.map((o) => o.textContent?.trim())).toEqual(['Colonia']);
    });

    test('submit sends navigation.plan with mode, efficiency and picked id64', async () => {
      vi.useFakeTimers();
      const calls: string[] = [];
      vi.stubGlobal('fetch', fetchRouter(calls));
      live.topics['navigation.plan'] = planTopic();
      render(PlannerPanel);
      const input = target.querySelector('[data-testid="plan-to"]') as HTMLInputElement;
      typeIn(input, 'Col');
      await vi.advanceTimersByTimeAsync(300);
      flushSync();
      (target.querySelector('[data-testid="suggestion"]') as HTMLElement).click();
      flushSync();
      (target.querySelector('[data-testid="mode-galaxy"]') as HTMLInputElement).click();
      flushSync();
      typeIn(target.querySelector('[data-testid="plan-efficiency"]') as HTMLInputElement, '80');
      (target.querySelector('[data-testid="plan-submit"]') as HTMLButtonElement).click();
      await vi.advanceTimersByTimeAsync(0);
      const planCall = calls.find((c) => c.startsWith('/api/actions/navigation.plan'))!;
      const body = JSON.parse(planCall.split('|')[1]);
      expect(body.to).toBe('Colonia');
      expect(body.to_id64).toBe(3238296097059);
      expect(body.mode).toBe('galaxy');
      expect(body.efficiency).toBe(80);
      expect(body.use_ship).toBe(true);
      expect(body.from).toBeUndefined();
      expect(body.from_id64).toBeUndefined();
    });

    test('computing state is text only', () => {
      live.topics['navigation.plan'] = planTopic({ status: 'computing' });
      render(PlannerPanel);
      expect(target.querySelector('[data-testid="plan-computing"]')?.textContent).toContain(
        t('nav-plan-computing'),
      );
    });

    test('ready result lists hops and accept calls action', async () => {
      const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
      vi.stubGlobal('fetch', fetchMock);
      live.topics['navigation.plan'] = planTopic({
        status: 'ready',
        result: {
          total_ly: 1234.5,
          hops_total: 60,
          hops: [
            { name: 'Alpha', id64: 1, distance_ly: 50.25, jumps: 2, neutron: true, refuel: null, star_class: 'N' },
            { name: 'Beta', id64: 2, distance_ly: 40, jumps: 1, neutron: false, refuel: true, star_class: 'K' },
          ],
        },
      });
      render(PlannerPanel);
      const rows = [...target.querySelectorAll('[data-testid="hop"]')];
      expect(rows.length).toBe(2);
      expect(rows[0].textContent).toContain('Alpha');
      expect(rows[0].textContent).toContain('50,25 ly');
      expect(rows[0].textContent).toContain(t('strip-neutron'));
      expect(rows[0].textContent).toContain('—');
      expect(rows[1].textContent).toContain(t('nav-plan-refuel-yes'));
      expect(target.querySelector('[data-testid="plan-total"]')?.textContent).toContain('1234,50 ly');
      expect(target.querySelector('[data-testid="plan-more"]')?.textContent).toContain(
        t('nav-plan-more', { count: 58 }),
      );
      (target.querySelector('[data-testid="plan-accept"]') as HTMLButtonElement).click();
      await Promise.resolve();
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/actions/navigation.accept-plan',
        expect.objectContaining({ method: 'POST' }),
      );
    });

    test('clear plan calls action when active', async () => {
      const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
      vi.stubGlobal('fetch', fetchMock);
      live.topics['navigation.plan'] = planTopic({
        status: 'active',
        active: { destination: 'Colonia', current_index: 3, total: 10, off_route: false },
      });
      render(PlannerPanel);
      expect(target.textContent).toContain('Colonia');
      (target.querySelector('[data-testid="plan-clear"]') as HTMLButtonElement).click();
      await Promise.resolve();
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/actions/navigation.clear-plan',
        expect.objectContaining({ method: 'POST' }),
      );
    });

    test('failed plan shows translated error', () => {
      live.topics['navigation.plan'] = planTopic({ status: 'failed', error_key: 'nav-plan-error-no-route' });
      render(PlannerPanel);
      expect(target.querySelector('[data-testid="plan-error"]')?.textContent).toContain(
        'No se encontró ninguna ruta.',
      );
    });

    test('disabled planner explains why', () => {
      live.topics['navigation.plan'] = planTopic({ status: 'disabled', error_key: 'nav-plan-disabled-offline' });
      render(PlannerPanel);
      expect(target.querySelector('[data-testid="plan-error"]')?.textContent).toContain(
        'El planificador necesita los servicios en línea activados.',
      );
      expect((target.querySelector('[data-testid="plan-to"]') as HTMLInputElement).disabled).toBe(true);
      expect((target.querySelector('[data-testid="plan-submit"]') as HTMLButtonElement).disabled).toBe(true);
    });
  });
});
