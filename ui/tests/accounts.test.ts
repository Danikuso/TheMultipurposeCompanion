import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import AccountsPanel from '../src/modules/accounts/AccountsPanel.svelte';
import contractSnapshot from './fixtures/accounts-topics.json';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}

const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);

const baseDisabledService = (reason: string) => ({
  state: 'disabled',
  reason,
  pending: 0,
  recovered_session: 0,
  sent_total: 0,
  sent_session: 0,
  rejected_total: 0,
  rejected_session: 0,
  expired_total: 0,
  omitted_total: 0,
  last_confirmed_at: null,
  next_attempt_at: null,
  recent_rejections: [],
});

beforeEach(() => {
  disconnect();
  setLocale('es');
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined;
  target.remove();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

test('renders state pill text for waiting and disabled from contract fixture', () => {
  const snap = contractSnapshot as Record<string, unknown>;
  mountWith(AccountsPanel, {
    'accounts.status': snap['accounts.status'],
  });

  expect(q('accounts-privacy-banner')).toBeTruthy();
  expect(q('accounts-eddn-card')).toBeTruthy();
  expect(q('accounts-edsm-card')).toBeTruthy();
  expect(q('accounts-inara-card')).toBeTruthy();
  expect(q('accounts-capi-card')).toBeTruthy();

  expect(q('inara-state-pill')?.textContent).toContain('En espera');
  expect(q('edsm-state-pill')?.textContent).toContain('Desactivado');
  expect(q('eddn-state-pill')?.textContent).toContain('Desactivado');
  expect(q('inara-whitelist-badge')).toBeTruthy();

  // Check 4 metrics exist on each service card and no fake uploader/session counters appear
  for (const svc of ['eddn', 'edsm', 'inara']) {
    expect(q(`${svc}-metric-sent`)).toBeTruthy();
    expect(q(`${svc}-metric-pending`)).toBeTruthy();
    expect(q(`${svc}-metric-rejected`)).toBeTruthy();
    expect(q(`${svc}-metric-last-confirmed`)).toBeTruthy();
  }
  expect(q('eddn-uploader-id')).toBeNull();
  expect(q('eddn-messages-count')).toBeNull();
});

test('never shows connected without sent_total/last_confirmed_at', () => {
  const snap = contractSnapshot as Record<string, unknown>;
  mountWith(AccountsPanel, {
    'accounts.status': snap['accounts.status'],
  });

  expect(target.textContent).not.toContain('Conectado');
  expect(q('inara-metric-last-confirmed')?.textContent?.trim()).toBe('—');
  expect(q('edsm-metric-last-confirmed')?.textContent?.trim()).toBe('—');
  expect(q('eddn-metric-last-confirmed')?.textContent?.trim()).toBe('—');
});

test('renders error state with retry countdown and connected metrics when confirmed', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-11-13T00:00:00Z'));

  mountWith(AccountsPanel, {
    'accounts.status': {
      current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
      inara: {
        ...baseDisabledService('network'),
        state: 'error',
        reason: 'network',
        pending: 4,
        rejected_total: 1,
        next_attempt_at: '2026-11-13T00:02:00Z',
        has_key: true,
        key_hint: '1234',
      },
      edsm: {
        ...baseDisabledService('no_key'),
        state: 'connected',
        reason: null,
        pending: 0,
        sent_total: 9,
        sent_session: 3,
        rejected_total: 2,
        last_confirmed_at: '2026-11-12T23:55:00Z',
        has_key: true,
        key_hint: '5678',
      },
      eddn: {
        ...baseDisabledService('switch_off'),
        state: 'connected',
        reason: null,
        pending: 1,
        sent_total: 12,
        sent_session: 12,
        last_confirmed_at: '2026-11-12T23:59:30Z',
        enabled: true,
        paused_schemas: [],
      },
      commanders: [],
      other_cmdrs_pending: { inara: 0, edsm: 0, eddn: 0 },
      unassigned_keys: [],
      capi: {
        configured: true,
        connected: false,
        commander_name: 'Danikuso',
        token_expires_at: null,
        remote_ships_count: 0,
        last_sync_time: null,
        last_error: null,
      },
    },
  });

  expect(q('inara-state-pill')?.textContent).toContain('Error');
  expect(q('inara-state-pill')?.textContent).toContain('2 min');
  expect(q('inara-retry-btn')).toBeTruthy();
  expect(q('edsm-retry-btn')).toBeNull();
  expect(q('edsm-state-pill')?.textContent).toContain('Conectado');
  expect(q('edsm-metric-sent')?.textContent?.trim()).toBe('9');
  expect(q('edsm-metric-rejected')?.textContent?.trim()).toBe('2');
  expect(q('eddn-state-pill')?.textContent).toContain('Conectado');
  expect(q('eddn-metric-sent')?.textContent?.trim()).toBe('12');
});

test('AccountsPanel renders connected CAPI status with remote fleet list', () => {
  mountWith(AccountsPanel, {
    'accounts.status': {
      current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
      inara: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      edsm: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      eddn: { ...baseDisabledService('switch_off'), enabled: false, paused_schemas: [] },
      commanders: [],
      other_cmdrs_pending: { inara: 0, edsm: 0, eddn: 0 },
      unassigned_keys: [],
      capi: {
        configured: true,
        connected: true,
        commander_name: 'Danikuso',
        token_expires_at: '2026-10-23T22:00:00Z',
        remote_ships_count: 3,
        last_sync_time: '2026-10-23T20:15:00Z',
        last_error: null,
      },
    },
  });

  expect(q('accounts-capi-card')).toBeTruthy();
  expect(q('accounts-capi-card')?.classList.contains('capi-card')).toBe(true);
  expect(q('capi-unlink-btn')).toBeTruthy();
  expect(q('capi-refresh-fleet-btn')).toBeTruthy();
  expect(q('capi-remote-fleet-list')).toBeTruthy();
  expect(q('accounts-capi-card')?.textContent).toContain('CMDR Danikuso');
  expect(q('accounts-capi-card')?.textContent).toContain('No hay naves almacenadas en astilleros remotos');
  expect(q('remote-fleet-select')).toBeNull();
});

test('AccountsPanel saveEdsmKey dispatches edsm.save_key and clears input', async () => {
  let actionCalled = false;
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (typeof url === 'string' && url.includes('edsm.save_key')) {
      actionCalled = true;
      return new Response(JSON.stringify({ ok: true }), { status: 200 });
    }
    return new Response(JSON.stringify({}), { status: 200 });
  }));

  mountWith(AccountsPanel, {
    'accounts.status': {
      current_cmdr: null,
      inara: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      edsm: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      eddn: { ...baseDisabledService('switch_off'), enabled: false, paused_schemas: [] },
      commanders: [],
      other_cmdrs_pending: { inara: 0, edsm: 0, eddn: 0 },
      unassigned_keys: [],
      capi: {
        configured: false,
        connected: false,
        commander_name: null,
        token_expires_at: null,
        remote_ships_count: 0,
        last_sync_time: null,
        last_error: null,
      },
    },
  });

  const input = q('edsm-key-input') as HTMLInputElement;
  const saveBtn = q('edsm-save-btn') as HTMLButtonElement;
  input.value = 'secret-edsm-key';
  input.dispatchEvent(new Event('input'));
  flushSync();

  saveBtn.click();
  await new Promise((r) => setTimeout(r, 10));
  flushSync();

  expect(actionCalled).toBe(true);
  expect(input.value).toBe('');
});
