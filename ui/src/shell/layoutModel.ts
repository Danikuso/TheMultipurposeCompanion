// Modelo puro de disposición de tarjetas: sin Svelte ni componentes (solo tipos).
import type { CardPlacement, CardWidth } from './cards';

export const H_STEP = 40;
export const H_MIN = 160;
export const H_MAX = 2000;
export const MAX_CUSTOM_TABS = 12;
export const NAME_MAX = 32;
/** Iconos de navegación (existen en `lib/icons`) que se pueden elegir para una pestaña propia. */
export const CUSTOM_TAB_ICONS: readonly string[] = [
  'exploration', 'combat', 'mining', 'trade', 'missions',
  'engineering', 'ship', 'dashboard', 'navigation', 'exobiology',
];
export const DOC_VERSION = 1;

export interface ThemedTabDoc { cards: CardPlacement[]; removed: string[] }
export interface CustomTabDoc { id: string; name: string; icon: string; cards: CardPlacement[] }
export interface LayoutDoc {
  version: 1;
  tabs: Record<string, ThemedTabDoc>;
  custom: CustomTabDoc[];
  customOrder: string[];
}
export type CardOp =
  | { kind: 'move'; to: number }
  | { kind: 'place'; to: number; x: number }
  | { kind: 'column'; x: number }
  | { kind: 'width'; w: CardWidth }
  | { kind: 'height'; h: number | null }
  | { kind: 'remove' };

type Known = (id: string) => boolean;
type WidthOf = (id: string) => CardWidth;

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const isWidth = (w: unknown): w is CardWidth => w === 1 || w === 2 || w === 3 || w === 'full';
const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

export function emptyDoc(): LayoutDoc {
  return { version: 1, tabs: {}, custom: [], customOrder: [] };
}

export function snapHeight(px: number): number {
  return clamp(Math.round(px / H_STEP) * H_STEP, H_MIN, H_MAX);
}

export function sanitizePlacements(list: unknown, known: Known, widthOf: WidthOf = () => 1): CardPlacement[] {
  if (!Array.isArray(list)) return [];
  const seen = new Set<string>();
  const out: CardPlacement[] = [];
  for (const item of list) {
    if (!isRecord(item) || typeof item.id !== 'string') continue;
    const id = item.id;
    if (!known(id) || seen.has(id)) continue;
    seen.add(id);
    const w = isWidth(item.w) ? item.w : widthOf(id);
    const h = typeof item.h === 'number' && Number.isFinite(item.h) && item.h >= H_MIN ? snapHeight(item.h) : null;
    const placement: CardPlacement = { id, w, h };
    if (typeof item.x === 'number' && Number.isInteger(item.x) && item.x >= 0 && item.x <= 3) placement.x = item.x;
    out.push(placement);
  }
  return out;
}

function sanitizeRemoved(removed: unknown, known: Known): string[] {
  if (!Array.isArray(removed)) return [];
  return [...new Set(removed.filter((r): r is string => typeof r === 'string' && known(r)))];
}

export function mergeWithDefaults(
  saved: ThemedTabDoc | undefined, factory: CardPlacement[], known: Known,
): ThemedTabDoc {
  if (!saved) return { cards: factory.map((c) => ({ ...c })), removed: [] };
  const factoryWidth = new Map(factory.map((c) => [c.id, c.w]));
  const removed = sanitizeRemoved(saved.removed, known);
  const cards = sanitizePlacements(saved.cards, known, (id) => factoryWidth.get(id) ?? 1);
  const present = new Set(cards.map((c) => c.id));
  for (const f of factory) {
    if (!present.has(f.id) && !removed.includes(f.id)) cards.push({ ...f });
  }
  return { cards, removed };
}

export function applyCardOp(list: CardPlacement[], index: number, op: CardOp): CardPlacement[] {
  if (index < 0 || index >= list.length) return list;
  const next = list.map((c) => ({ ...c }));
  switch (op.kind) {
    case 'move': {
      const [item] = next.splice(index, 1);
      next.splice(clamp(op.to, 0, next.length), 0, item);
      return next;
    }
    case 'place': {
      const [item] = next.splice(index, 1);
      item.x = op.x;
      next.splice(clamp(op.to, 0, next.length), 0, item);
      return next;
    }
    case 'column':
      next[index].x = op.x;
      return next;
    case 'width':
      next[index].w = op.w;
      return next;
    case 'height':
      next[index].h = op.h === null ? null : snapHeight(op.h);
      return next;
    case 'remove':
      next.splice(index, 1);
      return next;
  }
}

export function removeFromThemed(tab: ThemedTabDoc, id: string, isFactory: boolean): ThemedTabDoc {
  const removed = isFactory && !tab.removed.includes(id) ? [...tab.removed, id] : [...tab.removed];
  return { cards: tab.cards.filter((c) => c.id !== id), removed };
}

export function addToThemed(tab: ThemedTabDoc, id: string, w: CardWidth): ThemedTabDoc {
  const removed = tab.removed.filter((r) => r !== id);
  if (tab.cards.some((c) => c.id === id)) return { cards: tab.cards, removed };
  return { cards: [...tab.cards, { id, w, h: null }], removed };
}

