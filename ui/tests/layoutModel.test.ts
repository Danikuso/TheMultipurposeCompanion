import { describe, expect, test } from 'vitest';
import type { CardPlacement, CardWidth } from '../src/shell/cards';
import {
  CUSTOM_TAB_ICONS, addToCustom, addToThemed, applyCardOp, columnsForWidth, createCustomTab, deleteCustomTab, dropIndex,
  effectiveWidth, emptyDoc, mergeWithDefaults, moveCustomTab, normalizeTabName, parseLayoutDoc,
  removeFromThemed, sanitizePlacements, packLayout, pinColumns, normalizeOrder, verticalNeighbor, dropTarget,
  type PackItem, snapHeight, snapResize, updateCustomTab, visibleCards,
  type LayoutDoc,
} from '../src/shell/layoutModel';

const p = (id: string, w: CardWidth = 1, h: number | null = null): CardPlacement => ({ id, w, h });
const all = () => true;

describe('mergeWithDefaults', () => {
  const factory = [p('a', 1), p('b', 2), p('c', 1)];
  test('limpia guardado', () => {
    const known = (id: string) => id !== 'x' && id !== 'zz';
    const saved = { cards: [p('c', 1), p('x', 2), p('a', 2), p('a', 1)], removed: ['b', 'zz'] };
    expect(mergeWithDefaults(saved, factory, known)).toEqual({ cards: [p('c', 1), p('a', 2)], removed: ['b'] });
  });
  test('sin guardado devuelve fábrica', () => {
    expect(mergeWithDefaults(undefined, factory, all)).toEqual({ cards: factory, removed: [] });
  });
  test('tarjeta nueva al final', () => {
    const saved = { cards: [p('c'), p('a'), p('b', 2)], removed: [] };
    const out = mergeWithDefaults(saved, [...factory, p('d', 3)], all);
    expect(out.cards.map((c) => c.id)).toEqual(['c', 'a', 'b', 'd']);
  });
  test('w inválido usa el ancho de fábrica', () => {
    const saved = { cards: [{ id: 'b', w: 9 as unknown as CardWidth, h: null }], removed: [] };
    expect(mergeWithDefaults(saved, factory, all).cards[0]).toEqual(p('b', 2));
  });
});

describe('sanitizePlacements', () => {
  test('w inválido usa el defaultWidth', () => {
    expect(sanitizePlacements([{ id: 'a', w: 5, h: null }], all)).toEqual([p('a', 1)]);
    expect(sanitizePlacements([{ id: 'a', w: 5, h: null }], all, () => 2)).toEqual([p('a', 2)]);
  });
  test('h se corrige', () => {
    const h = (v: unknown) => sanitizePlacements([{ id: 'a', w: 1, h: v }], all)[0].h;
    expect(h(150)).toBeNull();
    expect(h(2050)).toBe(2000);
    expect(h(183)).toBe(200);
    expect(h('x')).toBeNull();
  });
  test('descarta basura, desconocidos y duplicados', () => {
    expect(sanitizePlacements('no', all)).toEqual([]);
    expect(sanitizePlacements([null, { id: 3 }, p('a'), p('a', 2), p('z')], (id) => id !== 'z')).toEqual([p('a')]);
  });
});

test('snapHeight', () => {
  expect(snapHeight(159)).toBe(160);
  expect(snapHeight(181)).toBe(200);
  expect(snapHeight(179)).toBe(160);
  expect(snapHeight(5000)).toBe(2000);
});

