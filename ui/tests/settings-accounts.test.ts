import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Settings from '../src/settings/Settings.svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';

describe('Settings with integrated Accounts', () => {
  let target: HTMLElement;
  let instance: Record<string, unknown> | null = null;

  beforeEach(() => {
    disconnect();
    setLocale('es');
    live.topics['accounts.status'] = {
      current_cmdr: { cmdr_ref: 'cmdr-1', name: 'Danikuso' },
      inara: {
        state: 'connected',
        reason: null,
        pending: 3,
        recovered_session: 0,
        sent_total: 10,
        sent_session: 3,
        rejected_total: 0,
        rejected_session: 0,
        expired_total: 0,
        omitted_total: 0,
        last_confirmed_at: '2026-10-30T16:10:00Z',
        next_attempt_at: null,
        recent_rejections: [],
        has_key: true,
        key_hint: '1234',
      },
      edsm: {
        state: 'connected',
        reason: null,
        pending: 0,
        recovered_session: 0,
        sent_total: 4,
        sent_session: 4,
        rejected_total: 0,
        rejected_session: 0,
        expired_total: 0,
        omitted_total: 0,
        last_confirmed_at: '2026-10-30T16:05:00Z',
        next_attempt_at: null,
        recent_rejections: [],
        has_key: true,
        key_hint: '5678',
      },
      eddn: {
        state: 'connected',
        reason: null,
        pending: 0,
        recovered_session: 0,
        sent_total: 42,
        sent_session: 42,
        rejected_total: 0,
        rejected_session: 0,
        expired_total: 0,
        omitted_total: 0,
        last_confirmed_at: '2026-10-30T16:00:00Z',
        next_attempt_at: null,
        recent_rejections: [],
        enabled: true,
        paused_schemas: [],
      },
      commanders: [],
      other_cmdrs_pending: { inara: 0, edsm: 0, eddn: 0 },
      unassigned_keys: [],
      capi: {
        configured: true,
        connected: true,
        commander_name: 'Danikuso',
        token_expires_at: null,
        remote_ships_count: 5,
        last_sync_time: '2026-10-30T16:15:00Z',
        last_error: null,
      },
    };

    target = document.createElement('div');
    document.body.appendChild(target);
  });

  afterEach(() => {
    if (instance) unmount(instance);
    instance = null;
    target.remove();
    disconnect();
  });

  test('Settings renders both the main settings pane and the accounts pane on the right', () => {
    instance = mount(Settings, { target });
    flushSync();

    // Check main panel has full width / span
    const panel = target.querySelector('section.hud-panel') as HTMLElement;
    expect(panel).not.toBeNull();
    expect(panel.style.gridColumn).toBe('1 / -1');

    // Check settings-accounts-pane exists
    const accountsPane = target.querySelector('[data-testid="settings-accounts-pane"]');
    expect(accountsPane).not.toBeNull();

    // Check Accounts cards are rendered inside Settings
    expect(target.querySelector('[data-testid="accounts-privacy-banner"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="accounts-eddn-card"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="accounts-edsm-card"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="accounts-inara-card"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="accounts-capi-card"]')).not.toBeNull();

    // Check Inara Whitelist badge is visible
    expect(target.querySelector('[data-testid="inara-whitelist-badge"]')).not.toBeNull();

    // Check Commander pills
    expect(accountsPane?.textContent).toContain('CMDR Danikuso');
  });

  test('settings shows EDSM key field per commander with masked hint', async () => {
    const posted: Array<{ url: string; body: unknown }> = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init?: RequestInit) => {
        posted.push({
          url,
          body: init?.body ? JSON.parse(String(init.body)) : null,
        });
        return new Response(
          JSON.stringify({ ok: true, verified: true, state: 'active' }),
          { status: 200 },
        );
      }),
    );

    (live.topics['accounts.status'] as Record<string, unknown>).commanders = [
      {
        cmdr_ref: 'cmdr-1',
        name: 'Danikuso',
        inara_key: true,
        edsm_key: true,
        key_assignment: 'confirmed',
        inara_pending: 0,
        edsm_pending: 0,
        eddn_pending: 0,
      },
      {
        cmdr_ref: 'cmdr-2',
        name: 'AltCmdr',
        inara_key: false,
        edsm_key: false,
        key_assignment: null,
        inara_pending: 0,
        edsm_pending: 0,
        eddn_pending: 0,
      },
    ];

    instance = mount(Settings, { target });
    flushSync();

    const selector = target.querySelector('[data-testid="accounts-cmdr-select"]') as HTMLSelectElement;
    expect(selector).not.toBeNull();
    expect(selector.value).toBe('cmdr-1');
    expect(target.querySelector('[data-testid="edsm-key-hint"]')?.textContent).toContain('5678');

    // Cambiar al segundo comandante y guardar una clave EDSM para él.
    selector.value = 'cmdr-2';
    selector.dispatchEvent(new Event('change'));
    flushSync();

    const input = target.querySelector('[data-testid="edsm-key-input"]') as HTMLInputElement;
    input.value = 'NEW-EDSM-ALT-KEY';
    input.dispatchEvent(new Event('input'));
    flushSync();

    const saveBtn = target.querySelector('[data-testid="edsm-save-btn"]') as HTMLButtonElement;
    saveBtn.click();
    await new Promise((r) => setTimeout(r, 10));
    flushSync();

    const saveReq = posted.find((p) => p.url.includes('accounts.key.save'));
    expect(saveReq).toBeTruthy();
    expect(saveReq?.body).toEqual({
      service: 'edsm',
      cmdr_ref: 'cmdr-2',
      api_key: 'NEW-EDSM-ALT-KEY',
    });
  });
});
