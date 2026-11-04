import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import LayoutSettings from '../src/settings/LayoutSettings.svelte';
import { emptyDoc, type LayoutDoc } from '../src/shell/layoutModel';
import { askConfirm } from '../src/lib/confirm';
import { layoutState } from '../src/shell/layoutStore.svelte';

vi.mock('../src/lib/confirm', () => ({ askConfirm: vi.fn(() => true) }));

type Call = { url: string; method: string; body?: string };
let calls: Call[];
let target: HTMLElement;
let app: Record<string, unknown> | null = null;
let handler: (c: Call) => Response | Promise<Response>;

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const PROFILES = [
  { device_id: 'B', name: 'Portátil', updated_at: '2026-11-03T10:00:00Z' },
  { device_id: 'A', name: 'Escritorio', updated_at: '2026-11-01T10:00:00Z' },
  { device_id: 'C', name: 'Tablet', updated_at: '2026-11-04T10:00:00Z' },
];
const customDoc = (): LayoutDoc => ({
  ...emptyDoc(),
  tabs: { 'layout:engineering': { cards: [], removed: [] } },
  custom: [
    { id: 'c1', name: 'Uno', icon: 'ship', cards: [{ id: 'sources', w: 1, h: null }] },
    { id: 'c2', name: 'Dos', icon: 'trade', cards: [] },
  ],
  customOrder: ['c1', 'c2'],
});

async function open() {
  app = mount(LayoutSettings, { target });
  await vi.waitFor(() => expect(q('profile-row-A')).not.toBeNull());
}
const q = (id: string) => target.querySelector(`[data-testid="${id}"]`) as HTMLElement | null;
const click = async (id: string) => {
  (q(id) as HTMLElement).click();
  flushSync();
  await Promise.resolve();
};
const rowIds = () => Array.from(target.querySelectorAll('[data-testid^="profile-row-"]'))
  .map((e) => e.getAttribute('data-testid')!.slice('profile-row-'.length));

beforeEach(() => {
  setLocale('es');
  calls = [];
  handler = (c) => {
    if (c.method === 'GET' && c.url === '/api/ui/layouts') return json(200, PROFILES);
    return json(404, { error: 'not-found' });
  };
  vi.stubGlobal('localStorage', { getItem: () => 'A', setItem: () => {}, removeItem: () => {} });
  vi.stubGlobal('location', { search: '' });
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    const c: Call = { url, method: init?.method ?? 'GET', body: init?.body as string | undefined };
    calls.push(c);
    return handler(c);
  }));
  Object.assign(layoutState, {
    ready: true, deviceId: 'A', profileName: 'Escritorio', offline: false, editing: false,
    draft: null, doc: customDoc(),
  });
  target = document.createElement('div');
  document.body.appendChild(target);
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
  vi.unstubAllGlobals();
  layoutState.doc = emptyDoc();
});

