import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { activeDrag, startMoveDrag, startResizeDrag } from '../src/shell/pointerDrag';

const rect = (left: number, top: number, width: number, height: number) =>
  ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) }) as DOMRect;

let handle: HTMLElement;
let frames: HTMLElement[];
let rafs: FrameRequestCallback[];

function ev(type: string, x: number, y: number, pointerType = 'mouse'): Event {
  const e = new Event(type, { bubbles: true });
  Object.assign(e, { clientX: x, clientY: y, pointerId: 1, pointerType });
  return e;
}
const down = (x: number, y: number, pointerType = 'mouse') => ev('pointerdown', x, y, pointerType) as PointerEvent;
const flushFrame = () => { const cb = rafs.shift(); cb?.(0); };
const START = { w: 1 as const, h: null, heightPx: 400, widthPx: 300 };
const GEO = { colWidthPx: 300, gapPx: 20, columns: 3 };

beforeEach(() => {
  handle = document.createElement('div');
  (handle as unknown as { setPointerCapture: () => void }).setPointerCapture = vi.fn();
  (handle as unknown as { releasePointerCapture: () => void }).releasePointerCapture = vi.fn();
  // Tres marcos en fila: 0..200, 200..400, 400..600 de ancho, 100 de alto.
  frames = [0, 1, 2].map((i) => {
    const el = document.createElement('div');
    el.getBoundingClientRect = vi.fn(() => rect(i * 200, 0, 200, 100));
    return el;
  });
  rafs = [];
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => { rafs.push(cb); return rafs.length; });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => { rafs.splice(id - 1, 1); });
});
afterEach(() => {
  activeDrag()?.cancel();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe.each(['mouse', 'touch'])('move drag (%s)', (kind) => {
  test('drops at computed index', () => {
    const ondrop = vi.fn();
    const s = startMoveDrag(down(50, 50, kind), handle, frames, 0, vi.fn(), ondrop);
    expect(s.active).toBe(true);
    expect(activeDrag()).toBe(s);
    handle.dispatchEvent(ev('pointermove', 260, 50, kind));
    handle.dispatchEvent(ev('pointerup', 260, 50, kind));
    expect(ondrop).toHaveBeenCalledWith(1);
    expect(s.active).toBe(false);
    expect(activeDrag()).toBeNull();
  });

  test('far target gives last index', () => {
    const ondrop = vi.fn();
    startMoveDrag(down(50, 50, kind), handle, frames, 0, vi.fn(), ondrop);
    handle.dispatchEvent(ev('pointerup', 560, 50, kind));
    expect(ondrop).toHaveBeenCalledWith(2);
  });
});

describe('pointerDrag', () => {
  test('move coalesces to one frame', () => {
    const onpreview = vi.fn();
    startMoveDrag(down(50, 50), handle, frames, 0, onpreview, vi.fn());
    for (let i = 1; i <= 5; i++) handle.dispatchEvent(ev('pointermove', 50 + i * 40, 50));
    expect(rafs).toHaveLength(1);
    flushFrame();
    expect(onpreview).toHaveBeenCalledTimes(1);
    expect(onpreview).toHaveBeenCalledWith(200, 0, 1);
  });

  test('rects are read once at start', () => {
    startMoveDrag(down(50, 50), handle, frames, 0, vi.fn(), vi.fn());
    handle.dispatchEvent(ev('pointermove', 100, 50));
    flushFrame();
    handle.dispatchEvent(ev('pointermove', 200, 50));
    flushFrame();
    for (const f of frames) expect(f.getBoundingClientRect).toHaveBeenCalledTimes(1);
  });

  test('resize drag snaps', () => {
    const ondrop = vi.fn();
    startResizeDrag(down(300, 400), handle, START, GEO, vi.fn(), ondrop);
    handle.dispatchEvent(ev('pointermove', 620, 495));
    handle.dispatchEvent(ev('pointerup', 620, 495));
    expect(ondrop).toHaveBeenCalledWith({ w: 2, h: 480 });
  });

  test('resize preview fires once per frame', () => {
    const onpreview = vi.fn();
    startResizeDrag(down(300, 400), handle, START, GEO, onpreview, vi.fn());
    for (let i = 0; i < 4; i++) handle.dispatchEvent(ev('pointermove', 620, 495));
    flushFrame();
    expect(onpreview).toHaveBeenCalledTimes(1);
    expect(onpreview).toHaveBeenCalledWith({ w: 2, h: 480 });
  });

  test('cancel aborts without drop', () => {
    const ondrop = vi.fn();
    const s = startMoveDrag(down(50, 50), handle, frames, 0, vi.fn(), ondrop);
    handle.dispatchEvent(ev('pointermove', 260, 50));
    s.cancel();
    handle.dispatchEvent(ev('pointerup', 260, 50));
    expect(ondrop).not.toHaveBeenCalled();
    expect(s.active).toBe(false);
    expect(rafs).toHaveLength(0);
  });

  test('pointercancel aborts without drop', () => {
    const ondrop = vi.fn();
    const s = startMoveDrag(down(50, 50), handle, frames, 0, vi.fn(), ondrop);
    handle.dispatchEvent(ev('pointercancel', 50, 50));
    expect(ondrop).not.toHaveBeenCalled();
    expect(s.active).toBe(false);
  });

  test('no listeners remain after drop or cancel', () => {
    const add = vi.spyOn(handle, 'addEventListener');
    const remove = vi.spyOn(handle, 'removeEventListener');
    const balanced = () => {
      const net = new Map<string, number>();
      for (const [t] of add.mock.calls) net.set(t, (net.get(t) ?? 0) + 1);
      for (const [t] of remove.mock.calls) net.set(t, (net.get(t) ?? 0) - 1);
      return [...net.values()].every((n) => n === 0);
    };
    startMoveDrag(down(50, 50), handle, frames, 0, vi.fn(), vi.fn());
    expect(add).toHaveBeenCalled();
    handle.dispatchEvent(ev('pointerup', 60, 50));
    expect(balanced()).toBe(true);
    add.mockClear();
    remove.mockClear();
    const s = startResizeDrag(down(0, 0), handle, START, GEO, vi.fn(), vi.fn());
    s.cancel();
    expect(add).toHaveBeenCalled();
    expect(balanced()).toBe(true);
  });
});
