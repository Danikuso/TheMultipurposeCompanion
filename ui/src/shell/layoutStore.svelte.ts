import { diag } from '../lib/diag.svelte';
import { t } from '../lib/i18n';
import { isUuid, uuidv4 } from '../lib/uuid';
import {
  ApiError, copyProfile, getDesktopDeviceId, getProfile, putProfile, type Profile,
} from '../lib/layoutApi';
import { cards, factoryLayout, isFactoryCard, type CardPlacement } from './cards';
import {
  applyCardOp, emptyDoc, mergeWithDefaults, normalizeOrder, packLayout, parseLayoutDoc, pinColumns, removeFromThemed,
  snapHeight, type CardOp, type CustomTabDoc, type LayoutDoc, type PackItem, type ThemedTabDoc,
} from './layoutModel';

export const DEVICE_KEY = 'tmc.ui.device';

export type ProfileMode =
  | { kind: 'factory' }
  | { kind: 'copy'; from: string }
  | { kind: 'claim'; id: string };

export const layoutState = $state({
  ready: false,
  deviceId: null as string | null,
  deviceKind: 'browser' as 'desktop' | 'browser',
  profileName: null as string | null,
  needsPrompt: false,
  offline: false,
  doc: emptyDoc() as LayoutDoc,
  editing: false,
  draft: null as LayoutDoc | null,
  saving: false,
  saveError: false,
  /** Petición puntual para que la barra de edición abra el catálogo de tarjetas (la barra la consume). */
  catalogRequested: false,
});

const known = (id: string) => Object.hasOwn(cards, id);
const shouldAdd = (id: string) => cards[id]?.addToSaved !== false;
const widthOf = (id: string) => cards[id]?.defaultWidth ?? 1;
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
// Solo el estado HTTP (0 = sin red): las cadenas libres se redactan en el servidor.
const errStatus = (e: unknown) => (e instanceof ApiError ? e.status : 0);

function readStorage(): string | null {
  return localStorage.getItem(DEVICE_KEY);
}

function applyProfile(p: Profile): void {
  const parsed = parseLayoutDoc(p.doc, known, widthOf);
  if (parsed === null) diag('ui.layout.merge', { invalid: true });
  layoutState.doc = parsed ?? emptyDoc();
  layoutState.profileName = p.name;
  layoutState.needsPrompt = false;
}

async function loadProfile(id: string): Promise<void> {
  try {
    const p = await getProfile(id);
    if (p === null) {
      layoutState.needsPrompt = true;
      layoutState.doc = emptyDoc();
    } else {
      applyProfile(p);
    }
  } catch (e) {
    diag('ui.layout.load', { status: errStatus(e) });
    layoutState.offline = true;
    layoutState.doc = emptyDoc();
  }
}

export async function initLayouts(): Promise<void> {
  Object.assign(layoutState, {
    ready: false, deviceId: null, deviceKind: 'browser', profileName: null, needsPrompt: false,
    offline: false, doc: emptyDoc(), editing: false, draft: null, saving: false, saveError: false,
    catalogRequested: false,
  });
  let id: string | null = null;
  const wantsDesktop = new URLSearchParams(globalThis.location?.search ?? '').get('shell') === 'desktop';
  if (wantsDesktop) {
    try {
      const desktop = await getDesktopDeviceId();
      if (isUuid(desktop)) {
        id = desktop;
        layoutState.deviceKind = 'desktop';
      } else if (desktop !== null) {
        diag('ui.layout.load', { status: 0, desktop: true, invalid: true });
      }
    } catch (e) {
      diag('ui.layout.load', { status: errStatus(e), desktop: true });
    }
  }
  if (id === null) {
    try {
      const stored = readStorage();
      if (isUuid(stored)) {
        id = stored;
      } else {
        id = uuidv4();
        localStorage.setItem(DEVICE_KEY, id);
        // Un id guardado que no es UUID (corrupto o antiguo) se sustituye; uno ausente es el primer arranque.
        if (stored) diag('ui.layout.load', { status: 0, regenerated: true });
      }
    } catch {
      id = null;
      layoutState.offline = true;
      diag('ui.layout.load', { status: 0, storage: true });
    }
  }
  layoutState.deviceId = id;
  if (id !== null) await loadProfile(id);
  layoutState.ready = true;
}

