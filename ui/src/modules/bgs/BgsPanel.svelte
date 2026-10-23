<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import SuperpowerIcon from '../../lib/icons/SuperpowerIcon.svelte';
  import type { BgsSystemTopic, BgsTickTopic } from './types';

  const system = $derived(topic<BgsSystemTopic>('bgs.system'));
  const tick = $derived(topic<BgsTickTopic>('bgs.tick'));

  function isConflictState(state: string | null | undefined): boolean {
    if (!state) return false;
    return ['War', 'CivilWar', 'Election', 'CivilUnrest', 'Lockdown'].includes(state);
  }
</script>

<Panel titleKey="panel-bgs-system" span={2}>
  {#if !system || !system.star_system}
    <p class="muted" data-testid="bgs-no-data">{t('bgs-no-data')}</p>
  {:else}
    <div class="bgs-header" data-testid="bgs-header">
      <div class="identity">
        <span class="system-label mono">{t('bgs-star-system')}</span>
        <h3 class="system-name" data-testid="bgs-system-name">{system.star_system}</h3>
      </div>
      {#if system.controlling_faction}
        <div class="controlling-info">
          <span class="sub">{t('bgs-controlling-faction')}:</span>
          <span class="bold" data-testid="bgs-controlling">{system.controlling_faction}</span>
          {#if system.controlling_faction_state}
            <span class="badge {isConflictState(system.controlling_faction_state) ? 'badge-orange' : 'badge-neutral'}">
              {system.controlling_faction_state}
            </span>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Tick Galáctico (EDCD) -->
    <div class="tick-card" data-testid="bgs-tick-card">
      <div class="tick-info">
        <span class="metric-label">{t('bgs-tick')}</span>
        <span class="tick-hours mono bold" data-testid="bgs-tick-hours">
          {tick?.hours_since_tick != null ? `Hace ${decimal(tick.hours_since_tick, 1)} h` : '—'}
        </span>
      </div>
      {#if tick}
        <span
          class="badge {tick.is_stale ? 'badge-danger' : 'badge-green'}"
          data-testid="bgs-tick-badge"
        >
          {tick.is_stale ? t('bgs-tick-stale') : t('bgs-tick-fresh')}
        </span>
      {/if}
    </div>

    <!-- Facciones del Sistema -->
    <div class="table-wrap">
      <table data-testid="factions-table">
        <thead>
          <tr>
            <th>{t('bgs-col-faction')}</th>
            <th class="meter-head">{t('bgs-col-influence')}</th>
            <th>{t('bgs-col-state')}</th>
            <th>{t('bgs-col-allegiance')}</th>
            <th>{t('bgs-col-government')}</th>
          </tr>
        </thead>
        <tbody>
          {#each system.factions as f (f.name)}
            <tr data-testid="faction-row">
              <td class="bold">
                {f.name}
                {#if f.is_controlling}
                  <span class="badge badge-green mini" data-testid="controlling-badge">CONTROLADORA</span>
                {/if}
              </td>
              <td class="influence-col">
                <div class="influence-meter-group">
                  <div
                    class="hud-meter-track"
                    role="progressbar"
                    aria-valuenow={f.influence_percent}
                    aria-valuemin={0}
                    aria-valuemax={100}
                  >
                    <div
                      class="hud-meter-fill {f.is_controlling ? 'fill-green' : 'fill-cyan'}"
                      style:width="{Math.min(100, Math.max(0, f.influence_percent))}%"
                    ></div>
                  </div>
                  <span class="influence-val mono" data-testid="influence-val">
                    {decimal(f.influence_percent, 1)} %
                  </span>
                </div>
              </td>
              <td>
                <span class="badge {isConflictState(f.state) ? 'badge-danger' : ''}">
                  {f.state ?? 'None'}
                </span>
              </td>
              <td class="sub allegiance-cell">
                {#if f.allegiance}
                  <span class="allegiance-group">
                    <SuperpowerIcon superpower={f.allegiance} size={13} />
                    <span>{f.allegiance}</span>
                  </span>
                {:else}
                  —
                {/if}
              </td>
              <td class="sub">{f.government ?? '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</Panel>

<style>
  .bgs-header {
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
  .system-label {
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    color: var(--hud-muted);
  }
  .system-name {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--hud-text);
  }
  .controlling-info {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
  }
  .tick-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.65rem 0.85rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }
  .tick-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .tick-hours {
    font-size: 1.1rem;
    color: var(--hud-text);
  }
  .meter-head {
    width: 180px;
  }
  .influence-col {
    width: 180px;
  }
  .influence-meter-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .influence-meter-group .hud-meter-track {
    flex: 1;
    height: 7px;
  }
  .influence-val {
    font-size: 0.8rem;
    font-weight: 700;
    min-width: 48px;
    text-align: right;
  }
  .allegiance-group {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }
  .mini {
    font-size: 0.65rem;
    padding: 0.1rem 0.3rem;
    margin-left: 0.35rem;
  }
  .sub {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .bold {
    font-weight: 700;
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
