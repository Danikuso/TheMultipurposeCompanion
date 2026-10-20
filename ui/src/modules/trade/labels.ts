import { getLocale, t } from '../../lib/i18n';
import type { PadSize, PriceOrigin } from './types';

export function padLabel(p: PadSize | null): string {
  return p === 'Small' ? t('mining-pad-small') : p === 'Medium' ? t('mining-pad-medium') : p === 'Large' ? t('mining-pad-large') : '?';
}

export function originLabel(o: PriceOrigin | null | undefined): string {
  return o === 'Local' ? t('trade-origin-local') : o === 'Eddn' ? t('trade-origin-eddn') : o === 'Ardent' ? t('trade-origin-ardent') : '';
}

/** Créditos con separador de miles siempre (es no agrupa los números de 4 cifras por defecto). */
export function cr(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return `${new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0, useGrouping: 'always' as unknown as boolean }).format(n)} CR`;
}
