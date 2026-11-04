import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { setLocale } from '../src/lib/i18n';
import { factoryLayout } from '../src/shell/cards';
import { emptyDoc } from '../src/shell/layoutModel';
import {
  DEVICE_KEY, applyOp, beginEdit, cancelEdit, commitEdit, createProfile, editDoc, initLayouts,
  layoutState, mutateAndSave, resetTab, themedTab,
} from '../src/shell/layoutStore.svelte';

const TAB = 'layout:engineering';
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
    handler = (c) => (c.url === '/api/ui/device/desktop' ? json(200, { device_id: 'D' }) : json(404, {}));
    await initLayouts();
    expect(layoutState.deviceId).toBe('D');
    expect(layoutState.deviceKind).toBe('desktop');
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
});
