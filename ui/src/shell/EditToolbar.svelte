<script lang="ts">
  import type { Snippet } from 'svelte';
  import { diag } from '../lib/diag.svelte';
  import { askConfirm } from '../lib/confirm';
  import { t } from '../lib/i18n';
  import { commitEdit, layoutState, resetTab } from './layoutStore.svelte';
  import { cancelEditWithDiag, editTab } from './editDiag';
  import { pushToast } from './Toasts.svelte';
  import CardCatalog from './CardCatalog.svelte';

  let { tabKey, extra }: { tabKey: string; extra?: Snippet } = $props();

  let catalogOpen = $state(false);
  let addBtn = $state<HTMLButtonElement>();

  function closeCatalog() {
    catalogOpen = false;
    addBtn?.focus();
  }

  // Petición de abrir el catálogo (p. ej. tras crear una pestaña): se consume al verla.
  $effect(() => {
    if (!layoutState.catalogRequested) return;
    layoutState.catalogRequested = false;
    catalogOpen = true;
  });

  async function done() {
    const ok = await commitEdit();
    diag('ui.layout.edit', { commit: true, ok, ...editTab(tabKey) });
    if (!ok) pushToast(t('layout-edit-save-error'), 'Warning', 'layout-save-error');
  }

  function cancel() {
    cancelEditWithDiag(tabKey);
  }

  function reset() {
    if (askConfirm(t('layout-edit-reset-confirm'))) resetTab(tabKey);
  }
</script>

<div class="edit-toolbar" role="toolbar" aria-label={t('layout-edit-toolbar')} data-testid="edit-toolbar">
  <button type="button" class="primary" data-testid="layout-done" disabled={layoutState.saving} onclick={done}>{t('layout-edit-done')}</button>
  <button type="button" data-testid="layout-cancel" disabled={layoutState.saving} onclick={cancel}>{t('layout-edit-cancel')}</button>
  <button type="button" data-testid="layout-add" bind:this={addBtn} disabled={layoutState.saving} onclick={() => (catalogOpen = true)}>{t('layout-edit-add')}</button>
  <button type="button" data-testid="layout-reset" disabled={layoutState.saving} onclick={reset}>{t('layout-edit-reset')}</button>
  {@render extra?.()}
</div>

{#if catalogOpen}
  <CardCatalog {tabKey} onclose={closeCatalog} />
{/if}

<style>
  .edit-toolbar {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    padding: 0.4rem;
    background: var(--hud-surface);
    border: 1px solid var(--hud-primary);
    border-radius: 4px;
  }

  .edit-toolbar :global(button) {
    padding: 0.35rem 0.75rem;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  .edit-toolbar :global(button.primary) { color: var(--hud-primary); border-color: var(--hud-primary); }
  .edit-toolbar :global(button:disabled) { opacity: 0.5; }
</style>
