import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import { disconnect } from '../src/lib/ws.svelte';
import CardHost from '../src/shell/CardHost.svelte';
import { grid } from '../src/shell/gridColumns.svelte';
import { beginEdit, cancelEdit, layoutState } from '../src/shell/layoutStore.svelte';
import { activeDrag } from '../src/shell/pointerDrag';
import { emptyDoc, packLayout, type PackItem } from '../src/shell/layoutModel';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

// Ejemplo del propietario (T1…T7): columnas de inicio y altos.
const IDS = ['dashboard-cmdr', 'dashboard-location', 'dashboard-status', 'dashboard-route', 'dashboard-session',
  'dashboard-alerts', 'jump'];
const XS = [0, 1, 2, 0, 1, 0, 0];
const HS = [200, 120, 300, 160, 200, 160, 160];
const BOARD = 900;
const GAP = 0.85 * 16; // jsdom no define font-size: se usa 16 px

class FakeRO {
  static all: FakeRO[] = [];
  observed = new Set<Element>();
  disconnected = false;
  constructor(readonly cb: ResizeObserverCallback) { FakeRO.all.push(this); }
  observe(el: Element) { this.observed.add(el); }
  unobserve(el: Element) { this.observed.delete(el); }
  disconnect() { this.disconnected = true; this.observed.clear(); }
  fire() {
    this.cb(Array.from(this.observed).map((target) => ({ target }) as ResizeObserverEntry), this as unknown as ResizeObserver);
  }
}

let target: HTMLElement;
let inst: Record<string, unknown> | null = null;
let measured: Record<string, number>;

const board = () => target.querySelector('.card-board') as HTMLElement;
const slot = (id: string) => target.querySelector(`[data-layout-slot="${id}"]`) as HTMLElement;
const px = (v: string) => parseFloat(v);
const fire = () => { FakeRO.all.forEach((r) => r.fire()); flushSync(); };

function items(): PackItem[] {
  return IDS.map((id, i) => ({ id, x: XS[i], w: 1, height: measured[id] ?? HS[i] }));
}

function mountHost(boardWidth = BOARD) {
  vi.spyOn(Element.prototype, 'clientWidth', 'get').mockImplementation(function (this: Element) {
    return this.classList.contains('card-board') ? boardWidth : 0;
  });
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return measured[this.getAttribute('data-layout-slot') ?? ''] ?? 0;
  });
  layoutState.doc = {
    ...emptyDoc(),
    custom: [{ id: 'c1', name: 'Mi', icon: '', cards: IDS.map((id, i) => ({ id, w: 1, h: null, x: XS[i] })) }],
    customOrder: ['c1'],
  };
  inst = mount(CardHost, { target, props: { tabKey: 'custom:c1', ctx: { modules: [], manual: null } } });
  flushSync();
}

beforeEach(() => {
  disconnect();
  setLocale('es');
  layoutState.editing = false;
  layoutState.draft = null;
  grid.columns = 3;
  measured = Object.fromEntries(IDS.map((id, i) => [id, HS[i]]));
  FakeRO.all = [];
  target = document.createElement('div');
  document.body.appendChild(target);
});
afterEach(() => {
  if (inst) unmount(inst);
  inst = null;
  target.remove();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  disconnect();
});

describe('CardHost board', () => {
  test('frames are absolutely placed from packLayout', () => {
    vi.stubGlobal('ResizeObserver', FakeRO);
    mountHost();
    fire();
    const packed = packLayout(items(), 3, GAP);
    const colW = (BOARD - GAP * 2) / 3;
    for (const c of packed.cards) {
      const el = slot(c.id);
      expect(px(el.style.top)).toBeCloseTo(c.top, 3);
      expect(px(el.style.left)).toBeCloseTo(c.col * (colW + GAP), 3);
      expect(px(el.style.width)).toBeCloseTo(colW, 3);
    }
    expect(px(slot('dashboard-alerts').style.top)).toBeCloseTo(200 + GAP + 160 + GAP, 3);
    // Literales a mano (gap = 13.6): T1…T3 arriba, T4 bajo T1, T5 bajo T2, T6 bajo T4, T7 bajo T6.
    const literal: Record<string, number> = {
      'dashboard-cmdr': 0, 'dashboard-location': 0, 'dashboard-status': 0,
      'dashboard-route': 213.6, 'dashboard-session': 133.6, 'dashboard-alerts': 387.2, jump: 560.8,
    };
    for (const [id, top] of Object.entries(literal)) expect(px(slot(id).style.top)).toBeCloseTo(top, 3);
  });

  test('one observer per board, covering the board and every frame, disconnected on destroy', () => {
    vi.stubGlobal('ResizeObserver', FakeRO);
    mountHost();
    expect(FakeRO.all).toHaveLength(1);
    const ro = FakeRO.all[0];
    expect(ro.observed.size).toBe(IDS.length + 1);
    expect(ro.observed.has(board())).toBe(true);
    unmount(inst!);
    inst = null;
    expect(ro.disconnected).toBe(true);
  });

  test('fallback height avoids overlap before measuring', () => {
    // Sin ResizeObserver (jsdom): alto de reserva 160 por marco, sin cruces dentro de una columna.
    mountHost();
    const boxes = IDS.map((id, i) => ({
      col: XS[i], top: px(slot(id).style.top), bottom: px(slot(id).style.top) + 160,
    }));
    expect(boxes.every((b) => Number.isFinite(b.top))).toBe(true);
    for (const a of boxes) {
      for (const b of boxes) {
        if (a === b || a.col !== b.col) continue;
        expect(a.top < b.bottom && b.top < a.bottom).toBe(false);
      }
    }
  });

  test('without a measured width frames stay in normal flow', () => {
    mountHost(0);
    expect(slot(IDS[0]).style.top).toBe('');
    expect(board().style.height).toBe('');
  });

  test('growing content repacks below only', () => {
    vi.stubGlobal('ResizeObserver', FakeRO);
    mountHost();
    fire();
    const before = Object.fromEntries(IDS.map((id) => [id, slot(id).style.top]));
    measured['dashboard-location'] = 240;
    fire();
    expect(px(slot('dashboard-session').style.top)).toBeCloseTo(240 + GAP, 3);
    for (const id of ['dashboard-cmdr', 'dashboard-status', 'dashboard-route']) {
      expect(slot(id).style.top).toBe(before[id]);
    }
  });

  test('board height equals packed height', () => {
    vi.stubGlobal('ResizeObserver', FakeRO);
    mountHost();
    fire();
    expect(px(board().style.height)).toBeCloseTo(packLayout(items(), 3, GAP).height, 3);
    measured['dashboard-location'] = 240;
    fire();
    expect(px(board().style.height)).toBeCloseTo(packLayout(items(), 3, GAP).height, 3);
  });

  test('a card that hides itself takes no room', () => {
    vi.stubGlobal('ResizeObserver', FakeRO);
    mountHost();
    fire();
    measured['dashboard-route'] = 0;
    fire();
    expect(px(slot('dashboard-alerts').style.top)).toBeCloseTo(200 + GAP, 3);
  });
});

