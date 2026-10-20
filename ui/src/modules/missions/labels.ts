import { t } from '../../lib/i18n';
import { integer } from '../../lib/format';
import type { Mission, MissionKind } from './types';

export function kindLabel(k: MissionKind | null | undefined): string {
  switch (k) {
    case 'Massacre': return t('missions-kind-massacre');
    case 'Sightseeing': return t('missions-kind-sightseeing');
    case 'Passenger': return t('missions-kind-passenger');
    case 'Delivery': return t('missions-kind-delivery');
    case 'Collect': return t('missions-kind-collect');
    case 'Courier': return t('missions-kind-courier');
    default: return t('missions-kind-other');
  }
}

/** Progreso legible de una misión; `—` si no hay forma de medirlo. */
export function progressLabel(m: Mission): string {
  const p = m.progress;
  const item = m.commodity ? ` ${m.commodity}` : '';
  if (m.kind === 'Delivery' && p.total) return t('missions-progress', { done: p.delivered, total: p.total }) + item;
  if (m.kind === 'Collect' && p.total) return t('missions-progress', { done: p.collected, total: p.total }) + item;
  if ((m.kind === 'Passenger' || m.kind === 'Sightseeing') && p.passengers != null) {
    return t('missions-passenger-count', { count: integer(p.passengers) });
  }
  return '—';
}
