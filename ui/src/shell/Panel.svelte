<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../lib/i18n';

  let { titleKey, children }: { titleKey: string; children?: Snippet } = $props();
</script>

<section class="hud-panel">
  <header class="hud-panel-header">
    <h2>{t(titleKey)}</h2>
  </header>
  <div class="hud-panel-body">
    {@render children?.()}
  </div>
</section>

<style>
  .hud-panel {
    background: var(--hud-surface);
    border: 1px solid var(--hud-border);
    border-top: 2px solid var(--hud-primary);
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .hud-panel-header {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--hud-border);
    background: var(--hud-surface-alt);
  }

  .hud-panel-header h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--hud-primary);
  }

  .hud-panel-body {
    padding: 0.75rem;
    flex: 1;
  }
</style>