describe('LayoutSettings', () => {
  test('lists profiles and marks current', async () => {
    await open();
    expect(rowIds()).toEqual(['A', 'C', 'B']);
    expect(q('profile-row-A')!.textContent).toContain('Escritorio');
    expect(q('profile-current-A')).not.toBeNull();
    expect(q('profile-current-B')).toBeNull();
  });

  test('rename current updates state', async () => {
    handler = (c) => {
      if (c.method === 'PATCH') return json(200, { device_id: 'A', name: 'Mi PC', updated_at: 't', doc: {} });
      return json(200, PROFILES);
    };
    await open();
    await click('profile-rename-A');
    const input = q('profile-rename-input') as HTMLInputElement;
    input.value = 'Mi PC';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await click('profile-rename-save');
    await vi.waitFor(() => expect(layoutState.profileName).toBe('Mi PC'));
    const patch = calls.find((c) => c.method === 'PATCH')!;
    expect(patch.url).toBe('/api/ui/layouts/A');
    expect(JSON.parse(patch.body!)).toEqual({ name: 'Mi PC' });
  });

  test('delete disabled for current', async () => {
    await open();
    expect((q('profile-delete-A') as HTMLButtonElement).disabled).toBe(true);
    expect((q('profile-delete-A') as HTMLButtonElement).title).not.toBe('');
    expect((q('profile-delete-B') as HTMLButtonElement).disabled).toBe(false);
    await click('profile-delete-B');
    await vi.waitFor(() => expect(calls.some((c) => c.method === 'DELETE' && c.url === '/api/ui/layouts/B')).toBe(true));
  });

  test('copy to this device reloads doc', async () => {
    const copied = { device_id: 'A', name: 'Escritorio', updated_at: 't', doc: { version: 1, tabs: {}, custom: [], customOrder: [] } };
    handler = (c) => {
      if (c.method === 'POST') return json(200, copied);
      if (c.url === '/api/ui/layouts/A') return json(200, copied);
      return json(200, PROFILES);
    };
    await open();
    await click('profile-copy-B');
    await vi.waitFor(() => expect(layoutState.doc.custom).toEqual([]));
    const post = calls.find((c) => c.method === 'POST')!;
    expect(post.url).toBe('/api/ui/layouts/A/copy-from/B');
    expect(JSON.parse(post.body!)).toEqual({ name: 'Escritorio' });
  });

  test('reset all keeps custom tabs', async () => {
    handler = (c) => (c.method === 'PUT' ? json(200, { device_id: 'A', name: 'Escritorio', updated_at: 't', doc: {} }) : json(200, PROFILES));
    await open();
    await click('layout-reset-all');
    await vi.waitFor(() => expect(layoutState.doc.tabs).toEqual({}));
    expect(layoutState.doc.custom).toHaveLength(2);
    const put = JSON.parse(calls.find((c) => c.method === 'PUT')!.body!);
    expect(put.doc.tabs).toEqual({});
    expect(put.doc.custom).toHaveLength(2);
  });

  test('reorder custom tabs persists', async () => {
    handler = (c) => (c.method === 'PUT' ? json(200, { device_id: 'A', name: 'Escritorio', updated_at: 't', doc: {} }) : json(200, PROFILES));
    await open();
    expect((q('custom-up-c1') as HTMLButtonElement).disabled).toBe(true);
    await click('custom-down-c1');
    await vi.waitFor(() => expect(layoutState.doc.customOrder).toEqual(['c2', 'c1']));
    const put = JSON.parse(calls.find((c) => c.method === 'PUT')!.body!);
    expect(put.doc.customOrder).toEqual(['c2', 'c1']);
  });

  test('delete custom tab persists', async () => {
    handler = (c) => (c.method === 'PUT' ? json(200, { device_id: 'A', name: 'Escritorio', updated_at: 't', doc: {} }) : json(200, PROFILES));
    await open();
    await click('custom-delete-c1');
    await vi.waitFor(() => expect(layoutState.doc.customOrder).toEqual(['c2']));
    expect(layoutState.doc.custom.map((c) => c.id)).toEqual(['c2']);
  });

  test('mutating actions are disabled while editing', async () => {
    layoutState.editing = true;
    await open();
    expect(q('layout-settings-editing-hint')).not.toBeNull();
    for (const id of ['profile-rename-B', 'profile-copy-B', 'profile-delete-B', 'layout-reset-all', 'custom-down-c1', 'custom-delete-c1']) {
      expect((q(id) as HTMLButtonElement).disabled, id).toBe(true);
    }
    layoutState.editing = false;
  });

  test('mutating actions are disabled while offline', async () => {
    layoutState.offline = true;
    await open();
    for (const id of ['profile-rename-B', 'profile-copy-B', 'profile-delete-B', 'layout-reset-all', 'custom-down-c1', 'custom-delete-c1']) {
      expect((q(id) as HTMLButtonElement).disabled, id).toBe(true);
    }
  });

  test('api failure shows inline error without crashing', async () => {
    handler = (c) => (c.method === 'DELETE' ? json(500, { error: 'boom' }) : json(200, PROFILES));
    await open();
    await click('profile-delete-B');
    await vi.waitFor(() => expect(q('layout-settings-error')).not.toBeNull());
    expect(q('profile-row-B')).not.toBeNull();
  });
  test('reset button precedes the custom tabs heading', async () => {
    await open();
    const reset = q('layout-reset-all')!;
    const heading = q('layout-settings-tabs-title')!;
    expect(reset.compareDocumentPosition(heading) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  async function startRenameB() {
    await click('profile-rename-B');
    const input = q('profile-rename-input') as HTMLInputElement;
    input.value = 'Nuevo';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    return input;
  }

  test('rename input submits on Enter', async () => {
    handler = (c) => (c.method === 'PATCH' ? json(200, { device_id: 'B', name: 'Nuevo', updated_at: 't', doc: {} }) : json(200, PROFILES));
    await open();
    const input = await startRenameB();
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await vi.waitFor(() => expect(calls.some((c) => c.method === 'PATCH' && c.url === '/api/ui/layouts/B')).toBe(true));
  });

  test('rename input cancels on Escape without request and stops propagation', async () => {
    await open();
    const input = await startRenameB();
    const outer = vi.fn();
    document.addEventListener('keydown', outer);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    flushSync();
    document.removeEventListener('keydown', outer);
    expect(outer).not.toHaveBeenCalled();
    expect(q('profile-rename-input')).toBeNull();
    expect(calls.some((c) => c.method === 'PATCH')).toBe(false);
  });

  test('declining confirmation sends no request', async () => {
    await open();
    vi.mocked(askConfirm).mockReturnValueOnce(false);
    await click('profile-delete-B');
    vi.mocked(askConfirm).mockReturnValueOnce(false);
    await click('layout-reset-all');
    await new Promise((r) => setTimeout(r, 10));
    expect(calls.filter((c) => c.method !== 'GET')).toEqual([]);
  });
});

