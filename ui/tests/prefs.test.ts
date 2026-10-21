import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { prefs, setAutoCompanion } from '../src/lib/prefs.svelte';
import { setLocale } from '../src/lib/i18n';
import { disconnect, live } from '../src/lib/ws.svelte';
import TopBar from '../src/shell/TopBar.svelte';

const cfg = { language: 'es', auto_companion: true, server: { port: 47320, lan: false } };

describe('auto-companion toggle', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown>;
  let fetchMock: ReturnType<typeof vi.fn>;
  beforeEach(() => {
    disconnect(); setLocale('es'); prefs.autoCompanion = true;
    fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
      if (init?.method === 'PUT') return { ok: true, status: 200, json: async () => JSON.parse(init.body as string) };
      return { ok: true, status: 200, json: async () => cfg };
    });
    vi.stubGlobal('fetch', fetchMock);
    target = document.createElement('div'); document.body.appendChild(target);
    instance = mount(TopBar, { target }); flushSync();
  });
  afterEach(() => { unmount(instance); target.remove(); vi.unstubAllGlobals(); disconnect(); void live; });

  test('TopBar shows an accessible switch with label and tooltip, ON by default', () => {
    const sw = target.querySelector('[data-testid="auto-companion"]') as HTMLInputElement;
    expect(sw).not.toBeNull();
    expect(sw.getAttribute('role')).toBe('switch');
    expect(sw.checked).toBe(true);
    expect(sw.getAttribute('aria-label') || target.querySelector('label')?.textContent).toBeTruthy();
    expect(target.querySelector('label[title]')).not.toBeNull();
  });

  test('toggling sends PUT /api/config with auto_companion false and keeps the rest', async () => {
    const sw = target.querySelector('[data-testid="auto-companion"]') as HTMLInputElement;
    sw.click(); flushSync();
    expect(prefs.autoCompanion).toBe(false);
    await vi.waitFor(() => expect(fetchMock.mock.calls.some((c) => c[1]?.method === 'PUT')).toBe(true));
    const put = fetchMock.mock.calls.find((c) => c[1]?.method === 'PUT')!;
    expect(JSON.parse(put[1].body)).toEqual({ ...cfg, auto_companion: false });
  });

  test('a failed PUT reverts the switch', async () => {
    fetchMock.mockImplementation(async (_url: string, init?: RequestInit) =>
      init?.method === 'PUT' ? { ok: false, status: 400, json: async () => ({}) } : { ok: true, status: 200, json: async () => cfg });
    await setAutoCompanion(false);
    expect(prefs.autoCompanion).toBe(true);
  });
});
