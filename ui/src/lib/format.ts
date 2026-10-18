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

export function ageShort(seconds: number | null | undefined): string {
  if (seconds == null || Number.isNaN(seconds) || seconds < 0) return '0 s';
  const s = Math.floor(seconds);
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86400)} d`;
}

export function starClassLabel(cls: string | null | undefined): string {
  if (!cls) return '—';
  const base = cls.split('_')[0]?.trim().toUpperCase();
  return base ? base : '—';
}

export function num1(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n)) return '—';
  return n.toFixed(1);
}
