// Geometría del tablero de tarjetas (px). Mismo `gap` que `.hud-grid`: 0.85 rem.
export const BOARD_GAP_REM = 0.85;
/** Alto de reserva de un marco aún sin medir (y mínimo de alto manual). */
export const FALLBACK_HEIGHT = 160;

/** Separación entre tarjetas: 0.85 × font-size del elemento raíz (16 px si no se puede leer). */
export function boardGapPx(): number {
  const fs = typeof document === 'undefined' ? NaN : parseFloat(getComputedStyle(document.documentElement).fontSize);
  return BOARD_GAP_REM * (Number.isFinite(fs) && fs > 0 ? fs : 16);
}

export function columnWidthPx(boardWidth: number, columns: number, gapPx: number): number {
  return (boardWidth - gapPx * (columns - 1)) / columns;
}

export function spanWidthPx(span: number, colWidth: number, gapPx: number): number {
  return span * colWidth + (span - 1) * gapPx;
}