describe('applyCardOp', () => {
  const list = [p('a'), p('b'), p('c')];
  const ids = (l: CardPlacement[]) => l.map((c) => c.id).join('');
  test('move', () => {
    expect(ids(applyCardOp(list, 0, { kind: 'move', to: 2 }))).toBe('bca');
    expect(ids(applyCardOp(list, 2, { kind: 'move', to: 0 }))).toBe('cab');
    expect(ids(applyCardOp(list, 0, { kind: 'move', to: 99 }))).toBe('bca');
    expect(ids(applyCardOp(list, 2, { kind: 'move', to: -5 }))).toBe('cab');
    expect(ids(list)).toBe('abc');
  });
  test('width, height, remove', () => {
    expect(applyCardOp(list, 1, { kind: 'width', w: 'full' })[1]).toEqual(p('b', 'full'));
    const tall = [p('a', 1, 400)];
    expect(applyCardOp(tall, 0, { kind: 'height', h: null })[0].h).toBeNull();
    expect(ids(applyCardOp(list, 1, { kind: 'remove' }))).toBe('ac');
    expect(list[1]).toEqual(p('b'));
  });
});

describe('themed', () => {
  test('quitar y añadir', () => {
    const tab = { cards: [p('a'), p('b', 2), p('c')], removed: [] as string[] };
    const rem = removeFromThemed(tab, 'b', true);
    expect(rem).toEqual({ cards: [p('a'), p('c')], removed: ['b'] });
    const back = addToThemed(rem, 'b', 2);
    expect(back).toEqual({ cards: [p('a'), p('c'), p('b', 2)], removed: [] });
    expect(addToThemed(back, 'b', 2).cards).toHaveLength(3);
  });
  test('no de fábrica no entra en removed', () => {
    expect(removeFromThemed({ cards: [p('q')], removed: [] }, 'q', false)).toEqual({ cards: [], removed: [] });
  });
});

test('columnsForWidth', () => {
  expect([899, 900, 1599, 1600, 2400].map(columnsForWidth)).toEqual([1, 2, 2, 3, 4]);
});

test('effectiveWidth', () => {
  const ws: CardWidth[] = [1, 2, 3, 'full'];
  const expected = { 1: [1, 1, 1, 1], 2: [1, 2, 2, 2], 3: [1, 2, 3, 3], full: [1, 2, 3, 4] };
  for (const w of ws) {
    [1, 2, 3, 4].forEach((c, i) => expect(effectiveWidth(w, c)).toBe(expected[w][i]));
  }
});

test('visibleCards', () => {
  const list = [p('a'), p('b')];
  const vis = (id: string) => id !== 'b';
  expect(visibleCards(list, vis, false)).toEqual([{ placement: p('a'), ghost: false }]);
  expect(visibleCards(list, vis, true)).toEqual([
    { placement: p('a'), ghost: false }, { placement: p('b'), ghost: true },
  ]);
});

describe('pestañas personalizadas', () => {
  test('normalizeTabName', () => {
    expect(normalizeTabName('  Caza AX ')).toBe('Caza AX');
    expect(normalizeTabName('')).toBeNull();
    expect(normalizeTabName('x'.repeat(33))).toBeNull();
    expect(normalizeTabName('x'.repeat(32))).toBe('x'.repeat(32));
  });
  test('createCustomTab límite 12', () => {
    let doc: LayoutDoc | null = emptyDoc();
    for (let i = 0; i < 12; i++) doc = createCustomTab(doc!, `T${i}`, 'star', `id${i}`);
    expect(doc!.custom).toHaveLength(12);
    expect(doc!.customOrder).toHaveLength(12);
    expect(createCustomTab(doc!, 'T13', 'star', 'id13')).toBeNull();
    expect(createCustomTab(emptyDoc(), '  ', 'star', 'x')).toBeNull();
  });
  test('update, delete, move, addToCustom', () => {
    let doc = createCustomTab(emptyDoc(), 'A', 'i', 'a')!;
    doc = createCustomTab(doc, 'B', 'i', 'b')!;
    doc = updateCustomTab(doc, 'a', { name: ' Nuevo ', cards: [p('q')] });
    expect(doc.custom[0]).toMatchObject({ name: 'Nuevo', cards: [p('q')] });
    expect(moveCustomTab(doc, 'a', -1)).toEqual(doc);
    expect(moveCustomTab(doc, 'b', 1)).toEqual(doc);
    expect(moveCustomTab(doc, 'a', 1).customOrder).toEqual(['b', 'a']);
    expect(deleteCustomTab(doc, 'a').customOrder).toEqual(['b']);
    expect(deleteCustomTab(doc, 'a').custom.map((c) => c.id)).toEqual(['b']);
    const tab = addToCustom(doc.custom[0], 'z', 2);
    expect(addToCustom(tab, 'z', 2).cards).toHaveLength(2);
  });
});

