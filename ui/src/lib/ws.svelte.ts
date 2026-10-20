import { setLocale } from './i18n';
import { applyTheme, type HudThemeInput } from './theme';
import { diag, disableDiagnostics, setDiagnosticsCapability } from './diag.svelte';

export interface Alert {
  id: string;
  level: 'Info' | 'Warning' | 'Critical';
  key: string;
  args: Record<string, string | number>;
  since: string;
}

export const live = $state({
  connected: false,
  topics: {} as Record<string, unknown>,
  seq: 0,
  importProgress: null as { done: number; total: number } | null,
});

const BACKOFF_MS = [1000, 2000, 5000, 10000];

let currentSocket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let attemptIndex = 0;
let shouldReconnect = false;
let targetUrl = '';
let visibilityHandler: (() => void) | null = null;
// Última secuencia fiable; `null` tras un snapshot sin `seq` (no hay referencia para huecos).
let lastSeq: number | null = null;
const alertListeners = new Set<(a: Alert) => void>();

function defaultWsUrl(): string {
  if (typeof location === 'undefined') {
    return 'ws://127.0.0.1:47320/ws';
  }
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${location.host}/ws`;
}

function applyTopics(incoming: Record<string, unknown>, isSnapshot: boolean): void {
  const names = Object.keys(incoming);
  diag('ui.topic.applied', { snapshot: isSnapshot, count: names.length, topic: names.slice(0, 32) });
  if (isSnapshot) {
    live.topics = { ...live.topics, ...incoming };
  } else {
    for (const [k, v] of Object.entries(incoming)) {
      live.topics[k] = v;
    }
  }
  const theme = incoming['core.theme'] as HudThemeInput | undefined;
  if (theme && typeof theme.primary === 'string') {
    applyTheme(theme);
  }
}

function scheduleReconnect(): void {
  if (!shouldReconnect) return;
  const delay = BACKOFF_MS[Math.min(attemptIndex, BACKOFF_MS.length - 1)];
  attemptIndex += 1;
  diag('ui.ws.connection', { state: 'reconnect', attempt: attemptIndex, delay_ms: delay });
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    openSocket();
  }, delay);
}

function openSocket(): void {
  if (!shouldReconnect) return;
  const ws = new WebSocket(targetUrl);
  currentSocket = ws;

  ws.onopen = () => {
    attemptIndex = 0;
    live.connected = true;
    lastSeq = null;
    diag('ui.ws.connection', { state: 'open', hidden: typeof document !== 'undefined' && document.hidden });
    if (typeof document !== 'undefined' && document.hidden) {
      ws.send(JSON.stringify({ type: 'subscribe', topics: ['core.game'] }));
    }
  };

  ws.onmessage = (ev: { data: string }) => {
    let msg: Record<string, unknown>;
    try {
      msg = JSON.parse(ev.data) as Record<string, unknown>;
    } catch {
      diag('ui.ws.fault', { kind: 'parse', size: typeof ev.data === 'string' ? ev.data.length : 0 });
      return;
    }

    try {
      handleMessage(msg);
    } catch (err) {
      diag('ui.ws.fault', { kind: 'handler', type: typeof msg.type === 'string' ? msg.type.length : 0 });
      throw err;
    }
  };

  ws.onerror = () => {
    diag('ui.ws.fault', { kind: 'socket' });
  };

  ws.onclose = (ev?: { code?: number }) => {
    live.connected = false;
    diag('ui.ws.connection', { state: 'closed', code: ev?.code ?? 0 });
    if (currentSocket === ws) {
      currentSocket = null;
      scheduleReconnect();
    }
  };
}

function handleMessage(msg: Record<string, unknown>): void {
    switch (msg.type) {
      case 'hello': {
        const caps = msg.capabilities as { diagnostics?: unknown } | undefined;
        setDiagnosticsCapability(caps?.diagnostics === true);
        diag('ui.ws.hello', { topics: Array.isArray(msg.topics) ? msg.topics.length : 0 });
        if (typeof msg.locale === 'string') {
          setLocale(msg.locale);
        }
        break;
      }
      case 'snapshot': {
        const hasSeq = typeof msg.seq === 'number';
        if (typeof msg.seq === 'number') {
          live.seq = msg.seq;
        }
        lastSeq = hasSeq ? (msg.seq as number) : null;
        diag('ui.ws.snapshot', { has_seq: hasSeq, seq: hasSeq ? msg.seq : 0 });
        if (msg.topics && typeof msg.topics === 'object') {
          applyTopics(msg.topics as Record<string, unknown>, true);
        }
        break;
      }
      case 'update': {
        if (typeof msg.seq === 'number') {
          if (lastSeq !== null && msg.seq !== lastSeq + 1) {
            diag('ui.ws.gap', { expected: lastSeq + 1, got: msg.seq });
          }
          lastSeq = msg.seq;
          live.seq = msg.seq;
        }
        diag('ui.ws.update', { seq: typeof msg.seq === 'number' ? msg.seq : 0 });
        if (msg.topics && typeof msg.topics === 'object') {
          applyTopics(msg.topics as Record<string, unknown>, false);
        }
        break;
      }
      case 'alert': {
        const alertObj: Alert = {
          id: String(msg.id ?? ''),
          level: (msg.level as Alert['level']) ?? 'Info',
          key: String(msg.key ?? ''),
          args: (msg.args as Record<string, string | number>) ?? {},
          since: String(msg.since ?? ''),
        };
        diag('ui.ws.alert', { level: String(alertObj.level).toLowerCase(), listeners: alertListeners.size });
        for (const cb of alertListeners) {
          cb(alertObj);
        }
        break;
      }
      case 'import-progress': {
        const done = Number(msg.done ?? 0);
        const total = Number(msg.total ?? 0);
        live.importProgress = { done, total };
        diag('ui.ws.import_progress', { done, total });
        break;
      }
    }
}

export function connect(url?: string): void {
  disconnect();
  targetUrl = url ?? defaultWsUrl();
  shouldReconnect = true;
  attemptIndex = 0;

  if (typeof document !== 'undefined') {
    visibilityHandler = () => {
      if (!currentSocket || currentSocket.readyState !== 1) return;
      const topics = document.hidden ? ['core.game'] : ['*'];
      diag('ui.ws.subscribe', { hidden: document.hidden, patterns: topics.length });
      currentSocket.send(JSON.stringify({ type: 'subscribe', topics }));
    };
    document.addEventListener('visibilitychange', visibilityHandler);
  }

  openSocket();
}

export function disconnect(): void {
  shouldReconnect = false;
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (visibilityHandler && typeof document !== 'undefined') {
    document.removeEventListener('visibilitychange', visibilityHandler);
    visibilityHandler = null;
  }
  const sock = currentSocket;
  currentSocket = null;
  if (sock) {
    sock.onclose = null;
    sock.close();
  }
  live.connected = false;
  live.topics = {};
  live.seq = 0;
  live.importProgress = null;
  lastSeq = null;
  disableDiagnostics();
}

export function topic<T>(name: string): T | undefined {
  return live.topics[name] as T | undefined;
}

export function onAlert(cb: (a: Alert) => void): () => void {
  alertListeners.add(cb);
  return () => {
    alertListeners.delete(cb);
  };
}

export async function action(name: string, args?: unknown): Promise<unknown> {
  const init: RequestInit = { method: 'POST' };
  if (args !== undefined) {
    init.headers = { 'content-type': 'application/json' };
    init.body = JSON.stringify(args);
  }
  const started = Date.now();
  diag('ui.action.sent', { action: name, has_args: args !== undefined });
  let res: Response;
  try {
    res = await fetch(`/api/actions/${encodeURIComponent(name)}`, init);
  } catch (err) {
    diag('ui.action.result', { action: name, ok: false, status: 0, duration_ms: Date.now() - started });
    throw err;
  }
  diag('ui.action.result', { action: name, ok: res.ok, status: res.status, duration_ms: Date.now() - started });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as { error?: string }).error ?? `action ${name} failed`);
  }
  return res.json();
}
