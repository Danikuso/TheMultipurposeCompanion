<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import { modules } from '../modules/registry';
  import { catalogFor } from './cards';
  import { addCards, customTab, themedTab } from './layoutStore.svelte';

  let { tabKey, onclose }: { tabKey: string; onclose: () => void } = $props();

  const ROW = 44;
  const HEAD = 28;
  const MIN_H = 88;
  const MAX_H = 360;

  const isCustom = $derived(tabKey.startsWith('custom:'));
  let query = $state('');
  let searchEl = $state<HTMLInputElement>();
  let listEl = $state<HTMLElement>();
  let dialogEl = $state<HTMLElement>();
  let viewportH = $state(window.innerHeight);

  const moduleLabel = (id: string) => {
    const mod = modules.find((m) => m.id === id);
    return mod ? t(mod.titleKey) : id;
  };

  const all = $derived.by(() => {
    if (isCustom) return catalogFor(tabKey, customTab(tabKey.slice('custom:'.length))?.cards ?? [], []);
    const tab = themedTab(tabKey);
    return catalogFor(tabKey, tab.cards, tab.removed);
  });
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? all.filter((e) => t(e.card.titleKey).toLowerCase().includes(q)) : all;
  });
  const groups = $derived.by(() => {
    const out: { module: string; label: string; items: typeof shown }[] = [];
    for (const e of shown) {
      let g = out.find((x) => x.module === e.card.module);
      if (!g) out.push((g = { module: e.card.module, label: moduleLabel(e.card.module), items: [] }));
      g.items.push(e);
    }
    return out;
  });
  const natural = $derived(shown.length * ROW + groups.length * HEAD);
  // Única fuente del tope: el menor entre MAX_H y el 60 % del alto de ventana.
  const maxH = $derived(Math.max(MIN_H, Math.min(MAX_H, Math.floor(viewportH * 0.6))));
  const listHeight = $derived(Math.max(MIN_H, Math.min(maxH, natural)));

  // La selección no se vacía al filtrar: cuenta también las entradas ocultas.
  let selected = $state(new Set<string>());
  const count = $derived(selected.size);

  function toggle(id: string, on: boolean) {
    const next = new Set(selected);
    if (on) next.add(id);
    else next.delete(id);
    selected = next;
  }

  function addSelected() {
    const picked = all.filter((e) => !e.disabled && selected.has(e.card.id)).map((e) => e.card.id);
    if (picked.length === 0) return;
    addCards(tabKey, picked);
    onclose();
  }

  function trapTab(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !dialogEl) return;
    const items = Array.from(dialogEl.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled)'));
    if (items.length === 0) return;
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (e.shiftKey && (active === first || !dialogEl.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !dialogEl.contains(active))) {
      e.preventDefault();
      first.focus();
    }
  }

  onMount(() => {
    (searchEl ?? listEl?.querySelector<HTMLElement>('[data-catalog-entry] input:not(:disabled)'))?.focus();
    // Captura en window: Esc cierra el catálogo y no llega al listener que cancela la edición.
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.stopPropagation();
      e.preventDefault();
      onclose();
    };
    const onResize = () => (viewportH = window.innerHeight);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('resize', onResize);
    };
  });
</script>