describe('parseLayoutDoc', () => {
  test('rechaza no válidos', () => {
    expect(parseLayoutDoc({ version: 2 }, all)).toBeNull();
    expect(parseLayoutDoc('texto', all)).toBeNull();
    expect(parseLayoutDoc(null, all)).toBeNull();
  });
  test('descarta tarjeta desconocida en personalizada', () => {
    const raw = {
      version: 1, tabs: {},
      custom: [{ id: 'a', name: 'A', icon: 'i', cards: [p('ok'), p('bad')] }],
      customOrder: ['a'],
    };
    const doc = parseLayoutDoc(raw, (id) => id !== 'bad')!;
    expect(doc.custom[0].cards).toEqual([p('ok')]);
    expect(doc.customOrder).toEqual(['a']);
  });
  test('icono de pestaña propia solo del catálogo', () => {
    const tab = (icon: unknown) => ({ id: 'a', name: 'A', icon, cards: [] });
    const icons = (...list: unknown[]) =>
      parseLayoutDoc({ version: 1, custom: list.map(tab), customOrder: [] }, all)!.custom.map((c) => c.icon);
    expect(icons(CUSTOM_TAB_ICONS[0])).toEqual([CUSTOM_TAB_ICONS[0]]);
    expect(icons('constructor')).toEqual(['']);
    expect(icons('toString')).toEqual(['']);
    expect(icons(7)).toEqual(['']);
  });
});

test('dropIndex', () => {
  const rects = [0, 110, 220].map((left) => ({ left, top: 0, width: 100, height: 100 }));
  expect(dropIndex(260, 50, rects, 0)).toBe(2);
  expect(dropIndex(5, 50, rects, 0)).toBe(0);
});

test('snapResize', () => {
  const start = { w: 1 as CardWidth, h: null, heightPx: 400 };
  expect(snapResize(start, 320, 0, 300, 14, 3).w).toBe(2);
  expect(snapResize(start, 640, 0, 300, 14, 3).w).toBe('full');
  expect(snapResize({ ...start, w: 'full' }, 0, 0, 300, 14, 3).w).toBe('full');
  expect(snapResize(start, 0, 95, 300, 14, 3).h).toBe(480);
  expect(snapResize({ ...start, h: 320 }, 0, 0, 300, 14, 3).h).toBe(320);
});

describe('snapResize conserva el ancho si las columnas no cambian', () => {
  const s = (w: CardWidth, h: number | null = null) => ({ w, h, heightPx: 400 });
  test('solo alto a 2 columnas con w:2', () => {
    expect(snapResize(s(2), 0, 120, 300, 14, 2)).toEqual({ w: 2, h: 520 });
  });
  test('clic sin mover no cambia nada', () => {
    expect(snapResize(s(2), 0, 0, 300, 14, 2)).toEqual({ w: 2, h: null });
    expect(snapResize(s(3, 320), 0, 0, 300, 14, 2)).toEqual({ w: 3, h: 320 });
    expect(snapResize(s(1), 0, 0, 300, 14, 3)).toEqual({ w: 1, h: null });
  });
  test('w:3 a 2 columnas conserva 3', () => {
    expect(snapResize(s(3), 0, 0, 300, 14, 2).w).toBe(3);
    expect(snapResize(s(3), 5, 0, 300, 14, 2).w).toBe(3);
  });
  test('w:3 a 1 columna con dy conserva 3', () => {
    expect(snapResize(s(3), 0, 80, 300, 14, 1)).toEqual({ w: 3, h: 480 });
  });
  test("'full' a 1 columna con dx conserva 'full'", () => {
    expect(snapResize(s('full'), 3, 0, 300, 14, 1).w).toBe('full');
  });
  test("w:1 a 3 columnas arrastrando dos columnas pasa a 'full'", () => {
    expect(snapResize(s(1), 2 * 314, 0, 300, 14, 3).w).toBe('full');
  });
});

