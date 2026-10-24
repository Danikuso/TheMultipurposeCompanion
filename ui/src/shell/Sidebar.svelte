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

  const LAYOUT_ICONS: Record<string, string> = {
    exploration: '🧭',
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
          <span class="nav-icon" aria-hidden="true">{LAYOUT_ICONS[layout.id] ?? '◈'}</span>
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
            <span class="nav-icon" aria-hidden="true">{MOD_ICONS[mod.id] ?? '◈'}</span>
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
    width: 190px;
    background: var(--hud-surface);
    border-right: 1px solid var(--hud-border-subtle);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0.6rem;
    gap: 0.5rem;
    flex-shrink: 0;
    box-shadow: 2px 0 15px rgba(0, 0, 0, 0.3);
  }

  .sidebar-top,
  .sidebar-bottom {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
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
