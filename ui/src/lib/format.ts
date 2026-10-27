import { getLocale, t } from './i18n';

export function credits(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return `${integer(n)} CR`;
}

/** Entero con separador de miles siempre (es no agrupa por defecto los de 4 cifras). */
export function integer(n: number): string {
  // `'always'` es válido en runtime pero aún falta en los tipos de lib.
  const opts = { maximumFractionDigits: 0, useGrouping: 'always' } as unknown as Intl.NumberFormatOptions;
  return new Intl.NumberFormat(getLocale(), opts).format(n);
}

export function ly(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  const fmt = new Intl.NumberFormat(getLocale(), {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  return `${fmt.format(n)} ly`;
}

export function pct(x: number | null | undefined): string {
  if (x == null || Number.isNaN(x)) return '—';
  const normalized = x <= 1.0 && x >= 0.0 ? Math.round(x * 100) : Math.round(x);
  const fmt = new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0 });
  return `${fmt.format(normalized)} %`;
}

export function duration(ms: number | null | undefined): string {
  if (ms == null || Number.isNaN(ms) || ms < 0) return '0m';
  const totalMinutes = Math.floor(ms / 60000);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours > 0) {
    return `${hours}h ${minutes.toString().padStart(2, '0')}m`;
  }
  return `${minutes}m`;
}

function unitFmt(unit: 'second' | 'minute' | 'hour' | 'day', n: number): string {
  return new Intl.NumberFormat(getLocale(), {
    style: 'unit',
    unit,
    unitDisplay: 'narrow',
    maximumFractionDigits: 0,
  }).format(n);
}

export function ageShort(seconds: number | null | undefined): string {
  if (seconds == null || Number.isNaN(seconds) || seconds < 0) return unitFmt('second', 0);
  const s = Math.floor(seconds);
  if (s < 60) return unitFmt('second', s);
  if (s < 3600) return unitFmt('minute', Math.floor(s / 60));
  if (s < 86400) return unitFmt('hour', Math.floor(s / 3600));
  return unitFmt('day', Math.floor(s / 86400));
}

export function starClassLabel(cls: string | null | undefined): string {
  if (!cls) return '—';
  const base = cls.split('_')[0]?.trim().toUpperCase();
  return base ? base : '—';
}

const SCOOPABLE_CLASSES = new Set(['O', 'B', 'A', 'F', 'G', 'K', 'M']);

export function isScoopable(cls: string | null | undefined): boolean {
  if (!cls) return false;
  const base = cls.split('_')[0]?.trim().toUpperCase();
  return base ? SCOOPABLE_CLASSES.has(base) : false;
}

export function meters(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return `${new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0 }).format(n)} m`;
}

export function gravity(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return `${new Intl.NumberFormat(getLocale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(n)} g`;
}

export function kelvin(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return `${new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0 }).format(n)} K`;
}

export function num1(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return new Intl.NumberFormat(getLocale(), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(n);
}

/** Número con hasta `max` decimales (sin ceros sobrantes), según el idioma. */
export function decimal(n: number | null | undefined, max = 1): string {
  if (n == null || Number.isNaN(n)) return '—';
  return new Intl.NumberFormat(getLocale(), { minimumFractionDigits: 0, maximumFractionDigits: max }).format(n);
}

/** Anchura CSS en porcentaje (siempre con punto decimal), acotada a 0-100. */
export function cssPct(n: number | null | undefined): string {
  const v = n == null || Number.isNaN(n) ? 0 : Math.min(100, Math.max(0, n));
  return `${Math.round(v * 10) / 10}%`;
}

/**
 * Desfase entre el reloj del diario (`as_of`) y el local, en ms. `null` si falta
 * o es el instante cero (módulo sin eventos): entonces no hay base de tiempo.
 */
export function skewFrom(asOf: string | null | undefined, localMs: number = Date.now()): number | null {
  const ts = asOf ? Date.parse(asOf) : NaN;
  if (Number.isNaN(ts) || ts < Date.UTC(2000, 0, 1)) return null;
  return ts - localMs;
}

/** Tiempo relativo ("hace 2 h", "ahora") a partir de una marca ISO; `—` si no hay. */
export function relativeAge(iso: string | null | undefined, nowMs: number | null): string {
  const ts = iso ? Date.parse(iso) : NaN;
  if (Number.isNaN(ts) || nowMs == null || ts < Date.UTC(2000, 0, 1)) return '—';
  const secs = Math.max(0, Math.round((nowMs - ts) / 1000));
  if (secs < 60) return t('mining-age-now');
  const rtf = new Intl.RelativeTimeFormat(getLocale(), { numeric: 'always', style: 'narrow' });
  if (secs < 3600) return rtf.format(-Math.floor(secs / 60), 'minute');
  if (secs < 86400) return rtf.format(-Math.floor(secs / 3600), 'hour');
  return rtf.format(-Math.floor(secs / 86400), 'day');
}

export { formatShipName } from './ships';

