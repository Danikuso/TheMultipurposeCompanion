import { columnsForWidth } from './layoutModel';

const BREAKPOINTS = [900, 1600, 2400] as const;

/** Columnas actuales de la rejilla (mismos puntos de corte que el CSS de Grid.svelte). */
export const grid = $state({ columns: 1 as 1 | 2 | 3 | 4 });

/** Sigue los puntos de corte con matchMedia (sin listeners de resize); devuelve la limpieza. */
export function startGridColumns(): () => void {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    grid.columns = columnsForWidth(typeof window === 'undefined' ? 0 : window.innerWidth);
    return () => {};
  }
  const queries = BREAKPOINTS.map((px) => window.matchMedia(`(min-width: ${px}px)`));
  const update = () => {
    const matched = BREAKPOINTS.filter((_, i) => queries[i].matches);
    grid.columns = columnsForWidth(matched.length > 0 ? matched[matched.length - 1] : 0);
  };
  update();
  for (const q of queries) q.addEventListener('change', update);
  return () => {
    for (const q of queries) q.removeEventListener('change', update);
  };
}
