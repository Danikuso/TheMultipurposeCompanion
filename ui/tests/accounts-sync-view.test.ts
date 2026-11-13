import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import AccountsPanel from '../src/modules/accounts/AccountsPanel.svelte';
import { KEY_PAGES } from '../src/modules/accounts/syncView';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountPanel(status: Record<string, unknown>) {
  live.topics['accounts.status'] = status;
  mounted = mount(AccountsPanel, { target });
  flushSync();
}

const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);

const makeService = (overrides: Record<string, unknown> = {}) => ({
  state: 'disabled',
  reason: 'no_key',
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
  ...overrides,
});

const baseStatus = (overrides: Record<string, unknown> = {}) => ({
  current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
  inara: makeService({ state: 'waiting', reason: null, has_key: true, key_hint: '1234' }),
  edsm: makeService({ state: 'waiting', reason: null, has_key: true, key_hint: '5678' }),
  eddn: makeService({ state: 'waiting', reason: null, enabled: true, paused_schemas: [] }),
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
  ...overrides,
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

describe('E4-1 sync summary view', () => {
  test('six states render text and icon (incl. waiting, unavailable)', () => {
    const cases: Array<{ state: string; reason: string | null; expectedText: string; expectedIcon: string }> = [
      { state: 'connected', reason: null, expectedText: 'Conectado', expectedIcon: '✓' },
      { state: 'waiting', reason: null, expectedText: 'En espera', expectedIcon: '⏳' },
      { state: 'error', reason: 'network', expectedText: 'Error', expectedIcon: '✕' },
      { state: 'paused', reason: 'online_off', expectedText: 'En pausa', expectedIcon: '⏸' },
      { state: 'unavailable', reason: 'platform_unsupported', expectedText: 'No disponible', expectedIcon: '⊘' },
      { state: 'disabled', reason: 'switch_off', expectedText: 'Desactivado', expectedIcon: '○' },
    ];

    for (const item of cases) {
      if (mounted) {
        unmount(mounted);
        mounted = undefined;
      }
      mountPanel(
        baseStatus({
          eddn: makeService({
            state: item.state,
            reason: item.reason,
            enabled: item.state !== 'disabled',
            paused_schemas: [],
          }),
        }),
      );
      const pill = q('eddn-state-pill')?.textContent ?? '';
      expect(pill).toContain(item.expectedText);
      expect(pill).toContain(item.expectedIcon);
    }
  });

  test('other instance shows paused other_instance text', () => {
    mountPanel(
      baseStatus({
        eddn: makeService({
          state: 'paused',
          reason: 'other_instance',
          enabled: true,
          paused_schemas: [],
        }),
      }),
    );

    const row = q('sync-row-eddn')?.textContent ?? '';
    expect(q('eddn-state-pill')?.textContent).toContain('⏸');
    expect(row).toContain('otra instancia está sincronizando');
  });

  test('retry button only in error', () => {
    mountPanel(
      baseStatus({
        inara: makeService({
          state: 'error',
          reason: 'network',
          has_key: true,
          key_hint: '1234',
        }),
        edsm: makeService({
          state: 'waiting',
          reason: null,
          has_key: true,
          key_hint: '5678',
        }),
        eddn: makeService({
          state: 'connected',
          reason: null,
          last_confirmed_at: '2026-11-13T10:00:00Z',
          sent_total: 4,
          enabled: true,
          paused_schemas: [],
        }),
      }),
    );

    expect(q('inara-retry-btn')).toBeTruthy();
    expect(q('edsm-retry-btn')).toBeNull();
    expect(q('eddn-retry-btn')).toBeNull();
  });

  test('guide only when key missing', () => {
    mountPanel(baseStatus());
    expect(q('sync-unlock-guide')).toBeNull();

    unmount(mounted!);
    mounted = undefined;

    mountPanel(
      baseStatus({
        inara: makeService({ state: 'disabled', reason: 'no_key', has_key: false, key_hint: null }),
      }),
    );
    expect(q('sync-unlock-guide')).toBeTruthy();
    expect(q('sync-unlock-guide')?.classList.contains('companion-unlock-guide')).toBe(true);
    expect(q('inara-go-settings-btn')).toBeTruthy();
  });

  test('copy link copies KEY_PAGES[service]', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });

    mountPanel(
      baseStatus({
        inara: makeService({ state: 'disabled', reason: 'no_key', has_key: false, key_hint: null }),
        edsm: makeService({ state: 'disabled', reason: 'no_key', has_key: false, key_hint: null }),
      }),
    );

    (q('guide-copy-inara') as HTMLButtonElement).click();
    await Promise.resolve();
    expect(writeText).toHaveBeenCalledWith(KEY_PAGES.inara);
    expect(KEY_PAGES.inara).toBe('https://inara.cz/elite/cmdr-settings-api/');

    (q('guide-copy-edsm') as HTMLButtonElement).click();
    await Promise.resolve();
    expect(writeText).toHaveBeenCalledWith(KEY_PAGES.edsm);
    expect(KEY_PAGES.edsm).toBe('https://www.edsm.net/en/settings/api');
  });

  test('last confirmed shows dash when null', () => {
    mountPanel(baseStatus());
    expect(q('inara-metric-last-confirmed')?.textContent?.trim()).toBe('—');
    expect(q('edsm-metric-last-confirmed')?.textContent?.trim()).toBe('—');
    expect(q('eddn-metric-last-confirmed')?.textContent?.trim()).toBe('—');
  });
});
