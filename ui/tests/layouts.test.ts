import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { autoLayout, contextPanel, layouts, nextPin } from '../src/shell/layouts';
import { ageShort, starClassLabel } from '../src/lib/format';
import { setLocale } from '../src/lib/i18n';
import Panel from '../src/shell/Panel.svelte';
import App from '../src/App.svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
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
    instance = mount(App, { target });
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
    instance = mount(App, { target }); flushSync();
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
});
