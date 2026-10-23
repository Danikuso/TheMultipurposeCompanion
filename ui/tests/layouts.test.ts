import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { autoLayout, contextPanel, layouts, nextPin } from '../src/shell/layouts';
import { ageShort, starClassLabel } from '../src/lib/format';
import { setLocale } from '../src/lib/i18n';
import Panel from '../src/shell/Panel.svelte';
import App from '../src/App.svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { loadPrefs, prefs } from '../src/lib/prefs.svelte';
import { RouteStrip, PlannerPanel, JumpPanel } from '../src/modules/navigation';
import SystemBodiesPanel from '../src/modules/exploration/SystemBodiesPanel.svelte';
import ExobiologyPanel from '../src/modules/exobiology/ExobiologyPanel.svelte';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

describe('layouts', () => {
  test('exploration layout puts route strip first at full width', () => {
    const layout = layouts.find((layout) => layout.id === 'exploration');
    expect(layout).toBeDefined();
    expect(layout?.titleKey).toBe('layout-exploration');
    expect(layout?.icon).toBe('compass');
    expect(layout?.slots[0]).toMatchObject({ panel: RouteStrip, span: 'full' });
    expect(layout?.slots).toEqual(expect.arrayContaining([
      expect.objectContaining({ panel: PlannerPanel, span: 1 }),
      expect.objectContaining({ panel: JumpPanel, span: 1 }),
    ]));
  });

  test('context slot switches to exobiology when it becomes active', () => {
    const layout = layouts.find((layout) => layout.id === 'exploration');
    expect(layout).toBeDefined();
    const visible = (relevance: string, manual: string | null = null) => layout!.slots
      .filter((slot) => !slot.when || slot.when({ modules: [{ id: 'exobiology', relevance }], manual }))
      .map((slot) => slot.panel);
    expect(visible('normal')).toContain(SystemBodiesPanel);
    expect(visible('normal')).not.toContain(ExobiologyPanel);
    expect(visible('active')).toContain(ExobiologyPanel);
    expect(visible('active')).not.toContain(SystemBodiesPanel);
    expect(visible('urgent', 'exploration')).toContain(SystemBodiesPanel);
    expect(visible('normal', 'exobiology')).toContain(ExobiologyPanel);
  });
  test('contextPanel prefers manual, then active exobiology, else exploration', () => {
    const mods = [
      { id: 'exploration', relevance: 'normal' },
      { id: 'exobiology', relevance: 'active' },
    ];
    expect(contextPanel(mods, null)).toBe('exobiology');
    expect(contextPanel(mods, 'exploration')).toBe('exploration');
    expect(contextPanel([{ id: 'exobiology', relevance: 'urgent' }], null)).toBe('exobiology');
    expect(contextPanel([{ id: 'exobiology', relevance: 'normal' }], null)).toBe('exploration');
    expect(contextPanel([], null)).toBe('exploration');
    expect(contextPanel([], 'exobiology')).toBe('exobiology');
    expect(Array.isArray(layouts)).toBe(true);
  });

  test('format helpers', () => {
    setLocale('es');
    expect(ageShort(45)).toBe('45s');
    expect(ageShort(125)).toBe('2min');
    expect(ageShort(7300)).toBe('2h');
    expect(ageShort(200000)).toBe('2d');
    expect(ageShort(-5)).toBe('0s');
    setLocale('en');
    expect(ageShort(125)).toBe('2m');
    expect(ageShort(45)).toBe('45s');
    setLocale('es');
    expect(starClassLabel('K')).toBe('K');
    expect(starClassLabel('da')).toBe('DA');
    expect(starClassLabel('O_BLUE')).toBe('O');
    expect(starClassLabel(null)).toBe('—');
  });
});

