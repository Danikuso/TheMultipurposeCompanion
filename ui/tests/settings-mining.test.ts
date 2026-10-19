import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Settings from '../src/settings/Settings.svelte';
import { setLocale } from '../src/lib/i18n';

let target: HTMLElement;
let inst: Record<string, unknown>;
const cfg = { mining_interest: ['platinum'], mining_threshold_cr: 1000000, sell_search_radius_ly: 100 };
const tick = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); flushSync(); };

beforeEach(() => { setLocale('es'); });
afterEach(() => { unmount(inst); target.remove(); vi.unstubAllGlobals(); });

async function render(putStatus: number) {
  const put = vi.fn(async (_u: string, _i: RequestInit) => ({ ok: putStatus === 200, status: putStatus, json: async () => ({ ...cfg, sell_search_radius_ly: 250 }) }));
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    if (url === '/api/config' && init?.method === 'PUT') return put(url, init);
    return { ok: true, status: 200, json: async () => cfg };
  }));
  target = document.createElement('div');
  document.body.appendChild(target);
  inst = mount(Settings, { target });
  await tick();
  return put;
}

test('settings put sends json with content type and reverts on 400', async () => {
  const put = await render(400);
  const radius = target.querySelector('[data-testid="sell-radius"]') as HTMLInputElement;
  expect(radius.value).toBe('100');
  radius.value = '250';
  radius.dispatchEvent(new Event('input', { bubbles: true }));
  radius.dispatchEvent(new Event('change', { bubbles: true }));
  await tick();
  expect(put).toHaveBeenCalledTimes(1);
  const init = (put.mock.calls[0] as unknown as [string, RequestInit])[1];
  expect((init.headers as Record<string, string>)['content-type']).toBe('application/json');
  expect(JSON.parse(init.body as string).sell_search_radius_ly).toBe(250);
  expect(radius.value).toBe('100');
});

test('out of range radius is not sent; mineral toggle and free text', async () => {
  const put = await render(200);
  const radius = target.querySelector('[data-testid="sell-radius"]') as HTMLInputElement;
  radius.value = '5';
  radius.dispatchEvent(new Event('input', { bubbles: true }));
  radius.dispatchEvent(new Event('change', { bubbles: true }));
  await tick();
  expect(put).not.toHaveBeenCalled();
  radius.value = '100';
  radius.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  const extra = target.querySelector('[data-testid="mining-extra"]') as HTMLInputElement;
  extra.value = 'Void Opal';
  extra.dispatchEvent(new Event('input', { bubbles: true }));
  (target.querySelector('[data-testid="mining-add"]') as HTMLButtonElement).click();
  await tick();
  const init = (put.mock.calls[0] as unknown as [string, RequestInit])[1];
  expect(JSON.parse(init.body as string).mining_interest).toEqual(['platinum', 'voidopal']);
});
