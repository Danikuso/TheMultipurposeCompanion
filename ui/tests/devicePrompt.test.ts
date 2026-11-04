import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import DevicePrompt from '../src/shell/DevicePrompt.svelte';
import GridHeader from '../src/shell/GridHeader.svelte';
import { DEVICE_KEY, layoutState } from '../src/shell/layoutStore.svelte';

type Call = { url: string; method: string; body?: string };
let calls: Call[];
let storage: Map<string, string>;
let target: HTMLElement;
let app: Record<string, unknown> | null;
const PROFILES = [
  { device_id: 'old', name: 'Viejo', updated_at: '2026-01-01T00:00:00Z' },
  { device_id: 'new', name: 'Nuevo', updated_at: '2026-06-01T00:00:00Z' },
];
let list: unknown = PROFILES;

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const q = <T extends Element>(sel: string) => target.querySelector(sel) as T;
const click = (el: Element | null) => { (el as HTMLElement).click(); flushSync(); };
async function show() {
  app = mount(DevicePrompt, { target });
  await tick();
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
}
const settle = async () => { await new Promise((r) => setTimeout(r, 0)); await tick(); };

beforeEach(() => {
  setLocale('es');
  list = PROFILES;
  calls = [];
  storage = new Map();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => storage.get(k) ?? null,
    setItem: (k: string, v: string) => { storage.set(k, v); },
    removeItem: (k: string) => { storage.delete(k); },
  });
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    const c: Call = { url, method: init?.method ?? 'GET', body: init?.body as string | undefined };
    calls.push(c);
    if (c.method === 'GET' && url === '/api/ui/layouts') return json(200, list);
    if (c.method === 'GET') return json(200, { device_id: 'old', name: 'Viejo', updated_at: 't', doc: { version: 1, tabs: {}, custom: [], customOrder: [] } });
    return json(200, { device_id: 'dev', name: 'n', updated_at: 't', doc: { version: 1, tabs: {}, custom: [], customOrder: [] } });
  }));
  Object.assign(layoutState, { deviceId: 'dev', deviceKind: 'browser', needsPrompt: true, offline: false, profileName: null, ready: true });
  target = document.createElement('div');
  document.body.appendChild(target);
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
  vi.unstubAllGlobals();
});

describe('DevicePrompt', () => {
  test('factory option creates profile with name', async () => {
    await show();
    const input = q<HTMLInputElement>('[data-testid="device-name"]');
    expect(input.value).toBe('Navegador');
    input.value = 'PC';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    click(q('[data-testid="device-accept"]'));
    await settle();
    const put = calls.find((c) => c.method === 'PUT')!;
    expect(put.url).toBe('/api/ui/layouts/dev');
    const body = JSON.parse(put.body!);
    expect(body.name).toBe('PC');
    expect(body.doc.version).toBe(1);
    expect(layoutState.needsPrompt).toBe(false);
  });

  test('copy option calls copy-from', async () => {
    await show();
    click(q('[data-testid="device-mode-copy"]'));
    expect(q<HTMLSelectElement>('[data-testid="device-copy-select"]').value).toBe('new');
    click(q('[data-testid="device-accept"]'));
    await settle();
    const post = calls.find((c) => c.method === 'POST')!;
    expect(post.url).toBe('/api/ui/layouts/dev/copy-from/new');
  });

  test('claim option switches device id', async () => {
    await show();
    click(q('[data-testid="device-mode-claim"]'));
    click(q('[data-testid="device-accept"]'));
    await settle();
    expect(storage.get(DEVICE_KEY)).toBe('new');
  });

  test('claim hidden for desktop device', async () => {
    layoutState.deviceKind = 'desktop';
    await show();
    expect(q('[data-testid="device-mode-copy"]')).not.toBeNull();
    expect(q('[data-testid="device-mode-claim"]')).toBeNull();
    expect(q<HTMLInputElement>('[data-testid="device-name"]').value).toBe('Escritorio');
  });

  test('dismiss creates default-named factory profile', async () => {
    await show();
    const input = q<HTMLInputElement>('[data-testid="device-name"]');
    input.value = 'Otro';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    click(q('[data-testid="device-dismiss"]'));
    await settle();
    const put = calls.find((c) => c.method === 'PUT')!;
    expect(JSON.parse(put.body!).name).toBe('Navegador');
  });

  test('copy and claim hidden when no profiles', async () => {
    list = [];
    await show();
    expect(q('[data-testid="device-mode-factory"]')).not.toBeNull();
    expect(q('[data-testid="device-mode-copy"]')).toBeNull();
    expect(q('[data-testid="device-mode-claim"]')).toBeNull();
  });

  test('profile list failure is treated as empty', async () => {
    list = [{ device_id: 'x', name: 'X', updated_at: 't' }];
    (fetch as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => { throw new Error('down'); });
    await show();
    expect(q('[data-testid="device-mode-copy"]')).toBeNull();
  });

  test('failure keeps prompt open with error', async () => {
    await show();
    (fetch as ReturnType<typeof vi.fn>).mockImplementationOnce(async () => json(500, { error: 'x' }));
    click(q('[data-testid="device-accept"]'));
    await settle();
    expect(layoutState.needsPrompt).toBe(true);
    expect(q('[data-testid="device-error"]')).not.toBeNull();
  });
});

describe('offline pill', () => {
  test('offline pill shown when offline', () => {
    app = mount(GridHeader, { target, props: { title: 'X', tabKey: null } });
    expect(q('[data-testid="layout-offline"]')).toBeNull();
    unmount(app);
    layoutState.offline = true;
    app = mount(GridHeader, { target, props: { title: 'X', tabKey: null } });
    flushSync();
    const pill = q('[data-testid="layout-offline"]');
    expect(pill.textContent).toContain('Sin guardado');
    expect(pill.getAttribute('title')).toBeTruthy();
  });
});
