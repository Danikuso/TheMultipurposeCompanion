import { getLocale } from './i18n';

export function credits(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  const fmt = new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0 });
  return `${fmt.format(n)} CR`;
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