describe('CardHost interaction', () => {
  const handleOf = (id: string) => slot(id).querySelector('[data-op="drag"]') as HTMLElement;
  const btn = (id: string, op: string) => slot(id).querySelector(`button[data-op="${op}"]`) as HTMLButtonElement;
  const click = (el: Element) => { (el as HTMLElement).click(); flushSync(); };
  const ev = (type: string, x: number, y: number) => {
    const e = new Event(type, { bubbles: true });
    Object.assign(e, { clientX: x, clientY: y, pointerId: 1, pointerType: 'mouse', button: 0 });
    return e;
  };
  const draftCards = () => layoutState.draft!.custom[0].cards;
  const COLSTEP = (BOARD - GAP * 2) / 3 + GAP;

  function editing(columns: 1 | 2 | 3 | 4 = 3) {
    vi.stubGlobal('ResizeObserver', FakeRO);
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => { cb(0); return 1; });
    grid.columns = columns;
    mountHost();
    beginEdit();
    flushSync();
    fire();
  }
  afterEach(() => { activeDrag()?.cancel(); cancelEdit(); });

  test('left and right emit column op and disable at edges', () => {
    editing(2);
    // Con 2 columnas: T1 está en la columna 0 (x = 0), T2 en la 1.
    expect(btn('dashboard-cmdr', 'left').disabled).toBe(true);
    expect(btn('dashboard-cmdr', 'right').disabled).toBe(false);
    expect(btn('dashboard-location', 'right').disabled).toBe(true);
    click(btn('dashboard-cmdr', 'right'));
    expect(draftCards().find((c) => c.id === 'dashboard-cmdr')!.x).toBe(1);
  });

  test('left and right are disabled with one column or full width', () => {
    editing(1);
    expect(btn('dashboard-cmdr', 'left').disabled).toBe(true);
    expect(btn('dashboard-cmdr', 'right').disabled).toBe(true);
  });

  test('up/down follow the column neighbour', () => {
    editing(3);
    expect(btn('dashboard-cmdr', 'down').disabled).toBe(false); // T4 está debajo de T1
    expect(btn('dashboard-cmdr', 'up').disabled).toBe(true);
    expect(btn('dashboard-location', 'up').disabled).toBe(true);
    click(btn('dashboard-session', 'up')); // T5 sube por encima de T2
    const ids = draftCards().map((c) => c.id);
    expect(ids.indexOf('dashboard-session')).toBeLessThan(ids.indexOf('dashboard-location'));
  });

  test('drag drop emits place with dropTarget', () => {
    editing(3);
    const h = handleOf('jump');
    h.dispatchEvent(ev('pointerdown', 10, 10));
    h.dispatchEvent(ev('pointermove', 10 + 2 * COLSTEP, 10 - 400));
    h.dispatchEvent(ev('pointerup', 10 + 2 * COLSTEP, 10 - 400));
    flushSync();
    const jump = draftCards().find((c) => c.id === 'jump')!;
    expect(jump.x).toBe(2);
    const ids = draftCards().map((c) => c.id);
    expect(ids.indexOf('jump')).toBeLessThan(ids.indexOf('dashboard-alerts'));
  });

  test('mid-drag shows the placeholder and cancel restores everything', () => {
    editing(3);
    const before = Object.fromEntries(IDS.map((id) => [id, slot(id).style.top]));
    const h = handleOf('jump');
    h.dispatchEvent(ev('pointerdown', 10, 10));
    h.dispatchEvent(ev('pointermove', 10 + 2 * COLSTEP, 10 - 400));
    flushSync();
    const ph = board().querySelector('.drop-placeholder') as HTMLElement;
    expect(ph).not.toBeNull();
    expect(px(ph.style.left)).toBeCloseTo(2 * COLSTEP, 3);
    expect(slot('jump').style.transform).toContain('translate(');
    activeDrag()!.cancel();
    flushSync();
    expect(board().querySelector('.drop-placeholder')).toBeNull();
    expect(slot('jump').style.transform).toBe('');
    for (const id of IDS) expect(slot(id).style.top).toBe(before[id]);
  });
});