describe('default exploration layout', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  beforeEach(() => {
    disconnect();
    setLocale('es');
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 1' };
    live.topics['core.modules'] = [
      { id: 'exploration', relevance: 'normal' },
      { id: 'exobiology', relevance: 'normal' },
      { id: 'navigation', relevance: 'hidden' },
    ];
    target = document.createElement('div');
    document.body.appendChild(target);
    prefs.loaded = true; instance = mount(App, { target });
    flushSync();
  });
  afterEach(() => { unmount(instance); target.remove(); disconnect(); });

  test('defaults to exploration and keeps all four modules manually accessible', () => {
    expect(target.querySelector('[data-layout-slot="route"]')).not.toBeNull();
    expect(target.querySelector('[data-layout-slot="context-bodies"]')).not.toBeNull();
    const buttons = Array.from(target.querySelectorAll('aside button')) as HTMLButtonElement[];
    expect(buttons[0].textContent).toContain('Exploración');
    for (const id of ['dashboard', 'navigation', 'exploration', 'exobiology']) {
      const button = target.querySelector(`[data-module="${id}"]`) as HTMLButtonElement;
      expect(button).not.toBeNull();
      button.click(); flushSync();
      expect(target.querySelector('[data-layout-slot]')).toBeNull();
      expect(target.querySelector('main section')).not.toBeNull();
    }
  });

  test('manual context remains through status updates and resets on the next body change', () => {
    live.topics['core.modules'] = [{ id: 'exobiology', relevance: 'active' }]; flushSync();
    expect(target.querySelector('[data-layout-slot="context-exobiology"]')).not.toBeNull();
    (target.querySelector('[data-testid="context-bodies"]') as HTMLButtonElement).click(); flushSync();
    expect(target.querySelector('[data-layout-slot="context-bodies"]')).not.toBeNull();
    live.topics['core.status'] = { flags: 0, fuel: 2 }; flushSync();
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 1', mode: 'Landed' }; flushSync();
    expect(target.querySelector('[data-layout-slot="context-bodies"]')).not.toBeNull();
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 2' }; flushSync();
    expect(target.querySelector('[data-layout-slot="context-exobiology"]')).not.toBeNull();
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 1' }; flushSync();
    expect(target.querySelector('[data-layout-slot="context-exobiology"]')).not.toBeNull();
  });

  test.each([null, 'Testia 1'])('body id resets manual context when core body is %s', (bodyName) => {
    const body = (id: number) => ({ system_address: 1, body_id: id, body: `Testia ${id}`,
      candidates: [], confirmed_genera: [], bio_signals: 1, completed: 0 });
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: bodyName };
    live.topics['exobiology.body'] = body(1);
    live.topics['core.modules'] = [{ id: 'exobiology', relevance: 'active' }]; flushSync();
    (target.querySelector('[data-testid="context-bodies"]') as HTMLButtonElement).click(); flushSync();
    live.topics['exobiology.body'] = { ...body(1), bio_signals: 2 }; flushSync();
    expect(target.querySelector('[data-layout-slot="context-bodies"]')).not.toBeNull();
    live.topics['exobiology.body'] = body(2); flushSync();
    expect(target.querySelector('[data-layout-slot="context-exobiology"]')).not.toBeNull();
  });

  test('late core body name does not reset manual selection already made for that body id', () => {
    const body = (id: number) => ({ system_address: 1, body_id: id, body: `Testia ${id}`,
      candidates: [], confirmed_genera: [], bio_signals: 1, completed: 0 });
    live.topics['exobiology.body'] = body(1);
    live.topics['core.modules'] = [{ id: 'exobiology', relevance: 'active' }]; flushSync();
    (target.querySelector('[data-testid="context-bodies"]') as HTMLButtonElement).click(); flushSync();
    live.topics['exobiology.body'] = body(2); flushSync();
    expect(target.querySelector('[data-layout-slot="context-exobiology"]')).not.toBeNull();
    (target.querySelector('[data-testid="context-bodies"]') as HTMLButtonElement).click(); flushSync();
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 2' }; flushSync();
    expect(target.querySelector('[data-layout-slot="context-bodies"]')).not.toBeNull();
  });
});

describe('Panel span and size', () => {
  let target: HTMLElement;
  let inst: Record<string, unknown> | null = null;
  beforeEach(() => {
    setLocale('es');
    target = document.createElement('div');
    document.body.appendChild(target);
  });
  afterEach(() => {
    if (inst) unmount(inst);
    inst = null;
    target.remove();
  });

  test('full span panel spans all grid columns', () => {
    inst = mount(Panel, { target, props: { titleKey: 'panel-route', span: 'full', size: 'large' } });
    flushSync();
    const el = target.querySelector('section') as HTMLElement;
    expect(el.style.gridColumn).toBe('1 / -1');
    expect(el.classList.contains('large')).toBe(true);
  });

  test('numeric span uses span N', () => {
    inst = mount(Panel, { target, props: { titleKey: 'panel-route', span: 2 } });
    flushSync();
    const el = target.querySelector('section') as HTMLElement;
    expect(el.style.gridColumn).toBe('span 2');
    expect(el.classList.contains('large')).toBe(false);
  });
});

