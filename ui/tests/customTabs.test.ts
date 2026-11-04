import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setLocale, t } from '../src/lib/i18n';
import { disconnect, live } from '../src/lib/ws.svelte';
import { prefs } from '../src/lib/prefs.svelte';
import App from '../src/App.svelte';
import { cancelEdit, layoutState } from '../src/shell/layoutStore.svelte';
import { CUSTOM_TAB_ICONS, MAX_CUSTOM_TABS, NAME_MAX, emptyDoc, type LayoutDoc } from '../src/shell/layoutModel';
import { resolveNavIcon } from '../src/lib/icons';

vi.mock('../src/lib/ws.svelte', async (original) => ({
  ...(await original<typeof import('../src/lib/ws.svelte')>()),
  connect: vi.fn(),
}));

let target: HTMLElement;
let app: Record<string, unknown>;
let autoBefore = true;

const q = <T extends Element = HTMLElement>(sel: string) => target.querySelector(sel) as T | null;
const click = (el: Element | null) => { (el as HTMLElement).click(); flushSync(); };
const customBtns = () => Array.from(target.querySelectorAll('[data-custom]')) as HTMLElement[];
const activeBtn = () => q('.nav-btn.active');
const typeName = (name: string) => {
  const input = q<HTMLInputElement>('[data-testid="custom-tab-name"]')!;
  input.value = name;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
};
const docWith = (n: number): LayoutDoc => {
  const d = emptyDoc();
  for (let i = 0; i < n; i++) {
    d.custom.push({ id: `c-${i}`, name: `Tab ${i}`, icon: 'combat', cards: [] });
    d.customOrder.push(`c-${i}`);
  }
  return d;
};

async function boot(opts: { cfg?: object; doc?: LayoutDoc; placement?: object } = {}) {
  const doc = opts.doc ?? emptyDoc();
  localStorage.removeItem('tmc.ui.placement');
  if (opts.placement) localStorage.setItem('tmc.ui.placement', JSON.stringify(opts.placement));
  prefs.loaded = false;
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (url === '/api/config') return new Response(JSON.stringify(opts.cfg ?? { auto_companion: true }), { status: 200 });
    return new Response(JSON.stringify({ device_id: 'x', name: 'n', updated_at: 't', doc }), { status: 200 });
  }));
  target = document.createElement('div');
  document.body.appendChild(target);
  live.topics['core.modules'] = [{ id: 'engineering', relevance: 'active' }];
  app = mount(App, { target });
  await vi.waitFor(() => expect(layoutState.ready).toBe(true));
  await vi.waitFor(() => expect(prefs.loaded).toBe(true));
  flushSync();
}

async function createTab(name: string, icon = 'combat') {
  click(q('[data-testid="custom-tab-new"]'));
  typeName(name);
  click(q(`[data-icon="${icon}"]`));
  click(q('[data-testid="custom-tab-submit"]'));
}

beforeEach(() => {
  disconnect();
  setLocale('es');
  autoBefore = prefs.autoCompanion;
});
afterEach(() => {
  cancelEdit();
  unmount(app);
  target.remove();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  localStorage.removeItem('tmc.ui.placement');
  disconnect();
  prefs.autoCompanion = autoBefore;
});

