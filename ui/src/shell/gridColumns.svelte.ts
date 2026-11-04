import { columnsForWidth } from './layoutModel';

const BREAKPOINTS = [900, 1600, 2400] as const;

/** Columnas actuales de la rejilla (mismos puntos de corte que el CSS de Grid.svelte). */
export const grid = $state({ columns: 1 as 1 | 2 | 3 | 4 });

/**
 * Sigue las columnas de la rejilla. La fuente de verdad son los MediaQueryList de los mismos puntos de
 * corte que el CSS, leídos de nuevo cada vez que el ResizeObserver avisa de un cambio de tamaño del
 * elemento de la rejilla (los eventos `change` de matchMedia no siempre llegan al redimensionar).
 * Basado en eventos, sin sondeo. Devuelve la limpieza.
 */
export function startGridColumns(el?: HTMLElement | null): () => void {
  if (typeof window === 'undefined') return () => {};
  if (typeof window.matchMedia !== 'function') {
    grid.columns = columnsForWidth(window.innerWidth);
    return () => {};
  }
  const queries = BREAKPOINTS.map((px) => window.matchMedia(`(min-width: ${px}px)`));
  const update = () => {
    const matched = BREAKPOINTS.filter((_, i) => queries[i].matches);
    const next = columnsForWidth(matched.length > 0 ? matched[matched.length - 1] : 0);
    if (grid.columns !== next) grid.columns = next;
  };
  update();
  for (const q of queries) q.addEventListener('change', update);
  const ro = el && typeof ResizeObserver !== 'undefined' ? new ResizeObserver(update) : null;
  if (el) ro?.observe(el);
  return () => {
    for (const q of queries) q.removeEventListener('change', update);
    ro?.disconnect();
  };
}
