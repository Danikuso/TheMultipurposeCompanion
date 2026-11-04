import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { grid, startGridColumns } from '../src/shell/gridColumns.svelte';

// Simula el viewport: matchMedia refleja el ancho actual y ResizeObserver avisa del cambio de tamaño de la rejilla.
let width = 1920;
let roCallbacks: (() => void)[];

beforeEach(() => {
  roCallbacks = [];
  vi.stubGlobal('matchMedia', (q: string) => {
    const min = Number(/min-width:\s*(\d+)px/.exec(q)![1]);
    return {
      get matches() { return width >= min; },
      addEventListener: () => {},
      removeEventListener: () => {},
    };
  });
  vi.stubGlobal('ResizeObserver', class {
    cb: () => void;
    constructor(cb: () => void) { this.cb = cb; roCallbacks.push(cb); }
    observe() {}
    disconnect() { roCallbacks = roCallbacks.filter((c) => c !== this.cb); }
  });
});
afterEach(() => vi.unstubAllGlobals());

const resize = (px: number) => {
  width = px;
  for (const cb of roCallbacks) cb();
};

test('grid columns follow the grid element size without relying on change events', () => {
  width = 375;
  const stop = startGridColumns(document.createElement('main'));
  expect(grid.columns).toBe(1);
  resize(1280);
  expect(grid.columns).toBe(2);
  resize(1920);
  expect(grid.columns).toBe(3);
  resize(375);
  expect(grid.columns).toBe(1);
  resize(2560);
  expect(grid.columns).toBe(4);
  stop();
  resize(375);
  expect(grid.columns).toBe(4);
});
