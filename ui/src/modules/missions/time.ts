import { ageShort } from '../../lib/format';

export const SOON_MS = 3_600_000;

/** Quedan como máximo 60 min (o ya caducó): se muestra el aviso aunque no lleguen eventos. */
export function isExpiringSoon(expiry: string | null | undefined, nowMs: number | null): boolean {
  const ts = expiry ? Date.parse(expiry) : NaN;
  if (Number.isNaN(ts) || nowMs == null) return false;
  return ts - nowMs <= SOON_MS;
}

/** Cuenta atrás: `mm:ss` por debajo de 1 h; si no, la unidad mayor ("2 d", "5 h"). */
export function countdown(expiry: string | null | undefined, nowMs: number | null): string {
  const ts = expiry ? Date.parse(expiry) : NaN;
  if (Number.isNaN(ts) || nowMs == null) return '—';
  const secs = Math.max(0, Math.floor((ts - nowMs) / 1000));
  if (secs < 3600) {
    return `${String(Math.floor(secs / 60)).padStart(2, '0')}:${String(secs % 60).padStart(2, '0')}`;
  }
  return ageShort(secs);
}