describe('combat and mining layouts', () => {
  beforeEach(() => { disconnect(); });
  afterEach(() => { disconnect(); });
  const visible = (id: string) => layouts.find((l) => l.id === id)!.slots
    .filter((slot) => !slot.when || slot.when({ modules: [], manual: null }));

  test('combat active switches to combat layout', () => {
    expect(autoLayout([{ id: 'combat', relevance: 'active' }], null)).toBe('combat');
    expect(autoLayout([{ id: 'combat', relevance: 'urgent' }, { id: 'mining', relevance: 'active' }], null)).toBe('combat');
    expect(autoLayout([{ id: 'mining', relevance: 'active' }], null)).toBe('mining');
    expect(autoLayout([{ id: 'combat', relevance: 'normal' }], null)).toBeNull();
  });
  test('manual choice wins', () => {
    expect(autoLayout([{ id: 'combat', relevance: 'active' }], 'mining')).toBe('mining');
  });
  test('ax and cz slots hidden when not applicable', () => {
    live.topics['combat.ax'] = { active: false };
    live.topics['combat.conflict_zone'] = null;
    expect(visible('combat')).toHaveLength(3);
    live.topics['combat.ax'] = { active: true };
    live.topics['combat.conflict_zone'] = { system: 'X', side: null, kills: 0, bonds_by_faction: {} };
    expect(visible('combat')).toHaveLength(5);
    expect(visible('mining')).toHaveLength(3);
  });
});

describe('nextPin', () => {
  test('pin holds while auto is unchanged, clears when auto changes', () => {
    expect(nextPin('combat', 'combat', 'layout:mining')).toBe('layout:mining');
    expect(nextPin('combat', null, 'layout:mining')).toBeNull();
    expect(nextPin(null, 'combat', 'layout:mining')).toBeNull();
  });
});

describe('app pin behaviour', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  beforeEach(() => {
    disconnect(); setLocale('es');
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }];
    target = document.createElement('div'); document.body.appendChild(target);
    prefs.loaded = true; instance = mount(App, { target }); flushSync();
  });
  afterEach(() => { unmount(instance); target.remove(); disconnect(); });
  const slot = (id: string) => target.querySelector(`[data-layout-slot="${id}"]`);
  const nav = (label: string) => (Array.from(target.querySelectorAll('aside button')) as HTMLButtonElement[])
    .find((b) => b.textContent?.includes(label))!;

  test('manual mining stays while combat active, auto returns when combat ends; settings does not pin', () => {
    expect(slot('target')).not.toBeNull();
    nav('Minería').click(); flushSync();
    expect(slot('prospector')).not.toBeNull();
    live.topics['core.status'] = { flags: 0 }; flushSync();
    expect(slot('prospector')).not.toBeNull();
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'normal' }]; flushSync();
    expect(slot('prospector')).toBeNull();
    expect(slot('route')).not.toBeNull();
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }]; flushSync();
    nav('Ajustes').click(); flushSync();
    expect(slot('target')).toBeNull();
    nav('Combate').click(); flushSync();
    expect(slot('target')).not.toBeNull();
  });

  test('manual trade pin stays while auto missions is stable, drops when auto becomes combat', () => {
    live.topics['core.modules'] = [{ id: 'missions', relevance: 'active' }]; flushSync();
    expect(slot('mission-alerts')).not.toBeNull();
    nav('Comercio').click(); flushSync();
    expect(slot('trade-session')).not.toBeNull();
    live.topics['core.status'] = { flags: 0 }; flushSync();
    expect(slot('trade-session')).not.toBeNull();
    live.topics['core.modules'] = [{ id: 'missions', relevance: 'active' }, { id: 'combat', relevance: 'active' }]; flushSync();
    expect(slot('trade-session')).toBeNull();
    expect(slot('target')).not.toBeNull();
  });
});

