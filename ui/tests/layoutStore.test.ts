import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { setLocale } from '../src/lib/i18n';
import { factoryLayout } from '../src/shell/cards';
import { emptyDoc, type CustomTabDoc } from '../src/shell/layoutModel';
import {
  DEVICE_KEY, addCard, addCards, applyOp, beginEdit, cancelEdit, commitEdit, createProfile, editDoc, initLayouts,
  layoutState, mutateAndSave, resetTab, themedTab,
} from '../src/shell/layoutStore.svelte';

const TAB = 'layout:engineering';
const DESKTOP = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd';
const V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
type Call = { url: string; method: string; body?: string };
let calls: Call[];
let storage: Map<string, string>;
let handler: (c: Call) => Response | Promise<Response>;

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const echo = (c: Call) =>
  json(200, { device_id: 'x', name: 'n', updated_at: 't', doc: JSON.parse(c.body ?? '{}').doc });
const ids = () => themedTab(TAB).cards.map((c) => c.id);

function stubEnv(search = '') {
  calls = [];
  storage = new Map();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => storage.get(k) ?? null,
    setItem: (k: string, v: string) => { storage.set(k, v); },
    removeItem: (k: string) => { storage.delete(k); },
  });
  vi.stubGlobal('location', { search });
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    const c: Call = { url, method: init?.method ?? 'GET', body: init?.body as string | undefined };
    calls.push(c);
    return handler(c);
  }));
}

beforeEach(() => {
  setLocale('es');
  handler = () => json(404, { error: 'not-found' });
  stubEnv();
});
afterEach(() => { vi.unstubAllGlobals(); });

