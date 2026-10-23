import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Settings from '../src/settings/Settings.svelte';
import { getLocale, setLocale } from '../src/lib/i18n';

let target: HTMLElement;
let inst: Record<string, unknown>;
const cfg = { engineering_rolls: [2,3,4,5,6], eddn_radius_ly: 200, tourism_reference_cr_ly: 100000, server: { port: 47320, lan: false, token: 'synthetic' }, extra_setting: true };
const settle = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); flushSync(); };
const response = (body: unknown, status = 200) => ({ ok: status === 200, status, json: async () => body });
const deferred = () => { let resolve!: (value: ReturnType<typeof response>) => void; const promise = new Promise<ReturnType<typeof response>>(r => { resolve = r; }); return { promise, resolve }; };
beforeEach(() => { setLocale('es'); target = document.createElement('div'); document.body.appendChild(target); });
afterEach(() => { if (inst) unmount(inst); target.remove(); vi.unstubAllGlobals(); });
const input = (grade: number) => target.querySelector(`[data-testid="engineering-rolls-${grade}"]`) as HTMLInputElement;
const edit = (grade: number, value: string, commit = true) => {
  const el = input(grade); el.value = value; el.dispatchEvent(new Event('input', { bubbles: true }));
  if (commit) el.dispatchEvent(new Event('change', { bubbles: true })); flushSync(); return el;
};
async function render(config: unknown = cfg, put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string)))) {
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    if (url === '/api/config' && init?.method === 'PUT') return put(url, init);
    return response(config);
  }));
  inst = mount(Settings, { target }); await settle(); return put;
}

test('loads saved rolls and PUT preserves unrelated configuration', async () => {
  const put = await render();
  expect([1,2,3,4,5].map(g => input(g).value)).toEqual(['2','3','4','5','6']);
  expect(input(1).labels?.[0].textContent).toContain('G1');
  edit(5, '10'); await settle();
  expect(put).toHaveBeenCalledTimes(1);
  expect(JSON.parse(put.mock.calls[0][1].body as string)).toMatchObject({ ...cfg, engineering_rolls: [2,3,4,5,10] });
  expect(input(5).value).toBe('10');
});

test('first roll edit back to the default is saved against the loaded configuration', async () => {
  const put = await render({ ...cfg, engineering_rolls: [1,1,1,1,6] });
  edit(5, '4'); await settle();
  expect(put).toHaveBeenCalledTimes(1);
  expect(JSON.parse(put.mock.calls[0][1].body as string).engineering_rolls).toEqual([1,1,1,1,4]);
  expect(input(5).value).toBe('4');
});

test('first unrelated edit back to the default is saved against the loaded configuration', async () => {
  const put = await render({ ...cfg, eddn_radius_ly: 350 });
  const radius = target.querySelector('[data-testid="eddn-radius"]') as HTMLInputElement;
  radius.value = '200'; radius.dispatchEvent(new Event('input', { bubbles: true })); radius.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();
  expect(put).toHaveBeenCalledTimes(1);
  expect(JSON.parse(put.mock.calls[0][1].body as string).eddn_radius_ly).toBe(200);
  expect(radius.value).toBe('200');
});

test.each([{}, { engineering_rolls: [1,2] }, { engineering_rolls: [1,1,1,1,11] }, { engineering_rolls: [1,1,1,1,null] }])('missing or invalid saved rolls use complete defaults: %j', async (config) => {
  await render(config); expect([1,2,3,4,5].map(g => input(g).value)).toEqual(['1','1','1','1','4']);
});

test.each(['0','11','1.5','', 'NaN'])('invalid roll %s blocks PUT, including incomplete arrays', async (value) => {
  const put = await render(); edit(5, value); await settle(); expect(put).not.toHaveBeenCalled();
  expect(target.querySelector('[data-testid="engineering-rolls-error"]')).not.toBeNull();
});

test.each(['1','10'])('roll limit %s is accepted', async (value) => {
  const put = await render(); edit(1, value); await settle(); expect(put).toHaveBeenCalledTimes(1);
});

test('400 rolls back to the last confirmed rolls', async () => {
  const put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string)));
  await render(cfg, put); edit(5, '7'); await settle();
  put.mockImplementationOnce(async () => response({}, 400)); edit(5, '8'); await settle();
  expect(input(5).value).toBe('7'); expect(target.textContent).toContain('No se han podido guardar');
});

test.each([200,400])('late PUT %s preserves a newer unsubmitted roll edit and array snapshot', async (status) => {
  const pending = deferred(); const put = vi.fn((_url: string, _init: RequestInit) => pending.promise);
  await render(cfg, put); edit(1, '7'); await settle(); edit(2, '8', false);
  pending.resolve(response({ ...cfg, engineering_rolls: [7,3,4,5,6] }, status)); await settle();
  expect(input(2).value).toBe('8');
  expect(input(1).value).toBe(status === 200 ? '7' : '2');
  expect(JSON.parse(put.mock.calls[0][1].body as string).engineering_rolls).toEqual([7,3,4,5,6]);
});

