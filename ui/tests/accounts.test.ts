import type { Component } from 'svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
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

test('AccountsPanel configure key button navigates to Settings accounts', () => {
  const events: Array<{ section?: string }> = [];
  const onOpen = (e: Event) => {
    events.push((e as CustomEvent<{ section?: string }>).detail ?? {});
  };
  window.addEventListener('tmc:open-settings', onOpen);

  mountWith(AccountsPanel, {
    'accounts.status': {
      current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
      inara: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      edsm: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      eddn: { ...baseDisabledService('switch_off'), enabled: false, paused_schemas: [] },
      commanders: [
        {
          cmdr_ref: 'cmdr-1',
          name: 'Danikuso',
          edsm_key: false,
          inara_key: false,
          key_assignment: 'confirmed',
          edsm_pending: 0,
          inara_pending: 0,
          eddn_pending: 0,
        },
      ],
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

  (q('edsm-go-settings-btn') as HTMLButtonElement).click();
  window.removeEventListener('tmc:open-settings', onOpen);

  expect(events).toEqual([{ section: 'accounts' }]);
});

test('eddn switch sends accounts.eddn.set_enabled and never shows sent without report', async () => {
  const posted: Array<{ url: string; body: unknown }> = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init?: RequestInit) => {
      posted.push({
        url,
        body: init?.body ? JSON.parse(String(init.body)) : null,
      });
      return new Response(JSON.stringify({ ok: true, enabled: true }), { status: 200 });
    }),
  );

  mountWith(AccountsPanel, {
    'accounts.status': {
      current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
      inara: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      edsm: { ...baseDisabledService('no_key'), has_key: false, key_hint: null },
      eddn: {
        ...baseDisabledService('switch_off'),
        enabled: false,
        paused_schemas: ['journal/1'],
      },
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

  expect(q('eddn-state-pill')?.textContent).toContain('Desactivado');
  expect(q('eddn-metric-sent')?.textContent?.trim()).toBe('0');
  expect(q('eddn-paused-schemas')?.textContent).toContain('journal/1');

  const sw = q('eddn-switch') as HTMLButtonElement;
  sw.click();
  await new Promise((r) => setTimeout(r, 10));
  flushSync();

  expect(posted.length).toBe(1);
  expect(posted[0].url).toContain('accounts.eddn.set_enabled');
  expect(posted[0].body).toEqual({ enabled: true });

  // Sin un nuevo reporte del backend no se muestra optimísticamente como Conectado/enviado.
  expect(q('eddn-state-pill')?.textContent).toContain('Desactivado');
  expect(q('eddn-state-pill')?.textContent).not.toContain('Conectado');
  expect(q('eddn-metric-sent')?.textContent?.trim()).toBe('0');
});

describe('E4-2 collapsible sync details', () => {
  const mkRejection = (idx: number) => ({
    kind: idx === 0 ? 'journal/1' : `FSDJump#${idx}`,
    class: 'invalid_event',
    code: idx === 0 ? '400' : `E${idx}`,
    at: `2026-11-13T10:${String(idx % 60).padStart(2, '0')}:00Z`,
    hint: `Server detail ${idx} secret-hint`,
  });

  test('details collapsed by default and expands on toggle', () => {
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

    const detailsWrap = target.querySelector('.hud-panel-details');
    expect(detailsWrap?.classList.contains('collapsed')).toBe(true);
    expect(q('sync-details-section')).toBeTruthy();

    const toggle = target.querySelector('.panel-header-toggle') as HTMLButtonElement;
    toggle.click();
    flushSync();
    expect(detailsWrap?.classList.contains('collapsed')).toBe(false);
  });

  test('breakdown, paused schemas, per-CMDR pending, privacy table, EDMC/EDDiscovery warning, and rejections (max 20, hint only in title)', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-11-13T10:00:00Z'));

    const manyRejections = Array.from({ length: 24 }, (_, i) => mkRejection(i));

    mountWith(AccountsPanel, {
      'accounts.status': {
        current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
        inara: {
          ...baseDisabledService('network'),
          state: 'error',
          pending: 5,
          recovered_session: 3,
          sent_total: 42,
          sent_session: 7,
          rejected_total: 2,
          rejected_session: 1,
          expired_total: 4,
          omitted_total: 6,
          next_attempt_at: '2026-11-13T10:04:00Z',
          recent_rejections: manyRejections,
          has_key: true,
          key_hint: '1234',
        },
        edsm: {
          ...baseDisabledService('no_key'),
          state: 'waiting',
          reason: null,
          pending: 1,
          recovered_session: 1,
          sent_total: 10,
          sent_session: 2,
          has_key: true,
          key_hint: '5678',
        },
        eddn: {
          ...baseDisabledService('schema_outdated'),
          state: 'paused',
          enabled: true,
          paused_schemas: ['journal/1', 'commodity/3'],
          pending: 2,
          recovered_session: 4,
          sent_total: 100,
          sent_session: 15,
        },
        commanders: [
          {
            cmdr_ref: 'cmdr-1',
            name: 'Danikuso',
            inara_key: true,
            edsm_key: true,
            key_assignment: 'confirmed',
            inara_pending: 5,
            edsm_pending: 1,
            eddn_pending: 2,
          },
          {
            cmdr_ref: 'cmdr-2',
            name: 'AltExplorer',
            inara_key: true,
            edsm_key: false,
            key_assignment: 'confirmed',
            inara_pending: 8,
            edsm_pending: 0,
            eddn_pending: 0,
          },
        ],
        other_cmdrs_pending: { inara: 8, edsm: 0, eddn: 0 },
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

    (target.querySelector('.panel-header-toggle') as HTMLButtonElement).click();
    flushSync();

    const details = q('sync-details-section')!;
    expect(q('details-inara-breakdown')?.textContent).toContain('42');
    expect(q('details-inara-breakdown')?.textContent).toContain('7');
    expect(q('details-inara-breakdown')?.textContent).toContain('4');
    expect(q('details-inara-breakdown')?.textContent).toContain('6');
    expect(q('details-inara-breakdown')?.textContent).toContain('4 min');
    expect(q('details-recovered-total')?.textContent).toContain('8');
    expect(q('details-eddn-paused-schemas')?.textContent).toContain('journal/1');
    expect(q('details-eddn-paused-schemas')?.textContent).toContain('commodity/3');

    const rejRows = details.querySelectorAll('[data-testid="sync-rejection-row"]');
    expect(rejRows.length).toBe(20);
    expect(rejRows[0].textContent).toContain('Evento no válido');
    expect(rejRows[0].textContent).not.toContain('Server detail');
    expect((rejRows[0] as HTMLElement).getAttribute('title')).toContain('Server detail');

    const cmdrRows = details.querySelectorAll('[data-testid="details-cmdr-pending-row"]');
    expect(cmdrRows.length).toBe(2);
    expect(cmdrRows[0].textContent).toContain('Danikuso');
    expect(cmdrRows[1].textContent).toContain('AltExplorer');
    expect(cmdrRows[1].textContent).toContain('8');

    expect(q('details-privacy-table')?.textContent).toContain('EDDN');
    expect(q('details-privacy-table')?.textContent).toContain('EDSM');
    expect(q('details-privacy-table')?.textContent).toContain('Inara');
    expect(q('details-duplicate-warning')?.textContent).toContain('EDMC');
    expect(q('details-duplicate-warning')?.textContent).toContain('EDDiscovery');
  });

  test('list height adapts to rows with zero phantom scroll when short and enables scroll only when overflowed', () => {
    mountWith(AccountsPanel, {
      'accounts.status': {
        current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
        inara: {
          ...baseDisabledService('network'),
          state: 'error',
          has_key: true,
          key_hint: '1234',
          recent_rejections: [mkRejection(0), mkRejection(1)],
        },
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

    (target.querySelector('.panel-header-toggle') as HTMLButtonElement).click();
    flushSync();

    const shortList = q('sync-rejections-list') as HTMLElement;
    expect(shortList.style.height).toBe('72px');
    expect(shortList.style.overflowY).toBe('hidden');
  });
});
