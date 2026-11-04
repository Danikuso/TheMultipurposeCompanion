<script lang="ts">
  import { t } from '../lib/i18n';
  import { beginEdit, layoutState } from './layoutStore.svelte';
  import { diag } from '../lib/diag.svelte';
  import { editTab } from './editDiag';

  let { titleKey, title, tabKey }: { titleKey?: string; title?: string; tabKey: string | null } = $props();
  const text = $derived(title ?? (titleKey ? t(titleKey) : ''));

  function start() {
    beginEdit();
    diag('ui.layout.edit', { begin: true, ...editTab(tabKey) });
  }
</script>

<header class="grid-header">
  <h1 class="grid-title">{text}</h1>
  {#if tabKey !== null && !layoutState.editing}
    <button type="button" class="edit-btn" data-testid="layout-edit" onclick={start}>{t('layout-edit-start')}</button>
  {/if}
</header>

<style>
  .grid-header {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    min-width: 0;
  }

  .grid-title {
    margin: 0;
    font-size: 1.1rem;
    color: var(--hud-text);
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .edit-btn {
    flex-shrink: 0;
    padding: 0.35rem 0.75rem;
    background: var(--hud-surface);
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }
</style>
