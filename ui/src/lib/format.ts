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