<!-- Cierra con pointerdown sobre el propio fondo: soltar una selección encima no debe cerrarlo. -->
<div class="catalog-backdrop" role="presentation" data-testid="catalog-backdrop"
  onpointerdown={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div class="catalog-dialog" role="dialog" aria-modal="true" aria-label={t('layout-edit-catalog-title')}
    tabindex="-1" bind:this={dialogEl} onkeydown={trapTab}>
    <header class="catalog-head">
      <h3>{t('layout-edit-catalog-title')}</h3>
      <button type="button" class="catalog-close" aria-label={t('layout-edit-catalog-close')} onclick={onclose}>✕</button>
    </header>
    {#if isCustom}
      <input bind:this={searchEl} bind:value={query} type="search" class="catalog-search"
        placeholder={t('layout-edit-catalog-search')} aria-label={t('layout-edit-catalog-search')} />
    {/if}
    {#if all.length === 0}
      <p class="catalog-empty" data-testid="catalog-empty">{t('layout-edit-catalog-empty')}</p>
    {:else if shown.length === 0}
      <p class="catalog-empty" data-testid="catalog-no-results">{t('layout-edit-catalog-no-results')}</p>
    {:else}
      <div class="catalog-list" bind:this={listEl}
        style="height: {listHeight}px; overflow-y: {natural > maxH ? 'auto' : 'hidden'};">
        {#each groups as g (g.module)}
          <h4 class="catalog-group">{g.label}</h4>
          {#each g.items as e (e.card.id)}
            <label class="catalog-entry" class:present={e.disabled} data-catalog-entry={e.card.id}
              title={e.disabled ? t('layout-edit-catalog-present') : undefined}>
              <input type="checkbox" disabled={e.disabled} checked={e.disabled || selected.has(e.card.id)}
                onchange={(ev) => toggle(e.card.id, ev.currentTarget.checked)} />
              <span class="catalog-name" title={t(e.card.titleKey)}>{t(e.card.titleKey)}</span>
            </label>
          {/each}
        {/each}
      </div>
    {/if}
    <footer class="catalog-foot">
      <button type="button" class="catalog-btn" data-testid="catalog-cancel" onclick={onclose}>{t('layout-edit-catalog-cancel')}</button>
      <button type="button" class="catalog-btn primary" data-testid="catalog-add" disabled={count === 0}
        onclick={addSelected}>{t('layout-edit-catalog-add', { count })}</button>
    </footer>
  </div>
</div>

<style>
  .catalog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(4, 8, 16, 0.85);
  }

  .catalog-dialog {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    box-sizing: border-box;
    width: 100%;
    max-width: 420px;
    max-height: 100%;
    padding: 0.85rem;
    background: rgba(10, 16, 26, 0.94);
    border: 1px solid var(--hud-primary);
    border-radius: 8px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.7);
    overflow: hidden;
    contain: layout paint;
  }

  .catalog-head { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .catalog-head h3 { margin: 0; font-size: 1rem; color: var(--hud-primary); text-transform: uppercase; letter-spacing: 0.04em; }

  .catalog-close {
    flex-shrink: 0;
    padding: 0.2rem 0.5rem;
    background: transparent;
    color: var(--hud-text-muted, #7890a8);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .catalog-search {
    box-sizing: border-box;
    width: 100%;
    padding: 0.4rem 0.6rem;
    background: rgba(6, 10, 18, 0.8);
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  .catalog-empty { margin: 0; padding: 0.5rem 0; color: var(--hud-text-muted, #7890a8); }

  .catalog-list {
    min-height: 0;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 170, 0, 0.3) transparent;
  }

  .catalog-group {
    box-sizing: border-box;
    height: 28px;
    margin: 0;
    padding-top: 0.5rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--hud-text-muted, #7890a8);
  }

  .catalog-entry {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    box-sizing: border-box;
    width: 100%;
    height: 44px;
    padding: 0 0.6rem;
    color: var(--hud-text);
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
  }

  .catalog-entry input { flex-shrink: 0; width: 1rem; height: 1rem; margin: 0; accent-color: var(--hud-primary); cursor: inherit; }
  .catalog-entry:hover:not(.present), .catalog-entry:focus-within { border-color: var(--hud-primary); background: rgba(0, 195, 255, 0.08); }
  .catalog-entry.present { opacity: 0.45; cursor: default; }
  .catalog-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .catalog-foot { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 0.5rem; flex-shrink: 0; }
  .catalog-btn {
    flex: 1 1 8rem;
    padding: 0.45rem 0.8rem;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
    cursor: pointer;
  }
  .catalog-btn.primary { color: var(--hud-primary); border-color: var(--hud-primary); }
  .catalog-btn:disabled { opacity: 0.45; cursor: default; }
</style>
