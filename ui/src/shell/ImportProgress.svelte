<script lang="ts">
  import { t } from '../lib/i18n';
  import { live } from '../lib/ws.svelte';

  const progress = $derived(live.importProgress);
  const visible = $derived(progress != null && progress.total > 0 && progress.done < progress.total);
  const widthPct = $derived(
    progress && progress.total > 0 ? Math.min(100, Math.round((progress.done / progress.total) * 100)) : 0,
  );
</script>

{#if visible && progress}
  <div class="import-bar" role="status">
    <div class="import-label">
      {t('import-progress', { done: progress.done, total: progress.total })}
    </div>
    <div class="import-track">
      <div class="import-fill" style="width: {widthPct}%"></div>
    </div>
  </div>
{/if}

<style>
  .import-bar {
    background: var(--hud-surface-alt);
    border-bottom: 1px solid var(--hud-border);
    padding: 0.35rem 1rem;
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .import-label {
    font-size: 0.8rem;
    color: var(--hud-secondary);
    white-space: nowrap;
  }

  .import-track {
    flex: 1;
    height: 6px;
    background: var(--hud-bg);
    border: 1px solid var(--hud-border);
    border-radius: 3px;
    overflow: hidden;
  }

  .import-fill {
    height: 100%;
    background: var(--hud-secondary);
    transition: width 150ms ease;
  }
</style>
