<script lang="ts">
  import { gameLabel, resolveGameLabel } from '../../lib/gameLabels';

  import { t } from '../../lib/i18n';
  import { credits, decimal, integer } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { ColonisationDepotTopic } from './types';
  import type { SearchTopic } from '../trade/types';

  const depot = $derived(topic<ColonisationDepotTopic>('colonisation.depot'));
  const marketSearch = $derived(topic<SearchTopic>('trade.search'));
  const pendingResources = $derived(
    depot ? depot.resources.filter((r) => r.remaining > 0) : [],
  );

  let copiedItem = $state<string | null>(null);
  let searchedCommodity = $state<string | null>(null);
  let searching = $state(false);

  async function copyResource(name: string) {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(name);
      copiedItem = name;
      setTimeout(() => {
        if (copiedItem === name) copiedItem = null;
      }, 2000);
    }
  }

  function normalizeCommodityKey(raw: string): string {
    const resolved = resolveGameLabel('commodity', raw);
    if (resolved) return resolved.id.toLowerCase();
    return raw
      .trim()
      .replace(/^\$/, '')
      .replace(/;$/, '')
      .replace(/_name$/i, '')
      .toLowerCase()
      .replace(/[^a-z0-9]/g, '');
  }

  async function findSupplyMarket(resourceName: string, localised?: string | null) {
    searchedCommodity = gameLabel('commodity', resourceName, localised);
    searching = true;
    try {
      await action('trade.search', { commodity: normalizeCommodityKey(resourceName) });
    } catch {
      /* Mantiene el panel operativo si la petición falla */
    } finally {
      searching = false;
    }
  }
</script>

<Panel titleKey="panel-colonisation-supply" span={1}>
  {#if !depot || !depot.active_project}
    <p class="muted" data-testid="supply-no-data">{t('colonisation-no-depot')}</p>
  {:else if pendingResources.length === 0}
    <div class="complete-card" data-testid="supply-all-complete">
      <span class="badge badge-green">✔ {t('colonisation-supply-complete')}</span>
      <p class="complete-msg">{t('colonisation-supply-delivered')}</p>
    </div>
  {:else}
    <div class="supply-list" data-testid="supply-list">
      {#each pendingResources as r (r.name)}
        <div class="supply-card" data-testid="supply-card">
          <div class="supply-info">
            <span class="supply-name bold" data-testid="supply-name">
              {gameLabel('commodity', r.name, r.name_localised)}
            </span>
            <span class="supply-detail sub">
              <b class="gold mono">{t('colonisation-resource-missing', { tons: integer(r.remaining), trips: r.trips_remaining })}</b>
            </span>
          </div>

          <div class="supply-actions">
            <button
              type="button"
              class="copy-btn search-market-btn"
              data-testid="find-supply-market-btn"
              disabled={searching}
              onclick={() => findSupplyMarket(r.name, r.name_localised)}
            >
              {t('colonisation-find-market')}
            </button>
            <button
              type="button"
              class="copy-btn"
              data-testid="copy-resource-btn"
              onclick={() => copyResource(gameLabel('commodity', r.name, r.name_localised))}
            >
              {copiedItem === (gameLabel('commodity', r.name, r.name_localised))
                ? t('colonisation-copied')
                : t('ui-copy')}
            </button>
          </div>
        </div>
      {/each}
    </div>

    {#if searchedCommodity !== null || (marketSearch && marketSearch.status !== 'idle')}
      <section class="market-results-box" data-testid="colonisation-market-results">
        <header class="market-results-head">
          <strong>{t('colonisation-market-results-title')}</strong>
          {#if searchedCommodity || marketSearch?.commodity}
            <span class="muted">
              {searchedCommodity ?? gameLabel('commodity', marketSearch?.commodity ?? '', marketSearch?.commodity ?? '')}
            </span>
          {/if}
        </header>

        {#if searching || marketSearch?.status === 'loading'}
          <p class="muted">{t('trade-search-loading')}</p>
        {:else if marketSearch?.status === 'ok' && marketSearch.rows.length > 0}
          <ul class="market-list">
            {#each marketSearch.rows.slice(0, 5) as row, i (row.station_name + row.system_name + i)}
              <li class="market-row">
                <div class="market-row-info">
                  <span class="bold">{row.station_name}</span>
                  <span class="sub mono">
                    {row.system_name} · {t('mining-distance', { distance: decimal(row.distance_ly, 1) })} · {credits(row.buy_price)} · {integer(row.stock)} t{#if row.pad} · {row.pad}{/if}
                  </span>
                </div>
                <button
                  type="button"
                  class="copy-btn"
                  data-testid="copy-market-system-btn"
                  onclick={() => copyResource(row.system_name)}
                >
                  {copiedItem === row.system_name ? t('colonisation-copied') : t('ui-copy')}
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted">{t('colonisation-market-empty')}</p>
        {/if}
      </section>
    {/if}
  {/if}
</Panel>

<style>
  .complete-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    text-align: center;
  }
  .complete-msg {
    margin: 0;
    font-size: 0.85rem;
    color: var(--hud-text);
  }
  .supply-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .supply-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.65rem 0.75rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .supply-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
    flex: 1;
  }
  .supply-actions {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
    align-items: center;
    flex-shrink: 0;
  }
  .supply-name {
    font-size: 0.95rem;
    color: var(--hud-text);
  }
  .supply-detail {
    font-size: 0.75rem;
  }
  .copy-btn {
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .market-results-box {
    margin-top: 0.75rem;
    padding: 0.65rem 0.75rem;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
  }
  .market-results-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.5rem;
    flex-wrap: wrap;
    font-size: 0.82rem;
    margin-bottom: 0.45rem;
  }
  .market-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .market-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    padding-top: 0.35rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }
  .market-row-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
    flex: 1;
    font-size: 0.78rem;
  }
  .sub {
    color: var(--hud-muted);
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
