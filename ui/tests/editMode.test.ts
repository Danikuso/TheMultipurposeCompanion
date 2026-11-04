import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import { disconnect, live } from '../src/lib/ws.svelte';
import { prefs } from '../src/lib/prefs.svelte';
import App from '../src/App.svelte';
import { cancelEdit, layoutState, themedTab } from '../src/shell/layoutStore.svelte';
import { emptyDoc } from '../src/shell/layoutModel';
import { grid } from '../src/shell/gridColumns.svelte';
import { startMoveDrag } from '../src/shell/pointerDrag';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

const TAB = 'layout:engineering';
let target: HTMLElement;
let app: Record<string, unknown>;
let autoBefore = true;
let calls: { url: string; method: string }[];

const slot = (id: string) => target.querySelector(`[data-layout-slot="${id}"]`) as HTMLElement;
const slots = () => Array.from(target.querySelectorAll('[data-layout-slot]')).map((e) => e.getAttribute('data-layout-slot'));
const btn = (id: string, op: string) => slot(id).querySelector(`button[data-op="${op}"]`) as HTMLButtonElement;
const click = (el: Element | null) => { (el as HTMLElement).click(); flushSync(); };
const edit = () => click(target.querySelector('[data-testid="layout-edit"]'));
const card = (id: string) => themedTab(TAB).cards.find((c) => c.id === id)!;

beforeEach(() => {
  disconnect();
  setLocale('es');
  autoBefore = prefs.autoCompanion;
  calls = [];
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    calls.push({ url, method: init?.method ?? 'GET' });
    if (url === '/api/config') return new Response(JSON.stringify({ auto_companion: true }), { status: 200 });
    return new Response(JSON.stringify({ device_id: 'x', name: 'n', updated_at: 't', doc: {} }), { status: 200 });
  }));
  target = document.createElement('div');
  document.body.appendChild(target);
  live.topics['core.modules'] = [{ id: 'engineering', relevance: 'active' }];
  prefs.loaded = true;
  app = mount(App, { target });
  Object.assign(layoutState, { doc: emptyDoc(), draft: null, editing: false, offline: false, deviceId: 'x', ready: true });
  flushSync();
});
afterEach(() => {
  cancelEdit();
  unmount(app);
  target.remove();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  disconnect();
  prefs.autoCompanion = autoBefore;
});

