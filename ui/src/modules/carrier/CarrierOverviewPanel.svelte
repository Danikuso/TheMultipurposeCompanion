<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, integer } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import StationIcon from '../../lib/icons/StationIcon.svelte';
  import type { CarrierOverviewTopic } from './types';

  const overview = $derived(topic<CarrierOverviewTopic>('carrier.overview'));
  const depotPct = $derived(overview ? Math.min(100, Math.max(0, (overview.fuel_depot / 1000) * 100)) : 0);
  const cargoPct = $derived(
    overview && overview.cargo_total > 0
      ? Math.min(100, Math.max(0, (overview.cargo_used / overview.cargo_total) * 100))
      : 0,
  );
</script>

<Panel titleKey="panel-carrier-overview" span={2} collapsible={!!overview} defaultExpanded={false}>
  {#if !overview || (!overview.carrier_id && !overview.callsign)}
    <p class="muted" data-testid="carrier-no-data">{t('carrier-no-carrier')}</p>
  {:else}
    <div class="carrier-header" data-testid="carrier-header">
      <div class="carrier-avatar-group">
        <div class="carrier-icon-badge">
          <StationIcon stationType="mega-ship" size={28} />
        </div>
        <div class="identity">
          <span class="callsign mono" data-testid="carrier-callsign">{overview.callsign ?? '—'}</span>
          <h3 class="name" data-testid="carrier-name">{overview.name ?? 'Fleet Carrier'}</h3>
        </div>
      </div>
      {#if overview.is_aboard}
        <span class="badge badge-green" data-testid="carrier-aboard">A BORDO</span>
      {/if}
    </div>

    <div class="metric-grid">
      <div class="metric-card">
        <span class="metric-label">{t('carrier-location')}</span>
        <span class="metric-val" data-testid="carrier-location">{overview.current_system ?? '—'}</span>
        {#if overview.current_body}
          <span class="sub">{overview.current_body}</span>
        {/if}
      </div>

      <div class="metric-card">
        <span class="metric-label">{t('carrier-jumps-possible')}</span>
        <span class="metric-val cyan" data-testid="carrier-jumps">{overview.max_jumps_possible}</span>
        <span class="sub">500 al / salto</span>
      </div>

      <div class="metric-card">
        <span class="metric-label">{t('carrier-docking-access')}</span>
        <span class="metric-val mono small" data-testid="carrier-access">{overview.docking_access ?? 'All'}</span>
      </div>
    </div>
  {/if}

  {#snippet details()}
    {#if overview}
      <!-- Combustible Tritio -->
      <div class="section-card">
        <div class="section-head">
          <span class="metric-label">{t('carrier-fuel')}</span>
          <span class="mono cyan bold" data-testid="carrier-fuel-total">{integer(overview.fuel_total)} t</span>
        </div>

        <div class="meter-group">
          <div class="meter-labels">
            <span class="sub">{t('carrier-fuel-depot')}: <b>{integer(overview.fuel_depot)} / 1.000 t</b></span>
            <span class="sub">{t('carrier-fuel-cargo')}: <b>{integer(overview.fuel_cargo)} t</b></span>
          </div>
          <div class="hud-meter-track" role="progressbar" aria-valuenow={overview.fuel_depot} aria-valuemin={0} aria-valuemax={1000}>
            <div class="hud-meter-fill fill-cyan" style:width="{depotPct}%"></div>
          </div>
        </div>
      </div>

      <!-- Capacidad de Almacenamiento -->
      <div class="section-card">
        <div class="section-head">
          <span class="metric-label">{t('carrier-capacity')}</span>
          <span class="mono bold" data-testid="carrier-cargo-total">{integer(overview.cargo_total)} t</span>
        </div>

        <div class="meter-group">
          <div class="meter-labels">
            <span class="sub">{t('carrier-used-space')}: <b>{integer(overview.cargo_used)} t</b></span>
            <span class="sub">{t('carrier-free-space')}: <b>{integer(overview.cargo_free)} t</b></span>
          </div>
          <div class="hud-meter-track" role="progressbar" aria-valuenow={overview.cargo_used} aria-valuemin={0} aria-valuemax={overview.cargo_total}>
            <div class="hud-meter-fill fill-orange" style:width="{cargoPct}%"></div>
          </div>
        </div>
      </div>
    {/if}
  {/snippet}
</Panel>

<style>
  .carrier-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--hud-border-subtle);
    margin-bottom: 0.75rem;
  }
  .carrier-avatar-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .carrier-icon-badge {
    width: 42px;
    height: 42px;
    background: rgba(0, 217, 255, 0.08);
    border: 1px solid rgba(0, 217, 255, 0.3);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 10px rgba(0, 217, 255, 0.15);
  }
  .identity {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .callsign {
    font-size: 0.8rem;
    letter-spacing: 0.08em;
    color: var(--hud-primary);
  }
  .name {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--hud-text);
  }
  .metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 0.65rem;
    margin-bottom: 0.75rem;
  }
  .section-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.75rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 0.65rem;
  }
  .section-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .meter-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .meter-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
  }
  .sub {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .small {
    font-size: 1rem;
  }
  .bold {
    font-weight: 700;
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