describe('rejilla libre', () => {
  const xs = [0, 1, 2, 0, 1, 0, 0];
  const hs = [200, 120, 300, 160, 200, 160, 160];
  const items = (over: Partial<PackItem>[] = []): PackItem[] =>
    xs.map((x, i) => ({ id: `T${i + 1}`, x, w: 1, height: hs[i], ...(over[i] ?? {}) }));
  const tops = (r: { cards: { top: number }[] }) => r.cards.map((c) => c.top);

  test('packLayout owner example', () => {
    const r = packLayout(items(), 3, 0);
    expect(tops(r)).toEqual([0, 0, 0, 200, 120, 360, 520]);
    expect(r.cards.map((c) => c.col)).toEqual(xs);
    expect(r.height).toBe(680);
  });
  test('packLayout taller T2 only moves T5', () => {
    const r = packLayout(items([{}, { height: 240 }]), 3, 0);
    expect(tops(r)).toEqual([0, 0, 0, 200, 240, 360, 520]);
  });
  test('packLayout wider T2 pushes T3 and T5 but never T1', () => {
    const r = packLayout(items([{}, { w: 2 }]), 3, 0);
    expect(r.cards[1]).toMatchObject({ col: 1, span: 2, top: 0 });
    expect(r.cards[2]).toMatchObject({ col: 2, top: 120 });
    expect(r.cards[4]).toMatchObject({ col: 1, top: 120 });
    expect(r.cards[0]).toMatchObject({ col: 0, top: 0 });
  });
  test('packLayout masonry for automatic x', () => {
    const r = packLayout([300, 100, 100].map((height, i) => ({ id: `a${i}`, w: 1 as const, height })), 2, 0);
    expect(r.cards.map((c) => [c.col, c.top])).toEqual([[0, 0], [1, 0], [1, 100]]);
  });
  test('packLayout clamps x without touching it', () => {
    const input: PackItem = { id: 'a', x: 2, w: 2, height: 100 };
    const r = packLayout([input], 3, 0);
    expect(r.cards[0].col).toBe(1);
    expect(input.x).toBe(2);
  });
  test('packLayout full width starts at 0', () => {
    const r = packLayout([{ id: 'a', x: 2, w: 'full', height: 100 }], 3, 0);
    expect(r.cards[0]).toMatchObject({ col: 0, span: 3 });
  });
  test('packLayout zero height takes no space nor gap', () => {
    const r = packLayout([
      { id: 'A', x: 0, w: 1, height: 100 }, { id: 'B', x: 0, w: 1, height: 0 }, { id: 'C', x: 0, w: 1, height: 50 },
    ], 3, 10);
    expect(r.cards[2].top).toBe(110);
    expect(r.height).toBe(160);
    expect(packLayout([{ id: 'A', w: 1, height: 0 }], 2, 0).height).toBe(0);
  });
  test('packLayout one column stacks in list order', () => {
    const r = packLayout(items(), 1, 0);
    expect(tops(r)).toEqual([0, 200, 320, 620, 780, 980, 1140]);
  });
  test('normalizeOrder keeps the layout', () => {
    const base = packLayout(items(), 3, 0);
    const byId = (id: string) => items().find((i) => i.id === id)!;
    const shuffled = ['T1', 'T4', 'T6', 'T7', 'T2', 'T5', 'T3'];
    const re = packLayout(shuffled.map(byId), 3, 0);
    const ordered = normalizeOrder(shuffled.map((id) => p(id)), re.cards);
    const again = packLayout(ordered.map((c) => byId(c.id)), 3, 0);
    for (const c of base.cards) {
      const o = again.cards.find((a) => a.id === c.id)!;
      expect([o.col, o.top]).toEqual([c.col, c.top]);
    }
    const keys = again.cards.map((c) => [c.top, c.col]);
    expect(keys).toEqual([...keys].sort((a, b) => a[0] - b[0] || a[1] - b[1]));
  });
  test('normalizeOrder puts unpacked cards last', () => {
    const packed = [{ id: 'b', col: 0, span: 1, top: 0, height: 1 }, { id: 'a', col: 0, span: 1, top: 5, height: 1 }];
    expect(normalizeOrder([p('z'), p('a'), p('b')], packed).map((c) => c.id)).toEqual(['b', 'a', 'z']);
  });
  test('pinColumns fills only automatic x', () => {
    const packed = [
      { id: 'a', col: 1, span: 1, top: 0, height: 1 }, { id: 'b', col: 2, span: 1, top: 0, height: 1 },
    ];
    const out = pinColumns([p('a'), { ...p('b'), x: 3 }, p('c')], packed);
    expect(out).toEqual([{ ...p('a'), x: 1 }, { ...p('b'), x: 3 }, p('c')]);
    expect('x' in out[2]).toBe(false);
  });
  test('verticalNeighbor finds the card above in shared columns', () => {
    const r = packLayout(items(), 3, 0);
    const list = items().map((i) => p(i.id));
    expect(verticalNeighbor(list, r.cards, 5, -1)).toBe(3);
    expect(verticalNeighbor(list, r.cards, 3, -1)).toBe(0);
    expect(verticalNeighbor(list, r.cards, 0, -1)).toBeNull();
    expect(verticalNeighbor(list, r.cards, 3, 1)).toBe(5);
    expect(verticalNeighbor(list, r.cards, 6, 1)).toBeNull();
    expect(verticalNeighbor(list, r.cards, 2, 1)).toBeNull();
  });
  test('dropTarget picks nearest column and insertion index', () => {
    const r = packLayout(items(), 3, 0);
    const list = items().map((i) => p(i.id));
    const d = { id: 'T7', span: 1, leftPx: 480, topPx: 150 };
    expect(dropTarget(d, list, r.cards, 300, 0, 3)).toEqual({ x: 2, index: 3 });
    expect(dropTarget({ ...d, leftPx: 5000, topPx: 9999 }, list, r.cards, 300, 0, 3)).toEqual({ x: 2, index: 6 });
  });
  test('sanitizePlacements keeps integer x 0..3 only', () => {
    const raw = [2, 4, -1, 1.5, '1', null].map((x, i) => ({ id: `c${i}`, x, w: 1, h: null }));
    const out = sanitizePlacements(raw, all);
    expect(out[0]).toEqual({ id: 'c0', x: 2, w: 1, h: null });
    for (const c of out.slice(1)) expect('x' in c).toBe(false);
  });
  test('applyCardOp place and column', () => {
    const list = [p('a'), p('b'), p('c')];
    expect(applyCardOp(list, 0, { kind: 'place', to: 1, x: 2 })).toEqual([p('b'), { ...p('a'), x: 2 }, p('c')]);
    expect(applyCardOp(list, 1, { kind: 'column', x: 1 })[1]).toEqual({ ...p('b'), x: 1 });
  });
  test('snapResize respects maxSpan', () => {
    const start = { w: 1 as const, h: null, heightPx: 200 };
    expect(snapResize(start, 2 * 314, 0, 300, 14, 3, 1).w).toBe(1);
    expect(snapResize(start, 314, 0, 300, 14, 3, 2).w).toBe(2);
    expect(snapResize(start, 2 * 314, 0, 300, 14, 3, 2).w).toBe(2);
  });
});
