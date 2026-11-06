import { describe, expect, test } from 'vitest';
import { cards, factoryLayout, isFactoryCard, tabKeyFor } from '../src/shell/cards';
import { layouts } from '../src/shell/layouts';
import { modules } from '../src/modules/registry';
import { localeKeys } from '../src/lib/i18n';

describe('card catalog', () => {
  test('every layout and module card exists in catalog', () => {
    for (const layout of layouts) {
      for (const slot of layout.slots) expect(cards[slot.card], `layout ${layout.id}: ${slot.card}`).toBeDefined();
    }
    for (const mod of modules) {
      for (const id of mod.cards) expect(cards[id], `module ${mod.id}: ${id}`).toBeDefined();
    }
  });

  test('catalog ids are unique and kebab-case', () => {
    const ids = Object.values(cards).map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const [key, def] of Object.entries(cards)) {
      expect(def.id).toBe(key);
      expect(def.id).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/);
    }
  });

  test('every titleKey exists in es locale', () => {
    const keys = new Set(localeKeys()['es']);
    for (const def of Object.values(cards)) expect(keys.has(def.titleKey), `${def.id}: ${def.titleKey}`).toBe(true);
  });
});

describe('factory layouts', () => {
  test('factoryLayout preserves current order and widths', () => {
    expect(factoryLayout('layout:engineering')).toEqual([
      { id: 'materials', w: 2, h: null },
      { id: 'engineers', w: 1, h: null },
      { id: 'engineer-guide', w: 2, h: null },
      { id: 'eng-alerts', w: 1, h: null },
      { id: 'eng-ship', w: 1, h: null },
      { id: 'eng-flow', w: 2, h: null },
      { id: 'shopping', w: 2, h: null },
      { id: 'trades', w: 1, h: null },
      { id: 'sources', w: 1, h: null },
    ]);
    expect(factoryLayout('layout:exploration').map((p) => [p.id, p.w])).toEqual([
      ['route', 'full'], ['exploration-context', 2], ['planner', 1], ['jump', 1],
    ]);
    expect(factoryLayout('module:navigation').map((p) => p.id)).toEqual(['route', 'jump', 'planner']);
    expect(factoryLayout('custom:c-1')).toEqual([]);
    expect(factoryLayout('layout:nope')).toEqual([]);
  });

  test('tabKeyFor maps tab ids', () => {
    expect(tabKeyFor('layout:combat')).toBe('layout:combat');
    expect(tabKeyFor('engineering')).toBe('module:engineering');
    expect(tabKeyFor('custom:c-1')).toBe('custom:c-1');
    expect(tabKeyFor('settings')).toBeNull();
    expect(tabKeyFor('accounts')).toBeNull();
  });

  test('isFactoryCard', () => {
    expect(isFactoryCard('layout:engineering', 'sources')).toBe(true);
    expect(isFactoryCard('layout:engineering', 'ax')).toBe(false);
  });
});