function activeDoc(): LayoutDoc {
  return layoutState.draft ?? layoutState.doc;
}

export function themedTab(tabKey: string): ThemedTabDoc {
  return mergeWithDefaults(activeDoc().tabs[tabKey], factoryLayout(tabKey), known, shouldAdd);
}

export function customTab(id: string): CustomTabDoc | undefined {
  return activeDoc().custom.find((c) => c.id === id);
}

/** Pestañas propias en su orden de usuario (borrador si se está editando). */
export function customTabs(): CustomTabDoc[] {
  const doc = activeDoc();
  return doc.customOrder.flatMap((id) => doc.custom.find((c) => c.id === id) ?? []);
}

export function beginEdit(): void {
  layoutState.draft = clone($state.snapshot(layoutState.doc));
  layoutState.editing = true;
  layoutState.saveError = false;
}

export function cancelEdit(): void {
  layoutState.catalogRequested = false;
  layoutState.draft = null;
  layoutState.editing = false;
  layoutState.saveError = false;
}

export function defaultProfileName(): string {
  return t(layoutState.deviceKind === 'desktop' ? 'layout-device-default-desktop' : 'layout-device-default-browser');
}

async function save(doc: LayoutDoc): Promise<boolean> {
  const id = layoutState.deviceId;
  // Sin carga fiable del perfil guardado no se escribe: se destruiría la disposición real.
  if (id === null || layoutState.offline) {
    layoutState.saveError = true;
    return false;
  }
  const name = layoutState.profileName ?? defaultProfileName();
  layoutState.saving = true;
  try {
    await putProfile(id, name, doc);
    layoutState.profileName = name;
    layoutState.needsPrompt = false;
    layoutState.saveError = false;
    return true;
  } catch (e) {
    diag('ui.layout.save', { status: errStatus(e) });
    layoutState.saveError = true;
    return false;
  } finally {
    layoutState.saving = false;
  }
}

export async function commitEdit(): Promise<boolean> {
  if (!layoutState.editing || layoutState.draft === null) return false;
  const next = clone($state.snapshot(layoutState.draft));
  if (!(await save(next))) return false;
  layoutState.doc = next;
  layoutState.draft = null;
  layoutState.editing = false;
  return true;
}

export function editDoc(fn: (d: LayoutDoc) => LayoutDoc): void {
  // Durante el PUT en curso el borrador no cambia: lo que se guarda es lo que luego queda como vigente.
  if (!layoutState.editing || layoutState.draft === null || layoutState.saving) return;
  layoutState.draft = fn(clone($state.snapshot(layoutState.draft)));
}

export interface LayoutGeometry { columns: number; gapPx: number; heights: Record<string, number> }

const FALLBACK_HEIGHT = 160;

function pack(list: CardPlacement[], geo: LayoutGeometry, override?: { id: string; height: number }) {
  const items: PackItem[] = list.map((c) => ({
    id: c.id,
    x: c.x,
    w: c.w,
    height: override && override.id === c.id ? override.height : (geo.heights[c.id] ?? c.h ?? FALLBACK_HEIGHT),
  }));
  return packLayout(items, geo.columns, geo.gapPx).cards;
}

/** Aplica la operación a una lista; con geometría fija las columnas antes y ordena según lo que se ve después. */
function opOnList(
  list: CardPlacement[], index: number, op: CardOp, geo: LayoutGeometry | undefined,
  remove: (id: string) => CardPlacement[],
): CardPlacement[] {
  const target = list[index];
  if (!target) return list;
  const base = geo ? pinColumns(list, pack(list, geo)) : list;
  const next = op.kind === 'remove' ? remove(target.id) : applyCardOp(base, index, op);
  if (!geo) return next;
  // `remove` conserva `x` ya fijadas solo si se hizo sobre `base`.
  const source = op.kind === 'remove' ? next.map((c) => base.find((b) => b.id === c.id) ?? c) : next;
  let override: { id: string; height: number } | undefined;
  if (op.kind === 'height') {
    override = { id: target.id, height: op.h === null ? (geo.heights[target.id] ?? FALLBACK_HEIGHT) : snapHeight(op.h) };
  }
  return normalizeOrder(source, pack(source, geo, override));
}