describe('layoutStore', () => {
  test('engineer guide can be added and persisted without changing pre-existing customized card positions', async () => {
    await initLayouts();
    const saved = factoryLayout(TAB).filter(card => card.id !== 'engineer-guide').map((card, index) => ({ ...card, x: index % 2, h: 320 }));
    const doc = emptyDoc(); doc.tabs[TAB] = { cards: saved, removed: [] };
    layoutState.doc = doc;
    expect(themedTab(TAB).cards).toEqual(saved);
    beginEdit(); addCard(TAB, 'engineer-guide');
    expect(themedTab(TAB).cards.slice(0, saved.length)).toEqual(saved);
    expect(themedTab(TAB).cards.at(-1)).toEqual({ id: 'engineer-guide', w: 2, h: null });
    handler = echo;
    expect(await commitEdit()).toBe(true);
    const write = calls.filter(call => call.method === 'PUT').at(-1)!;
    const persisted = JSON.parse(write.body!).doc;
    handler = () => json(200, { device_id: 'x', name: 'Synthetic', updated_at: 't', doc: persisted });
    await initLayouts();
    expect(themedTab(TAB).cards.slice(0, saved.length)).toEqual(saved);
    expect(ids().filter(id => id === 'engineer-guide')).toHaveLength(1);
  });
  test('engineer guide is available in a custom tab as well', async () => {
    await initLayouts();
    const doc = emptyDoc();
    doc.custom = [{ id: 'guide-tab', name: 'Synthetic', icon: 'wrench', cards: [] }];
    doc.customOrder = ['guide-tab'];
    layoutState.doc = doc;
    beginEdit(); addCards('custom:guide-tab', ['engineer-guide']);
    expect(layoutState.draft?.custom[0].cards).toEqual([{ id: 'engineer-guide', w: 2, h: null }]);
    handler = echo;
    expect(await commitEdit()).toBe(true);
    expect(layoutState.doc.custom[0].cards).toEqual([{ id: 'engineer-guide', w: 2, h: null }]);
  });
  test('init generates and stores device id when missing', async () => {
    await initLayouts();
    const id = storage.get(DEVICE_KEY);
    expect(id).toMatch(/^[0-9a-f-]{36}$/);
    expect(layoutState.deviceId).toBe(id);
    expect(layoutState.needsPrompt).toBe(true);
    expect(layoutState.ready).toBe(true);
  });

  test('init uses desktop id when shell=desktop', async () => {
    stubEnv('?shell=desktop');
    handler = (c) => (c.url === '/api/ui/device/desktop' ? json(200, { device_id: DESKTOP }) : json(404, {}));
    await initLayouts();
    expect(layoutState.deviceId).toBe(DESKTOP);
    expect(layoutState.deviceKind).toBe('desktop');
  });

  test('init ignores an invalid desktop id and falls back to storage', async () => {
    stubEnv('?shell=desktop');
    handler = (c) => (c.url === '/api/ui/device/desktop' ? json(200, { device_id: 'D' }) : json(404, {}));
    await initLayouts();
    expect(layoutState.deviceId).not.toBe('D');
    expect(layoutState.deviceId).toBe(storage.get(DEVICE_KEY));
    expect(layoutState.deviceKind).toBe('browser');
    expect(layoutState.offline).toBe(false);
  });

  test('init regenerates an invalid stored device id', async () => {
    storage.set(DEVICE_KEY, 'basura');
    await initLayouts();
    const id = storage.get(DEVICE_KEY);
    expect(id).not.toBe('basura');
    expect(id).toMatch(V4);
    expect(layoutState.deviceId).toBe(id);
    expect(layoutState.offline).toBe(false);
    expect(layoutState.needsPrompt).toBe(true);
    expect(calls.some((c) => c.url.includes('basura'))).toBe(false);
  });

  test('init works in an insecure context without crypto.randomUUID', async () => {
    vi.stubGlobal('crypto', {
      getRandomValues: (a: Uint8Array) => { for (let i = 0; i < a.length; i++) a[i] = (i * 17 + 3) & 0xff; return a; },
    });
    await initLayouts();
    expect(layoutState.offline).toBe(false);
    expect(layoutState.needsPrompt).toBe(true);
    expect(layoutState.deviceId).toMatch(V4);
  });

  test('init falls back to factory on corrupt doc and does not write', async () => {
    handler = () => json(200, { device_id: 'x', name: 'N', updated_at: 't', doc: { version: 99 } });
    await initLayouts();
    expect(themedTab(TAB).cards).toEqual(factoryLayout(TAB));
    expect(calls.some((c) => c.method === 'PUT')).toBe(false);
  });

  test('init survives blocked localStorage and network failure', async () => {
    vi.stubGlobal('localStorage', {
      getItem: () => { throw new Error('blocked'); },
      setItem: () => { throw new Error('blocked'); },
    });
    vi.stubGlobal('fetch', vi.fn(async () => { throw new TypeError('network'); }));
    await expect(initLayouts()).resolves.toBeUndefined();
    expect(layoutState.ready).toBe(true);
    expect(layoutState.offline).toBe(true);
  });

  test('edit session cancel restores doc', async () => {
    await initLayouts();
    const before = ids();
    beginEdit();
    applyOp(TAB, 0, { kind: 'move', to: 2 });
    expect(ids()).not.toEqual(before);
    cancelEdit();
    expect(layoutState.editing).toBe(false);
    expect(ids()).toEqual(before);
  });

  test('commit failure keeps draft and editing', async () => {
    await initLayouts();
    const before = ids();
    beginEdit();
    applyOp(TAB, 0, { kind: 'move', to: 2 });
    const moved = ids();
    expect(moved).not.toEqual(before);
    handler = () => json(500, { error: 'boom' });
    expect(await commitEdit()).toBe(false);
    expect(layoutState.editing).toBe(true);
    expect(layoutState.saveError).toBe(true);
    expect(ids()).toEqual(moved);
    handler = echo;
    expect(await commitEdit()).toBe(true);
    expect(layoutState.editing).toBe(false);
    expect(layoutState.saveError).toBe(false);
    expect(ids()).toEqual(moved);
  });

  test('edits are ignored while a save is in flight', async () => {
    await initLayouts();
    beginEdit();
    applyOp(TAB, 0, { kind: 'move', to: 2 });
    const snapshot = ids();
    let release!: (r: Response) => void;
    handler = () => new Promise<Response>((res) => { release = res; });
    const pending = commitEdit();
    expect(layoutState.saving).toBe(true);
    applyOp(TAB, 0, { kind: 'move', to: 1 });
    expect(ids()).toEqual(snapshot);
    release(json(200, { device_id: 'x', name: 'n', updated_at: 't', doc: {} }));
    expect(await pending).toBe(true);
    expect(ids()).toEqual(snapshot);
  });

  test('commit without profile uses default device name', async () => {
    await initLayouts();
    expect(layoutState.profileName).toBeNull();
    handler = echo;
    beginEdit();
    applyOp(TAB, 0, { kind: 'width', w: 'full' });
    expect(await commitEdit()).toBe(true);
    expect(layoutState.profileName).toBe('Navegador');
    expect(layoutState.needsPrompt).toBe(false);
    expect(JSON.parse(calls.find((c) => c.method === 'PUT')?.body ?? '{}').name).toBe('Navegador');
  });

  test('resetTab clears saved tab', async () => {
    await initLayouts();
    beginEdit();
    applyOp(TAB, 0, { kind: 'remove' });
    expect(themedTab(TAB).cards.length).toBe(factoryLayout(TAB).length - 1);
    expect(themedTab(TAB).removed).toHaveLength(1);
    resetTab(TAB);
    expect(themedTab(TAB).cards).toEqual(factoryLayout(TAB));
    expect(themedTab(TAB).removed).toEqual([]);
    editDoc((d) => d);
    cancelEdit();
  });

  test('mutateAndSave persists outside edit mode', async () => {
    await initLayouts();
    handler = echo;
    expect(await mutateAndSave((d) => ({ ...d, customOrder: [] }))).toBe(true);
    expect(layoutState.profileName).toBe('Navegador');
    expect(calls.filter((c) => c.method === 'PUT')).toHaveLength(1);
  });

  test('createProfile claim switches device id', async () => {
    await initLayouts();
    handler = (c) => (c.url.endsWith('/api/ui/layouts/OLD')
      ? json(200, { device_id: 'OLD', name: 'Viejo', updated_at: 't', doc: emptyDoc() })
      : json(404, {}));
    expect(await createProfile('', { kind: 'claim', id: 'OLD' })).toBe(true);
    expect(layoutState.deviceId).toBe('OLD');
    expect(storage.get(DEVICE_KEY)).toBe('OLD');
    expect(layoutState.profileName).toBe('Viejo');
    expect(layoutState.needsPrompt).toBe(false);
  });

  test('claim against missing profile leaves device untouched', async () => {
    await initLayouts();
    const before = layoutState.deviceId;
    const stored = storage.get(DEVICE_KEY);
    expect(await createProfile('', { kind: 'claim', id: 'GONE' })).toBe(false);
    expect(layoutState.deviceId).toBe(before);
    expect(storage.get(DEVICE_KEY)).toBe(stored);
    expect(layoutState.saveError).toBe(true);
  });

  test('load failure never overwrites saved profile', async () => {
    handler = () => json(500, { error: 'boom' });
    await initLayouts();
    expect(layoutState.offline).toBe(true);
    beginEdit();
    applyOp(TAB, 0, { kind: 'move', to: 2 });
    expect(await commitEdit()).toBe(false);
    expect(layoutState.editing).toBe(true);
    expect(layoutState.saveError).toBe(true);
    expect(await mutateAndSave((d) => d)).toBe(false);
    expect(await createProfile('X', { kind: 'factory' })).toBe(false);
    expect(calls.some((c) => c.method === 'PUT')).toBe(false);
  });

  describe('geometry and addCards', () => {
    const CT = 'custom:c1';
    const place = (id: string, w: 1 | 2 | 3 | 'full' = 1, x?: number) =>
      ({ id, w, h: null, ...(x === undefined ? {} : { x }) });
    async function withCustom(cardsList: ReturnType<typeof place>[]) {
      await initLayouts();
      const tab: CustomTabDoc = { id: 'c1', name: 'X', icon: 'ship', cards: cardsList };
      layoutState.doc = { ...emptyDoc(), custom: [tab], customOrder: ['c1'] };
      beginEdit();
    }
    const custom = () => layoutState.draft!.custom[0].cards;

    test('applyOp pins automatic columns of the whole tab', async () => {
      await withCustom([place('route'), place('jump'), place('target')]);
      applyOp(CT, 2, { kind: 'height', h: 200 }, {
        columns: 2, gapPx: 10, heights: { route: 300, jump: 100, target: 100 },
      });
      expect(custom().map((c) => c.x)).toEqual([0, 1, 1]);
    });

    test('applyOp place reorders by top then column', async () => {
      const ids7 = ['target', 'ship', 'jump', 'sell', 'search', 'rewards'];
      await withCustom(ids7.map((i) => place(i)).concat(place('trade-session')));
      const heights = Object.fromEntries(custom().map((c) => [c.id, 100]));
      applyOp(CT, 6, { kind: 'place', to: 3, x: 2 }, { columns: 3, gapPx: 10, heights });
      const list = custom();
      const moved = list.find((c) => c.id === 'trade-session')!;
      expect(moved.x).toBe(2);
      const geoPacked = list.map((c) => ({ id: c.id, x: c.x }));
      expect(geoPacked.every((c) => Number.isInteger(c.x))).toBe(true);
      expect(list.map((c) => c.id)).toEqual(['target', 'ship', 'jump', 'sell', 'search', 'trade-session', 'rewards']);
    });

    test('applyOp keeps clamped explicit x', async () => {
      await withCustom([place('planner', 2, 2), place('jump')]);
      applyOp(CT, 1, { kind: 'height', h: 200 }, { columns: 3, gapPx: 10, heights: {} });
      expect(custom().find((c) => c.id === 'planner')!.x).toBe(2);
    });

    test('applyOp without geometry neither pins nor sorts', async () => {
      await withCustom([place('route'), place('jump')]);
      applyOp(CT, 1, { kind: 'width', w: 2 });
      expect(custom().every((c) => !('x' in c))).toBe(true);
    });

    test('addCards adds several in one draft change', async () => {
      await withCustom([place('jump')]);
      // Every editDoc clones the draft with exactly one JSON.stringify, so this counts draft changes.
      const spy = vi.spyOn(JSON, 'stringify');
      addCards(CT, ['ax', 'cargo', 'nope', 'jump', 'ax']);
      const changes = spy.mock.calls.length;
      spy.mockRestore();
      expect(changes).toBe(1);
      expect(custom().map((c) => c.id)).toEqual(['jump', 'ax', 'cargo']);
      expect(custom().every((c) => !('x' in c))).toBe(true);
    });

    test('addCards with geometry pins existing automatic columns', async () => {
      await withCustom([place('route'), place('jump')]);
      addCards(CT, ['ax'], { columns: 2, gapPx: 10, heights: { route: 300, jump: 100 } });
      expect(custom().map((c) => c.x)).toEqual([0, 1, undefined]);
      expect(custom().map((c) => c.id)).toEqual(['route', 'jump', 'ax']);
    });

    test('addCard wraps addCards', async () => {
      await withCustom([place('jump')]);
      addCard(CT, 'ax');
      expect(custom().map((c) => c.id)).toEqual(['jump', 'ax']);
    });

    test('addCards in themed tab clears removed', async () => {
      await initLayouts();
      beginEdit();
      applyOp(TAB, 0, { kind: 'remove' });
      const gone = factoryLayout(TAB)[0].id;
      expect(layoutState.draft!.tabs[TAB].removed).toContain(gone);
      addCards(TAB, [gone]);
      expect(layoutState.draft!.tabs[TAB].removed).not.toContain(gone);
      expect(ids()).toContain(gone);
    });

    test('old profile without x renders and pins on first edit', async () => {
      await initLayouts();
      expect(themedTab(TAB).cards.every((c) => !('x' in c))).toBe(true);
      beginEdit();
      const heights = Object.fromEntries(themedTab(TAB).cards.map((c) => [c.id, 120]));
      applyOp(TAB, 0, { kind: 'width', w: 1 }, { columns: 3, gapPx: 10, heights });
      expect(themedTab(TAB).cards.every((c) => Number.isInteger(c.x))).toBe(true);
    });
  });
});
