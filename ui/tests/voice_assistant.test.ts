import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import TopBar from '../src/shell/TopBar.svelte';
import Settings from '../src/settings/Settings.svelte';
import { voiceAssistant } from '../src/lib/voiceAssistant.svelte';
import { setLocale } from '../src/lib/i18n';

let target: HTMLElement;
let inst: Record<string, unknown> | null = null;

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
  localStorage.clear();
  voiceAssistant.setEnabled(false);
  voiceAssistant.setMode('continuous');
  voiceAssistant.lastQuery = null;
  voiceAssistant.lastResponse = null;
  voiceAssistant.voiceAttackStatus = null;
});

afterEach(() => {
  if (inst) {
    unmount(inst);
    inst = null;
  }
  target.remove();
  vi.unstubAllGlobals();
});

test('voiceAssistant service toggles enabled and persists in localStorage', () => {
  expect(voiceAssistant.enabled).toBe(false);
  expect(voiceAssistant.state).toBe('inactive');

  voiceAssistant.toggleEnabled();
  expect(voiceAssistant.enabled).toBe(true);
  expect(localStorage.getItem('tmc.voice.enabled')).toBe('true');

  voiceAssistant.setMode('ptt');
  expect(voiceAssistant.mode).toBe('ptt');
  expect(localStorage.getItem('tmc.voice.mode')).toBe('ptt');

  voiceAssistant.toggleEnabled();
  expect(voiceAssistant.enabled).toBe(false);
  expect(localStorage.getItem('tmc.voice.enabled')).toBe('false');
});

test('voiceAssistant sendQuery dispatches POST /api/voice/intent and updates state', async () => {
  const fakeResponse = {
    intent: 'ship_status',
    spoken_text: 'Casco al 100%, combustible al 80%.',
    clipboard_copied: null,
    action_executed: null,
    data: {},
  };

  const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
    if (url === '/api/voice/intent' && init?.method === 'POST') {
      const body = JSON.parse(init.body as string);
      expect(body.query).toBe('¿Estado de la nave?');
      expect(body.locale).toBe('es');
      return response(fakeResponse);
    }
    return response({});
  });

  vi.stubGlobal('fetch', fetchMock);

  voiceAssistant.setEnabled(true);
  const result = await voiceAssistant.sendQuery('¿Estado de la nave?');

  expect(fetchMock).toHaveBeenCalledTimes(1);
  expect(result).toEqual(fakeResponse);
  expect(voiceAssistant.lastQuery).toBe('¿Estado de la nave?');
  expect(voiceAssistant.lastResponse).toEqual(fakeResponse);
  expect(voiceAssistant.state).toBe('speaking');
});

test('voiceAssistant fetchVoiceAttackStatus populates status from REST endpoint', async () => {
  const fakeVaStatus = {
    installed: true,
    running: false,
    profile_available: true,
    install_path: 'C:\\Program Files\\VoiceAttack',
  };

  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => {
      if (url === '/api/voice/voiceattack') {
        return response(fakeVaStatus);
      }
      return response({});
    }),
  );

  const status = await voiceAssistant.fetchVoiceAttackStatus();
  expect(status).toEqual(fakeVaStatus);
  expect(voiceAssistant.voiceAttackStatus).toEqual(fakeVaStatus);
});

test('TopBar renders microphone pill and responds to clicks and states', async () => {
  inst = mount(TopBar, { target });
  await settle();

  const micBtn = target.querySelector('[data-testid="topbar-voice-mic"]') as HTMLButtonElement;
  expect(micBtn).not.toBeNull();
  expect(micBtn.classList.contains('active')).toBe(false);
  expect(micBtn.textContent).toContain('VOZ OFF');

  // Clic en la píldora activa la asistencia de voz
  micBtn.click();
  await settle();

  expect(voiceAssistant.enabled).toBe(true);
  expect(micBtn.classList.contains('active')).toBe(true);

  // Simular estado escuchando
  voiceAssistant.state = 'listening';
  await settle();
  expect(micBtn.classList.contains('listening')).toBe(true);

  // Simular estado procesando
  voiceAssistant.state = 'processing';
  await settle();
  expect(micBtn.classList.contains('processing')).toBe(true);

  // Simular estado hablando
  voiceAssistant.state = 'speaking';
  await settle();
  expect(micBtn.classList.contains('speaking')).toBe(true);
});

test('Settings renders voice assistant section and voiceattack profile download', async () => {
  const fakeVaStatus = {
    installed: true,
    running: true,
    profile_available: true,
    install_path: 'C:\\Program Files\\VoiceAttack',
  };

  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => {
      if (url === '/api/voice/voiceattack') return response(fakeVaStatus);
      return response({});
    }),
  );

  inst = mount(Settings, { target });
  await settle();

  const toggle = target.querySelector('[data-testid="voice-assistant-toggle"]') as HTMLInputElement;
  expect(toggle).not.toBeNull();
  expect(toggle.checked).toBe(false);

  // Activar asistencia
  toggle.click();
  await settle();

  expect(voiceAssistant.enabled).toBe(true);

  const modeSelect = target.querySelector('[data-testid="voice-mode-select"]') as HTMLSelectElement;
  expect(modeSelect).not.toBeNull();
  expect(modeSelect.value).toBe('continuous');

  // Monitor VoiceAttack
  const vaBadge = target.querySelector('[data-testid="voiceattack-badge"]');
  expect(vaBadge).not.toBeNull();

  const downloadBtn = target.querySelector('[data-testid="download-vap-btn"]') as HTMLAnchorElement;
  expect(downloadBtn).not.toBeNull();
  expect(downloadBtn.getAttribute('href')).toBe('/api/voice/voiceattack/profile');
  expect(downloadBtn.getAttribute('download')).toBe('TMC-Copiloto.vap');
});
