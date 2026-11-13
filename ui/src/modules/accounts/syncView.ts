import { relativeAge } from '../../lib/format';
import { t } from '../../lib/i18n';
import type { ErrorClass, SyncReason, SyncServiceStatus, SyncState } from './types';

export const KEY_PAGES = {
  inara: 'https://inara.cz/elite/cmdr-settings-api/',
  edsm: 'https://www.edsm.net/en/settings/api',
} as const;

export const SERVICE_NAMES = {
  eddn: 'EDDN',
  edsm: 'EDSM',
  inara: 'Inara',
} as const;

export type KeyedService = keyof typeof KEY_PAGES;

export function stateIcon(state: SyncState | undefined): string {
  switch (state) {
    case 'connected':
      return '✓';
    case 'waiting':
      return '⏳';
    case 'error':
      return '✕';
    case 'paused':
      return '⏸';
    case 'unavailable':
      return '⊘';
    default:
      return '○';
  }
}

export function stateBadgeClass(state: SyncState | undefined): string {
  switch (state) {
    case 'connected':
      return 'badge-green';
    case 'error':
      return 'badge-red';
    case 'waiting':
    case 'paused':
      return 'badge-amber';
    default:
      return 'badge-muted';
  }
}

export function reasonText(reason: SyncReason | null | undefined): string | null {
  if (!reason) return null;
  const kebab = reason.replaceAll('_', '-');
  return t(`accounts-sync-reason-${kebab}`);
}

export function errorClassText(cls: ErrorClass | string): string {
  const kebab = String(cls).replaceAll('_', '-');
  return t(`accounts-sync-class-${kebab}`);
}

export function adaptiveListBox(
  rowCount: number,
  rowHeight = 36,
  maxRows = 6,
): { heightPx: number; overflowY: 'hidden' | 'auto' } {
  const maxHeight = rowHeight * maxRows;
  const rawHeight = rowCount * rowHeight;
  const heightPx = rowCount === 0 ? rowHeight : Math.min(maxHeight, rawHeight);
  return {
    heightPx,
    overflowY: rawHeight > maxHeight ? 'auto' : 'hidden',
  };
}

export function formatRetryTime(nextAttemptAt: string | null | undefined, nowMs = Date.now()): string | null {
  if (!nextAttemptAt) return null;
  const targetMs = Date.parse(nextAttemptAt);
  if (Number.isNaN(targetMs)) return null;
  const secs = Math.max(1, Math.ceil((targetMs - nowMs) / 1000));
  if (secs < 60) return `${secs} s`;
  if (secs < 3600) return `${Math.ceil(secs / 60)} min`;
  return `${Math.ceil(secs / 3600)} h`;
}

export function formatStateLabel(svc: SyncServiceStatus | null | undefined, nowMs = Date.now()): string {
  const state = svc?.state ?? 'disabled';
  if (state === 'error') {
    const retryTime = formatRetryTime(svc?.next_attempt_at, nowMs);
    if (retryTime) {
      return t('accounts-sync-retry-in', { time: retryTime });
    }
  }
  return t(`accounts-sync-state-${state}`);
}

export function formatLastConfirmed(iso: string | null | undefined, nowMs = Date.now()): string {
  if (!iso) return t('accounts-sync-never');
  return relativeAge(iso, nowMs);
}

export function formatKeyHint(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const clean = raw.replace(/^[•.\s]+/, '');
  return clean ? `••••${clean}` : null;
}

export function openSettingsAccounts(): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('tmc:open-settings', { detail: { section: 'accounts' } }));
  }
}

export async function copyKeyPage(service: KeyedService): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(KEY_PAGES[service]);
    return true;
  } catch {
    return false;
  }
}
