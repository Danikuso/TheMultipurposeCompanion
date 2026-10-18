import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import ProspectorPanel from '../src/modules/mining/ProspectorPanel.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mat(key: string, name: string | null, proportion: number, est_value: number | null, interest = false) {
  return { key, name, proportion, est_value, interest };
}
function asteroid(extra: Record<string, unknown> = {}) {
  return {
    content: 'high', motherlode: null,
    materials: [
      mat('platinum', 'Platino', 31.5, 2079000, true),
      mat('palladium', 'Paladio', 12, 600000),
    ],
    remaining: 100, verdict: { kind: 'worth', est_value: 2079000 }, est_total: 2679000, ...extra,
  };
}
function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}
const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);

beforeEach(() => {
  disconnect(); setLocale('es');
  target = document.createElement('div'); document.body.appendChild(target);
});
afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined; target.remove(); vi.useRealTimers(); vi.unstubAllGlobals();
});

test('stacked composition widths follow proportions', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid(), history: [] } });
  const segs = [...target.querySelectorAll('[data-testid="stack-segment"]')] as HTMLElement[];
  expect(segs.map((s) => s.style.width)).toEqual(['31.5%', '12%', '56.5%']);
  expect(q('stack-rest')).not.toBeNull();
});

test('worth verdict shows approx value', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid(), history: [] } });
  const v = q('verdict')!;
  expect(v.textContent).toContain('≈');
  expect(v.textContent).toContain('2.079.000');
  expect(v.getAttribute('data-kind')).toBe('worth');
});

test('no data verdict', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid({ verdict: { kind: 'no_data' } }), history: [] } });
  expect(q('verdict')!.textContent).toContain('sin datos');
});

test('history lists 8 rows', () => {
  const history = Array.from({ length: 8 }, (_, i) => asteroid({ est_total: (i + 1) * 1000 }));
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid(), history } });
  expect(target.querySelectorAll('[data-testid="history-item"]').length).toBe(8);
});

test('material table uses localized name, marks interest and core', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: asteroid({ motherlode: 'Painita' }), history: [] } });
  const rows = target.querySelectorAll('[data-testid="material-row"]');
  expect(rows.length).toBe(2);
  expect(rows[0].textContent).toContain('Platino');
  expect(rows[0].textContent).toContain('★');
  expect(rows[0].textContent).toContain('≈');
  expect(rows[0].textContent).toContain('31,5');
  expect(q('core')!.textContent).toContain('Painita');
  expect(target.querySelector('[role="progressbar"][data-testid="remaining-bar"]')).not.toBeNull();
});

test('empty topic shows no data', () => {
  mountWith(ProspectorPanel, { 'mining.prospector': { current: null, history: [] } });
  expect(q('prospector-empty')!.textContent).toContain('sin datos');
});
