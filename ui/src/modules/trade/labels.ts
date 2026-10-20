import { t } from '../../lib/i18n';
import type { PadSize, PriceOrigin } from './types';

export function padLabel(p: PadSize | null): string {
  return p === 'Small' ? t('mining-pad-small') : p === 'Medium' ? t('mining-pad-medium') : p === 'Large' ? t('mining-pad-large') : '?';
}

export function originLabel(o: PriceOrigin | null | undefined): string {
  return o === 'Local' ? t('trade-origin-local') : o === 'Eddn' ? t('trade-origin-eddn') : o === 'Ardent' ? t('trade-origin-ardent') : '';
}
