import { describe, expect, test } from 'vitest';
import type { CardPlacement, CardWidth } from '../src/shell/cards';
import {
  addToCustom, addToThemed, applyCardOp, columnsForWidth, createCustomTab, deleteCustomTab, dropIndex,
  effectiveWidth, emptyDoc, mergeWithDefaults, moveCustomTab, normalizeTabName, parseLayoutDoc,
  removeFromThemed, sanitizePlacements, snapHeight, snapResize, updateCustomTab, visibleCards,
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
