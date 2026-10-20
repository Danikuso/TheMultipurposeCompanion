<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../lib/i18n';
  import { diag, diagOn } from '../lib/diag.svelte';

  let {
    titleKey,
    span,
    size = 'normal',
    children,
  }: {
    titleKey: string;
    span?: 'full' | 1 | 2;
    size?: 'normal' | 'large';
    children?: Snippet;
  } = $props();

  $effect(() => {
    if (!diagOn()) return;
    const shape = { panel: titleKey, span: span === 'full' ? 0 : (span ?? 1), large: size === 'large', has_children: children !== undefined };
    diag('ui.panel.mounted', shape);
    return () => diag('ui.panel.unmounted', { panel: shape.panel });
  });

  const gridColumn = $derived(
    span === 'full' ? '1 / -1' : span === 1 || span === 2 ? `span ${span}` : undefined,
  );
</script>

<section class="hud-panel" class:large={size === 'large'} style:grid-column={gridColumn}>
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

  .hud-panel.large .hud-panel-header h2 {
    font-size: var(--font-title-lg);
  }

  .hud-panel.large .hud-panel-body {
    font-size: var(--font-body-lg);
  }

  .hud-panel-body {
    padding: 0.75rem;
    flex: 1;
  }
</style>
