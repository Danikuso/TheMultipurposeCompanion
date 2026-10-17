import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { action, connect, disconnect, live, topic } from '../src/lib/ws.svelte';

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  url: string;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 0;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  open() {
    this.readyState = 1;
    this.onopen?.();
  }

  receive(msg: unknown) {
    this.onmessage?.({ data: JSON.stringify(msg) });
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = 3;
    this.onclose?.();
  }
}

describe('ws client', () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
    vi.useFakeTimers();
    disconnect();
  });

  afterEach(() => {
    disconnect();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  test('ws applies snapshot then update', () => {
    connect('ws://127.0.0.1:47320/ws');
    expect(MockWebSocket.instances.length).toBe(1);
    const ws = MockWebSocket.instances[0];
    ws.open();
    expect(live.connected).toBe(true);

    ws.receive({
      type: 'hello',
      version: 1,
      locale: 'es',
      topics: ['core.status', 'core.location'],
    });
    ws.receive({
      type: 'snapshot',
      seq: 1,
      topics: {
        'core.status': { shields_up: true },
        'core.location': { system: 'Sol' },
      },
    });

    expect(topic<{ shields_up: boolean }>('core.status')?.shields_up).toBe(true);
    expect(topic<{ system: string }>('core.location')?.system).toBe('Sol');
    expect(live.seq).toBe(1);

    ws.receive({
      type: 'update',
      seq: 2,
      topics: {
        'core.status': { shields_up: false },
      },
    });

    expect(topic<{ shields_up: boolean }>('core.status')?.shields_up).toBe(false);
    expect(topic<{ system: string }>('core.location')?.system).toBe('Sol');
    expect(live.seq).toBe(2);
  });

  test('ws reconnects with backoff', () => {
    connect('ws://127.0.0.1:47320/ws');
    expect(MockWebSocket.instances.length).toBe(1);

    // 1st disconnect -> 1000 ms
    MockWebSocket.instances[0].close();
    vi.advanceTimersByTime(999);
    expect(MockWebSocket.instances.length).toBe(1);
    vi.advanceTimersByTime(1);
    expect(MockWebSocket.instances.length).toBe(2);

    // 2nd disconnect -> 2000 ms
    MockWebSocket.instances[1].close();
    vi.advanceTimersByTime(1999);
    expect(MockWebSocket.instances.length).toBe(2);
    vi.advanceTimersByTime(1);
    expect(MockWebSocket.instances.length).toBe(3);

    // 3rd disconnect -> 5000 ms
    MockWebSocket.instances[2].close();
    vi.advanceTimersByTime(4999);
    expect(MockWebSocket.instances.length).toBe(3);
    vi.advanceTimersByTime(1);
    expect(MockWebSocket.instances.length).toBe(4);

    // 4th disconnect -> 10000 ms
    MockWebSocket.instances[3].close();
    vi.advanceTimersByTime(9999);
    expect(MockWebSocket.instances.length).toBe(4);
    vi.advanceTimersByTime(1);
    expect(MockWebSocket.instances.length).toBe(5);

    // 5th disconnect -> capped at 10000 ms
    MockWebSocket.instances[4].close();
    vi.advanceTimersByTime(9999);
    expect(MockWebSocket.instances.length).toBe(5);
    vi.advanceTimersByTime(1);
    expect(MockWebSocket.instances.length).toBe(6);
  });

  test('action posts args as json body', async () => {
    const fetchMock = vi.fn(async () => ({ ok: true, json: async () => ({ accepted: true }) }));
    vi.stubGlobal('fetch', fetchMock);
    const out = await action('navigation.plan', { to: 'Colonia' });
    expect(out).toEqual({ accepted: true });
    const [url, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe('/api/actions/navigation.plan');
    expect(init.method).toBe('POST');
    expect(init.body).toBe('{"to":"Colonia"}');
    expect((init.headers as Record<string, string>)['content-type']).toBe('application/json');

    await action('navigation.clear');
    const [, init2] = fetchMock.mock.calls[1] as unknown as [string, RequestInit];
    expect(init2.body).toBeUndefined();
  });

  test('ws narrows subscription when page hidden', () => {
    connect('ws://127.0.0.1:47320/ws');
    const ws = MockWebSocket.instances[0];
    ws.open();

    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    document.dispatchEvent(new Event('visibilitychange'));
    expect(ws.sent.map((s) => JSON.parse(s))).toContainEqual({
      type: 'subscribe',
      topics: ['core.game'],
    });

    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
    document.dispatchEvent(new Event('visibilitychange'));
    expect(ws.sent.map((s) => JSON.parse(s))).toContainEqual({
      type: 'subscribe',
      topics: ['*'],
    });
  });
});
