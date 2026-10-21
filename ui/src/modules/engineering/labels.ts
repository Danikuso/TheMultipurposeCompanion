import { t, translationPending } from '../../lib/i18n';
import type { MaterialRow } from './types';
export function materialName(row: MaterialRow): string { return row.name_localised ?? row.name_en ?? row.key; }
const origins: Record<string, string> = { Mission: 'mission', Scan: 'scan', Mining: 'mining', Collected: 'collected', Trade: 'trade', Synthesis: 'synthesis', Engineering: 'engineering', Discarded: 'discarded' };
export function originLabel(origin: string): string { return origins[origin] ? t(`engineering-origin-${origins[origin]}`) : t('engineering-no-data'); }

const engineerStatuses: Record<string, string> = {
  Known: 'known', Invited: 'invited', Acquainted: 'acquainted',
  Unlocked: 'unlocked', Barred: 'barred', Unknown: 'unknown',
};

export function engineerStatusLabel(status: string): string {
  return t(`engineering-engineer-${engineerStatuses[status] ?? 'unknown'}`);
}

export function blueprintLabel(key: string, english?: string): { name: string; pending: boolean } {
  const fluentKey = `bp-${key}`;
  const name = t(fluentKey);
  return {
    name: name === fluentKey ? (english ?? key) : name,
    pending: name === fluentKey || translationPending(fluentKey),
  };
}

export function contributionCommodityLabel(key: string, journalName?: string | null): { name: string; pending: boolean } {
  if (journalName) return { name: journalName, pending: false };
  const fluentKey = `engineering-commodity-${key}`;
  const name = t(fluentKey);
  return {
    name: name === fluentKey ? key : name,
    pending: name === fluentKey || translationPending(fluentKey),
  };
}
