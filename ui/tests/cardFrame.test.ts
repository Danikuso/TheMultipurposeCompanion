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

  test('virtual list grows with fixed card height', () => {
    const rows = Array.from({ length: 60 }, (_, i) => ({
      body_id: i, short_name: `${i} a`, kind_key: 'rocky-ice', distance_ls: 10, scanned_value: 1, mapped_value: 2,
    }));
    live.topics['exploration.system'] = {
      system: 'Testia', system_address: 1, discovered: false,
      fss: { progress: 1, body_count: 60, scanned: 60, complete: true }, rows,
    };
    const bodies = cards['exploration-system-bodies'];
    const listPx = () => parseFloat(
      (target.querySelector('[data-testid="virtual-list"]') as HTMLElement).style.height,
    );
    frame({ id: bodies.id, w: 'full', h: null }, bodies, 2);
    const auto = listPx();
    unmount(inst!);
    inst = null;
    frame({ id: bodies.id, w: 'full', h: 1200 }, bodies, 2);
    expect(listPx()).toBeGreaterThan(auto);
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
