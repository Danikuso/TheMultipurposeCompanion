<script lang="ts">
  import { credits } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { pushToast } from '../../shell/Toasts.svelte';

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
    game_mode?: string | null;
    group?: string | null;
    ranks?: RanksMap;
    progress?: RanksMap;
  }

  interface ShipTopic {
    rebuy?: number | null;
  }

  const RANK_GROUPS: { id: string; titleKey: string; keys: (keyof RanksMap)[] }[] = [
    {
      id: 'career',
      titleKey: 'rank-group-career',
      keys: ['combat', 'trade', 'explore', 'soldier', 'exobiologist', 'cqc'],
    },
    {
      id: 'superpowers',
      titleKey: 'rank-group-superpowers',
      keys: ['federation', 'empire'],
    },
  ];

  const cmdr = $derived(topic<CmdrTopic>('core.cmdr'));
  const ship = $derived(topic<ShipTopic>('core.ship'));

  function getRankTitle(key: keyof RanksMap, val: number): string {
    if (key === 'combat' || key === 'trade' || key === 'explore' || key === 'soldier' || key === 'exobiologist' || key === 'cqc') {
      if (val > 8) {
        const prestige = val - 8;
        const romans = ['', 'I', 'II', 'III', 'IV', 'V'];
        return `Elite ${romans[prestige] ?? prestige}`;
      }
    }
    const keyName = `${key}-rank-${val}`;
    const localized = t(keyName);
    return localized !== keyName ? localized : String(val);
  }

  let copied = $state(false);
  function copyCmdrName() {
    if (!cmdr?.name) return;
    navigator.clipboard?.writeText(cmdr.name);
    copied = true;
    pushToast(t('cmdr-copied'), 'Info');
    setTimeout(() => {
      copied = false;
    }, 2000);
  }
</script>

<Panel titleKey="panel-cmdr">
  {#if !cmdr || (!cmdr.name && cmdr.credits == null)}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="cmdr-card">
      <div class="cmdr-head">
        <div class="cmdr-identity">
          <div class="cmdr-name-group">
            <strong class="cmdr-name" data-testid="cmdr-name">{cmdr.name || '—'}</strong>
            {#if cmdr.name}
              <button
                type="button"
                class="copy-btn"
                title={t('cmdr-copy-name')}
                onclick={copyCmdrName}
              >
                {copied ? '✓' : '📋'}
              </button>
            {/if}
            {#if cmdr.game_mode}
              <span class="badge mode-badge" data-testid="cmdr-mode">
                {#if cmdr.game_mode === 'Open'}
                  {t('cmdr-game-mode-open')}
                {:else if cmdr.game_mode === 'Solo'}
                  {t('cmdr-game-mode-solo')}
                {:else if cmdr.game_mode === 'Group'}
                  {cmdr.group ? t('cmdr-game-mode-group', { group: cmdr.group }) : t('cmdr-game-mode-group-generic')}
                {:else}
                  {cmdr.game_mode}
                {/if}
              </span>
            {/if}
          </div>
          {#if cmdr.fid}
            <span class="cmdr-fid font-mono">FID: {cmdr.fid}</span>
          {/if}
        </div>

        <div class="cmdr-finances">
          <div class="finance-item">
            <span class="finance-label">{t('field-credits')}:</span>
            <strong class="cmdr-credits gold font-mono">{credits(cmdr.credits)}</strong>
          </div>
          {#if (cmdr.loan ?? 0) > 0}
            <div class="finance-item">
              <span class="finance-label">{t('cmdr-loan')}:</span>
              <span class="loan-val font-mono">-{credits(cmdr.loan)}</span>
            </div>
          {/if}
          {#if (ship?.rebuy ?? 0) > 0}
            <div class="finance-item">
              <span class="finance-label">{t('cmdr-rebuy')}:</span>
              <span class="rebuy-val font-mono">{credits(ship?.rebuy)}</span>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <div class="rank-sections">
      {#each RANK_GROUPS as group (group.id)}
        <div class="rank-section" data-testid="rank-group-{group.id}">
          <h3 class="rank-section-title">{t(group.titleKey)}</h3>
          <div class="ranks-grid">
            {#each group.keys as key (key)}
              {@const rankVal = cmdr.ranks?.[key] ?? 0}
              {@const progVal = Math.min(100, Math.max(0, cmdr.progress?.[key] ?? 0))}
              {@const title = getRankTitle(key, rankVal)}
              <div class="rank-row" data-rank-key={key}>
                <div class="rank-meta">
                  <span class="rank-name">{t(`rank-${key}`)}</span>
                  <span class="rank-num" title="{title} ({progVal} %)">
                    <span class="rank-title-text">{title}</span>
                    <span class="rank-pct">({progVal} %)</span>
                  </span>
                </div>
                <div class="bar-track">
                  <div class="bar-fill" style="width: {progVal}%"></div>
                </div>
              </div>
            {/each}
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

  .cmdr-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.6rem 0.75rem;
    margin-bottom: 0.75rem;
  }

  .cmdr-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .cmdr-identity {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .cmdr-name-group {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .cmdr-name {
    color: var(--hud-primary);
    font-size: 1.05rem;
    font-weight: 700;
  }

  .copy-btn {
    background: transparent;
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.75rem;
    cursor: pointer;
    line-height: 1;
    transition: all 120ms ease;
  }

  .copy-btn:hover {
    border-color: var(--hud-primary);
    color: var(--hud-primary);
  }

  .mode-badge {
    font-size: 0.72rem;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border);
    color: var(--hud-cyan, #38bdf8);
    background: rgba(56, 189, 248, 0.08);
  }

  .cmdr-fid {
    font-size: 0.72rem;
    color: var(--hud-muted);
  }

  .cmdr-finances {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.2rem;
    font-size: 0.8rem;
  }

  .finance-item {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .finance-label {
    color: var(--hud-muted);
    font-size: 0.75rem;
  }

  .gold {
    color: #fbbf24;
  }

  .loan-val {
    color: var(--hud-danger, #ef4444);
    font-weight: 600;
  }

  .rebuy-val {
    color: var(--hud-text);
  }

  .font-mono {
    font-family: var(--font-mono);
  }

  .rank-sections {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .rank-section + .rank-section {
    padding-top: 0.5rem;
    border-top: 1px solid var(--hud-border);
  }

  .rank-section-title {
    margin: 0 0 0.4rem;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--hud-primary);
  }

  .ranks-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem 0.75rem;
  }

  .rank-meta {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 0.75rem;
    color: var(--hud-muted);
    margin-bottom: 0.15rem;
    gap: 0.4rem;
  }

  .rank-name {
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .rank-num {
    font-family: var(--font-mono);
    color: var(--hud-text);
    text-align: right;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .rank-title-text {
    color: var(--hud-text);
    font-weight: 600;
  }

  .rank-pct {
    color: var(--hud-muted);
    font-size: 0.72rem;
    margin-left: 0.2rem;
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
