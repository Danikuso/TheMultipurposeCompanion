import { setLocale } from './i18n';
import { applyTheme, type HudThemeInput } from './theme';

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
const alertListeners = new Set<(a: Alert) => void>();

function defaultWsUrl(): string {
  if (typeof location === 'undefined') {
    return 'ws://127.0.0.1:47320/ws';
  }
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${location.host}/ws`;
}

function applyTopics(incoming: Record<string, unknown>, isSnapshot: boolean): void {
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
    if (typeof document !== 'undefined' && document.hidden) {
      ws.send(JSON.stringify({ type: 'subscribe', topics: ['core.game'] }));
    }
  };

  ws.onmessage = (ev: { data: string }) => {
    let msg: Record<string, unknown>;
    try {
      msg = JSON.parse(ev.data) as Record<string, unknown>;
    } catch {
      return;
    }

    switch (msg.type) {
      case 'hello': {
        if (typeof msg.locale === 'string') {
          setLocale(msg.locale);
        }
        break;
      }
      case 'snapshot': {
        if (typeof msg.seq === 'number') {
          live.seq = msg.seq;
        }
        if (msg.topics && typeof msg.topics === 'object') {
          applyTopics(msg.topics as Record<string, unknown>, true);
        }
        break;
      }
      case 'update': {
        if (typeof msg.seq === 'number') {
          live.seq = msg.seq;
        }
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
        for (const cb of alertListeners) {
          cb(alertObj);
        }
        break;
      }
      case 'import-progress': {
        const done = Number(msg.done ?? 0);
        const total = Number(msg.total ?? 0);
        live.importProgress = { done, total };
        break;
      }
    }
  };

  ws.onclose = () => {
    live.connected = false;
    if (currentSocket === ws) {
      currentSocket = null;
      scheduleReconnect();
    }
  };
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
  const res = await fetch(`/api/actions/${encodeURIComponent(name)}`, init);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body as { error?: string }).error ?? `action ${name} failed`);
  }
  return res.json();
}
