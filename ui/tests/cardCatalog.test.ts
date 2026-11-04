import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import CardCatalog from '../src/shell/CardCatalog.svelte';
import EditToolbar from '../src/shell/EditToolbar.svelte';
import { cards, catalogFor, factoryLayout } from '../src/shell/cards';
import { emptyDoc, type LayoutDoc } from '../src/shell/layoutModel';
import { beginEdit, cancelEdit, customTab, layoutState, themedTab } from '../src/shell/layoutStore.svelte';

const ENG = 'layout:engineering';
const CUSTOM = 'custom:c1';
let target: HTMLElement;
let app: Record<string, unknown> | null = null;
let closed = 0;

function startEdit(doc: LayoutDoc) {
  layoutState.doc = doc;
  beginEdit();
}
const open = (tabKey: string) => {
  app = mount(CardCatalog, { target, props: { tabKey, onclose: () => { closed += 1; } } });
  flushSync();
};
const entries = () => Array.from(target.querySelectorAll('[data-catalog-entry]')) as HTMLButtonElement[];
const ids = () => entries().map((e) => e.getAttribute('data-catalog-entry'));
const customDoc = (cardIds: string[]): LayoutDoc => ({
  ...emptyDoc(),
  custom: [{ id: 'c1', name: 'X', icon: 'a', cards: cardIds.map((id) => ({ id, w: 1, h: null })) }],
  customOrder: ['c1'],
});

beforeEach(() => {
  setLocale('es');
  closed = 0;
  target = document.createElement('div');
  document.body.appendChild(target);
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  cancelEdit();
  layoutState.doc = emptyDoc();
  target.remove();
});

describe('catalogFor', () => {
  test('themed catalog lists only removed factory cards', () => {
    const current = factoryLayout(ENG).filter((p) => p.id !== 'sources');
    const list = catalogFor(ENG, current, ['sources']);
    expect(list.map((e) => e.card.id)).toEqual(['sources']);
    expect(list[0].disabled).toBe(false);
  });

  test('themed card missing from current but not removed appears', () => {
    const current = factoryLayout(ENG).filter((p) => p.id !== 'trades');
    expect(catalogFor(ENG, current, []).map((e) => e.card.id)).toEqual(['trades']);
  });

  test('themed card both removed and missing appears once', () => {
    const current = factoryLayout(ENG).filter((p) => p.id !== 'sources');
    expect(catalogFor(ENG, current, ['sources', 'sources']).map((e) => e.card.id)).toEqual(['sources']);
  });

  test('custom catalog lists all cards and disables present', () => {
    const list = catalogFor(CUSTOM, [{ id: 'ax', w: 1, h: null }], []);
    expect(list).toHaveLength(Object.keys(cards).length);
    expect(list.filter((e) => e.disabled).map((e) => e.card.id)).toEqual(['ax']);
  });
});

