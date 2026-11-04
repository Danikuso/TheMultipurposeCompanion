// Controlador de arrastre con puntero (ratón y táctil) sin Svelte. Sin bucles continuos: un único
// requestAnimationFrame pendiente como máximo y solo durante un arrastre; cero listeners en reposo.
import type { CardWidth } from './cards';
import { dropIndex, snapResize } from './layoutModel';

export interface DragSession { cancel(): void; readonly active: boolean }
type Next = { w: CardWidth; h: number | null };

let current: DragSession | null = null;

export function activeDrag(): DragSession | null {
  return current;
}

interface Point { x: number; y: number }

/** Núcleo común: captura el puntero, coalesce los movimientos a un frame y limpia todo al terminar. */
function begin(
  e: PointerEvent, handle: HTMLElement,
  preview: (p: Point) => void, drop: (p: Point) => void, oncancel?: () => void,
): DragSession {
  current?.cancel();
  let active = true;
  let last: Point = { x: e.clientX, y: e.clientY };
  let frame: number | null = null;
  const pointerId = e.pointerId;

  const onMove = (ev: PointerEvent) => {
    last = { x: ev.clientX, y: ev.clientY };
    if (frame !== null) return;
    frame = requestAnimationFrame(() => {
      frame = null;
      if (active) preview(last);
    });
  };
  const onUp = (ev: PointerEvent) => {
    const p = { x: ev.clientX, y: ev.clientY };
    end(true);
    drop(p);
  };
  const onCancel = () => end();
  const cancel = () => end();

  function end(dropped = false) {
    if (!active) return;
    active = false;
    if (frame !== null) cancelAnimationFrame(frame);
    frame = null;
    handle.removeEventListener('pointermove', onMove);
    handle.removeEventListener('pointerup', onUp);
    handle.removeEventListener('pointercancel', onCancel);
    try { handle.releasePointerCapture?.(pointerId); } catch { /* el puntero ya no existe */ }
    if (current === session) current = null;
    if (!dropped) oncancel?.(); // aborto: quien pintó la vista previa debe limpiarla
  }

  const session: DragSession = {
    cancel,
    get active() { return active; },
  };
  try { handle.setPointerCapture?.(pointerId); } catch { /* puntero sintético o ya liberado */ }
  handle.addEventListener('pointermove', onMove);
  handle.addEventListener('pointerup', onUp);
  handle.addEventListener('pointercancel', onCancel);
  current = session;
  return session;
}

export function startMoveDrag(
  e: PointerEvent, handle: HTMLElement, frames: HTMLElement[], from: number,
  onpreview: (dx: number, dy: number, target: number) => void, ondrop: (target: number) => void,
  oncancel?: () => void,
): DragSession {
  const rects = frames.map((f) => f.getBoundingClientRect()); // una sola vez, al empezar
  const x0 = e.clientX;
  const y0 = e.clientY;
  return begin(
    e, handle,
    (p) => onpreview(p.x - x0, p.y - y0, dropIndex(p.x, p.y, rects, from)),
    (p) => ondrop(dropIndex(p.x, p.y, rects, from)),
    oncancel,
  );
}

export function startResizeDrag(
  e: PointerEvent, handle: HTMLElement,
  start: { w: CardWidth; h: number | null; heightPx: number; widthPx: number },
  geometry: { colWidthPx: number; gapPx: number; columns: number },
  onpreview: (next: Next) => void, ondrop: (next: Next) => void,
  oncancel?: () => void,
): DragSession {
  const x0 = e.clientX;
  const y0 = e.clientY;
  const next = (p: Point) => snapResize(start, p.x - x0, p.y - y0, geometry.colWidthPx, geometry.gapPx, geometry.columns);
  return begin(e, handle, (p) => onpreview(next(p)), (p) => ondrop(next(p)), oncancel);
}
