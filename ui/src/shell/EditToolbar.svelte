<script lang="ts">
  import type { Snippet } from 'svelte';
  import { diag } from '../lib/diag.svelte';
  import { askConfirm } from '../lib/confirm';
  import { t } from '../lib/i18n';
  import { commitEdit, layoutState, resetTab } from './layoutStore.svelte';
  import { cancelEditWithDiag, editTab } from './editDiag';
  import { pushToast } from './Toasts.svelte';

  let { tabKey, onaddcard, extra }: { tabKey: string; onaddcard: () => void; extra?: Snippet } = $props();

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
  <button type="button" data-testid="layout-cancel" onclick={cancel}>{t('layout-edit-cancel')}</button>
  <button type="button" data-testid="layout-add" onclick={onaddcard}>{t('layout-edit-add')}</button>
  <button type="button" data-testid="layout-reset" onclick={reset}>{t('layout-edit-reset')}</button>
  {@render extra?.()}
</div>

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

  .edit-toolbar button {
    padding: 0.35rem 0.75rem;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  .edit-toolbar button.primary { color: var(--hud-primary); border-color: var(--hud-primary); }
  .edit-toolbar button:disabled { opacity: 0.5; }
</style>
