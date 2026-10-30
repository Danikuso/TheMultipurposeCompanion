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
      eddn: {
        enabled: true,
        connected: true,
        uploader_id: 'TMC-Tester',
        messages_sent_session: 42,
        last_upload_time: '2026-10-30T16:00:00Z',
        last_error: null,
      },
      edsm: {
        enabled: true,
        has_key: true,
        commander_name: 'Danikuso',
        last_sync_time: '2026-10-30T16:05:00Z',
        last_error: null,
      },
      inara: {
        enabled: true,
        has_key: true,
        commander_name: 'Danikuso',
        inara_url: 'https://inara.cz/cmdr/12345/',
        pending_events: 3,
        last_sync_time: '2026-10-30T16:10:00Z',
        last_error: null,
      },
      capi: {
        connected: true,
        commander_name: 'Danikuso',
        remote_ships_count: 5,
        last_sync_time: '2026-10-30T16:15:00Z',
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
});