describe('edit mode', () => {
  test('edit button enters edit mode and shows toolbar', () => {
    expect(target.querySelector('[data-testid="edit-toolbar"]')).toBeNull();
    edit();
    expect(layoutState.editing).toBe(true);
    expect(target.querySelector('[data-testid="edit-toolbar"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="layout-edit"]')).toBeNull();
  });

  test('arrow buttons reorder', () => {
    edit();
    expect(slots().slice(0, 2)).toEqual(['materials', 'engineers']);
    expect(btn('materials', 'up').disabled).toBe(true);
    click(btn('materials', 'down'));
    expect(slots().slice(0, 2)).toEqual(['engineers', 'materials']);
  });

  test('width and height buttons emit ops', () => {
    edit();
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(
      { top: 0, left: 0, right: 0, bottom: 160, width: 100, height: 160, x: 0, y: 0, toJSON: () => ({}) } as DOMRect,
    );
    click(btn('materials', 'width-2'));
    click(btn('materials', 'height-more'));
    expect(card('materials')).toMatchObject({ w: 2, h: 200 });
    expect(slot('materials').style.gridColumn).toContain('span');
    expect((slot('materials').querySelector('.card-content') as HTMLElement).style.height).toContain('200px');
    click(btn('materials', 'height-auto'));
    expect(card('materials').h).toBeNull();
    expect((slot('materials').querySelector('.card-content') as HTMLElement).style.height).toBe('');
  });

  test('width 3 is disabled when columns < 3', () => {
    edit();
    expect(btn('materials', 'width-3').disabled).toBe(grid.columns < 3);
  });

  test('remove hides card and cancel restores', () => {
    edit();
    click(btn('engineers', 'remove'));
    expect(slots()).not.toContain('engineers');
    click(target.querySelector('[data-testid="layout-cancel"]'));
    expect(layoutState.editing).toBe(false);
    expect(slots()).toContain('engineers');
  });

  test('done saves via commitEdit', async () => {
    edit();
    click(btn('engineers', 'remove'));
    click(target.querySelector('[data-testid="layout-done"]'));
    await vi.waitFor(() => expect(layoutState.editing).toBe(false));
    expect(calls.some((c) => c.method === 'PUT')).toBe(true);
    expect(slots()).not.toContain('engineers');
  });

  test('failed save keeps editing and shows error toast', async () => {
    layoutState.offline = true;
    edit();
    click(target.querySelector('[data-testid="layout-done"]'));
    await tick();
    await vi.waitFor(() => expect(document.body.textContent).toContain('No se pudo guardar la disposición'));
    expect(layoutState.editing).toBe(true);
  });

  test('reset tab asks for confirmation', () => {
    edit();
    click(btn('engineers', 'remove'));
    const confirm = vi.fn().mockReturnValueOnce(false).mockReturnValueOnce(true);
    vi.stubGlobal('confirm', confirm);
    click(target.querySelector('[data-testid="layout-reset"]'));
    expect(slots()).not.toContain('engineers');
    click(target.querySelector('[data-testid="layout-reset"]'));
    expect(slots()).toContain('engineers');
    expect(confirm).toHaveBeenCalledTimes(2);
  });

  test('content is not clickable while editing', () => {
    edit();
    const content = slot('materials').querySelector('.card-content') as HTMLElement;
    expect(content.inert).toBe(true);
    expect(content.classList.contains('dimmed')).toBe(true);
  });

  test('ghost shows auto pill for inactive conditional card', () => {
    unmount(app);
    live.topics['core.modules'] = [{ id: 'combat', relevance: 'active' }];
    app = mount(App, { target });
    flushSync();
    edit();
    const ax = slot('ax');
    expect(ax).not.toBeNull();
    expect(ax.classList.contains('ghost')).toBe(true);
    expect(ax.querySelector('.ph-pill')?.textContent).toBe('Aparece automáticamente');
  });

  test('conditional cards stay hidden in layout tabs and show in module tabs', () => {
    expect(slots()).not.toContain('eng-flow');
    click(target.querySelector('[data-module="engineering"]'));
    expect(slot('eng-flow')).not.toBeNull();
    expect(slot('eng-flow').classList.contains('ghost')).toBe(false);
    expect(slot('eng-flow').textContent).toContain('Elige un módulo para calcular');
    edit();
    expect(slot('eng-flow').classList.contains('ghost')).toBe(false);
    expect(slot('eng-flow').querySelector('.ph-pill')).toBeNull();
    expect(slot('eng-flow').textContent).toContain('Elige un módulo para calcular');
  });

  test('auto-companion does not switch tabs while editing', () => {
    prefs.autoCompanion = true;
    flushSync();
    edit();
    live.topics['core.modules'] = [
      { id: 'engineering', relevance: 'active' }, { id: 'combat', relevance: 'active' },
    ];
    flushSync();
    expect(slots()).toContain('engineers');
    cancelEdit();
    flushSync();
    expect(target.querySelector('.grid-title')?.textContent).toBe('Combate');
  });

  test('escape in settings keeps the edit session and its draft', () => {
    edit();
    click(btn('engineers', 'remove'));
    expect(slots()).not.toContain('engineers');
    click(target.querySelector('button[title="Ajustes"]'));
    expect(target.querySelector('[data-testid="edit-toolbar"]')).toBeNull();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    flushSync();
    expect(layoutState.editing).toBe(true);
    expect(layoutState.draft).not.toBeNull();
    click(target.querySelector('.nav-btn[title="Ingeniería"]:not([data-module])'));
    expect(target.querySelector('[data-testid="edit-toolbar"]')).not.toBeNull();
    expect(slots()).not.toContain('engineers');
  });

  test('escape cancels edit', () => {
    edit();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    flushSync();
    expect(layoutState.editing).toBe(false);
  });

  test('escape while saving does not cancel the edit', () => {
    edit();
    layoutState.saving = true;
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    flushSync();
    expect(layoutState.editing).toBe(true);
    expect(layoutState.draft).not.toBeNull();
    layoutState.saving = false;
  });

  test('escape during drag only aborts drag', () => {
    edit();
    const handle = document.createElement('div');
    const frames = [0, 1].map((i) => {
      const el = document.createElement('div');
      el.getBoundingClientRect = () => ({ left: i * 200, top: 0, width: 200, height: 100 }) as DOMRect;
      return el;
    });
    const ondrop = vi.fn();
    const down = Object.assign(new Event('pointerdown'), { clientX: 10, clientY: 10, pointerId: 1, pointerType: 'mouse' });
    const s = startMoveDrag(down as PointerEvent, handle, frames, 0, vi.fn(), ondrop);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    flushSync();
    expect(s.active).toBe(false);
    expect(ondrop).not.toHaveBeenCalled();
    expect(layoutState.editing).toBe(true);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    flushSync();
    expect(layoutState.editing).toBe(false);
  });

  test('escape cancel emits cancel diag like the button', async () => {
    const diagMod = await import('../src/lib/diag.svelte');
    const spy = vi.spyOn(diagMod, 'diag');
    edit();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    flushSync();
    expect(spy).toHaveBeenCalledWith('ui.layout.edit', expect.objectContaining({ cancel: true }));
  });
});
