import { t } from '../../lib/i18n';
import type { MaterialRow } from './types';
export function materialName(row: MaterialRow): string { return row.name_localised ?? row.name_en ?? row.key; }
const origins: Record<string, string> = { Mission: 'mission', Scan: 'scan', Mining: 'mining', Collected: 'collected', Trade: 'trade', Synthesis: 'synthesis', Engineering: 'engineering', Discarded: 'discarded' };
export function originLabel(origin: string): string { return origins[origin] ? t(`engineering-origin-${origins[origin]}`) : t('engineering-no-data'); }
