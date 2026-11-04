import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import { disconnect, live } from '../src/lib/ws.svelte';
import { prefs } from '../src/lib/prefs.svelte';
import { cards, factoryLayout, type CardDef, type CardPlacement } from '../src/shell/cards';
import CardFrame from '../src/shell/CardFrame.svelte';
import Panel from '../src/shell/Panel.svelte';
import App from '../src/App.svelte';
import { layoutState } from '../src/shell/layoutStore.svelte';
import { activeDrag } from '../src/shell/pointerDrag';
import { emptyDoc } from '../src/shell/layoutModel';
import PanelCard from './fixtures/PanelCard.svelte';
import EmptyCard from './fixtures/EmptyCard.svelte';
import frameSrc from '../src/shell/CardFrame.svelte?raw';
import panelSrc from '../src/shell/Panel.svelte?raw';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

const ctx = { modules: [], manual: null };
const def = (component: CardDef['component'], id = 'test-card'): CardDef => ({
  id, titleKey: 'panel-route', module: 'test', component, defaultWidth: 1,
});

let target: HTMLElement;
let inst: Record<string, unknown> | null = null;

function frame(placement: CardPlacement, card: CardDef, columns = 2, editing = false) {
  inst = mount(CardFrame, {
    target,
    props: { placement, card, index: 0, count: 1, ghost: false, editing, columns, ctx },
  });
  flushSync();
  return target.querySelector('.card-frame') as HTMLElement;
}

beforeEach(() => {
  disconnect();
  setLocale('es');
  target = document.createElement('div');
  document.body.appendChild(target);
});
afterEach(() => {
  if (inst) unmount(inst);
  inst = null;
  target.remove();
  disconnect();
});

