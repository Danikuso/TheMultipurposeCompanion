import { journalSkew } from '../../lib/clock.svelte';
import { SOON_MS } from './time';

/**
 * Reloj anclado al diario para las cuentas atrás. Solo hay temporizador si existe
 * alguna caducidad: 1 s con alguna a menos de 1 h por vencer; 30 s en el resto.
 * Debe llamarse durante la inicialización de un componente.
 */
export function createNow(asOf: () => string | null | undefined, expiries: () => (string | null | undefined)[]) {
  let tick = $state(Date.now());
  let skew = $state<number | null>(null);
  const times = $derived(
    expiries()
      .map((e) => (e ? Date.parse(e) : NaN))
      .filter((n) => !Number.isNaN(n)),
  );
  const has = $derived(times.length > 0);
  const nowMs = $derived(skew == null ? null : tick + skew);
  const fast = $derived(nowMs != null && times.some((x) => x - nowMs > 0 && x - nowMs <= SOON_MS));

  $effect(() => {
    if (!has) return;
    skew = journalSkew(asOf());
    tick = Date.now();
  });
  $effect(() => {
    if (!has) return;
    const id = setInterval(
      () => {
        tick = Date.now();
      },
      fast ? 1000 : 30_000,
    );
    return () => clearInterval(id);
  });

  return {
    get ms() {
      return nowMs;
    },
  };
}
