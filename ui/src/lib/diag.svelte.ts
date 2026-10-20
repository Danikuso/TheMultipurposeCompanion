// Diagnóstico de depuración de la UI. Solo actúa si el backend anuncia la capacidad
// `diagnostics` en `hello` (únicamente las compilaciones debug lo hacen). Sin capacidad:
// sin cola, sin temporizadores, sin listeners y sin peticiones.
//
// El envío usa `fetch` directo y nunca llama a `diag()`: no hay recursión.

export type DiagCode =
  | 'ui.diagnostics.enabled'
  | 'ui.queue.dropped'
  | 'ui.ws.hello'
  | 'ui.ws.snapshot'
  | 'ui.ws.update'
  | 'ui.ws.gap'
  | 'ui.ws.alert'
  | 'ui.ws.import_progress'
  | 'ui.ws.connection'
  | 'ui.ws.fault'
  | 'ui.ws.subscribe'
  | 'ui.topic.applied'
  | 'ui.action.sent'
  | 'ui.action.result'
  | 'ui.app.layout'
  | 'ui.panel.mounted'
  | 'ui.panel.unmounted'
  | 'ui.list.render'
  | 'ui.card.system'
  | 'ui.card.route_strip'
  | 'ui.topbar.state'
  | 'ui.toast.shown'
  | 'ui.toast.dismissed'
  | 'ui.import.progress'
  | 'ui.fetch.result'
  | 'ui.window.fault';

export const MAX_QUEUE = 200;
export const MAX_BATCH_RECORDS = 50;
export const MAX_BATCH_BYTES = 6 * 1024;
export const MAX_PAYLOAD_BYTES = 1024;
export const MIN_INTERVAL_MS = 1000;
const ENDPOINT = '/api/actions/app.diagnostics';
const MAX_DEPTH = 4;
const MAX_ARRAY = 32;
const MAX_STRING = 64;
// Defensa en profundidad: ninguna clave sensible sale de la UI aunque un llamador se equivoque.
const SENSITIVE_KEY = /token|url|uri|qr|svg|secret|password|cookie|authorization|pairing|key|chat|path/i;

interface Pending {
  code: DiagCode;
  ts: number;
  payload: Record<string, unknown>;
}

// Reactivo para que los efectos de los componentes solo rastreen una señal cuando está apagado.
const gate = $state({ on: false });
let enabled = false;
let queue: Pending[] = [];
let dropped = 0;
let timer: ReturnType<typeof setTimeout> | null = null;
let inflight = false;
let lastSent = -Infinity;
let failures = 0;
let listening = false;

export function diagEnabled(): boolean {
  return enabled;
}

/** Lectura reactiva de la capacidad (para guardas al inicio de `$effect`). */
export function diagOn(): boolean {
  return gate.on;
}

function clean(value: unknown, depth: number): unknown {
  if (value === null || typeof value === 'boolean') return value;
  if (typeof value === 'number') return Number.isFinite(value) ? value : null;
  if (typeof value === 'string') return value.length > MAX_STRING ? value.slice(0, MAX_STRING) : value;
  if (depth >= MAX_DEPTH) return null;
  if (Array.isArray(value)) return value.slice(0, MAX_ARRAY).map((v) => clean(v, depth + 1));
  if (typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      if (SENSITIVE_KEY.test(k) || typeof v === 'function' || v === undefined) continue;
      out[k] = clean(v, depth + 1);
    }
    return out;
  }
  return null;
}

function cleanPayload(payload: Record<string, unknown>): Record<string, unknown> {
  const out = clean(payload, 0) as Record<string, unknown>;
  const bytes = JSON.stringify(out).length;
  return bytes > MAX_PAYLOAD_BYTES ? { truncated: true, bytes } : out;
}

/** Encola un registro. No-op sin capacidad. Nunca lanza. */
export function diag(code: DiagCode, payload: Record<string, unknown> = {}): void {
  if (!enabled) return;
  try {
    if (queue.length >= MAX_QUEUE) {
      dropped += 1;
      return;
    }
    queue.push({ code, ts: Date.now(), payload: cleanPayload(payload) });
    schedule();
  } catch {
    // El diagnóstico nunca afecta a la aplicación.
  }
}

function schedule(): void {
  if (!enabled || timer !== null || inflight || queue.length === 0) return;
  const wait = Math.max(0, lastSent + MIN_INTERVAL_MS - Date.now());
  timer = setTimeout(() => {
    timer = null;
    void flush();
  }, wait);
}

function takeBatch(): Pending[] {
  const batch: Pending[] = [];
  let bytes = 2;
  if (dropped > 0) {
    batch.push({ code: 'ui.queue.dropped', ts: Date.now(), payload: { count: dropped } });
    dropped = 0;
  }
  while (queue.length > 0 && batch.length < MAX_BATCH_RECORDS) {
    const next = queue[0];
    const size = JSON.stringify(next).length + 1;
    if (batch.length > 0 && bytes + size > MAX_BATCH_BYTES) break;
    bytes += size;
    batch.push(queue.shift() as Pending);
  }
  return batch;
}

async function flush(): Promise<void> {
  if (!enabled || inflight || queue.length === 0) return;
  inflight = true;
  lastSent = Date.now();
  const batch = takeBatch();
  try {
    const res = await fetch(ENDPOINT, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(batch),
    });
    if (res.status === 404) {
      disableDiagnostics();
      return;
    }
    failures = res.ok ? 0 : failures + 1;
  } catch {
    failures += 1;
  } finally {
    inflight = false;
  }
  if (failures >= 5) {
    disableDiagnostics();
    return;
  }
  schedule();
}

function onWindowError(ev: ErrorEvent): void {
  diag('ui.window.fault', { kind: 'error', line: ev.lineno ?? 0, col: ev.colno ?? 0 });
}

function onRejection(): void {
  diag('ui.window.fault', { kind: 'rejection' });
}

/** Activa o desactiva el diagnóstico según la capacidad anunciada por `hello`. */
export function setDiagnosticsCapability(capable: boolean): void {
  if (capable === enabled) return;
  if (!capable) {
    disableDiagnostics();
    return;
  }
  enabled = true;
  gate.on = true;
  if (typeof window !== 'undefined' && !listening) {
    window.addEventListener('error', onWindowError);
    window.addEventListener('unhandledrejection', onRejection);
    listening = true;
  }
  diag('ui.diagnostics.enabled', { max_queue: MAX_QUEUE });
}

export function disableDiagnostics(): void {
  enabled = false;
  gate.on = false;
  queue = [];
  dropped = 0;
  failures = 0;
  inflight = false;
  lastSent = -Infinity;
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
  if (listening && typeof window !== 'undefined') {
    window.removeEventListener('error', onWindowError);
    window.removeEventListener('unhandledrejection', onRejection);
  }
  listening = false;
}