export function applyOp(tabKey: string, index: number, op: CardOp, geo?: LayoutGeometry): void {
  editDoc((d) => {
    if (tabKey.startsWith('custom:')) {
      const id = tabKey.slice('custom:'.length);
      return {
        ...d,
        custom: d.custom.map((c) => (c.id === id
          ? { ...c, cards: opOnList(c.cards, index, op, geo, (rid) => c.cards.filter((x) => x.id !== rid)) }
          : c)),
      };
    }
    const tab = mergeWithDefaults(d.tabs[tabKey], factoryLayout(tabKey), known, shouldAdd);
    const target = tab.cards[index];
    if (!target) return d;
    let removed = tab.removed;
    const cardsNext = opOnList(tab.cards, index, op, geo, (rid) => {
      const r = removeFromThemed(tab, rid, isFactoryCard(tabKey, rid));
      removed = r.removed;
      return r.cards;
    });
    return { ...d, tabs: { ...d.tabs, [tabKey]: { cards: cardsNext, removed } } };
  });
}

/** Añade varias tarjetas al final de la pestaña en una sola modificación del borrador. */
export function addCards(tabKey: string, cardIds: string[], geo?: LayoutGeometry): void {
  editDoc((d) => {
    const custom = tabKey.startsWith('custom:');
    const customId = tabKey.slice('custom:'.length);
    const tab: ThemedTabDoc = custom
      ? { cards: d.custom.find((c) => c.id === customId)?.cards ?? [], removed: [] }
      : mergeWithDefaults(d.tabs[tabKey], factoryLayout(tabKey), known, shouldAdd);
    if (custom && !d.custom.some((c) => c.id === customId)) return d;
    let cur = geo ? pinColumns(tab.cards, pack(tab.cards, geo)) : tab.cards;
    let removed = tab.removed;
    for (const id of cardIds) {
      if (!known(id)) continue;
      removed = removed.filter((r) => r !== id);
      if (cur.some((c) => c.id === id)) continue;
      cur = [...cur, { id, w: widthOf(id), h: null }];
    }
    if (custom) return { ...d, custom: d.custom.map((c) => (c.id === customId ? { ...c, cards: cur } : c)) };
    return { ...d, tabs: { ...d.tabs, [tabKey]: { cards: cur, removed } } };
  });
}

/** Añade una tarjeta al final de la pestaña, con su ancho por defecto (borrador de edición). */
export function addCard(tabKey: string, cardId: string, geo?: LayoutGeometry): void {
  addCards(tabKey, [cardId], geo);
}

export function resetTab(tabKey: string): void {
  editDoc((d) => {
    const tabs = { ...d.tabs };
    delete tabs[tabKey];
    return { ...d, tabs };
  });
}

export async function mutateAndSave(fn: (d: LayoutDoc) => LayoutDoc): Promise<boolean> {
  const next = fn(clone($state.snapshot(layoutState.doc)));
  if (!(await save(next))) return false;
  layoutState.doc = next;
  return true;
}

export async function createProfile(name: string, mode: ProfileMode): Promise<boolean> {
  const id = layoutState.deviceId;
  try {
    if (mode.kind === 'claim') {
      const p = await getProfile(mode.id);
      if (p === null) {
        layoutState.saveError = true;
        return false;
      }
      let stored = true;
      try { localStorage.setItem(DEVICE_KEY, mode.id); } catch { stored = false; }
      layoutState.deviceId = mode.id;
      layoutState.deviceKind = 'browser';
      layoutState.offline = !stored;
      layoutState.saveError = false;
      applyProfile(p);
      return true;
    }
    if (id === null || layoutState.offline) {
      layoutState.saveError = true;
      return false;
    }
    if (mode.kind === 'factory') {
      await putProfile(id, name, emptyDoc());
      layoutState.doc = emptyDoc();
      layoutState.profileName = name;
    } else {
      applyProfile(await copyProfile(id, mode.from, name));
    }
    layoutState.needsPrompt = false;
    return true;
  } catch (e) {
    diag('ui.layout.save', { status: errStatus(e), claim: mode.kind === 'claim' });
    layoutState.saveError = true;
    return false;
  }
}