describe('CardCatalog', () => {
  test('themed catalog empty message', () => {
    startEdit(emptyDoc());
    open(ENG);
    expect(entries()).toHaveLength(0);
    expect(target.querySelector('[data-testid="catalog-empty"]')?.textContent).toContain('Todas las tarjetas');
    expect(target.querySelector('input[type="search"]')).toBeNull();
  });

  test('themed catalog shows removed cards and focuses the first', () => {
    startEdit({ ...emptyDoc(), tabs: { [ENG]: { cards: factoryLayout(ENG).filter((p) => p.id !== 'sources'), removed: ['sources'] } } });
    open(ENG);
    expect(ids()).toEqual(['sources']);
    expect(document.activeElement).toBe(entries()[0]);
  });

  test('search filters by translated title', () => {
    startEdit(customDoc([]));
    open(CUSTOM);
    const input = target.querySelector('input[type="search"]') as HTMLInputElement;
    expect(document.activeElement).toBe(input);
    input.value = 'MATERIALES';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(ids()).toContain('materials');
    expect(ids()).not.toContain('ax');
    expect(entries().length).toBeLessThan(Object.keys(cards).length);
  });

  test('adding appends at end with default width', () => {
    startEdit(customDoc(['ax']));
    open(CUSTOM);
    expect(entries().find((e) => e.getAttribute('data-catalog-entry') === 'ax')!.disabled).toBe(true);
    entries().find((e) => e.getAttribute('data-catalog-entry') === 'route')!.click();
    flushSync();
    expect(customTab('c1')!.cards.map((c) => [c.id, c.w])).toEqual([['ax', 1], ['route', 'full']]);
    expect(closed).toBe(1);
  });

  test('adding to a themed tab restores the removed card', () => {
    startEdit({ ...emptyDoc(), tabs: { [ENG]: { cards: factoryLayout(ENG).filter((p) => p.id !== 'sources'), removed: ['sources'] } } });
    open(ENG);
    entries()[0].click();
    flushSync();
    const tab = themedTab(ENG);
    expect(tab.removed).toEqual([]);
    expect(tab.cards.at(-1)).toEqual({ id: 'sources', w: 1, h: null });
  });

  test('backdrop pointerdown closes, pointerdown inside dialog does not', () => {
    startEdit(customDoc([]));
    open(CUSTOM);
    const down = (el: Element) => el.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    down(target.querySelector('[role="dialog"]')!);
    down(target.querySelector('input[type="search"]')!);
    expect(closed).toBe(0);
    down(target.querySelector('[data-testid="catalog-backdrop"]')!);
    expect(closed).toBe(1);
  });

  test('search without matches shows a no-results message', () => {
    startEdit(customDoc([]));
    open(CUSTOM);
    const input = target.querySelector('input[type="search"]') as HTMLInputElement;
    input.value = 'zzzzqq';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    expect(entries()).toHaveLength(0);
    expect(target.querySelector('[data-testid="catalog-no-results"]')).not.toBeNull();
    expect(target.querySelector('[data-testid="catalog-empty"]')).toBeNull();
  });

  test('entry names carry a title for truncation', () => {
    startEdit(customDoc([]));
    open(CUSTOM);
    const name = entries()[0].querySelector('.catalog-name') as HTMLElement;
    expect(name.getAttribute('title')).toBe(name.textContent);
  });

  test('Tab and Shift+Tab cycle inside the dialog', () => {
    startEdit(customDoc([]));
    open(CUSTOM);
    const focusable = Array.from(target.querySelectorAll('button:not(:disabled), input')) as HTMLElement[];
    const first = focusable[0];
    const last = focusable.at(-1)!;
    const press = (shiftKey: boolean) => {
      const ev = new KeyboardEvent('keydown', { key: 'Tab', shiftKey, bubbles: true, cancelable: true });
      document.activeElement!.dispatchEvent(ev);
      return ev.defaultPrevented;
    };
    last.focus();
    expect(press(false)).toBe(true);
    expect(document.activeElement).toBe(first);
    expect(press(true)).toBe(true);
    expect(document.activeElement).toBe(last);
  });

  test('list height is capped by the viewport and scrolls only when needed', () => {
    const prev = window.innerHeight;
    window.innerHeight = 375;
    try {
      startEdit(customDoc([]));
      open(CUSTOM);
      const list = target.querySelector('.catalog-list') as HTMLElement;
      expect(list.style.height).toBe('225px');
      expect(list.style.overflowY).toBe('auto');
      const input = target.querySelector('input[type="search"]') as HTMLInputElement;
      input.value = 'materiales';
      input.dispatchEvent(new Event('input', { bubbles: true }));
      flushSync();
      expect(list.style.overflowY).toBe('hidden');
      expect(parseInt(list.style.height)).toBeLessThanOrEqual(225);
    } finally {
      window.innerHeight = prev;
    }
  });
});

describe('EditToolbar integration', () => {
  test('Esc closes the catalog without cancelling the edit; focus returns to the button', () => {
    startEdit(customDoc([]));
    const esc = () => document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    const winEsc: EventListener = () => cancelEdit(); // simula el listener de App
    window.addEventListener('keydown', winEsc);
    try {
      app = mount(EditToolbar, { target, props: { tabKey: CUSTOM } });
      const add = target.querySelector('[data-testid="layout-add"]') as HTMLButtonElement;
      add.click();
      flushSync();
      expect(target.querySelector('[role="dialog"]')).not.toBeNull();
      esc();
      flushSync();
      expect(target.querySelector('[role="dialog"]')).toBeNull();
      expect(layoutState.editing).toBe(true);
      expect(document.activeElement).toBe(add);
      esc();
      expect(layoutState.editing).toBe(false);
    } finally {
      window.removeEventListener('keydown', winEsc);
    }
  });
});
