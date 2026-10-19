import { skewFrom } from './format';
import { topic } from './ws.svelte';

/**
 * Desfase entre el reloj del diario y el local, en ms. Usa `core.clock` (el
 * instante más reciente del diario, también en el snapshot de una reconexión) y,
 * si no existe, el `as_of` propio del topic. Reactivo: léelo dentro de un efecto.
 */
export function journalSkew(fallbackAsOf?: string | null): number | null {
  const clock = topic<{ as_of?: string }>('core.clock');
  return skewFrom(clock?.as_of) ?? skewFrom(fallbackAsOf);
}
