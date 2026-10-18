<script lang="ts">
  import { t } from '../lib/i18n';
  import { topic } from '../lib/ws.svelte';
  import { modules } from '../modules/registry';
  import { layouts } from './layouts';

  interface LiveModuleState {
    id: string;
    title_key?: string;
    relevance: string;
  }

  let {
    activeTab = 'layout:exploration',
    onSelect,
  }: {
    activeTab?: string;
    onSelect?: (id: string) => void;
  } = $props();

  const liveModules = $derived(topic<LiveModuleState[]>('core.modules'));

  const displayModules = $derived(
    modules.map((regMod) => {
      const liveState = liveModules?.find((m) => m.id === regMod.id);
      return {
        id: regMod.id,
        titleKey: regMod.titleKey,
        relevance: (liveState?.relevance ?? 'normal').toLowerCase(),
      };
    }),
  );
</script>

<aside class="hud-sidebar" aria-label={t('nav-modules')}>
  <div class="sidebar-top">
    <h2>{t('nav-layouts')}</h2>
    {#each layouts as layout (layout.id)}
      <button type="button" class="nav-btn" class:active={activeTab === `layout:${layout.id}`}
        title={t(layout.titleKey)} onclick={() => onSelect?.(`layout:${layout.id}`)}>
        <span class="nav-label">{t(layout.titleKey)}</span>
      </button>
    {/each}
    <h2>{t('nav-modules')}</h2>
    {#each displayModules as mod (mod.id)}
        <button
          type="button"
          class="nav-btn"
          class:active={activeTab === mod.id}
          title={t(mod.titleKey)}
          data-module={mod.id}
          onclick={() => onSelect?.(mod.id)}
        >
          <span class="nav-label">{t(mod.titleKey)}</span>
          <span class="relevance-dot relevance-{mod.relevance}"></span>
        </button>
    {/each}
  </div>
  <div class="sidebar-bottom">
    <button
      type="button"
      class="nav-btn"
      class:active={activeTab === 'settings'}
      title={t('nav-settings')}
      onclick={() => onSelect?.('settings')}
    >
      <span class="nav-label">{t('nav-settings')}</span>
    </button>
  </div>
</aside>

<style>
  .hud-sidebar {
    width: 180px;
    background: var(--hud-surface);
    border-right: 1px solid var(--hud-border);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0.6rem;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .sidebar-top,
  .sidebar-bottom {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  h2 { font-size: 0.75rem; color: var(--hud-muted); margin: 0.6rem 0 0; }

  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.65rem;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid transparent;
  }

  .nav-btn.active {
    border-color: var(--hud-primary);
    background: var(--hud-primary-dim);
    color: var(--hud-primary);
    font-weight: 600;
  }

  .relevance-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }

  .relevance-low {
    background: var(--hud-muted);
  }

  .relevance-active {
    background: var(--hud-warning);
  }

  .relevance-urgent {
    background: var(--hud-danger);
  }

  .relevance-normal {
    background: var(--hud-secondary);
  }

  .relevance-high {
    background: var(--hud-warning);
  }

  .relevance-critical {
    background: var(--hud-danger);
  }
</style>
