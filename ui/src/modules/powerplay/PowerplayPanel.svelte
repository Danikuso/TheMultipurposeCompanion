<script lang="ts">
  import { t } from '../../lib/i18n';
  import { integer, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import PowerIcon from '../../lib/icons/PowerIcon.svelte';
  import SuperpowerIcon from '../../lib/icons/SuperpowerIcon.svelte';
  import { POWERS } from '../../lib/icons';
  import type { PowerplayStatusTopic, PowerplaySessionTopic } from './types';

  const status = $derived(topic<PowerplayStatusTopic>('powerplay.status'));
  const session = $derived(topic<PowerplaySessionTopic>('powerplay.session'));
  const superpower = $derived(status?.power ? POWERS[status.power]?.superpower : null);

  function formatCycleTime(seconds: number): string {
    if (seconds <= 0) return '0h';
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    if (days > 0) return `${days}d ${hours}h`;
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  }
</script>

<Panel titleKey="panel-powerplay-status" span={2}>
  {#if !status || !status.power}
    <p class="muted" data-testid="powerplay-no-data">{t('powerplay-no-pledge')}</p>
  {:else}
    <div class="power-header" data-testid="powerplay-header">
      <div class="power-identity-group">
        <PowerIcon power={status.power} size={56} allowToggle={true} />
        <div class="identity">
          <div class="superpower-tag">
            {#if superpower}
              <SuperpowerIcon superpower={superpower} size={13} />
              <span class="superpower-name sub mono uppercase">{superpower}</span>
            {/if}
            <span class="pledge-label mono">{t('powerplay-pledged-to')}</span>
          </div>
          <h3 class="power-name" data-testid="powerplay-power">{status.power}</h3>
        </div>
      </div>
      <div class="loyalty-badge">
        <span class="badge badge-cyan" data-testid="powerplay-weeks">
          {status.time_pledged_weeks} {t('powerplay-time-pledged').toLowerCase()}
        </span>
      </div>
    </div>

    <!-- Stepper de Rango y Progreso -->
    <div class="rank-card" data-testid="powerplay-rank-card">
      <div class="rank-head">
        <div class="rank-title">
          <span class="metric-label">{t('powerplay-rank')}</span>
          <span class="rank-number mono bold cyan" data-testid="powerplay-rank">
            {status.rank} <span class="rank-max">/ 100</span>
          </span>
        </div>
        <div class="rank-merits-head">
          <span class="sub">{t('powerplay-merits')}: <b class="mono" data-testid="powerplay-merits">{integer(status.merits)}</b></span>
          <span class="sub">{t('powerplay-merits-needed')}: <b class="mono gold" data-testid="powerplay-needed">{integer(status.merits_needed_for_next)}</b></span>
        </div>
      </div>

      <div class="hud-meter-track" role="progressbar" aria-valuenow={status.rank_progress_percent} aria-valuemin={0} aria-valuemax={100}>
        <div class="hud-meter-fill fill-cyan" style:width="{Math.min(100, Math.max(0, status.rank_progress_percent))}%"></div>
      </div>
      <div class="rank-foot">
        <span class="sub mono">{pct(status.rank_progress_percent)} al rango {Math.min(100, status.rank + 1)}</span>
      </div>
    </div>

    <!-- Métricas de Sesión y Ciclo -->
    <div class="metric-grid">
      <div class="metric-card">
        <span class="metric-label">{t('powerplay-session-merits')}</span>
        <span class="metric-val mono green" data-testid="powerplay-session-merits">
          +{integer(session?.session_merits ?? 0)}
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">{t('powerplay-merits-rate')}</span>
        <span class="metric-val mono cyan" data-testid="powerplay-merits-rate">
          {integer(session?.merits_per_hour ?? 0)} <span class="unit">/ h</span>
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">{t('powerplay-cycle-ends')}</span>
        <span class="metric-val mono gold" data-testid="powerplay-cycle-timer">
          {formatCycleTime(status.seconds_to_cycle_end)}
        </span>
        <span class="sub">{t('powerplay-cycle-thursday')}</span>
      </div>
    </div>
  {/if}
</Panel>

<style>
  .power-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 0.85rem;
    border-bottom: 1px solid var(--hud-border-subtle);
    margin-bottom: 0.85rem;
    gap: 1rem;
  }
  .power-identity-group {
    display: flex;
    align-items: center;
    gap: 0.9rem;
  }
  .superpower-tag {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.7rem;
    color: var(--hud-muted);
  }
  .superpower-name {
    font-weight: 600;
    letter-spacing: 0.05em;
  }
  .identity {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .pledge-label {
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    color: var(--hud-muted);
  }
  .power-name {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--hud-text);
  }
  .rank-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.75rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    margin-bottom: 0.75rem;
  }
  .rank-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
  }
  .rank-title {
    display: flex;
    flex-direction: column;
  }
  .rank-number {
    font-size: 1.6rem;
    line-height: 1.1;
  }
  .rank-max {
    font-size: 0.9rem;
    color: var(--hud-muted);
  }
  .rank-merits-head {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.15rem;
  }
  .rank-foot {
    display: flex;
    justify-content: flex-end;
  }
  .metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 0.65rem;
  }
  .sub {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .unit {
    font-size: 0.8rem;
    color: var(--hud-muted);
  }
  .bold {
    font-weight: 700;
  }
  .cyan {
    color: var(--hud-cyan);
  }
  .green {
    color: var(--hud-success);
  }
  .gold {
    color: #fbbf24;
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
