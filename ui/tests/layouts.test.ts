import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { contextPanel, layouts } from '../src/shell/layouts';
import { ageShort, starClassLabel } from '../src/lib/format';
import { setLocale } from '../src/lib/i18n';
import Panel from '../src/shell/Panel.svelte';

describe('layouts', () => {
  test('contextPanel prefers manual, then active exobiology, else exploration', () => {
    const mods = [
      { id: 'exploration', relevance: 'normal' },
      { id: 'exobiology', relevance: 'active' },
    ];
    expect(contextPanel(mods, null)).toBe('exobiology');
    expect(contextPanel(mods, 'exploration')).toBe('exploration');
    expect(contextPanel([{ id: 'exobiology', relevance: 'urgent' }], null)).toBe('exobiology');
    expect(contextPanel([{ id: 'exobiology', relevance: 'normal' }], null)).toBe('exploration');
    expect(contextPanel([], null)).toBe('exploration');
    expect(contextPanel([], 'exobiology')).toBe('exobiology');
    expect(Array.isArray(layouts)).toBe(true);
  });

  test('format helpers', () => {
    expect(ageShort(45)).toBe('45 s');
    expect(ageShort(125)).toBe('2 min');
    expect(ageShort(7300)).toBe('2 h');
    expect(ageShort(200000)).toBe('2 d');
    expect(ageShort(-5)).toBe('0 s');
    expect(starClassLabel('K')).toBe('K');
    expect(starClassLabel('da')).toBe('DA');
    expect(starClassLabel('O_BLUE')).toBe('O');
    expect(starClassLabel(null)).toBe('—');
  });
});

describe('Panel span and size', () => {
  let target: HTMLElement;
  let inst: Record<string, unknown> | null = null;
  beforeEach(() => {
    setLocale('es');
    target = document.createElement('div');
    document.body.appendChild(target);
  });
  afterEach(() => {
    if (inst) unmount(inst);
    inst = null;
    target.remove();
  });

  test('full span panel spans all grid columns', () => {
    inst = mount(Panel, { target, props: { titleKey: 'panel-route', span: 'full', size: 'large' } });
    flushSync();
    const el = target.querySelector('section') as HTMLElement;
    expect(el.style.gridColumn).toBe('1 / -1');
    expect(el.classList.contains('large')).toBe(true);
  });

  test('numeric span uses span N', () => {
    inst = mount(Panel, { target, props: { titleKey: 'panel-route', span: 2 } });
    flushSync();
    const el = target.querySelector('section') as HTMLElement;
    expect(el.style.gridColumn).toBe('span 2');
    expect(el.classList.contains('large')).toBe(false);
  });
});
