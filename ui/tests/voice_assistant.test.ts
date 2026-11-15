import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import TopBar from '../src/shell/TopBar.svelte';
import Settings from '../src/settings/Settings.svelte';
import { voiceAssistant } from '../src/lib/voiceAssistant.svelte';
import {
  isWhisperHallucination,
  preprocessAudioSamples,
  whisperEngine,
} from '../src/lib/whisperEngine';
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
  voiceAssistant.setSttEngine('whisper');
  voiceAssistant.setPttKey('F4');
  voiceAssistant.lastQuery = null;
  voiceAssistant.lastResponse = null;
  voiceAssistant.history = [];
  voiceAssistant.error = null;
  voiceAssistant.voiceAttackStatus = null;
  whisperEngine.unloadModel();
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
  expect(micBtn.textContent).toContain('Voz des.');

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

test('voiceAssistant switches STT engine and persists selection', () => {
  expect(voiceAssistant.sttEngine).toBe('whisper');

  voiceAssistant.setSttEngine('native');
  expect(voiceAssistant.sttEngine).toBe('native');
  expect(localStorage.getItem('tmc.voice.stt_engine')).toBe('native');

  voiceAssistant.setSttEngine('whisper');
  expect(voiceAssistant.sttEngine).toBe('whisper');
  expect(localStorage.getItem('tmc.voice.stt_engine')).toBe('whisper');
});

test('Settings renders STT engine selector and Whisper info when enabled', async () => {
  voiceAssistant.setEnabled(true);
  inst = mount(Settings, { target });
  await settle();

  const engineSelect = target.querySelector('[data-testid="voice-engine-select"]') as HTMLSelectElement;
  expect(engineSelect).not.toBeNull();
  expect(engineSelect.value).toBe('whisper');

  const whisperBadge = target.querySelector('[data-testid="whisper-status-badge"]');
  expect(whisperBadge).not.toBeNull();

  const preloadBtn = target.querySelector('[data-testid="preload-whisper-btn"]') as HTMLButtonElement;
  expect(preloadBtn).not.toBeNull();
});

test('whisperEngine trims silence, normalizes gain, filters hallucinations and unloads', () => {
  expect(isWhisperHallucination('')).toBe(true);
  expect(isWhisperHallucination('[BLANK_AUDIO]')).toBe(true);
  expect(isWhisperHallucination('[Música]')).toBe(true);
  expect(isWhisperHallucination('Gracias por ver')).toBe(true);
  expect(isWhisperHallucination('¿Dónde estoy?')).toBe(false);

  const silenceOnly = new Float32Array(1000);
  expect(preprocessAudioSamples(silenceOnly).length).toBe(0);

  // Leading/trailing silence around a quiet 0.1 peak speech burst -> normalized to ~0.85
  const samples = new Float32Array(3000);
  for (let i = 1000; i < 1500; i += 1) {
    samples[i] = 0.1;
  }
  const processed = preprocessAudioSamples(samples);
  expect(processed.length).toBeLessThan(samples.length);
  expect(processed.length).toBeGreaterThanOrEqual(500);
  const maxPeak = Math.max(...Array.from(processed, (x) => Math.abs(x)));
  expect(maxPeak).toBeGreaterThan(0.8);

  whisperEngine.status = 'ready';
  whisperEngine.loadProgress = 100;
  whisperEngine.unloadModel();
  expect(whisperEngine.status).toBe('unloaded');
  expect(whisperEngine.loadProgress).toBe(0);
});

test('TopBar stops speaking when clicked in speaking state', async () => {
  const fetchMock = vi.fn(async (_url: string, _opts?: RequestInit) =>
    response({ ok: true, stopped: true }),
  );
  vi.stubGlobal('fetch', fetchMock);

  voiceAssistant.setEnabled(true);
  voiceAssistant.state = 'speaking';
  inst = mount(TopBar, { target });
  await settle();

  const micBtn = target.querySelector('[data-testid="topbar-voice-mic"]') as HTMLButtonElement;
  expect(micBtn.classList.contains('speaking')).toBe(true);

  micBtn.click();
  await settle();

  expect(voiceAssistant.state).toBe('inactive');
  const stopCall = fetchMock.mock.calls.find((call) => call[0] === '/api/tts/stop');
  expect(stopCall).toBeDefined();
  expect(stopCall?.[1]).toMatchObject({ method: 'POST' });
});

