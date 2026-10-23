<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, integer, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import StationIcon from '../../lib/icons/StationIcon.svelte';
  import type { ColonisationDepotTopic } from './types';

  const depot = $derived(topic<ColonisationDepotTopic>('colonisation.depot'));
  const progressPct = $derived(depot ? Math.min(100, Math.max(0, depot.progress_percent)) : 0);
</script>

<Panel titleKey="panel-colonisation-depot" span={2}>
  {#if !depot || !depot.active_project}
    <p class="muted" data-testid="depot-no-data">{t('colonisation-no-depot')}</p>
  {:else}
    <div class="depot-header" data-testid="depot-header">
      <div class="identity">
        <span class="type-label mono">{depot.construction_type ?? 'CONSTRUCTION PROJECT'}</span>
        <div class="depot-title-row">
          <StationIcon stationType="outpost" size={20} />
          <h3 class="name" data-testid="depot-station-name">{depot.station_name ?? t('colonisation-depot')}</h3>
        </div>
        {#if depot.market_id}
          <span class="sub">{t('colonisation-market-id')}: {depot.market_id}</span>
        {/if}
      </div>
      {#if depot.is_docked}
        <span class="badge badge-green" data-testid="depot-docked">ATRACADO</span>
      {/if}
    </div>

    <!-- Progreso Global -->
    <div class="overall-progress-card">
      <div class="progress-head">
        <span class="metric-label">{t('colonisation-construction-progress')}</span>
        <span class="metric-val green mono bold" data-testid="depot-progress-val">
          {pct(progressPct)}
        </span>
      </div>
      <div
        class="hud-meter-track"
        role="progressbar"
        aria-valuenow={progressPct}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div class="hud-meter-fill fill-green" style:width="{progressPct}%"></div>
      </div>
    </div>

    <!-- Métricas Clave -->
    <div class="metric-grid">
      <div class="metric-card">
        <span class="metric-label">{t('colonisation-col-provided')}</span>
        <span class="metric-val mono" data-testid="depot-provided">
          {integer(depot.total_provided)} <span class="unit">/ {integer(depot.total_required)} t</span>
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">{t('colonisation-col-remaining')}</span>
        <span class="metric-val mono gold" data-testid="depot-remaining">
          {integer(depot.total_remaining)} t
        </span>
      </div>

      <div class="metric-card">
        <span class="metric-label">{t('colonisation-trips-left')}</span>
        <span class="metric-val mono cyan" data-testid="depot-trips">
          ~{depot.total_trips_remaining}
        </span>
        <span class="sub">{t('colonisation-current-hold', { capacity: String(depot.cargo_capacity) })}</span>
      </div>
    </div>

    <!-- Desglose de Recursos -->
    <div class="table-wrap">
      <table data-testid="resources-table">
        <thead>
          <tr>
            <th>{t('colonisation-col-resource')}</th>
            <th>{t('colonisation-construction-progress')}</th>
            <th class="n">{t('colonisation-col-provided')}</th>
            <th class="n">{t('colonisation-col-remaining')}</th>
            <th class="n">{t('colonisation-trips-left')}</th>
          </tr>
        </thead>
        <tbody>
          {#each depot.resources as r (r.name)}
            <tr data-testid="resource-row">
              <td class="bold">
                {r.name_localised ?? r.name}
              </td>
              <td class="meter-col">
                <div class="mini-meter">
                  <div
                    class="hud-meter-track"
                    role="progressbar"
                    aria-valuenow={r.progress_percent}
                    aria-valuemin={0}
                    aria-valuemax={100}
                  >
                    <div
                      class="hud-meter-fill fill-green"
                      style:width="{Math.min(100, Math.max(0, r.progress_percent))}%"
                    ></div>
                  </div>
                  <span class="sub mono">{pct(r.progress_percent)}</span>
                </div>
              </td>
              <td class="n mono">
                {integer(r.provided)} / {integer(r.required)}
              </td>
              <td class="n mono {r.remaining > 0 ? 'gold' : 'muted'}">
                {integer(r.remaining)} t
              </td>
              <td class="n mono cyan">
                {r.remaining > 0 ? `~${r.trips_remaining}` : '✔'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</Panel>

<style>
  .depot-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--hud-border-subtle);
    margin-bottom: 0.75rem;
  }
  .identity {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .type-label {
    font-size: 0.75rem;
    letter-spacing: 0.08em;
    color: var(--hud-primary);
  }
  .depot-title-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }
  .name {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--hud-text);
  }
  .overall-progress-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.75rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
  }
  .progress-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 0.65rem;
    margin-bottom: 0.75rem;
  }
  .meter-col {
    width: 140px;
  }
  .mini-meter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .mini-meter .hud-meter-track {
    flex: 1;
    height: 6px;
  }
  .sub {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .unit {
    font-size: 0.85rem;
    color: var(--hud-muted);
  }
  .bold {
    font-weight: 700;
  }
  .gold {
    color: #fbbf24;
  }
  .cyan {
    color: var(--hud-cyan);
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