export function columnsForWidth(px: number): 1 | 2 | 3 | 4 {
  if (px >= 2400) return 4;
  if (px >= 1600) return 3;
  if (px >= 900) return 2;
  return 1;
}

export function effectiveWidth(w: CardWidth, columns: number): number {
  return w === 'full' ? columns : Math.min(w, columns);
}

export function visibleCards(
  list: CardPlacement[], isVisible: (id: string) => boolean, editing: boolean,
): { placement: CardPlacement; ghost: boolean }[] {
  const out: { placement: CardPlacement; ghost: boolean }[] = [];
  for (const placement of list) {
    const visible = isVisible(placement.id);
    if (visible || editing) out.push({ placement, ghost: !visible });
  }
  return out;
}

export function normalizeTabName(s: string): string | null {
  const t = s.trim();
  return t.length >= 1 && t.length <= NAME_MAX ? t : null;
}

export function createCustomTab(doc: LayoutDoc, name: string, icon: string, id: string): LayoutDoc | null {
  const n = normalizeTabName(name);
  if (n === null || doc.custom.length >= MAX_CUSTOM_TABS || doc.custom.some((c) => c.id === id)) return null;
  return {
    ...doc,
    custom: [...doc.custom, { id, name: n, icon, cards: [] }],
    customOrder: [...doc.customOrder, id],
  };
}

export function updateCustomTab(
  doc: LayoutDoc, id: string, patch: { name?: string; icon?: string; cards?: CardPlacement[] },
): LayoutDoc {
  const name = patch.name === undefined ? null : normalizeTabName(patch.name);
  return {
    ...doc,
    custom: doc.custom.map((c) => (c.id !== id ? c : {
      ...c,
      name: name ?? c.name,
      icon: patch.icon ?? c.icon,
      cards: patch.cards ?? c.cards,
    })),
  };
}

export function deleteCustomTab(doc: LayoutDoc, id: string): LayoutDoc {
  return {
    ...doc,
    custom: doc.custom.filter((c) => c.id !== id),
    customOrder: doc.customOrder.filter((o) => o !== id),
  };
}

export function moveCustomTab(doc: LayoutDoc, id: string, delta: -1 | 1): LayoutDoc {
  const i = doc.customOrder.indexOf(id);
  const j = i + delta;
  if (i < 0 || j < 0 || j >= doc.customOrder.length) return doc;
  const order = [...doc.customOrder];
  [order[i], order[j]] = [order[j], order[i]];
  return { ...doc, customOrder: order };
}

export function addToCustom(tab: CustomTabDoc, id: string, w: CardWidth): CustomTabDoc {
  if (tab.cards.some((c) => c.id === id)) return tab;
  return { ...tab, cards: [...tab.cards, { id, w, h: null }] };
}

export function parseLayoutDoc(raw: unknown, known: Known, widthOf: WidthOf = () => 1): LayoutDoc | null {
  if (!isRecord(raw) || raw.version !== DOC_VERSION) return null;
  const doc = emptyDoc();
  if (isRecord(raw.tabs)) {
    for (const [key, tab] of Object.entries(raw.tabs)) {
      if (!isRecord(tab)) continue;
      doc.tabs[key] = {
        cards: sanitizePlacements(tab.cards, known, widthOf),
        removed: sanitizeRemoved(tab.removed, known),
      };
    }
  }
  if (Array.isArray(raw.custom)) {
    for (const c of raw.custom) {
      if (doc.custom.length >= MAX_CUSTOM_TABS) break;
      if (!isRecord(c) || typeof c.id !== 'string' || typeof c.name !== 'string') continue;
      const name = normalizeTabName(c.name);
      if (name === null || doc.custom.some((x) => x.id === c.id)) continue;
      doc.custom.push({
        id: c.id,
        name,
        icon: typeof c.icon === 'string' && CUSTOM_TAB_ICONS.includes(c.icon) ? c.icon : '',
        cards: sanitizePlacements(c.cards, known, widthOf),
      });
    }
  }
  const ids = new Set(doc.custom.map((c) => c.id));
  const order = Array.isArray(raw.customOrder)
    ? raw.customOrder.filter((o): o is string => typeof o === 'string' && ids.has(o))
    : [];
  doc.customOrder = [...new Set(order)];
  for (const c of doc.custom) if (!doc.customOrder.includes(c.id)) doc.customOrder.push(c.id);
  return doc;
}

export function dropIndex(
  x: number, y: number, rects: { left: number; top: number; width: number; height: number }[], from: number,
): number {
  if (rects.length === 0) return 0;
  let best = 0;
  let bestDist = Infinity;
  let bestCx = 0;
  let bestCy = 0;
  rects.forEach((r, i) => {
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2;
    const d = (x - cx) ** 2 + (y - cy) ** 2;
    if (d < bestDist) { best = i; bestDist = d; bestCx = cx; bestCy = cy; }
  });
  const idx = best < from && (x > bestCx || y > bestCy) ? best + 1 : best;
  return clamp(idx, 0, rects.length - 1);
}

