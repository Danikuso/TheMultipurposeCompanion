import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import AccountsPanel from '../src/modules/accounts/AccountsPanel.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}

const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);

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

test('AccountsPanel renders privacy notice banner and 4 glass cards', () => {
  mountWith(AccountsPanel, {
    'accounts.status': {
      eddn: {
        enabled: true,
        connected: true,
        uploader_id: 'TMC-Danikuso',
        messages_sent_session: 12,
        last_upload_time: '2026-10-23T20:00:00Z',
        last_error: null,
      },
      edsm: {
        enabled: true,
        has_key: true,
        commander_name: 'Danikuso',
        last_sync_time: '2026-10-23T20:05:00Z',
        last_error: null,
        syncing: false,
      },
      inara: {
        enabled: true,
        has_key: true,
        commander_name: 'Danikuso',
        inara_url: 'https://inara.cz/elite/cmdr/12345/',
        verified: true,
        last_sync_time: '2026-10-23T20:10:00Z',
        last_error: null,
        pending_events: 5,
        next_slot_secs: 42,
        syncing: false,
      },
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

  // 1. Banner de privacidad
  expect(q('accounts-privacy-banner')).toBeTruthy();
  expect(q('accounts-privacy-banner')?.textContent).toContain('Privacidad innegociable');

  // 2. Tarjeta EDDN
  expect(q('accounts-eddn-card')).toBeTruthy();
  expect(q('eddn-messages-count')?.textContent?.trim()).toBe('12');
  expect(q('eddn-uploader-id')?.textContent?.trim()).toBe('TMC-Danikuso');
  expect(q('eddn-copy-btn')).toBeTruthy();

  // 3. Tarjeta EDSM
  expect(q('accounts-edsm-card')).toBeTruthy();
  expect(q('accounts-edsm-card')?.textContent).toContain('CMDR Danikuso');
  expect(q('edsm-key-input')).toBeTruthy();
  expect(q('edsm-sync-btn')).toBeTruthy();

  // 4. Tarjeta Inara
  expect(q('accounts-inara-card')).toBeTruthy();
  expect(q('inara-whitelist-badge')).toBeTruthy();
  expect(q('inara-whitelist-badge')?.textContent).toContain('Whitelist oficial');
  expect(q('inara-whitelist-badge')?.getAttribute('title')).toContain('The Multipurpose Companion (TMC)');
  expect(q('inara-rate-meter')).toBeTruthy();
  expect(q('inara-rate-meter')?.textContent).toContain('2 req / 60s');
  expect(q('inara-test-btn')).toBeTruthy();
  expect(q('inara-send-btn')).toBeTruthy();
  expect(q('inara-profile-link')).toBeTruthy();
  expect((q('inara-profile-link') as HTMLAnchorElement)?.href).toContain('inara.cz/elite/cmdr/12345/');

  // 5. Tarjeta Frontier CAPI (sin vincular)
  expect(q('accounts-capi-card')).toBeTruthy();
  expect(q('capi-link-btn')).toBeTruthy();
  expect(q('capi-link-btn')?.textContent).toContain('Vincular con Frontier Store');
  expect(q('capi-manual-toggle')).toBeTruthy();
  (q('capi-manual-toggle') as HTMLButtonElement)?.click();
  flushSync();
  expect(q('capi-manual-input')).toBeTruthy();
  expect(q('capi-manual-submit')).toBeTruthy();
});

test('AccountsPanel renders connected CAPI status with remote fleet list', () => {
  mountWith(AccountsPanel, {
    'accounts.status': {
      eddn: { enabled: true, connected: true, uploader_id: 'TMC-Anon', messages_sent_session: 0, last_upload_time: null, last_error: null },
      edsm: { enabled: false, has_key: false, commander_name: null, last_sync_time: null, last_error: null, syncing: false },
      inara: { enabled: false, has_key: false, commander_name: null, last_sync_time: null, last_error: null, pending_events: 0, next_slot_secs: 0, syncing: false },
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
});

test('AccountsPanel remote fleet dropdown selector and accordion work compactly by default', () => {
  mountWith(AccountsPanel, {
    'accounts.status': {
      eddn: { enabled: true, connected: true, uploader_id: 'TMC-Anon', messages_sent_session: 0, last_upload_time: null, last_error: null },
      edsm: { enabled: false, has_key: false, commander_name: null, last_sync_time: null, last_error: null, syncing: false },
      inara: { enabled: false, has_key: false, commander_name: null, last_sync_time: null, last_error: null, pending_events: 0, next_slot_secs: 0, syncing: false },
      capi: {
        configured: true,
        connected: true,
        commander_name: 'Danikuso',
        token_expires_at: '2026-10-23T22:00:00Z',
        remote_ships_count: 0,
        last_sync_time: '2026-10-23T20:15:00Z',
        last_error: null,
      },
    },
  });

  // When remote ships empty, no selector
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
      eddn: { enabled: true, connected: true, uploader_id: 'TMC-Anon', messages_sent_session: 0, last_upload_time: null, last_error: null },
      edsm: { enabled: false, has_key: false, commander_name: null, last_sync_time: null, last_error: null, syncing: false },
      inara: { enabled: false, has_key: false, commander_name: null, last_sync_time: null, last_error: null, pending_events: 0, next_slot_secs: 0, syncing: false },
      capi: { configured: false, connected: false, commander_name: null, token_expires_at: null, remote_ships_count: 0, last_sync_time: null, last_error: null },
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
