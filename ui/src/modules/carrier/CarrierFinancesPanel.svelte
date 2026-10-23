<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { CarrierFinancesTopic, MaintenanceHealth } from './types';

  const fin = $derived(topic<CarrierFinancesTopic>('carrier.finances'));

  const healthBadgeClass = $derived.by(() => {
    switch (fin?.maintenance_health) {
      case 'Safe':
        return 'badge-green';
      case 'Warning':
        return 'badge-orange';
      case 'Critical':
        return 'badge-danger';
      default:
        return '';
    }
  });

  const healthLabel = $derived.by(() => {
    switch (fin?.maintenance_health) {
      case 'Safe':
        return t('carrier-health-safe');
      case 'Warning':
        return t('carrier-health-warning');
      case 'Critical':
        return t('carrier-health-critical');
      default:
        return '—';
    }
  });
</script>

<Panel titleKey="panel-carrier-finances" span={1}>
  {#if !fin}
    <p class="muted" data-testid="finances-no-data">{t('empty-no-data')}</p>
  {:else}
    <div class="finances-header">
      <span class="metric-label">{t('carrier-weeks-covered')}</span>
      <span class="badge {healthBadgeClass}" data-testid="carrier-health-badge">
        {healthLabel}
      </span>
    </div>

    <div class="covered-card" data-testid="covered-card">
      <span class="covered-val mono" data-testid="weeks-covered-val">
        {fin.weeks_covered}
      </span>
      <span class="sub">{t('carrier-weeks-covered')}</span>
    </div>

    <div class="metric-list">
      <div class="metric-row">
        <span class="label">{t('carrier-balance')}</span>
        <span class="mono bold" data-testid="carrier-balance">{credits(fin.balance)}</span>
      </div>

      <div class="metric-row">
        <span class="label">{t('carrier-weekly-cost')}</span>
        <span class="mono gold bold" data-testid="carrier-upkeep">{credits(fin.weekly_upkeep)}</span>
      </div>

      <div class="metric-row">
        <span class="label">{t('carrier-reserve')}</span>
        <span class="mono" data-testid="carrier-reserve">{credits(fin.reserve_balance)}</span>
      </div>

      <div class="metric-row sub-row">
        <span class="label">{t('carrier-next-upkeep')}</span>
        <span class="mono small" data-testid="carrier-next-payment">
          {new Date(fin.next_payment_time).toUTCString().replace(/GMT|UTC/, 'UTC')}
        </span>
      </div>
    </div>
  {/if}
</Panel>

<style>
  .finances-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .covered-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    margin-bottom: 0.75rem;
  }
  .covered-val {
    font-size: 2.2rem;
    font-weight: 700;
    color: var(--hud-text);
  }
  .metric-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .metric-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.5rem;
    background: var(--hud-surface-card);
    border-radius: 3px;
  }
  .sub-row {
    font-size: 0.75rem;
    opacity: 0.85;
  }
  .label {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .sub {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .small {
    font-size: 0.75rem;
  }
  .bold {
    font-weight: 700;
  }
  .gold {
    color: #fbbf24;
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