describe('CardFrame', () => {
  test('frame spans effective columns', () => {
    const el = frame({ id: 'test-card', w: 3, h: null }, def(PanelCard), 2);
    expect(el.style.gridColumn).toContain('span 2');
    expect(el.getAttribute('data-layout-slot')).toBe('test-card');
  });

  test('frame full width', () => {
    const el = frame({ id: 'test-card', w: 'full', h: null }, def(PanelCard), 3);
    expect(el.style.gridColumn).toBe('1 / -1');
  });

  test('fixed height sets cardBox and panel class', () => {
    const el = frame({ id: 'test-card', w: 1, h: 640 }, def(PanelCard), 2);
    expect((el.querySelector('.card-content') as HTMLElement).style.height).toBe('640px');
    const section = el.querySelector('section') as HTMLElement;
    expect(section.classList.contains('fixed-height')).toBe(true);
    expect(section.classList.contains('in-frame')).toBe(true);
    // Dentro del marco el Panel no impone su propio span.
    expect(section.style.gridColumn).toBe('');
    expect(el.querySelector('.details-content')).not.toBeNull();
  });

  test('auto height keeps panel without fixed-height', () => {
    const el = frame({ id: 'test-card', w: 1, h: null }, def(PanelCard), 2);
    expect(el.style.height).toBe('');
    const section = el.querySelector('section') as HTMLElement;
    expect(section.classList.contains('fixed-height')).toBe(false);
    expect(section.classList.contains('in-frame')).toBe(true);
  });

  test('one column caps fixed height to the viewport', () => {
    const el = frame({ id: 'test-card', w: 2, h: 640 }, def(PanelCard), 1);
    expect((el.querySelector('.card-content') as HTMLElement).style.height).toBe('min(640px, max(240px, 70vh))');
    expect(el.style.gridColumn).toContain('span 1');
  });

  test('auto-height panel stretches to fill its frame', () => {
    const el = frame({ id: 'test-card', w: 1, h: null }, def(PanelCard), 2);
    const content = el.firstElementChild as HTMLElement;
    const section = content.querySelector('section') as HTMLElement;
    expect(section.classList.contains('in-frame')).toBe(true);
    expect(section.classList.contains('fixed-height')).toBe(false);
    // jsdom no aplica CSS: se verifica la cadena de estiramiento en las reglas de los componentes.
    const rule = (src: string, selector: string) => {
      const esc = selector.replaceAll('.', '\\.');
      return new RegExp(String.raw`(^|\n)\s*${esc}\s*\{([^}]*)\}`).exec(src)?.[2] ?? '';
    };
    expect(rule(frameSrc, '.card-frame')).toMatch(/display:\s*flex/);
    expect(rule(frameSrc, '.card-frame')).toMatch(/flex-direction:\s*column/);
    expect(rule(frameSrc, '.card-content')).toMatch(/flex:\s*1/);
    expect(rule(frameSrc, '.card-content')).toMatch(/display:\s*flex/);
    expect(rule(frameSrc, '.card-content')).toMatch(/min-height:\s*0/);
    expect(rule(panelSrc, '.hud-panel.in-frame')).toMatch(/flex:\s*1/);
  });

  test('panel outside frame keeps span', () => {
    inst = mount(Panel, { target, props: { titleKey: 'panel-route', span: 2 } });
    flushSync();
    const section = target.querySelector('section') as HTMLElement;
    expect(section.style.gridColumn).toBe('span 2');
    expect(section.classList.contains('in-frame')).toBe(false);
  });

  test('empty card collapses outside edit mode', () => {
    const el = frame({ id: 'test-card', w: 1, h: null }, def(EmptyCard), 2);
    const content = el.firstElementChild as HTMLElement;
    expect(content.classList.contains('card-content')).toBe(true);
    expect(el.children).toHaveLength(1);
    // :empty de los navegadores ignora comentarios y nodos de texto de longitud 0 (anclas de Svelte),
    // pero no elementos ni texto con contenido, aunque sea un espacio. (matches(':empty') de jsdom no se
    // usa: su motor de selectores choca con los globales de runas de Svelte.)
    const significant = Array.from(content.childNodes)
      .filter((n) => n.nodeType === Node.ELEMENT_NODE || (n.nodeType === Node.TEXT_NODE && n.textContent !== ''));
    expect(significant).toHaveLength(0);
  });

  test('virtual list grows with fixed card height, fitting the measured space without body overflow', () => {
    const rows = Array.from({ length: 60 }, (_, i) => ({
      body_id: i, short_name: `${i} a`, kind_key: 'rocky-ice', distance_ls: 10, scanned_value: 1, mapped_value: 2,
    }));
    live.topics['exploration.system'] = {
      system: 'Testia', system_address: 1, discovered: false,
      fss: { progress: 1, body_count: 60, scanned: 60, complete: true }, rows,
    };
    // Maqueta de layout realista: cuerpo del panel de 1160 px (flex en columna con overflow-y: auto), 520 px
    // de contenido encima de la lista y 20 px debajo (barra horizontal fija). .hud-panel-details es flex: 1
    // y se estira hasta el fondo del cuerpo (holgura), y scrollHeight está acotado a >= clientHeight,
    // como en un navegador real.
    const BODY = 1160;
    const ABOVE = 520;
    const BELOW = 20;
    const listOf = () => target.querySelector('[data-testid="virtual-list"]') as HTMLElement;
    const listPx = () => parseFloat(listOf().style.height) || 0;
    const isBody = (el: Element) => el.classList.contains('hud-panel-body');
    const rect = (top: number, bottom: number) =>
      ({ top, height: bottom - top, bottom, left: 0, right: 0, width: 0, x: 0, y: top, toJSON: () => ({}) }) as DOMRect;
    vi.spyOn(Element.prototype, 'clientHeight', 'get').mockImplementation(function (this: Element) {
      return isBody(this) ? BODY : 0;
    });
    vi.spyOn(Element.prototype, 'scrollHeight', 'get').mockImplementation(function (this: Element) {
      return isBody(this) ? Math.max(BODY, ABOVE + listPx() + BELOW) : 0;
    });
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
      const list = listOf();
      const listTop = 40 + ABOVE;
      const listBottom = listTop + listPx();
      if (isBody(this)) return rect(40, 40 + BODY);
      if (this === list) return rect(listTop, listBottom);
      if (this.classList.contains('hud-panel-details')) return rect(listTop - 100, 40 + BODY);
      if (list && this.contains(list)) {
        const extra = this.classList.contains('table-container') || this.querySelector('.table-container') ? BELOW : 0;
        return rect(listTop - 40, listBottom + extra);
      }
      return rect(0, 0);
    });
    try {
      const bodies = cards['exploration-system-bodies'];
      frame({ id: bodies.id, w: 'full', h: null }, bodies, 2);
      const auto = listPx();
      unmount(inst!);
      inst = null;
      frame({ id: bodies.id, w: 'full', h: 1200 }, bodies, 2);
      flushSync();
      // Crece hasta la holgura real y cabe sin desbordar el cuerpo.
      expect(listPx()).toBeGreaterThan(auto);
      expect(listPx()).toBe(BODY - ABOVE - BELOW);
      expect(ABOVE + listPx() + BELOW).toBeLessThanOrEqual(BODY);
      // Una nueva medición con la lista ya crecida da el mismo resultado (sin quedarse pegada).
      live.topics['exploration.system'] = {
        ...(live.topics['exploration.system'] as object), rows: [...rows, { ...rows[0], body_id: 999 }],
      };
      flushSync();
      expect(listPx()).toBe(BODY - ABOVE - BELOW);
    } finally {
      vi.restoreAllMocks();
    }
  });
});