test.each([200,400])('theme PUT %s applies confirmed appearance while retaining a later numeric draft', async (status) => {
  const pending = deferred();
  await render({ ...cfg, language: 'es', theme_mode: 'hud' }, vi.fn(() => pending.promise));
  const theme = target.querySelectorAll('select')[1];
  expect(document.documentElement.style.getPropertyValue('--hud-primary')).toBe('#ff7100');
  theme.value = 'contrast'; theme.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  edit(2, '8', false);
  pending.resolve(response({ ...cfg, language: 'es', theme_mode: 'contrast' }, status)); await settle();
  expect(theme.value).toBe(status === 200 ? 'contrast' : 'hud');
  expect(document.documentElement.style.getPropertyValue('--hud-primary')).toBe(status === 200 ? '#FFFFFF' : '#ff7100');
  expect(input(2).value).toBe('8');
});

test.each([200,400])('language PUT %s applies confirmed locale while retaining a later numeric draft', async (status) => {
  const pending = deferred();
  await render({ ...cfg, language: 'es', theme_mode: 'hud' }, vi.fn(() => pending.promise));
  const language = target.querySelector('select')!;
  language.value = 'en'; language.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  edit(2, '8', false);
  pending.resolve(response({ ...cfg, language: 'en', theme_mode: 'hud' }, status)); await settle();
  expect(language.value).toBe(status === 200 ? 'en' : 'es');
  expect(getLocale()).toBe(status === 200 ? 'en' : 'es');
  expect(input(2).value).toBe('8');
});

test('an older queued response does not apply preferences before the latest revision confirms', async () => {
  const first = deferred(); const second = deferred();
  const put = vi.fn(() => first.promise).mockImplementationOnce(() => first.promise).mockImplementationOnce(() => second.promise);
  await render({ ...cfg, language: 'es', theme_mode: 'hud' }, put);
  const theme = target.querySelectorAll('select')[1]; const language = target.querySelector('select')!;
  theme.value = 'contrast'; theme.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  language.value = 'en'; language.dispatchEvent(new Event('change', { bubbles: true })); await settle(); edit(2, '8', false);
  first.resolve(response({ ...cfg, language: 'es', theme_mode: 'contrast' })); await settle();
  expect(document.documentElement.style.getPropertyValue('--hud-primary')).toBe('#ff7100'); expect(getLocale()).toBe('es');
  second.resolve(response({ ...cfg, language: 'en', theme_mode: 'contrast' })); await settle();
  expect(document.documentElement.style.getPropertyValue('--hud-primary')).toBe('#FFFFFF'); expect(getLocale()).toBe('en');
  expect(input(2).value).toBe('8');
});

test('late GET preserves edits made before configuration loads', async () => {
  const pending = deferred(); vi.stubGlobal('fetch', vi.fn(() => pending.promise));
  inst = mount(Settings, { target }); flushSync(); edit(5, '9', false);
  pending.resolve(response(cfg)); await settle(); expect(input(5).value).toBe('9');
});

test('an unsubmitted edit made before GET remains saveable after GET', async () => {
  const pending = deferred();
  const put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string)));
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => init?.method === 'PUT' ? put(url, init) : pending.promise));
  inst = mount(Settings, { target }); flushSync(); edit(5, '9', false);
  pending.resolve(response(cfg)); await settle();
  input(5).dispatchEvent(new Event('change', { bubbles: true })); await settle();
  expect(JSON.parse(put.mock.calls[0][1].body as string).engineering_rolls).toEqual([1,1,1,1,9]);
  expect(input(5).value).toBe('9');
});

test('saving before GET waits for confirmed config and preserves unrelated fields', async () => {
  const pending = deferred();
  const put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string)));
  vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
    if (init?.method === 'PUT') return put(url, init);
    return pending.promise;
  }));
  inst = mount(Settings, { target }); flushSync(); edit(5, '9'); await settle();
  expect(put).not.toHaveBeenCalled();
  pending.resolve(response({ ...cfg, eddn_radius_ly: 350 })); await settle();
  expect(put).toHaveBeenCalledTimes(1);
  expect(JSON.parse(put.mock.calls[0][1].body as string)).toMatchObject({ eddn_radius_ly: 350, engineering_rolls: [1,1,1,1,9], extra_setting: true });
});

test('failed rolls are not resubmitted by a queued unrelated settings change', async () => {
  const pending = deferred();
  const put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string)));
  put.mockImplementationOnce(() => pending.promise);
  await render(cfg, put); edit(5, '7'); await settle();
  const radius = target.querySelector('[data-testid="eddn-radius"]') as HTMLInputElement;
  radius.value = '300'; radius.dispatchEvent(new Event('input', { bubbles: true })); radius.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  pending.resolve(response({}, 400)); await settle();
  expect(put).toHaveBeenCalledTimes(2);
  expect(JSON.parse(put.mock.calls[1][1].body as string)).toMatchObject({ engineering_rolls: cfg.engineering_rolls, eddn_radius_ly: 300 });
  expect(input(5).value).toBe('6'); expect(radius.value).toBe('300');
});

test('queued edits keep roll snapshots separate and rollback to preceding successful PUT', async () => {
  const pending = deferred(); const put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string)));
  put.mockImplementationOnce(() => pending.promise); put.mockImplementationOnce(async () => response({}, 400));
  await render(cfg, put); edit(1, '7'); await settle(); edit(2, '8'); await settle();
  expect(put).toHaveBeenCalledTimes(1); pending.resolve(response({ ...cfg, engineering_rolls: [7,3,4,5,6] })); await settle();
  expect(put).toHaveBeenCalledTimes(2); expect(JSON.parse(put.mock.calls[1][1].body as string).engineering_rolls).toEqual([7,8,4,5,6]);
  expect([1,2].map(g => input(g).value)).toEqual(['7','3']);
});