describe('trade and missions layouts', () => {
  beforeEach(() => { disconnect(); });
  afterEach(() => { disconnect(); });
  const m = (id: string, relevance: string) => ({ id, relevance });
  const slotIds = (id: string) => layouts.find((l) => l.id === id)!.slots
    .filter((s) => !s.when || s.when({ modules: [], manual: null })).map((s) => s.id);

  test('missions beats trade', () => {
    expect(autoLayout([m('missions', 'active'), m('trade', 'active')], null)).toBe('missions');
  });
  test('mining beats missions', () => {
    expect(autoLayout([m('mining', 'active'), m('missions', 'active')], null)).toBe('mining');
  });
  test('combat beats everything automatic', () => {
    expect(autoLayout([m('combat', 'urgent'), m('mining', 'active'), m('missions', 'active'), m('trade', 'active')], null)).toBe('combat');
  });
  test('trade beats exploration', () => {
    expect(autoLayout([m('trade', 'active'), m('exploration', 'active')], null)).toBe('trade');
  });
  test('manual pin lasts until auto changes', () => {
    expect(autoLayout([m('missions', 'active')], 'trade')).toBe('trade');
    expect(nextPin('missions', 'missions', 'layout:trade')).toBe('layout:trade');
    expect(nextPin('missions', 'combat', 'layout:trade')).toBeNull();
  });
  test('trade layout slots', () => {
    expect(slotIds('trade')).toEqual(['market', 'cargo', 'trade-session', 'trade-route', 'search']);
  });
  test('missions optional slots follow topics', () => {
    expect(slotIds('missions')).toEqual(['missions', 'mission-alerts', 'rewards']);
    live.topics['missions.massacre'] = { piles: [{}] };
    live.topics['missions.passengers'] = { cabins: { seats_total: 4 }, groups: [] };
    live.topics['missions.tourism'] = { trips: [{}] };
    live.topics['missions.community_goals'] = { goals: [{}] };
    expect(slotIds('missions')).toEqual(['missions', 'mission-alerts', 'massacre', 'passengers', 'tourism', 'rewards', 'community-goals']);
    live.topics['missions.massacre'] = { piles: [] };
    live.topics['missions.passengers'] = { cabins: { seats_total: 0 }, groups: [{}] };
    live.topics['missions.tourism'] = { trips: [] };
    expect(slotIds('missions')).toEqual(['missions', 'mission-alerts', 'passengers', 'rewards', 'community-goals']);
  });
});

describe('auto-companion OFF', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  const slot = (id: string) => target.querySelector(`[data-layout-slot="${id}"]`);
  beforeEach(() => {
    disconnect(); setLocale('es'); prefs.autoCompanion = true;
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 1' };
    live.topics['core.modules'] = [{ id: 'exploration', relevance: 'normal' }, { id: 'exobiology', relevance: 'normal' }];
    target = document.createElement('div'); document.body.appendChild(target);
    prefs.loaded = true; instance = mount(App, { target }); flushSync();
  });
  afterEach(() => { unmount(instance); target.remove(); disconnect(); prefs.autoCompanion = true; });

  test('ON follows relevance (current behaviour)', () => {
    expect(slot('route')).not.toBeNull();
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }]; flushSync();
    expect(slot('target')).not.toBeNull();
  });

  test('OFF keeps layout when engineering becomes active, ON follows it', () => {
    prefs.autoCompanion = false; flushSync();
    live.topics['core.modules'] = [{ id: 'engineering', relevance: 'active' }]; flushSync();
    expect(slot('route')).not.toBeNull();
    expect(slot('materials')).toBeNull();
    prefs.autoCompanion = true; flushSync();
    expect(slot('materials')).not.toBeNull();
    expect(slot('route')).toBeNull();
  });

  test('OFF keeps the layout when relevance changes, manual navigation still works', () => {
    prefs.autoCompanion = false; flushSync();
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }]; flushSync();
    expect(slot('target')).toBeNull();
    expect(slot('route')).not.toBeNull();
    const combat = Array.from(target.querySelectorAll('aside button')).find((b) => b.textContent?.includes('Combate')) as HTMLButtonElement;
    combat.click(); flushSync();
    expect(slot('target')).not.toBeNull();
    live.topics['core.modules'] = [{ id: 'mining', relevance: 'active' }]; flushSync();
    expect(slot('target')).not.toBeNull();
    expect(slot('prospector')).toBeNull();
  });

  test('OFF keeps the context panel when exobiology becomes active, even across body changes', () => {
    prefs.autoCompanion = false; flushSync();
    live.topics['core.modules'] = [{ id: 'exobiology', relevance: 'active' }]; flushSync();
    expect(slot('context-bodies')).not.toBeNull();
    expect(slot('context-exobiology')).toBeNull();
    (target.querySelector('[data-testid="context-exobiology"]') as HTMLButtonElement).click(); flushSync();
    live.topics['core.location'] = { system: 'Testia', system_address: 1, body: 'Testia 2' };
    live.topics['core.modules'] = [{ id: 'exobiology', relevance: 'normal' }]; flushSync();
    expect(slot('context-exobiology')).not.toBeNull();
  });

  test('OFF with a manual pin keeps it and OFF->ON engages the auto layout immediately', () => {
    prefs.autoCompanion = false; flushSync();
    const nav = (label: string) => (Array.from(target.querySelectorAll('aside button')) as HTMLButtonElement[])
      .find((b) => b.textContent?.includes(label))!;
    nav('Minería').click(); flushSync();
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }]; flushSync();
    expect(slot('prospector')).not.toBeNull();
    prefs.autoCompanion = true; flushSync();
    expect(slot('target')).not.toBeNull();
    expect(slot('prospector')).toBeNull();
  });
});