describe('custom tabs', () => {
  test('create custom tab appears in sidebar and opens catalog', async () => {
    await boot();
    expect(customBtns()).toHaveLength(0);
    await createTab('Mi pestaña');
    expect(layoutState.editing).toBe(true);
    expect(layoutState.draft?.custom.map((c) => [c.name, c.icon])).toEqual([['Mi pestaña', 'combat']]);
    expect(customBtns()).toHaveLength(1);
    expect(customBtns()[0].textContent).toContain('Mi pestaña');
    expect(activeBtn()).toBe(customBtns()[0]);
    expect(q('[data-testid="custom-tab-dialog"]')).toBeNull();
    expect(q('[data-testid="catalog-backdrop"]')).not.toBeNull();
    expect(layoutState.doc.custom).toHaveLength(0); // solo se guarda con «Hecho»
  });

  test('cancelling the edit drops the new tab and falls back to the auto layout', async () => {
    await boot();
    await createTab('Efímera');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })); // cierra el catálogo
    flushSync();
    click(q('[data-testid="layout-cancel"]'));
    expect(customBtns()).toHaveLength(0);
    expect(activeBtn()?.hasAttribute('data-custom')).toBe(false);
  });

  test('submit is disabled for blank names and the input is length limited', async () => {
    await boot();
    click(q('[data-testid="custom-tab-new"]'));
    expect(q<HTMLInputElement>('[data-testid="custom-tab-name"]')!.maxLength).toBe(NAME_MAX);
    expect(q<HTMLButtonElement>('[data-testid="custom-tab-submit"]')!.disabled).toBe(true);
    typeName('   ');
    expect(q<HTMLButtonElement>('[data-testid="custom-tab-submit"]')!.disabled).toBe(true);
    typeName('Ok');
    expect(q<HTMLButtonElement>('[data-testid="custom-tab-submit"]')!.disabled).toBe(false);
  });

  test('Esc closes the dialog without cancelling the edit session', async () => {
    await boot({ doc: docWith(1) });
    click(q('[data-testid="layout-edit"]'));
    click(q('[data-testid="custom-tab-new"]'));
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    flushSync();
    expect(q('[data-testid="custom-tab-dialog"]')).toBeNull();
    expect(layoutState.editing).toBe(true);
  });

  test('custom tab is a manual view and survives auto-companion', async () => {
    await boot({ doc: docWith(1) });
    click(q('[data-custom="c-0"]'));
    expect(activeBtn()?.getAttribute('data-custom')).toBe('c-0');
    live.topics['core.modules'] = [{ id: 'engineering', relevance: 'active' }, { id: 'combat', relevance: 'urgent' }];
    flushSync();
    expect(activeBtn()?.getAttribute('data-custom')).toBe('c-0');
    expect(q('h1')?.textContent).toBe('Tab 0');
  });

  test('rename and delete custom tab', async () => {
    vi.stubGlobal('confirm', () => true);
    await boot({ doc: docWith(2) });
    click(q('[data-custom="c-1"]'));
    click(q('[data-testid="layout-edit"]'));
    click(q('[data-testid="custom-tab-rename"]'));
    expect(q<HTMLInputElement>('[data-testid="custom-tab-name"]')!.value).toBe('Tab 1');
    typeName('Renombrada');
    click(q('[data-testid="custom-tab-submit"]'));
    expect(q('h1')?.textContent).toBe('Renombrada');
    expect(q('[data-custom="c-1"]')!.textContent).toContain('Renombrada');
    click(q('[data-testid="custom-tab-delete"]'));
    expect(customBtns().map((b) => b.getAttribute('data-custom'))).toEqual(['c-0']);
  });

  test('delete asks for confirmation and keeps the tab when refused', async () => {
    vi.stubGlobal('confirm', () => false);
    await boot({ doc: docWith(1) });
    click(q('[data-custom="c-0"]'));
    click(q('[data-testid="layout-edit"]'));
    click(q('[data-testid="custom-tab-delete"]'));
    expect(customBtns()).toHaveLength(1);
  });

  test('deleting active custom tab falls back to auto layout', async () => {
    vi.stubGlobal('confirm', () => true);
    await boot({ doc: docWith(1) });
    click(q('[data-custom="c-0"]'));
    click(q('[data-testid="layout-edit"]'));
    click(q('[data-testid="custom-tab-delete"]'));
    expect(customBtns()).toHaveLength(0);
    expect(activeBtn()).not.toBeNull();
    expect(activeBtn()?.hasAttribute('data-custom')).toBe(false);
    expect(q('h1')?.textContent).toBe(t('layout-engineering'));
  });

  test('placement restores custom view only if it exists', async () => {
    await boot({ cfg: { auto_companion: false }, doc: docWith(1), placement: { layout: null, context: null, view: 'custom:c-0' } });
    expect(activeBtn()?.getAttribute('data-custom')).toBe('c-0');
    unmount(app);
    target.remove();
    await boot({ cfg: { auto_companion: false }, doc: docWith(1), placement: { layout: null, context: null, view: 'custom:c-ab' } });
    expect(activeBtn()).not.toBeNull();
    expect(activeBtn()?.hasAttribute('data-custom')).toBe(false);
  });

  test('empty custom tab shows edit call to action', async () => {
    await boot({ doc: docWith(1) });
    click(q('[data-custom="c-0"]'));
    expect(q('[data-testid="custom-empty"]')!.textContent).toContain(t('layout-custom-empty'));
    click(q('[data-testid="custom-empty-edit"]'));
    expect(layoutState.editing).toBe(true);
    expect(q('[data-testid="custom-empty"]')).toBeNull();
  });

  test('limit of 12 custom tabs shows warning', async () => {
    await boot({ doc: docWith(MAX_CUSTOM_TABS) });
    expect(customBtns()).toHaveLength(MAX_CUSTOM_TABS);
    await createTab('Una más');
    expect(layoutState.draft?.custom).toHaveLength(MAX_CUSTOM_TABS);
    expect(target.textContent).toContain(t('layout-custom-limit'));
  });

  test('custom tab icons exist', () => {
    expect(CUSTOM_TAB_ICONS).toHaveLength(10);
    expect(new Set(CUSTOM_TAB_ICONS).size).toBe(10);
    for (const id of CUSTOM_TAB_ICONS) expect(resolveNavIcon(id), id).not.toBeNull();
  });
});
