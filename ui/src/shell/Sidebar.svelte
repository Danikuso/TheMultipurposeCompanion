<script lang="ts">
  import { t } from '../lib/i18n';
  import { topic } from '../lib/ws.svelte';
  import { modules } from '../modules/registry';
  import { layouts } from './layouts';
  import { resolveNavIcon } from '../lib/icons';

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
    modules
      .filter((regMod) => regMod.id !== 'accounts')
      .map((regMod) => {
        const liveState = liveModules?.find((m) => m.id === regMod.id);
        return {
          id: regMod.id,
          titleKey: regMod.titleKey,
          relevance: (liveState?.relevance ?? 'normal').toLowerCase(),
        };
      }),
  );

  const LAYOUT_ICONS: Record<string, string> = {
    exploration: '🧭',
    ship: '🚀',
    combat: '🎯',
    mining: '⛏️',
    trade: '⚖️',
    missions: '📋',
    engineering: '🔧',
    carrier: '⚓',
    colonisation: '🏗️',
    powerplay: '👑',
  };

  const MOD_ICONS: Record<string, string> = {
    dashboard: '📊',
    ship: '🚀',
    navigation: '🛰️',
    exploration: '🪐',
    exobiology: '🧬',
    combat: '⚔️',
    mining: '💎',
    trade: '📦',
    missions: '📜',
    engineering: '🛠️',
    carrier: '⚓',
    colonisation: '🏗️',
    powerplay: '👑',
    bgs: '🌐',
    accounts: '📡',
    crime: '⚖️',
  };
</script>

<aside class="hud-sidebar" aria-label={t('nav-modules')}>
  <div class="sidebar-top">
    <h2>{t('nav-layouts')}</h2>
    {#each layouts as layout (layout.id)}
      <button type="button" class="nav-btn" class:active={activeTab === `layout:${layout.id}`}
        title={t(layout.titleKey)} onclick={() => onSelect?.(`layout:${layout.id}`)}>
        <span class="nav-content">
          {#if resolveNavIcon(layout.id)}
            <span class="nav-icon hud-svg-icon" aria-hidden="true">{@html resolveNavIcon(layout.id)}</span>
          {:else}
            <span class="nav-icon" aria-hidden="true">{LAYOUT_ICONS[layout.id] ?? '◈'}</span>
          {/if}
          <span class="nav-label">{t(layout.titleKey)}</span>
        </span>
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
          <span class="nav-content">
            {#if resolveNavIcon(mod.id)}
              <span class="nav-icon hud-svg-icon" aria-hidden="true">{@html resolveNavIcon(mod.id)}</span>
            {:else}
              <span class="nav-icon" aria-hidden="true">{MOD_ICONS[mod.id] ?? '◈'}</span>
            {/if}
            <span class="nav-label">{t(mod.titleKey)}</span>
          </span>
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
      <span class="nav-content">
        <span class="nav-icon" aria-hidden="true">⚙️</span>
        <span class="nav-label">{t('nav-settings')}</span>
      </span>
    </button>
  </div>
</aside>

<style>
  .hud-sidebar {
    width: 200px;
    height: 100%;
    max-height: 100%;
    background: var(--hud-surface);
    border-right: 1px solid var(--hud-border-subtle);
    display: flex;
    flex-direction: column;
    padding: 0.6rem 0.4rem;
    gap: 0.5rem;
    flex-shrink: 0;
    min-height: 0;
    overflow: hidden;
    box-shadow: 2px 0 15px rgba(0, 0, 0, 0.3);
  }

  .sidebar-top {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding-right: 0.25rem;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.2);
  }

  .sidebar-top::-webkit-scrollbar {
    width: 5px;
  }

  .sidebar-top::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 4px;
  }

  .sidebar-top::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.45);
    border-radius: 4px;
  }

  .sidebar-top::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }

  .sidebar-bottom {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding-top: 0.45rem;
    border-top: 1px solid var(--hud-border-subtle);
  }

  h2 {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--hud-muted);
    margin: 0.6rem 0 0.2rem 0.4rem;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    text-align: left;
    padding: 0.45rem 0.65rem;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid transparent;
    border-radius: 4px;
    transition: all 150ms ease;
  }

  .nav-btn:hover {
    background: var(--hud-surface-card);
    border-color: var(--hud-border-subtle);
  }

  .nav-btn.active {
    border-color: var(--hud-primary);
    background: rgba(255, 113, 0, 0.15);
    color: var(--hud-primary);
    font-weight: 700;
  }

  .nav-content {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    overflow: hidden;
  }

  .nav-icon {
    font-size: 0.95rem;
    line-height: 1;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
  }

  .hud-svg-icon {
    color: var(--hud-primary, #ff7100);
    transition: all 150ms ease;
  }

  .hud-svg-icon :global(svg) {
    width: 100%;
    height: 100%;
    max-width: 18px;
    max-height: 18px;
    display: block;
    object-fit: contain;
  }

  .nav-btn.active .hud-svg-icon {
    color: var(--hud-primary, #ff7100);
    filter: drop-shadow(0 0 5px rgba(255, 113, 0, 0.7));
  }

  .nav-btn:not(.active) .hud-svg-icon {
    color: var(--hud-muted, #8a96a3);
    opacity: 0.85;
  }

  .nav-btn:hover .hud-svg-icon {
    color: var(--hud-text, #e2e8f0);
    opacity: 1;
  }

  .nav-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.82rem;
  }

  .relevance-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .relevance-low {
    background: var(--hud-muted);
  }

  .relevance-active {
    background: var(--hud-warning);
    box-shadow: 0 0 6px var(--hud-warning);
  }

  .relevance-urgent {
    background: var(--hud-danger);
    box-shadow: 0 0 6px var(--hud-danger);
  }

  .relevance-normal {
    background: var(--hud-secondary);
  }

  .relevance-high {
    background: var(--hud-warning);
    box-shadow: 0 0 6px var(--hud-warning);
  }

  .relevance-critical {
    background: var(--hud-danger);
    box-shadow: 0 0 6px var(--hud-danger);
  }
</style>
