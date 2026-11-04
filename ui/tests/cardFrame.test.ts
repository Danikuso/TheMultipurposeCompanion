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
    expect(el.style.height).toBe('640px');
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
    expect(el.style.height).toBe('min(640px, max(240px, 70vh))');
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
    expect(el.childNodes).toHaveLength(1);
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
