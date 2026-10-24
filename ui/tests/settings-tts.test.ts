import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Settings from '../src/settings/Settings.svelte';
import { setLocale } from '../src/lib/i18n';

let target: HTMLElement;
let inst: Record<string, unknown>;

const initialConfig = {
  tts: {
    enabled: false,
    engine: 'native',
    voice: '',
    volume: 80,
    rate: 100,
    channels: {
      combat: true,
      navigation: true,
      exploration: true,
      mining: true,
      carrier: true,
    },
  },
  server: { port: 47320, lan: false, token: 'synthetic' },
};

const settle = async () => {
  for (let i = 0; i < 30; i++) await Promise.resolve();
  flushSync();
};

const response = (body: unknown, status = 200) => ({
  ok: status === 200,
  status,
  json: async () => body,
});

beforeEach(() => {
  setLocale('es');
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  if (inst) unmount(inst);
  target.remove();
  vi.unstubAllGlobals();
});

async function render(
  config: unknown = initialConfig,
  put = vi.fn(async (_url: string, init: RequestInit) => response(JSON.parse(init.body as string))),
  voices = [
    { id: 'native-1', name: 'Helena', lang: 'es-ES' },
    { id: 'native-2', name: 'Zira', lang: 'en-US' },
  ],
) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init?: RequestInit) => {
      if (url === '/api/config' && init?.method === 'PUT') return put(url, init);
      if (url.startsWith('/api/tts/voices')) return response({ voices });
      if (url === '/api/tts/test' && init?.method === 'POST') return response({ ok: true });
      return response(config);
    }),
  );
  inst = mount(Settings, { target });
  await settle();
  return put;
}

test('renders tts settings with switch initially off', async () => {
  await render();
  const fieldset = target.querySelector('[data-testid="tts-settings"]');
  expect(fieldset).not.toBeNull();

  const enabledSwitch = target.querySelector('[data-testid="tts-enabled"]') as HTMLInputElement;
  expect(enabledSwitch).not.toBeNull();
  expect(enabledSwitch.checked).toBe(false);

  // Subcontrols should be hidden when disabled
  expect(target.querySelector('[data-testid="tts-engine"]')).toBeNull();
  expect(target.querySelector('[data-testid="tts-test-btn"]')).toBeNull();
});

test('enabling tts displays controls and persists via PUT', async () => {
  const put = await render();
  const enabledSwitch = target.querySelector('[data-testid="tts-enabled"]') as HTMLInputElement;

  enabledSwitch.checked = true;
  enabledSwitch.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();

  expect(put).toHaveBeenCalledTimes(1);
  const sent = JSON.parse(put.mock.calls[0][1].body as string);
  expect(sent.tts.enabled).toBe(true);

  // Controls are now visible
  const engineSelect = target.querySelector('[data-testid="tts-engine"]') as HTMLSelectElement;
  const volumeSlider = target.querySelector('[data-testid="tts-volume"]') as HTMLInputElement;
  const rateSlider = target.querySelector('[data-testid="tts-rate"]') as HTMLInputElement;
  const testBtn = target.querySelector('[data-testid="tts-test-btn"]') as HTMLButtonElement;

  expect(engineSelect).not.toBeNull();
  expect(volumeSlider).not.toBeNull();
  expect(rateSlider).not.toBeNull();
  expect(testBtn).not.toBeNull();
  expect(volumeSlider.value).toBe('80');
  expect(rateSlider.value).toBe('100');
});

test('switching engine reloads voices and saves config', async () => {
  const cfgWithTts = {
    ...initialConfig,
    tts: { ...initialConfig.tts, enabled: true },
  };
  const put = await render(cfgWithTts);

  const engineSelect = target.querySelector('[data-testid="tts-engine"]') as HTMLSelectElement;
  expect(engineSelect).not.toBeNull();
  expect(engineSelect.value).toBe('native');

  engineSelect.value = 'kokoro';
  engineSelect.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();

  expect(put).toHaveBeenCalledTimes(1);
  const sent = JSON.parse(put.mock.calls[0][1].body as string);
  expect(sent.tts.engine).toBe('kokoro');
  expect(sent.tts.voice).toBeUndefined();
});

test('toggling a channel saves channel state', async () => {
  const cfgWithTts = {
    ...initialConfig,
    tts: { ...initialConfig.tts, enabled: true },
  };
  const put = await render(cfgWithTts);

  const combatBox = target.querySelector('[data-testid="tts-channel-combat"]') as HTMLInputElement;
  expect(combatBox.checked).toBe(true);

  combatBox.checked = false;
  combatBox.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();

  expect(put).toHaveBeenCalledTimes(1);
  const sent = JSON.parse(put.mock.calls[0][1].body as string);
  expect(sent.tts.channels.combat).toBe(false);
  expect(sent.tts.channels.navigation).toBe(true);
});

test('clicking test audio calls /api/tts/test', async () => {
  const cfgWithTts = {
    ...initialConfig,
    tts: { ...initialConfig.tts, enabled: true, volume: 90, rate: 110 },
  };
  await render(cfgWithTts);

  const testBtn = target.querySelector('[data-testid="tts-test-btn"]') as HTMLButtonElement;
  expect(testBtn).not.toBeNull();

  testBtn.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await settle();

  const fetchCalls = (globalThis.fetch as any).mock.calls;
  const testCall = fetchCalls.find((call: any[]) => call[0] === '/api/tts/test');
  expect(testCall).toBeDefined();
  const testBody = JSON.parse(testCall[1].body);
  expect(testBody).toMatchObject({
    engine: 'native',
    volume: 90,
    rate: 110,
  });
});
