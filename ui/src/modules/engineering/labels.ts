import materialCatalog from '../../../../crates/tmc-modules/data/materials.json';
import { decimal } from '../../lib/format';
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
  const fluentKey = `bp-${key.toLowerCase()}`;
  const name = t(fluentKey);
  return {
    name: name === fluentKey ? (english ?? key) : name,
    pending: name === fluentKey || translationPending(fluentKey),
  };
}

export function engineeringDataLabel(prefix: 'exp' | 'modtype', key: string): { name: string; pending: boolean } {
  const fluentKey = `${prefix}-${key.toLowerCase()}`;
  const name = t(fluentKey);
  return { name: name === fluentKey ? key : name, pending: name === fluentKey || translationPending(fluentKey) };
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

const materialEnglishNames = new Map(materialCatalog.entries.map(row => [row.key,row.name_en]));
export function inventoryMaterial(key: string, inventory: import('./types').InventoryTopic | undefined, english?: string | null): string {
 for (const grades of Object.values(inventory?.by_category ?? {})) for (const rows of Object.values(grades ?? {})) {
  const row = rows.find(row => row.key === key);
  if (row) return materialName(row);
 }
 return english ?? materialEnglishNames.get(key) ?? t('engineering-unknown-material',{name:key});
}
export function categoryLabel(category: string): string {
 const keys: Record<string,string> = {Raw:'raw',Manufactured:'manufactured',Encoded:'encoded'};
 return keys[category] ? t(`engineering-tab-${keys[category]}`) : t('engineering-no-data');
}
export function sourceLabel(status: import('./types').SourceStatus | undefined): string {
 if (status && typeof status === 'object' && 'Ok' in status) {
  const station=status.Ok;
  const pad: Record<string,string> = {Small:'small',Medium:'medium',Large:'large'};
  return `${station.station_name} (${station.system_name}, ${t('engineering-distance-ly',{distance:decimal(station.distance_ly)})}) · ${t('engineering-source-pad')} ${t(`mining-pad-${pad[station.max_pad ?? ''] ?? 'unknown'}`)}`;
 }
 if (status === 'Loading') return t('engineering-source-loading');
 if (status === 'Disabled') return `${t('engineering-no-data')} · ${t('engineering-source-disabled')}`;
 return t('engineering-no-data');
}
export function goalLabel(id: string): string {
 const parts=id.split(':');
 if (parts.length !== 5) return t('engineering-no-data');
 const label=blueprintLabel(parts[2]);
 return `${label.name} ${t('engineering-grade',{grade:parts[4]})}${label.pending ? ` (${t('engineering-translation-pending')})` : ''}`;
}
/** Motivo de un cálculo incompleto o de una receta no disponible, siempre localizado. */
export function blockerLabel(blocker: import('./types').PlanBlocker | string): string {
 if (typeof blocker === 'object' && blocker !== null && 'missing_recipe' in blocker) return t('engineering-blocker-missing-recipe', { grade: String(blocker.missing_recipe.grade) });
 const known = ['missing_base_mapping', 'base_required', 'ambiguous_recipe', 'incompatible_item', 'unknown_blueprint', 'unknown_experimental'];
 return typeof blocker === 'string' && known.includes(blocker) ? t(`engineering-blocker-${blocker.replaceAll('_', '-')}`) : t('engineering-no-data');
}
