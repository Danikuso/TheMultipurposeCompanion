import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import {
  MAX_BATCH_BYTES,
  MAX_BATCH_RECORDS,
  MAX_QUEUE,
  MIN_INTERVAL_MS,
  diag,
  diagEnabled,
  disableDiagnostics,
  setDiagnosticsCapability,
} from '../src/lib/diag.svelte';
import { action, connect, disconnect } from '../src/lib/ws.svelte';
import Panel from '../src/shell/Panel.svelte';
import Pairing from '../src/settings/Pairing.svelte';
import rust from '../../crates/tmc-server/src/diag_ui.rs?raw';
import ts from '../src/lib/diag.svelte.ts?raw';

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;
  onclose: ((ev?: { code?: number }) => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 0;
  constructor(public url: string) {
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

type Rec = { code: string; ts: number; payload: Record<string, unknown> };

function diagCalls(fetchMock: ReturnType<typeof vi.fn>): string[] {
  return fetchMock.mock.calls
    .filter((c) => String(c[0]).endsWith('/api/actions/app.diagnostics'))
    .map((c) => String((c[1] as RequestInit).body));
}

function records(fetchMock: ReturnType<typeof vi.fn>): Rec[] {
  return diagCalls(fetchMock).flatMap((b) => JSON.parse(b) as Rec[]);
}

describe('ui diagnostics', () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  let addSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
    fetchMock = vi.fn(async () => ({ ok: true, status: 200, json: async () => ({}) }));
    vi.stubGlobal('fetch', fetchMock);
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
    disableDiagnostics();
    disconnect();
    addSpy = vi.spyOn(window, 'addEventListener');
  });

  afterEach(() => {
    disableDiagnostics();
    disconnect();
    addSpy.mockRestore();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  function hello(capabilities?: unknown) {
    connect('ws://127.0.0.1:47320/ws');
    const ws = MockWebSocket.instances[0];
    ws.open();
    ws.receive({ type: 'hello', version: 1, locale: 'es', topics: ['core.status'], capabilities });
    return ws;
  }

  test('without capability there are no requests, timers or listeners', async () => {
    const ws = hello();
    ws.receive({ type: 'snapshot', seq: 1, topics: { 'core.status': {} } });
    ws.receive({ type: 'update', seq: 5, topics: { 'core.status': {} } });
    diag('ui.ws.update', { a: 1 });
    const panel = mount(Panel, { target: document.body, props: { titleKey: 'panel-x' } });
    flushSync();
    await action('navigation.recalculate');
    unmount(panel);
    expect(diagEnabled()).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
    expect(addSpy.mock.calls.filter(([t]) => t === 'error' || t === 'unhandledrejection')).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(5000);
    expect(diagCalls(fetchMock)).toHaveLength(0);
    expect(fetchMock).toHaveBeenCalledTimes(1); // solo la acción de negocio
  });

  test('capability false or malformed keeps it disabled', () => {
    hello({ diagnostics: 'yes' });
    expect(diagEnabled()).toBe(false);
  });

  test('capability on batches at most once per second', async () => {
    const ws = hello({ diagnostics: true });
    expect(diagEnabled()).toBe(true);
    expect(addSpy.mock.calls.some(([t]) => t === 'error')).toBe(true);
    for (let i = 1; i <= 3; i++) ws.receive({ type: 'update', seq: 100 + i, topics: { 'core.status': {} } });
    expect(fetchMock).not.toHaveBeenCalled(); // temporizador pendiente, nada enviado aún
    await vi.advanceTimersByTimeAsync(0);
    expect(diagCalls(fetchMock)).toHaveLength(1);
    const first = records(fetchMock);
    expect(first.map((r) => r.code)).toContain('ui.ws.hello');
    for (let t = 0; t < 3; t++) {
      diag('ui.ws.update', { t });
      await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS - 1);
      expect(diagCalls(fetchMock)).toHaveLength(1 + t);
      await vi.advanceTimersByTimeAsync(1);
      expect(diagCalls(fetchMock)).toHaveLength(2 + t);
    }
    // Sin pendientes no queda temporizador.
    await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS * 3);
    expect(vi.getTimerCount()).toBe(0);
    const after = diagCalls(fetchMock).length;
    await vi.advanceTimersByTimeAsync(10_000);
    expect(diagCalls(fetchMock)).toHaveLength(after);
  });

  test('batches respect record and byte limits and the queue is bounded', async () => {
    setDiagnosticsCapability(true);
    await vi.advanceTimersByTimeAsync(0);
    fetchMock.mockClear();
    for (let i = 0; i < MAX_QUEUE + 50; i++) diag('ui.ws.update', { seq: i, pad: 'x'.repeat(50) });
    const seen: Rec[] = [];
    for (let i = 0; i < 20 && vi.getTimerCount() > 0; i++) {
      await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS);
    }
    for (const body of diagCalls(fetchMock)) {
      const batch = JSON.parse(body) as Rec[];
      expect(batch.length).toBeLessThanOrEqual(MAX_BATCH_RECORDS);
      expect(body.length).toBeLessThanOrEqual(MAX_BATCH_BYTES);
      seen.push(...batch);
    }
    const dropped = seen.find((r) => r.code === 'ui.queue.dropped');
    expect(dropped?.payload.count).toBe(50);
    expect(seen.filter((r) => r.code === 'ui.ws.update')).toHaveLength(MAX_QUEUE);
  });

  test('sending never logs and a failed send does not loop', async () => {
    fetchMock.mockImplementation(async () => ({ ok: false, status: 429 }));
    setDiagnosticsCapability(true);
    await vi.advanceTimersByTimeAsync(0);
    expect(diagCalls(fetchMock)).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(30_000);
    expect(diagCalls(fetchMock)).toHaveLength(1); // nada pendiente: el fallo no se registra a sí mismo
    expect(vi.getTimerCount()).toBe(0);
  });

  test('404 from the backend disables diagnostics', async () => {
    fetchMock.mockImplementation(async () => ({ ok: false, status: 404 }));
    setDiagnosticsCapability(true);
    await vi.advanceTimersByTimeAsync(0);
    expect(diagEnabled()).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  test('snapshots without seq do not create false gaps', async () => {
    const ws = hello({ diagnostics: true });
    ws.receive({ type: 'snapshot', seq: 10, topics: {} });
    ws.receive({ type: 'update', seq: 11, topics: { 'core.status': {} } });
    ws.receive({ type: 'snapshot', topics: {} }); // sin seq
    ws.receive({ type: 'update', seq: 40, topics: { 'core.status': {} } }); // no es hueco
    ws.receive({ type: 'update', seq: 43, topics: { 'core.status': {} } }); // hueco real
    await vi.advanceTimersByTimeAsync(0);
    const gaps = records(fetchMock).filter((r) => r.code === 'ui.ws.gap');
    expect(gaps).toHaveLength(1);
    expect(gaps[0].payload).toMatchObject({ expected: 41, got: 43 });
  });

  test('topic application and errors are recorded', async () => {
    const ws = hello({ diagnostics: true });
    ws.receive({ type: 'update', seq: 1, topics: { 'core.status': {}, 'core.location': {} } });
    ws.onmessage?.({ data: '{not json' });
    await vi.advanceTimersByTimeAsync(0);
    const recs = records(fetchMock);
    const applied = recs.find((r) => r.code === 'ui.topic.applied' && r.payload.snapshot === false);
    expect(applied?.payload).toMatchObject({ count: 2, topic: ['core.status', 'core.location'] });
    expect(recs.some((r) => r.code === 'ui.ws.fault' && r.payload.kind === 'parse')).toBe(true);
  });

  test('shared Panel reports mount and unmount', async () => {
    setDiagnosticsCapability(true);
    const panel = mount(Panel, { target: document.body, props: { titleKey: 'panel-route-strip', span: 'full' } });
    flushSync();
    unmount(panel);
    flushSync();
    await vi.advanceTimersByTimeAsync(0);
    const codes = records(fetchMock).map((r) => r.code);
    expect(codes).toContain('ui.panel.mounted');
    expect(codes).toContain('ui.panel.unmounted');
  });

  test('canaries never leave the UI (lan_token, pairing url, qr svg)', async () => {
    fetchMock.mockImplementation(async (url: string) =>
      String(url) === '/api/pairing'
        ? {
            ok: true,
            status: 200,
            json: async () => ({ url: 'http://192.168.1.5:47320/?t=CANARYURL', qr_svg: '<svg>CANARYSVG</svg>' }),
          }
        : { ok: true, status: 200, json: async () => ({}) },
    );
    setDiagnosticsCapability(true);
    const view = mount(Pairing, { target: document.body });
    await vi.advanceTimersByTimeAsync(0);
    flushSync();
    // Un llamador descuidado tampoco filtra claves sensibles.
    diag('ui.fetch.result', {
      kind: 'config-put',
      fullConfig: { server: { lan_token: 'CANARYTOKEN' }, lan_token: 'CANARYTOKEN' },
      pairingData: { url: 'CANARYURL', qr_svg: 'CANARYSVG' },
      url: 'CANARYURL',
      qr_svg: 'CANARYSVG',
    });
    await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS * 2);
    unmount(view);
    const sent = diagCalls(fetchMock).join('');
    expect(sent).toContain('pairing-get');
    expect(sent).not.toMatch(/CANARY/);
    expect(sent).not.toContain('lan_token');
    expect(sent).not.toContain('qr_svg');
  });

  test('allowed code list matches the Rust allowlist', () => {
    const block = /pub\(crate\) const CODES[^=]*=\s*&\[([\s\S]*?)\];/.exec(rust)![1];
    const rustCodes = [...block.matchAll(/"(ui\.[a-z_.]+)"/g)].map((m) => m[1]).sort();
    const type = /export type DiagCode =([\s\S]*?);/.exec(ts)![1];
    const tsCodes = [...type.matchAll(/'(ui\.[a-z_.]+)'/g)].map((m) => m[1]).sort();
    expect(tsCodes).toEqual(rustCodes);
  });
});