describe('App renders tabs from the layout store', () => {
  let app: Record<string, unknown>;
  beforeEach(() => {
    live.topics['core.modules'] = [{ id: 'engineering', relevance: 'active' }];
    prefs.loaded = true;
    app = mount(App, { target });
    layoutState.doc = emptyDoc();
    layoutState.draft = null;
    layoutState.editing = false;
    flushSync();
  });
  afterEach(() => unmount(app));

  test('engineering tab renders factory order', () => {
    const expected = factoryLayout('layout:engineering')
      .map((p) => p.id)
      .filter((id) => cards[id].when?.(ctx) ?? true);
    const shown = Array.from(target.querySelectorAll('[data-layout-slot]'))
      .map((el) => el.getAttribute('data-layout-slot'));
    expect(shown).toEqual(expected);
    expect(shown.length).toBeGreaterThan(3);
    // El prefijo de App (selector de contexto) no deja texto suelto en las demás tarjetas.
    for (const content of target.querySelectorAll('.card-content')) {
      const text = Array.from(content.childNodes).filter((n) => n.nodeType === Node.TEXT_NODE && n.textContent !== '');
      expect(text).toHaveLength(0);
    }
  });

  test('unknown card ids are skipped without throwing', () => {
    layoutState.doc = { ...emptyDoc(), tabs: { 'layout:engineering': { cards: [
      { id: 'does-not-exist', w: 1, h: null }, { id: 'materials', w: 2, h: null },
    ], removed: [] } } };
    flushSync();
    const shown = Array.from(target.querySelectorAll('[data-layout-slot]'))
      .map((el) => el.getAttribute('data-layout-slot'));
    expect(shown).not.toContain('does-not-exist');
    expect(shown[0]).toBe('materials');
  });
});

describe('CardFrame edit controls overlay', () => {
  test('bar stays in flow above content and is not measured', () => {
    const emitted: unknown[] = [];
    inst = mount(CardFrame, {
      target,
      props: {
        placement: { id: 'test-card', w: 1, h: null }, card: def(PanelCard), index: 0, count: 1, ghost: false,
        editing: true, columns: 2, ctx, onop: (op: unknown) => emitted.push(op),
      },
    });
    flushSync();
    const el = target.querySelector('.card-frame') as HTMLElement;
    const content = el.querySelector('.card-content') as HTMLElement;
    const bar = el.querySelector('.card-controls') as HTMLElement;
    const rectOf = (h: number) => ({ top: 0, left: 0, right: 0, bottom: h, width: 100, height: h, x: 0, y: 0, toJSON: () => ({}) }) as DOMRect;
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
      if (this === content) return rectOf(160);
      if (this === bar) return rectOf(60);
      return rectOf(160 + 60 + 40); // el marco medido entero incluiria barra y relleno
    });
    (el.querySelector('button[data-op="height-more"]') as HTMLButtonElement).click();
    expect(emitted).toEqual([{ kind: 'height', h: 200 }]);
    vi.restoreAllMocks();
  });

  test('drag and resize handles exist while editing', () => {
    const el = frame({ id: 'test-card', w: 1, h: null }, def(PanelCard), 2, true);
    expect(el.querySelector('[data-op="drag"]')).not.toBeNull();
    expect(el.querySelector('[data-op="resize"]')).not.toBeNull();
  });
});

