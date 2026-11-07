import index from '../../../crates/tmc-i18n/data/game-label-index.json';
import { t } from './i18n';

export type GameLabelKind = keyof typeof index;
const normalize = (value: string) => value.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').toLowerCase().replace(/[^a-z0-9]/g, '');
export function resolveGameLabel(kind: GameLabelKind, value: string | null | undefined): { id: string; key: string } | undefined {
  if (!value) return undefined;
  const rows: Record<string, { id: string; key: string }> = index[kind];
  const lookup = (alias: string) => Object.hasOwn(rows, alias) ? rows[alias] : undefined;
  const raw = value.trim().replace(/^\$/, '').replace(/;$/, '').replace(/_name$/i, '');
  const direct = lookup(normalize(raw));
  if (direct) return direct;
  if (kind === 'security') {
    const security = lookup(normalize(`SYSTEM_SECURITY_${raw}`));
    if (security) return security;
  }
  if (kind === 'government' && normalize(raw) === 'corporation') return rows.corporate;
  const stripped = raw.replace(new RegExp(`^(?:${kind}|SYSTEM_SECURITY|GALAXY_MAP_INFO_state)_`, 'i'), '');
  return lookup(normalize(stripped));
}
export function gameLabel(kind: GameLabelKind, value: string | null | undefined, fallback?: string | null): string {
  const resolved = resolveGameLabel(kind, value);
  return resolved ? t(resolved.key) : fallback || value || '—';
}
