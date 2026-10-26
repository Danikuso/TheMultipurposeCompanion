<script lang="ts">
  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import { diag, diagOn } from '../lib/diag.svelte';

  let {
    titleKey,
    span,
    size = 'normal',
    collapsible = false,
    defaultExpanded = false,
    details,
    children,
  }: {
    titleKey: string;
    span?: 'full' | 1 | 2;
    size?: 'normal' | 'large';
    collapsible?: boolean;
    defaultExpanded?: boolean;
    details?: Snippet;
    children?: Snippet;
  } = $props();

  let isExpanded = $state(untrack(() => defaultExpanded));

  $effect(() => {
    if (!diagOn()) return;
    const shape = {
      panel: titleKey,
      span: span === 'full' ? 0 : (span ?? 1),
      large: size === 'large',
      has_children: children !== undefined,
      has_details: details !== undefined,
    };
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
    {#if details || collapsible}
      <button
        type="button"
        class="panel-header-toggle"
        onclick={() => (isExpanded = !isExpanded)}
        aria-expanded={isExpanded}
        title={isExpanded ? t('panel-hide-details') : t('panel-show-details')}
      >
        <span class="toggle-chevron">{isExpanded ? '▲' : '▼'}</span>
        <span class="toggle-label">{isExpanded ? t('panel-hide-details') : t('panel-show-details')}</span>
      </button>
    {/if}
  </header>
  <div class="hud-panel-body">
    <div class="hud-panel-basic">
      {@render children?.()}
    </div>
    {#if details}
      <div class="hud-panel-details" class:collapsed={!isExpanded}>
        <div class="details-divider">
          <span class="divider-line"></span>
          <button
            type="button"
            class="details-expand-btn"
            onclick={() => (isExpanded = !isExpanded)}
            aria-expanded={isExpanded}
          >
            <span>{isExpanded ? '▲' : '▼'}</span>
            <span>{isExpanded ? t('panel-hide-details') : t('panel-show-details')}</span>
          </button>
          <span class="divider-line"></span>
        </div>
        <div class="details-content">
          {@render details()}
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .hud-panel {
    background: var(--hud-surface);
    border: 1px solid var(--hud-border-subtle);
    border-top: 3px solid var(--hud-primary);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
  }

  .hud-panel-header {
    padding: 0.55rem 0.85rem;
    border-bottom: 1px solid var(--hud-border-subtle);
    background: var(--hud-surface-alt);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .hud-panel-header h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--hud-primary);
  }

  .panel-header-toggle {
    background: rgba(255, 113, 0, 0.1);
    color: var(--hud-primary, #ff7100);
    border: 1px solid rgba(255, 113, 0, 0.25);
    border-radius: 4px;
    padding: 0.2rem 0.55rem;
    font-size: 0.74rem;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .panel-header-toggle:hover {
    background: var(--hud-primary, #ff7100);
    color: #000;
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
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.15);
  }

  .hud-panel-body::-webkit-scrollbar {
    width: 6px;
  }

  .hud-panel-body::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.15);
    border-radius: 3px;
  }

  .hud-panel-body::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.45);
    border-radius: 3px;
  }

  .hud-panel-body::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }

  .hud-panel-basic {
    flex-shrink: 0;
  }

  .details-divider {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0.75rem 0 0.5rem 0;
  }

  .divider-line {
    flex: 1;
    height: 1px;
    background: var(--hud-border-subtle, rgba(255, 255, 255, 0.1));
  }

  .details-expand-btn {
    background: var(--hud-surface-card, rgba(20, 25, 35, 0.7));
    color: var(--hud-primary, #ff7100);
    border: 1px solid var(--hud-border, rgba(255, 113, 0, 0.3));
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    transition: all 150ms ease;
  }

  .details-expand-btn:hover {
    background: var(--hud-primary, #ff7100);
    color: #000;
  }

  .hud-panel-details {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .hud-panel-details.collapsed .details-content {
    display: none;
  }

  .details-content {
    max-height: 480px;
    overflow-y: auto;
    overflow-x: hidden;
    padding-right: 0.35rem;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.15);
  }

  .details-content::-webkit-scrollbar {
    width: 6px;
  }

  .details-content::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.15);
    border-radius: 3px;
  }

  .details-content::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.45);
    border-radius: 3px;
  }

  .details-content::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }
</style>