export function snapResize(
  start: { w: CardWidth; h: number | null; heightPx: number }, dx: number, dy: number,
  colWidthPx: number, gapPx: number, columns: number, maxSpan?: number,
): { w: CardWidth; h: number | null } {
  // El valor guardado solo cambia si cambia el ancho efectivo: ajustar el alto o un clic no lo tocan.
  let w: CardWidth = start.w;
  if (dx !== 0) {
    const cur = effectiveWidth(start.w, columns);
    const currentPx = cur * colWidthPx + (cur - 1) * gapPx;
    const n = clamp(Math.round((currentPx + dx) / (colWidthPx + gapPx)), 1, Math.max(1, maxSpan ?? columns));
    if (n !== cur) w = n === columns && columns >= 2 ? 'full' : (n as 1 | 2 | 3);
  }
  const h = dy === 0 ? start.h : snapHeight(start.heightPx + dy);
  return { w, h };
}

export interface PackItem { id: string; x?: number | null; w: CardWidth; height: number }
export interface PackedCard { id: string; col: number; span: number; top: number; height: number }

const hasX = (x: unknown): x is number => typeof x === 'number' && Number.isInteger(x);

/** Empaquetado con gravedad vertical: cada tarjeta conserva su columna de inicio y sube hasta tocar a otra. */
export function packLayout(
  items: PackItem[], columns: number, gapPx: number,
): { cards: PackedCard[]; height: number } {
  const bottoms: number[] = new Array(Math.max(1, columns)).fill(0);
  const cards: PackedCard[] = [];
  let height = 0;
  const topAt = (col: number, span: number) => {
    let top = 0;
    for (let c = col; c < col + span; c++) top = Math.max(top, bottoms[c]);
    return top;
  };
  for (const it of items) {
    const span = effectiveWidth(it.w, columns);
    let col = 0;
    if (it.w !== 'full') {
      if (hasX(it.x)) col = clamp(it.x, 0, columns - span);
      else {
        let best = Infinity;
        for (let c = 0; c + span <= columns; c++) {
          const t = topAt(c, span);
          if (t < best) { best = t; col = c; }
        }
      }
    }
    const top = topAt(col, span);
    cards.push({ id: it.id, col, span, top, height: it.height });
    if (it.height > 0) {
      for (let c = col; c < col + span; c++) bottoms[c] = top + it.height + gapPx;
      height = Math.max(height, top + it.height);
    }
  }
  return { cards, height };
}

/** Fija la columna calculada de las tarjetas automáticas; las que ya tienen `x` la conservan (aunque esté recortada). */
export function pinColumns(list: CardPlacement[], packed: PackedCard[]): CardPlacement[] {
  const byId = new Map(packed.map((c) => [c.id, c]));
  return list.map((c) => {
    const p = byId.get(c.id);
    return hasX(c.x) || !p ? { ...c } : { ...c, x: p.col };
  });
}

/** Ordena de forma estable por (top, col) del empaquetado; las que no están empaquetadas van al final. */
export function normalizeOrder(list: CardPlacement[], packed: PackedCard[]): CardPlacement[] {
  const byId = new Map(packed.map((c) => [c.id, c]));
  const known = list.filter((c) => byId.has(c.id));
  const rest = list.filter((c) => !byId.has(c.id));
  known.sort((a, b) => {
    const pa = byId.get(a.id)!;
    const pb = byId.get(b.id)!;
    return pa.top - pb.top || pa.col - pb.col;
  });
  return [...known, ...rest].map((c) => ({ ...c }));
}

/** Índice de la tarjeta anterior (-1) o siguiente (+1) de la lista que comparte columna; null si no hay. */
export function verticalNeighbor(
  list: CardPlacement[], packed: PackedCard[], index: number, dir: -1 | 1,
): number | null {
  const byId = new Map(packed.map((c) => [c.id, c]));
  const me = list[index] && byId.get(list[index].id);
  if (!me) return null;
  for (let i = index + dir; i >= 0 && i < list.length; i += dir) {
    const o = byId.get(list[i].id);
    if (o && o.col < me.col + me.span && me.col < o.col + o.span) return i;
  }
  return null;
}

/** Columna de inicio más cercana al borde izquierdo arrastrado e índice de inserción en la lista sin la tarjeta. */
export function dropTarget(
  dragged: { id: string; span: number; leftPx: number; topPx: number },
  list: CardPlacement[], packed: PackedCard[], colWidthPx: number, gapPx: number, columns: number,
): { x: number; index: number } {
  const x = clamp(Math.round(dragged.leftPx / (colWidthPx + gapPx)), 0, Math.max(0, columns - dragged.span));
  const byId = new Map(packed.map((c) => [c.id, c]));
  const others = list.filter((c) => c.id !== dragged.id);
  const found = others.findIndex((c) => {
    const p = byId.get(c.id);
    return p !== undefined && p.top >= dragged.topPx;
  });
  return { x, index: found === -1 ? others.length : found };
}
