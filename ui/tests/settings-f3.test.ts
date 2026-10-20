import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Settings from '../src/settings/Settings.svelte';
import { setLocale } from '../src/lib/i18n';

let target: HTMLElement;
let inst: Record<string, unknown>;
const cfg = { eddn_radius_ly: 200, tourism_reference_cr_ly: 100000 };
const tick = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); flushSync(); };

beforeEach(() => { setLocale('es'); });
afterEach(() => { unmount(inst); target.remove(); vi.unstubAllGlobals(); });

async function render(putStatus: number) {
  const put = vi.fn(async (_u: string, _i: RequestInit) => ({ ok: putStatus === 200, status: putStatus, json: async () => ({ ...cfg, eddn_radius_ly: 300 }) }));
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
const change = (testid: string, value: string) => {
  const el = target.querySelector(`[data-testid="${testid}"]`) as HTMLInputElement;
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  return el;
};

test('settings put eddn radius and reverts on 400', async () => {
  const put = await render(400);
  const el = change('eddn-radius', '250');
  await tick();
  expect(put).toHaveBeenCalledTimes(1);
  const body = JSON.parse((put.mock.calls[0][1] as RequestInit).body as string);
  expect(body.eddn_radius_ly).toBe(250);
  expect(body.tourism_reference_cr_ly).toBe(100000);
  expect(el.value).toBe('200');
});

test('settings put tourism reference on success', async () => {
  const put = await render(200);
  change('tourism-reference', '50000');
  await tick();
  expect(JSON.parse((put.mock.calls[0][1] as RequestInit).body as string).tourism_reference_cr_ly).toBe(50000);
});

test('out of range f3 values are rejected client side', async () => {
  const put = await render(200);
  change('eddn-radius', '600');
  await tick();
  expect(put).not.toHaveBeenCalled();
  expect(target.querySelector('[data-testid="validation-error"]')).not.toBeNull();
  change('tourism-reference', '999');
  await tick();
  expect(put).not.toHaveBeenCalled();
});
