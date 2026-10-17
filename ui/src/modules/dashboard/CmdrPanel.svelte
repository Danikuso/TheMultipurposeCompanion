<script lang="ts">
  import { credits } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';

  interface RanksMap {
    combat?: number | null;
    trade?: number | null;
    explore?: number | null;
    soldier?: number | null;
    exobiologist?: number | null;
    empire?: number | null;
    federation?: number | null;
    cqc?: number | null;
  }

  interface CmdrTopic {
    name?: string;
    fid?: string;
    credits?: number | null;
    loan?: number | null;
    ranks?: RanksMap;
    progress?: RanksMap;
  }

  const RANK_KEYS: (keyof RanksMap)[] = [
    'combat',
    'trade',
    'explore',
    'soldier',
    'exobiologist',
    'empire',
    'federation',
    'cqc',
  ];

  const cmdr = $derived(topic<CmdrTopic>('core.cmdr'));
</script>

<Panel titleKey="panel-cmdr">
  {#if !cmdr || (!cmdr.name && cmdr.credits == null)}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="cmdr-head">
      <strong class="cmdr-name">{cmdr.name || '—'}</strong>
      <span class="cmdr-credits">{t('field-credits')}: {credits(cmdr.credits)}</span>
    </div>
    <div class="ranks-grid">
      {#each RANK_KEYS as key (key)}
        {@const rankVal = cmdr.ranks?.[key] ?? 0}
        {@const progVal = Math.min(100, Math.max(0, cmdr.progress?.[key] ?? 0))}
        <div class="rank-row">
          <div class="rank-meta">
            <span class="rank-name">{key.toUpperCase()}</span>
            <span class="rank-num">{rankVal} ({progVal} %)</span>
          </div>
          <div class="bar-track">
            <div class="bar-fill" style="width: {progVal}%"></div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .cmdr-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.75rem;
    gap: 0.5rem;
  }

  .cmdr-name {
    color: var(--hud-primary);
    font-size: 1rem;
  }

  .cmdr-credits {
    font-family: var(--font-mono);
  }

  .ranks-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem 0.75rem;
  }

  .rank-meta {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--hud-muted);
    margin-bottom: 0.15rem;
  }

  .rank-num {
    font-family: var(--font-mono);
    color: var(--hud-text);
  }

  .bar-track {
    height: 5px;
    background: var(--hud-bg);
    border: 1px solid var(--hud-border);
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--hud-primary);
    transition: width 150ms ease;
  }
</style>
