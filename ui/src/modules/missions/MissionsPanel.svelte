<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { createNow } from './now.svelte';
  import { countdown, isExpiringSoon } from './time';
  import { kindLabel, progressLabel } from './labels';
  import type { ActiveTopic } from './types';

  const a = $derived(topic<ActiveTopic>('missions.active'));
  const groups = $derived(a?.groups ?? []);
  const clock = createNow(
    () => a?.as_of,
    () => groups.flatMap((g) => g.missions.map((m) => m.expiry)),
  );

  let selectedGroupIndex = $state<number>(0);
  let showAllGroups = $state(false);

  function toRoute(system: string) {
    void action('missions.to_route', { system }).catch(() => {});
  }
</script>

<Panel titleKey="panel-missions-active" span={2}>
  {#if !a || groups.length === 0}
    <p class="muted" data-testid="missions-empty">{t('missions-none-active')}</p>
  {:else}
    {#if groups.length > 1}
      <div class="missions-controls">
        <label class="compact-selector-label" for="mission-dest-select">
          <span class="label-text">Destino ({groups.length}):</span>
          <select
            id="mission-dest-select"
            class="glass-select"
            data-testid="mission-dest-select"
            bind:value={selectedGroupIndex}
          >
            {#each groups as g, gi}
              <option value={gi}>
                {g.system ?? t('missions-no-destination')}{g.station ? ` (${g.station})` : ''} · {g.missions.length} mis.{g.next ? ' ★' : ''}
              </option>
            {/each}
          </select>
        </label>
        <button
          type="button"
          class="glass-btn-secondary list-toggle-btn"
          data-testid="toggle-missions-view"
          onclick={() => (showAllGroups = !showAllGroups)}
        >
          {showAllGroups ? '▲ Vista compacta' : `▼ Ver todos (${groups.length})`}
        </button>
      </div>
    {/if}

    {#each groups as g, gi (`${g.system}|${g.station}|${gi}`)}
      <div
        class="dest"
        class:next={g.next}
        class:is-hidden={!showAllGroups && groups.length > 1 && gi !== selectedGroupIndex}
        data-testid="missions-group"
      >
        <h5>
          {#if g.next}<span class="warn" aria-hidden="true">▶</span>{/if}
          <b class:warn={g.next}>{g.system ?? t('missions-no-destination')}</b>
          {#if g.station}· {g.station}{/if}
          {#if g.distance_ly != null}<span class="muted">· {t('mining-distance', { distance: decimal(g.distance_ly, 1) })}</span>{/if}
          {#if g.next && g.system}
            <span class="muted">· {t('missions-next-stop', { destination: g.system })}</span>
          {/if}
          {#if g.system}
            <button type="button" class="btn-route" data-testid="mission-to-route" onclick={() => toRoute(g.system!)}>{t('missions-add-to-route')}</button>
          {/if}
        </h5>
        <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t('missions-col-type')}</th><th>{t('missions-col-faction')}</th><th>{t('missions-col-progress')}</th>
              <th class="n">{t('missions-col-expiry')}</th><th class="n">{t('missions-col-reward')}</th>
            </tr>
          </thead>
          <tbody>
            {#each g.missions as m (m.id)}
              <tr data-testid="mission-row">
                <td>
                  {kindLabel(m.kind)}
                  {#if m.redirected}<span class="tag warn">{t('missions-redirected')}</span>{/if}
                </td>
                <td>{m.faction ?? '—'}</td>
                <td>
                  {#if m.details_missing}
                    <span class="muted" data-testid="mission-details-missing">{t('missions-details-missing')}</span>
                  {:else}
                    {progressLabel(m)}
                  {/if}
                </td>
                <td class="n mono">
                  {#if m.expiry}
                    <span
                      data-testid="mission-countdown"
                      class:expiring-soon={isExpiringSoon(m.expiry, clock.ms)}
                      title={t('missions-expires-in', { remaining: countdown(m.expiry, clock.ms) })}
                    >{countdown(m.expiry, clock.ms)}</span>
                  {:else}—{/if}
                </td>
                <td class="n mono">{m.reward == null ? '—' : credits(m.reward)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        </div>
      </div>
    {/each}
  {/if}
</Panel>

<style>
  .muted {
    color: var(--hud-muted);
    margin: 0.2rem 0;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .dest {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--hud-border-subtle);
    border-left: 3px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.65rem 0.85rem;
    margin: 0.65rem 0;
    transition: all var(--transition-fast);
  }
  .dest.is-hidden {
    display: none;
  }
  .missions-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.6rem;
    flex-wrap: wrap;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .compact-selector-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--hud-muted);
    flex: 1;
    min-width: 220px;
  }
  .compact-selector-label .label-text {
    white-space: nowrap;
    text-transform: uppercase;
    font-size: 0.72rem;
    letter-spacing: 0.05em;
  }
  .glass-select {
    flex: 1;
    background: rgba(10, 16, 26, 0.9);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.3));
    color: var(--hud-primary, #00e5ff);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
    outline: none;
    cursor: pointer;
  }
  .list-toggle-btn {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.25));
    color: var(--hud-text, #c9d7e5);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .list-toggle-btn:hover {
    background: rgba(0, 229, 255, 0.12);
    border-color: var(--hud-primary, #00e5ff);
    color: var(--hud-primary, #00e5ff);
  }
  .dest.next {
    border-left-color: var(--hud-primary);
    background: rgba(255, 119, 0, 0.04);
    box-shadow: 0 0 12px rgba(255, 119, 0, 0.1);
  }
  h5 {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.5rem;
    font-size: 0.95rem;
    color: var(--hud-text);
  }
  .btn-route {
    margin-left: auto;
    background: rgba(255, 119, 0, 0.1);
    border: 1px solid rgba(255, 119, 0, 0.35);
    border-radius: 2px;
    color: var(--hud-primary);
    padding: 0.2rem 0.6rem;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
  }
  .btn-route:hover {
    background: rgba(255, 119, 0, 0.2);
    border-color: var(--hud-primary);
    box-shadow: 0 0 8px rgba(255, 119, 0, 0.3);
  }
  .table-wrap {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  th, td {
    text-align: left;
    padding: 0.3rem 0.6rem 0.3rem 0;
    white-space: nowrap;
    font-size: 0.88rem;
  }
  th {
    color: var(--hud-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    border-bottom: 1px solid var(--hud-border-subtle);
    padding-bottom: 0.35rem;
  }
  .n {
    text-align: right;
  }
  .warn {
    color: var(--hud-warning);
  }
  .expiring-soon {
    color: var(--hud-danger);
    font-weight: 700;
    background: rgba(255, 68, 68, 0.12);
    padding: 0.1rem 0.4rem;
    border-radius: 2px;
    border: 1px solid rgba(255, 68, 68, 0.3);
    box-shadow: 0 0 8px rgba(255, 68, 68, 0.25);
  }
  .tag {
    font-size: 0.72rem;
    padding: 0.05rem 0.35rem;
    border: 1px solid currentColor;
    border-radius: 2px;
    margin-left: 0.3rem;
    text-transform: uppercase;
  }
</style>
