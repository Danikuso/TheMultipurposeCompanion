<script lang="ts">
  import { credits, ly, pct } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';

  interface ShipTopic {
    ship_type?: string;
    ship_type_localised?: string | null;
    name?: string | null;
    ident?: string | null;
    hull_health?: number | null;
    fuel_main?: number | null;
    fuel_reservoir?: number | null;
    fuel_capacity_main?: number | null;
    fuel_capacity_reserve?: number | null;
    cargo_count?: number | null;
    cargo_capacity?: number | null;
    rebuy?: number | null;
    max_jump_range?: number | null;
    damaged_modules?: string[];
  }

  const ship = $derived(topic<ShipTopic>('core.ship'));
  const fuelMainPct = $derived(
    ship?.fuel_main != null && ship?.fuel_capacity_main && ship.fuel_capacity_main > 0
      ? Math.min(100, Math.max(0, Math.round((ship.fuel_main / ship.fuel_capacity_main) * 100)))
      : 0,
  );
  const fuelResPct = $derived(
    ship?.fuel_reservoir != null && ship?.fuel_capacity_reserve && ship.fuel_capacity_reserve > 0
      ? Math.min(100, Math.max(0, Math.round((ship.fuel_reservoir / ship.fuel_capacity_reserve) * 100)))
      : 0,
  );
</script>

<Panel titleKey="panel-ship">
  {#if !ship || (!ship.ship_type && ship.hull_health == null)}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="ship-header">
      <strong>{ship.ship_type_localised || ship.ship_type || '—'}</strong>
      {#if ship.name || ship.ident}
        <span class="ship-ident">
          {ship.name ?? ''}
          {#if ship.ident}[{ship.ident}]{/if}
        </span>
      {/if}
    </div>

    <div class="ship-rows">
      <div class="row">
        <span>{t('field-hull')}</span>
        <strong class="mono">{pct(ship.hull_health)}</strong>
      </div>

      <div class="fuel-section">
        <div class="row">
          <span>{t('field-fuel')}</span>
          <span class="mono">
            {ship.fuel_main?.toFixed(1) ?? '—'} / {ship.fuel_capacity_main?.toFixed(1) ?? '—'} t
          </span>
        </div>
        <div class="bar-track">
          <div
            class="bar-fill"
            data-testid="fuel-main-fill"
            style="width: {fuelMainPct}%"
          ></div>
        </div>
        {#if ship.fuel_reservoir != null}
          <div class="bar-track reserve-track">
            <div class="bar-fill reserve-fill" style="width: {fuelResPct}%"></div>
          </div>
        {/if}
      </div>

      <div class="row">
        <span>{t('field-cargo')}</span>
        <span class="mono">{ship.cargo_count ?? 0} / {ship.cargo_capacity ?? 0}</span>
      </div>

      <div class="row">
        <span>{t('field-jump-range')}</span>
        <span class="mono">{ly(ship.max_jump_range)}</span>
      </div>

      <div class="row">
        <span>{t('field-rebuy')}</span>
        <span class="mono">{credits(ship.rebuy)}</span>
      </div>

      {#if ship.damaged_modules && ship.damaged_modules.length > 0}
        <div class="damaged-list">
          {#each ship.damaged_modules as modName (modName)}
            <span class="damaged-chip">{modName}</span>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .ship-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.65rem;
    color: var(--hud-primary);
  }

  .ship-ident {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--hud-muted);
  }

  .ship-rows {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .fuel-section {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .bar-track {
    height: 7px;
    background: var(--hud-bg);
    border: 1px solid var(--hud-border);
    border-radius: 2px;
    overflow: hidden;
  }

  .reserve-track {
    height: 4px;
  }

  .bar-fill {
    height: 100%;
    background: var(--hud-primary);
    transition: width 150ms ease;
  }

  .reserve-fill {
    background: var(--hud-secondary);
  }

  .damaged-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.25rem;
  }

  .damaged-chip {
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    background: rgba(255, 43, 43, 0.2);
    border: 1px solid var(--hud-danger);
    color: var(--hud-danger);
  }
</style>