describe('engineering layout', () => {
  beforeEach(() => disconnect());
  afterEach(() => disconnect());
  const m = (id: string, relevance = 'active') => ({ id, relevance });

  test.each(['active', 'urgent', 'Active', 'Urgent'])('engineering %s beats missions, trade and exploration', (relevance) => {
    expect(autoLayout([m('engineering', relevance), m('missions'), m('trade'), m('exploration')], null)).toBe('engineering');
  });
  test('combat and mining beat engineering; manual still wins', () => {
    expect(autoLayout([m('engineering'), m('combat', 'urgent')], null)).toBe('combat');
    expect(autoLayout([m('engineering', 'urgent'), m('mining')], null)).toBe('mining');
    expect(autoLayout([m('engineering'), m('mining', 'urgent')], null)).toBe('engineering');
    expect(autoLayout([m('engineering', 'normal')], null)).toBeNull();
    expect(autoLayout([m('combat')], 'engineering')).toBe('engineering');
    expect(autoLayout([m('engineering')], 'trade')).toBe('trade');
  });
  test('engineering slots and flow visibility follow selected slot, including idle drafts', () => {
    const layout = layouts.find((l) => l.id === 'engineering')!;
    expect(layout).toMatchObject({ titleKey: 'layout-engineering', icon: 'wrench' });
    expect(layout.slots.map(({ id, span }) => [id, span])).toEqual([
      ['materials', 2], ['engineers', 1], ['eng-alerts', 1], ['eng-ship', 1],
      ['eng-flow', 2], ['shopping', 2], ['trades', 1], ['sources', 1],
    ]);
    const flow = layout.slots.find((s) => s.id === 'eng-flow')!;
    const ctx = { modules: [], manual: null };
    expect(flow.when!(ctx)).toBe(false);
    live.topics['engineering.flow'] = { status: 'idle', slot: null };
    expect(flow.when!(ctx)).toBe(false);
    live.topics['engineering.flow'] = { status: 'idle', slot: 'MainEngines' };
    expect(flow.when!(ctx)).toBe(true);
    live.topics['engineering.flow'] = { status: 'ok', slot: 'MainEngines' };
    expect(flow.when!(ctx)).toBe(true);
  });
});

describe('auto-companion startup', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  beforeEach(() => {
    disconnect(); setLocale('es'); loadPrefs({}); prefs.loaded = false;
    localStorage.setItem('tmc.ui.placement', JSON.stringify({ layout: 'mining', context: 'exobiology' }));
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, status: 200, json: async () => ({ auto_companion: false }) })));
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }];
    target = document.createElement('div'); document.body.appendChild(target);
  });
  afterEach(() => { unmount(instance); target.remove(); vi.unstubAllGlobals(); localStorage.clear(); disconnect(); loadPrefs({}); });

  test('config OFF with modules arriving first never auto-switches and restores the stored layout', async () => {
    instance = mount(App, { target }); flushSync();
    expect(target.querySelector('[data-layout-slot="target"]')).toBeNull();
    await vi.waitFor(() => expect(prefs.loaded).toBe(true));
    flushSync();
    expect(prefs.autoCompanion).toBe(false);
    expect(target.querySelector('[data-layout-slot="target"]')).toBeNull();
    expect(target.querySelector('[data-layout-slot="prospector"]')).not.toBeNull();
  });
});