describe('CardFrame height on content', () => {
  test('fixed height lands on card-content, not on the frame, in and out of edit mode', () => {
    for (const editing of [false, true]) {
      const el = frame({ id: 'test-card', w: 1, h: 640 }, def(PanelCard), 2, editing);
      const content = el.querySelector('.card-content') as HTMLElement;
      expect(el.style.height).toBe('');
      expect(content.style.height).toBe('640px');
      unmount(inst!);
      inst = null;
    }
  });
});

describe('CardFrame aborted drags clear their visuals', () => {
  const pe = (type: string, x: number, y: number) =>
    Object.assign(new Event(type, { bubbles: true }), { clientX: x, clientY: y, pointerId: 1, pointerType: 'mouse', button: 0 });
  const stubRaf = () => {
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => { cb(0); return 1; });
    vi.stubGlobal('cancelAnimationFrame', () => {});
  };
  function editFrame(emitted: unknown[]) {
    inst = mount(CardFrame, {
      target,
      props: {
        placement: { id: 'test-card', w: 1, h: null }, card: def(PanelCard), index: 0, count: 2, ghost: false,
        editing: true, columns: 2, ctx, onop: (op: unknown) => emitted.push(op),
      },
    });
    flushSync();
    return target.querySelector('.card-frame') as HTMLElement;
  }
  const clean = (el: HTMLElement) => {
    expect(el.style.transform).toBe('');
    expect(el.classList.contains('dragging')).toBe(false);
    expect(document.querySelector('[data-drop-target]')).toBeNull();
  };

  test.each(['cancel', 'pointercancel'])('move drag aborted by %s', async (how) => {
    stubRaf();
    const emitted: unknown[] = [];
    const sibling = document.createElement('div');
    sibling.setAttribute('data-layout-slot', 'other');
    const el = editFrame(emitted);
    el.after(sibling);
    el.getBoundingClientRect = () => ({ left: 0, top: 0, width: 100, height: 100 }) as DOMRect;
    sibling.getBoundingClientRect = () => ({ left: 200, top: 0, width: 100, height: 100 }) as DOMRect;
    const handle = el.querySelector('[data-op="drag"]') as HTMLElement;
    handle.dispatchEvent(pe('pointerdown', 50, 50));
    handle.dispatchEvent(pe('pointermove', 250, 50));
    flushSync();
    expect(el.style.transform).toContain('translate');
    expect(sibling.hasAttribute('data-drop-target')).toBe(true);
    if (how === 'cancel') activeDrag()!.cancel(); else handle.dispatchEvent(pe('pointercancel', 250, 50));
    flushSync();
    clean(el);
    expect(emitted).toEqual([]);
    sibling.remove();
    vi.unstubAllGlobals();
  });

  test('resize drag aborted clears the preview size', () => {
    stubRaf();
    const emitted: unknown[] = [];
    const el = editFrame(emitted);
    const handle = el.querySelector('[data-op="resize"]') as HTMLElement;
    handle.dispatchEvent(pe('pointerdown', 0, 0));
    handle.dispatchEvent(pe('pointermove', 100, 200));
    flushSync();
    expect((el.querySelector('.card-content') as HTMLElement).style.height).not.toBe('');
    activeDrag()!.cancel();
    flushSync();
    expect((el.querySelector('.card-content') as HTMLElement).style.height).toBe('');
    expect(emitted).toEqual([]);
    vi.unstubAllGlobals();
  });
});