test('Settings renders PTT key selector and interactive copilot console', async () => {
  const fetchMock = vi.fn(async (url: string, opts?: RequestInit) => {
    if (url === '/api/config') {
      return response({
        tts: { enabled: false },
        voice: { enabled: true },
        accounts: {},
      });
    }
    if (url === '/api/tts/voices' || url.startsWith('/api/tts/voices?')) {
      return response({ voices: [] });
    }
    if (url === '/api/voice/voiceattack') {
      return response({ installed: false, running: false, profile_available: true });
    }
    if (url === '/api/voice/intent' && opts?.body) {
      const parsed = JSON.parse(opts.body as string);
      if (parsed.query.includes('portanaves')) {
        return response({
          intent: 'carrier_status',
          action_executed: 'voice.carrier_status',
          spoken_text: 'Portanaves HMS Beagle en Sol con 450 t de tritio.',
          clipboard_copied: 'Sol',
        });
      }
      return response({
        intent: 'current_location',
        action_executed: 'voice.copy_current_system',
        spoken_text: 'Sistema actual Sol.',
        clipboard_copied: 'Sol',
      });
    }
    return response({});
  });
  vi.stubGlobal('fetch', fetchMock);

  voiceAssistant.setEnabled(true);
  voiceAssistant.setMode('ptt');

  inst = mount(Settings, { target });
  await settle();

  // Selector de tecla PTT visible y persistente
  const pttSelect = target.querySelector('[data-testid="voice-ptt-key-select"]') as HTMLSelectElement;
  expect(pttSelect).not.toBeNull();
  pttSelect.value = 'F8';
  pttSelect.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();
  expect(voiceAssistant.pttKey).toBe('F8');
  expect(localStorage.getItem('tmc.voice.ptt_key')).toBe('F8');

  // Consola manual + tarjeta de respuesta e insignia de portapapeles
  const queryInput = target.querySelector('[data-testid="voice-query-input"]') as HTMLInputElement;
  const submitBtn = target.querySelector('[data-testid="voice-query-submit"]') as HTMLButtonElement;
  expect(queryInput).not.toBeNull();
  expect(submitBtn.disabled).toBe(true);

  queryInput.value = '¿Dónde estoy?';
  queryInput.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
  expect(submitBtn.disabled).toBe(false);

  // Pulsar F8 mientras el foco está en un INPUT no debe activar PTT
  const startSpy = vi.spyOn(voiceAssistant, 'startListening').mockResolvedValue();
  queryInput.dispatchEvent(new KeyboardEvent('keydown', { key: 'F8', code: 'F8', bubbles: true }));
  expect(startSpy).not.toHaveBeenCalled();

  // Pulsar F8 fuera de campos editables sí activa PTT
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'F8', code: 'F8', bubbles: true }));
  expect(startSpy).toHaveBeenCalledTimes(1);
  startSpy.mockRestore();

  submitBtn.closest('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await settle();

  const responseCard = target.querySelector('[data-testid="voice-response-card"]');
  expect(responseCard?.textContent).toContain('Sistema actual Sol.');
  const clipboardBadge = target.querySelector('[data-testid="voice-clipboard-badge"]');
  expect(clipboardBadge?.textContent).toContain('Sol');

  // Píldora de acceso rápido para portanaves
  const carrierPill = target.querySelector('[data-testid="voice-quick-pill-carrier_status"]') as HTMLButtonElement;
  expect(carrierPill).not.toBeNull();
  carrierPill.click();
  await settle();

  const updatedCard = target.querySelector('[data-testid="voice-response-card"]');
  expect(updatedCard?.textContent).toContain('HMS Beagle');
});
