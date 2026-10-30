import { diag } from './diag.svelte';
import { DEFAULT_PRIORITIES } from '../shell/layouts';

/** Preferencias de la UI persistidas en `config.toml` (vía `/api/config`). `loaded` es false hasta conocer el valor guardado. */
export const prefs = $state({
  autoCompanion: true,
  autoCompanionPriorities: [...DEFAULT_PRIORITIES],
  autoCompanionLingerSeconds: 20,
  autoCompanionVoiceCues: false,
  autoCompanionSoundCues: true,
  autoCompanionPredictive: true,
  locked: false,
  loaded: false,
});

let confirmed = true;
let pending = 0;
let queue: Promise<void> = Promise.resolve();

export function toggleTacticalLock(): boolean {
  prefs.locked = !prefs.locked;
  return prefs.locked;
}

export function loadPrefs(cfg: {
  auto_companion?: unknown;
  auto_companion_priorities?: unknown;
  auto_companion_linger_seconds?: unknown;
  auto_companion_voice_cues?: unknown;
  auto_companion_sound_cues?: unknown;
  auto_companion_predictive?: unknown;
}): void {
  confirmed = typeof cfg.auto_companion === 'boolean' ? cfg.auto_companion : true;
  prefs.autoCompanion = confirmed;
  if (Array.isArray(cfg.auto_companion_priorities)) {
    prefs.autoCompanionPriorities = cfg.auto_companion_priorities.filter(
      (x): x is string => typeof x === 'string',
    );
  }
  if (typeof cfg.auto_companion_linger_seconds === 'number') {
    prefs.autoCompanionLingerSeconds = cfg.auto_companion_linger_seconds;
  }
  if (typeof cfg.auto_companion_voice_cues === 'boolean') {
    prefs.autoCompanionVoiceCues = cfg.auto_companion_voice_cues;
  }
  if (typeof cfg.auto_companion_sound_cues === 'boolean') {
    prefs.autoCompanionSoundCues = cfg.auto_companion_sound_cues;
  }
  if (typeof cfg.auto_companion_predictive === 'boolean') {
    prefs.autoCompanionPredictive = cfg.auto_companion_predictive;
  }
  prefs.loaded = true;
}

/** Si no se pudo leer la configuración, se usa el valor por defecto (activado). */
export function prefsFallback(): void {
  if (!prefs.loaded) loadPrefs({});
}

const STORE_KEY = 'tmc.ui.placement';

export interface Placement { layout: string | null; context: string | null }

/** Última disposición/contexto elegidos en este cliente (opcional: sin localStorage no pasa nada). */
export function readPlacement(): Placement {
  try {
    const raw = JSON.parse(localStorage.getItem(STORE_KEY) ?? 'null') as Partial<Placement> | null;
    const ok = (v: unknown) => (typeof v === 'string' && v.length < 40 ? v : null);
    return { layout: ok(raw?.layout), context: ok(raw?.context) };
  } catch {
    return { layout: null, context: null };
  }
}

export function savePlacement(p: Placement): void {
  try { localStorage.setItem(STORE_KEY, JSON.stringify(p)); } catch { /* sin almacenamiento */ }
}

/** Cambia el interruptor: optimista, PUTs serializados; si falla vuelve al último valor confirmado. */
export function setAutoCompanion(value: boolean): Promise<void> {
  prefs.autoCompanion = value;
  pending++;
  queue = queue.then(async () => {
    try {
      const got = await fetch('/api/config');
      if (!got.ok) throw new Error('config-get-failed');
      const cfg = (await got.json()) as Record<string, unknown>;
      const res = await fetch('/api/config', {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ ...cfg, auto_companion: value }),
      });
      diag('ui.fetch.result', { kind: 'config-put', ok: res.ok, status: res.status });
      if (!res.ok) throw new Error('config-save-failed');
      const saved = (await res.json()) as { auto_companion?: unknown };
      confirmed = typeof saved.auto_companion === 'boolean' ? saved.auto_companion : true;
      if (pending === 1) prefs.autoCompanion = confirmed;
    } catch {
      if (pending === 1) prefs.autoCompanion = confirmed;
    } finally {
      pending--;
    }
  });
  return queue;
}
