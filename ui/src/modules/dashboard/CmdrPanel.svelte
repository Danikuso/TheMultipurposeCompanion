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

  interface RankProcessConfig {
    unit: string;
    thresholds: number[];
    prestigeStep?: number;
  }

  interface RankProcessInfo {
    currentFmt: string;
    targetFmt: string;
    remainingFmt: string;
    unit: string;
    isMax: boolean;
    isPromoReady: boolean;
  }

  const RANK_CONFIGS: Record<keyof RanksMap, RankProcessConfig> = {
    trade: {
      unit: 'CR',
      thresholds: [0, 20_000, 150_000, 1_000_000, 5_000_000, 25_000_000, 130_000_000, 390_000_000, 1_050_000_000],
      prestigeStep: 1_050_000_000,
    },
    explore: {
      unit: 'CR',
      thresholds: [0, 20_000, 270_000, 1_000_000, 5_000_000, 35_000_000, 90_000_000, 180_000_000, 320_000_000],
      prestigeStep: 320_000_000,
    },
    exobiologist: {
      unit: 'CR',
      thresholds: [0, 250_000, 1_000_000, 4_500_000, 17_500_000, 68_000_000, 310_000_000, 1_450_000_000, 8_500_000_000],
      prestigeStep: 8_500_000_000,
    },
    soldier: {
      unit: 'CR',
      thresholds: [0, 100_000, 500_000, 2_500_000, 11_000_000, 45_000_000, 180_000_000, 600_000_000, 2_500_000_000],
      prestigeStep: 2_500_000_000,
    },
    combat: {
      unit: 'pts',
      thresholds: [0, 40, 90, 180, 360, 720, 1_200, 2_400, 4_400],
      prestigeStep: 2_200,
    },
    cqc: {
      unit: 'XP',
      thresholds: [0, 2_400, 12_000, 36_000, 84_000, 168_000, 300_000, 492_000, 750_000],
      prestigeStep: 375_000,
    },
    federation: {
      unit: 'mis.',
      thresholds: [2, 4, 7, 11, 16, 22, 30, 40, 52, 66, 82, 100, 120, 145, 175],
    },
    empire: {
      unit: 'mis.',
      thresholds: [2, 4, 7, 11, 16, 22, 30, 40, 52, 66, 82, 100, 120, 145, 175],
    },
  };

  const cmdr = $derived(topic<CmdrTopic>('core.cmdr'));
  const ship = $derived(topic<ShipTopic>('core.ship'));

  function getMaxRank(key: keyof RanksMap): number {
    if (key === 'federation' || key === 'empire') {
      return 14;
    }
    return 13; // Elite V
  }

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

  function getNextRankTitle(key: keyof RanksMap, val: number): string | null {
    const max = getMaxRank(key);
    if (val >= max) return null;
    return getRankTitle(key, val + 1);
  }

  function formatRankQty(val: number, unit: string): string {
    if (val === 0) return '0';
    if (unit === 'CR') {
      if (val >= 1_000_000_000) {
        const num = val / 1_000_000_000;
        const formatted = num >= 10 ? num.toFixed(1) : num.toFixed(num % 1 === 0 ? 0 : (Math.round(num * 10) % 10 === 0 ? 1 : 2));
        return `${formatted} B`;
      }
      if (val >= 1_000_000) {
        const num = val / 1_000_000;
        return `${num.toFixed(1)} M`;
      }
      if (val >= 1_000) {
        const num = val / 1_000;
        return `${num.toFixed(num % 1 === 0 ? 0 : 1)} k`;
      }
      return String(Math.round(val));
    }
    if (unit === 'XP') {
      if (val >= 1_000_000) {
        return `${(val / 1_000_000).toFixed(1)} M`;
      }
      if (val >= 1_000) {
        const num = val / 1_000;
        return `${num.toFixed(num % 1 === 0 ? 0 : 1)} k`;
      }
      return String(Math.round(val));
    }
    if (unit === 'pts') {
      if (val >= 1_000) {
        const num = val / 1_000;
        return `${num.toFixed(num % 1 === 0 ? 0 : 1)} k`;
      }
      return String(Math.round(val));
    }
    if (unit === 'mis.') {
      return String(Math.round(val));
    }
    return String(Math.round(val));
  }

  function getRankProcess(key: keyof RanksMap, rankVal: number, progVal: number): RankProcessInfo {
    const max = getMaxRank(key);
    const isMax = rankVal >= max;
    const isPromoReady = !isMax && progVal >= 100;
    const cfg = RANK_CONFIGS[key];
    const unit = cfg.unit;

    if (isMax) {
      return {
        currentFmt: '',
        targetFmt: '',
        remainingFmt: '',
        unit,
        isMax: true,
        isPromoReady: false,
      };
    }

    if (key === 'federation' || key === 'empire') {
      const targetMissions = cfg.thresholds[rankVal] ?? 20;
      const currentMissions = Math.round(targetMissions * (progVal / 100));
      const remainingMissions = Math.max(0, targetMissions - currentMissions);
      return {
        currentFmt: `~${currentMissions}`,
        targetFmt: `${targetMissions}`,
        remainingFmt: `~${remainingMissions}`,
        unit,
        isMax: false,
        isPromoReady,
      };
    }

    let base = 0;
    let target = 0;
    if (rankVal < 8) {
      base = cfg.thresholds[rankVal] ?? 0;
      target = cfg.thresholds[rankVal + 1] ?? base;
    } else {
      const prestige = rankVal - 8;
      const step = cfg.prestigeStep ?? 0;
      base = (cfg.thresholds[8] ?? 0) + prestige * step;
      target = base + step;
    }

    const delta = target - base;
    const current = base + delta * (progVal / 100);
    const remaining = Math.max(0, target - current);

    return {
      currentFmt: formatRankQty(current, unit),
      targetFmt: formatRankQty(target, unit),
      remainingFmt: formatRankQty(remaining, unit),
      unit,
      isMax: false,
      isPromoReady,
    };
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
              {@const currentTitle = getRankTitle(key, rankVal)}
              {@const nextTitle = getNextRankTitle(key, rankVal)}
              {@const process = getRankProcess(key, rankVal, progVal)}
              {@const tooltip = process.isPromoReady
                ? `${t(`rank-${key}`)}: ${currentTitle} (Nv. ${rankVal}) · ${t('cmdr-rank-promo-ready')} · 100 %`
                : process.isMax
                  ? `${t(`rank-${key}`)}: ${currentTitle} (Nv. ${rankVal} · ${t('cmdr-rank-max')}) · 100 %`
                  : `${t(`rank-${key}`)}: ${currentTitle} (Nv. ${rankVal}) → ${nextTitle} (Nv. ${rankVal + 1}) · ${process.currentFmt} / ${process.targetFmt} ${process.unit} (falta: ${process.remainingFmt} ${process.unit}) · ${progVal} %`}
              <div class="rank-card" data-rank-key={key}>
                <div class="rank-header-row">
                  <span class="rank-name">{t(`rank-${key}`)}</span>
                  <span class="rank-current-badge">{currentTitle}</span>
                </div>
                <div class="rank-values-row rank-num" title={tooltip}>
                  {#if process.isPromoReady}
                    <span class="rank-promo-pill font-mono">{t('cmdr-rank-promo-ready')}</span>
                    <span class="rank-pct font-mono">100 %</span>
                  {:else if process.isMax}
                    <span class="rank-max-pill font-mono">[{t('cmdr-rank-max')}]</span>
                    <span class="rank-pct font-mono">100 %</span>
                  {:else}
                    <span class="rank-quant-val font-mono">
                      <strong class="val-curr">{process.currentFmt}</strong>
                      <span class="val-sep">/</span>
                      <span class="val-target">{process.targetFmt} {process.unit}</span>
                    </span>
                    <span class="rank-pct font-mono">({progVal} %)</span>
                  {/if}
                </div>
                <div class="bar-track">
                  <div class="bar-fill" class:bar-fill-promo={process.isPromoReady} style="width: {progVal}%"></div>
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
    gap: 0.45rem 0.65rem;
  }

  .rank-card {
    background: rgba(22, 28, 45, 0.45);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.4rem 0.55rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    transition: border-color 150ms ease;
  }

  .rank-card:hover {
    border-color: rgba(255, 113, 0, 0.35);
  }

  .rank-header-row {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 0.35rem;
  }

  .rank-name {
    text-transform: uppercase;
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--hud-muted);
  }

  .rank-current-badge {
    font-size: 0.68rem;
    font-weight: 600;
    padding: 0.05rem 0.35rem;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-text);
    max-width: 100%;
    white-space: normal;
    overflow-wrap: anywhere;
    flex-shrink: 0;
    line-height: 1.2;
  }

  .rank-values-row {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: baseline;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    gap: 0.35rem;
  }

  .rank-quant-val {
    display: inline-flex;
    flex-wrap: wrap;
    max-width: 100%;
    align-items: baseline;
    gap: 0.15rem;
    white-space: normal;
  }

  .val-curr {
    color: #fff;
    font-weight: 600;
  }

  .val-sep {
    color: var(--hud-muted);
    opacity: 0.6;
    margin: 0 0.05rem;
  }

  .val-target {
    color: var(--hud-muted);
    font-size: 0.72rem;
  }

  .rank-promo-pill {
    color: var(--hud-success, #34d399);
    font-weight: 700;
    font-size: 0.72rem;
    white-space: nowrap;
  }

  .rank-max-pill {
    color: var(--hud-primary);
    font-size: 0.7rem;
    font-weight: 700;
  }

  .rank-pct {
    color: var(--hud-primary, #ff7100);
    font-weight: 600;
    font-size: 0.72rem;
    margin-left: auto;
    white-space: nowrap;
  }

  .bar-track {
    height: 4px;
    background: var(--hud-bg);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 0.05rem;
  }

  .bar-fill {
    height: 100%;
    background: var(--hud-primary);
    transition: width 150ms ease;
  }

  .bar-fill-promo {
    background: var(--hud-success, #34d399) !important;
  }

  @media (max-width: 640px) {
    .ranks-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
