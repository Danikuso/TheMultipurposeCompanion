import { diag } from './diag.svelte';
import { DEFAULT_PRIORITIES } from '../shell/priorities';

export type OverlayMode = 'minimal' | 'tactical';
export type OverlayAnchor = 'top_left' | 'top_center' | 'top_right' | 'bottom_left' | 'bottom_right' | 'custom';

export interface OverlayBlocks {
  navigation: boolean;
  exploration: boolean;
  exobiology: boolean;
  combat: boolean;
  mining: boolean;
  trade: boolean;
  colonisation: boolean;
}

export interface OverlaySettings {
  enabled: boolean;
  mode: OverlayMode;
  opacity: number;
  click_through: boolean;
  anchor: OverlayAnchor;
  custom_x?: number | null;
  custom_y?: number | null;
  pinned_role?: string | null;
  blocks: OverlayBlocks;
}

export function defaultOverlaySettings(): OverlaySettings {
  return {
    enabled: false,
    mode: 'tactical',
    opacity: 78,
    click_through: true,
    anchor: 'top_right',
    custom_x: null,
    custom_y: null,
    pinned_role: null,
    blocks: {
      navigation: true,
      exploration: true,
      exobiology: true,
      combat: true,
      mining: true,
      trade: true,
      colonisation: true,
    },
  };
}

const VALID_MODES = new Set<OverlayMode>(['minimal', 'tactical']);
const VALID_ANCHORS = new Set<OverlayAnchor>(['top_left', 'top_center', 'top_right', 'bottom_left', 'bottom_right', 'custom']);

export function normalizeOverlay(raw: unknown): OverlaySettings {
  const def = defaultOverlaySettings();
  if (!raw || typeof raw !== 'object') return def;
  const r = raw as Record<string, unknown>;
  const rawBlocks = (r.blocks && typeof r.blocks === 'object' ? r.blocks : {}) as Record<string, unknown>;
  return {
    enabled: typeof r.enabled === 'boolean' ? r.enabled : def.enabled,
    mode: typeof r.mode === 'string' && VALID_MODES.has(r.mode as OverlayMode) ? (r.mode as OverlayMode) : def.mode,
    opacity: typeof r.opacity === 'number' && Number.isFinite(r.opacity) ? Math.min(95, Math.max(35, Math.round(r.opacity))) : def.opacity,
    click_through: typeof r.click_through === 'boolean' ? r.click_through : def.click_through,
    anchor: typeof r.anchor === 'string' && VALID_ANCHORS.has(r.anchor as OverlayAnchor) ? (r.anchor as OverlayAnchor) : def.anchor,
    custom_x: typeof r.custom_x === 'number' && Number.isFinite(r.custom_x) ? Math.round(r.custom_x) : null,
    custom_y: typeof r.custom_y === 'number' && Number.isFinite(r.custom_y) ? Math.round(r.custom_y) : null,
    pinned_role: typeof r.pinned_role === 'string' && r.pinned_role.trim() ? r.pinned_role.trim() : null,
    blocks: {
      navigation: typeof rawBlocks.navigation === 'boolean' ? rawBlocks.navigation : true,
      exploration: typeof rawBlocks.exploration === 'boolean' ? rawBlocks.exploration : true,
      exobiology: typeof rawBlocks.exobiology === 'boolean' ? rawBlocks.exobiology : true,
      combat: typeof rawBlocks.combat === 'boolean' ? rawBlocks.combat : true,
      mining: typeof rawBlocks.mining === 'boolean' ? rawBlocks.mining : true,
      trade: typeof rawBlocks.trade === 'boolean' ? rawBlocks.trade : true,
      colonisation: typeof rawBlocks.colonisation === 'boolean' ? rawBlocks.colonisation : true,
    },
  };
}

/** Preferencias de la UI persistidas en `config.toml` (vía `/api/config`). `loaded` es false hasta conocer el valor guardado. */
export const prefs = $state({
  autoCompanion: true,
  autoCompanionPriorities: [...DEFAULT_PRIORITIES],
  autoCompanionLingerSeconds: 20,
  autoCompanionVoiceCues: false,
  autoCompanionSoundCues: true,
  autoCompanionPredictive: true,
  homeSystem: null as string | null,
  homeSystemCoords: null as [number, number, number] | null,
  overlay: defaultOverlaySettings(),
  locked: false,
  loaded: false,
});

let confirmed = true;
let confirmedOverlay: OverlaySettings = defaultOverlaySettings();
let overlayPending = 0;
let pending = 0;
let queue: Promise<void> = Promise.resolve();

export function applyOverlayTopic(raw: unknown): void {
  const next = normalizeOverlay(raw);
  confirmedOverlay = next;
  if (overlayPending === 0) {
    prefs.overlay = next;
  }
}

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
  home_system?: unknown;
  home_system_coords?: unknown;
  overlay?: unknown;
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
  if (typeof cfg.home_system === 'string') {
    prefs.homeSystem = cfg.home_system.trim() || null;
  } else if (cfg.home_system === null) {
    prefs.homeSystem = null;
  }
  if (Array.isArray(cfg.home_system_coords) && cfg.home_system_coords.length === 3) {
    prefs.homeSystemCoords = [
      Number(cfg.home_system_coords[0]),
      Number(cfg.home_system_coords[1]),
      Number(cfg.home_system_coords[2]),
    ];
  } else if (cfg.home_system_coords === null) {
    prefs.homeSystemCoords = null;
  }
  if (cfg.overlay !== undefined) {
    applyOverlayTopic(cfg.overlay);
  }
  prefs.loaded = true;
}

/** Si no se pudo leer la configuración, se usa el valor por defecto (activado). */
export function prefsFallback(): void {
  if (!prefs.loaded) loadPrefs({});
}

const STORE_KEY = 'tmc.ui.placement';

export interface Placement { layout: string | null; context: string | null; view: string | null }

/** Solo se restaura una pestaña propia, con un id de forma conocida (nunca rutas arbitrarias). */
const CUSTOM_VIEW = /^custom:c-[0-9a-f-]{1,40}$/;

/** Última disposición/contexto elegidos en este cliente (opcional: sin localStorage no pasa nada). */
export function readPlacement(): Placement {
  try {
    const raw = JSON.parse(localStorage.getItem(STORE_KEY) ?? 'null') as Partial<Placement> | null;
    const ok = (v: unknown) => (typeof v === 'string' && v.length < 40 ? v : null);
    const view = typeof raw?.view === 'string' && CUSTOM_VIEW.test(raw.view) ? raw.view : null;
    return { layout: ok(raw?.layout), context: ok(raw?.context), view };
  } catch {
    return { layout: null, context: null, view: null };
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

export function setHomeBase(
  system: string | null,
  coords: [number, number, number] | null = null,
): Promise<void> {
  const normSystem = system?.trim() || null;
  prefs.homeSystem = normSystem;
  prefs.homeSystemCoords = normSystem ? coords : null;

  queue = queue.then(async () => {
    try {
      const got = await fetch('/api/config');
      if (!got.ok) return;
      const cfg = (await got.json()) as Record<string, unknown>;
      await fetch('/api/config', {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          ...cfg,
          home_system: normSystem,
          home_system_coords: normSystem ? coords : null,
        }),
      });
    } catch {
      // Ignorar fallo de red puntual
    }
  });
  return queue;
}

export interface OverlayPatch {
  enabled?: boolean;
  mode?: OverlayMode;
  opacity?: number;
  click_through?: boolean;
  anchor?: OverlayAnchor;
  custom_x?: number | null;
  custom_y?: number | null;
  pinned_role?: string | null;
  blocks?: Partial<OverlayBlocks>;
}

function sendOverlayAction(actionName: string, payload: Record<string, unknown>, optimistic: OverlaySettings): Promise<void> {
  prefs.overlay = optimistic;
  overlayPending++;
  queue = queue.then(async () => {
    try {
      const res = await fetch(`/api/actions/${actionName}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error('overlay-action-failed');
      const saved = normalizeOverlay(await res.json());
      confirmedOverlay = saved;
      if (overlayPending === 1) prefs.overlay = saved;
    } catch {
      if (overlayPending === 1) prefs.overlay = confirmedOverlay;
    } finally {
      overlayPending--;
    }
  });
  return queue;
}

export function updateOverlaySettings(patch: OverlayPatch): Promise<void> {
  const optimistic = normalizeOverlay({
    ...prefs.overlay,
    ...patch,
    blocks: { ...prefs.overlay.blocks, ...(patch.blocks ?? {}) },
  });
  return sendOverlayAction('overlay.update', patch as Record<string, unknown>, optimistic);
}

export function toggleOverlay(enabled?: boolean): Promise<void> {
  const next = typeof enabled === 'boolean' ? enabled : !prefs.overlay.enabled;
  const optimistic = { ...prefs.overlay, enabled: next };
  return sendOverlayAction('overlay.toggle', { enabled: next }, optimistic);
}

export function toggleOverlayLock(extra?: {
  click_through?: boolean;
  anchor?: OverlayAnchor;
  custom_x?: number | null;
  custom_y?: number | null;
}): Promise<void> {
  const next = typeof extra?.click_through === 'boolean' ? extra.click_through : !prefs.overlay.click_through;
  const payload: Record<string, unknown> = { click_through: next };
  if (extra?.anchor !== undefined) payload.anchor = extra.anchor;
  if (extra?.custom_x !== undefined) payload.custom_x = extra.custom_x;
  if (extra?.custom_y !== undefined) payload.custom_y = extra.custom_y;
  const optimistic = normalizeOverlay({ ...prefs.overlay, ...payload });
  return sendOverlayAction('overlay.set_click_through', payload, optimistic);
}
