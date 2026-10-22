<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal, relativeAge } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { padLabel, originLabel } from './labels';
  import { commodityName, type MarketTopic } from './types';

  const m = $derived(topic<MarketTopic>('trade.market'));
  let tick = $state(Date.now());
  let skew = $state<number | null>(null);
  // Hora del diario; `null` si no se conoce (se ocultan las antigüedades).
  const now = $derived(skew == null ? null : tick + skew);
  const rows = $derived(m?.rows ?? []);

  // Un único setInterval de 30 s para las antigüedades; no corre sin filas.
  $effect(() => {
    if (!m || m.rows.length === 0) return;
    skew = journalSkew(m.as_of);
    tick = Date.now();
    const id = setInterval(() => {
      tick = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });
</script>

<Panel titleKey="panel-trade-market" span={2}>
  {#if !m || rows.length === 0}
    <p class="muted" data-testid="market-empty">{t('trade-no-data')}</p>
  {:else}
    <div class="header-card">
      <span class="station-title" data-testid="market-station">{m.station} · {m.system}</span>
      <span class="window-note">{t('trade-sale-window')}</span>
    </div>
    <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>{t('trade-col-commodity')}</th><th class="n">{t('trade-col-buy')}</th><th class="n">{t('trade-col-stock')}</th>
          <th>{t('trade-col-sell')}</th><th class="n">{t('trade-col-distance')}</th>
          <th class="n">{t('trade-col-profit-per-ton')}</th><th>{t('trade-col-age')}</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as r, i (r.commodity + i)}
          {@const b = r.best_sell}
          <tr data-testid="market-row">
            <td>
              <b class:best={i === 0}>{commodityName(r.commodity, r.name_localised)}</b>
              {#if r.rare != null}<span class="tag" data-testid="rare-badge">{t('trade-rare')}</span>{/if}
            </td>
            <td class="n mono">{credits(r.buy_price)}</td>
            <td class="n mono">{decimal(r.stock, 0)} t</td>
            {#if b}
              <td>
                {b.station_name} · {b.system_name} · <span data-testid="pad-label">{padLabel(b.pad)}</span>
                {#if b.pad_ok === null}<span class="warn" data-testid="pad-warning" role="img" aria-label={t('trade-pad-unknown')} title={t('trade-pad-unknown')}>⚠</span>{/if}
                <span class="muted origin-tag" data-testid="market-origin">{originLabel(b.origin)}</span>
              </td>
              <td class="n mono">{t('mining-distance', { distance: decimal(b.distance_ly, 1) })}</td>
            {:else if r.rare != null}
              <td class="muted">{t('trade-rare-distance', { distance: decimal(r.rare, 0) })}</td>
              <td class="n mono">—</td>
            {:else}
              <td class="muted">—</td><td class="n mono">—</td>
            {/if}
            <td class="n mono ok" data-testid="market-profit">{r.profit_per_ton == null ? '—' : credits(r.profit_per_ton)}</td>
            <td class="mono muted">{b ? relativeAge(b.updated_at, now) : '—'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    </div>
  {/if}
</Panel>

<style>
  .muted {
    color: var(--hud-muted);
    margin: 0.2rem 0;
  }
  .header-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.45rem 0.75rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    margin-bottom: 0.65rem;
  }
  .station-title {
    font-size: 0.95rem;
    color: var(--hud-primary);
    font-weight: 600;
  }
  .window-note {
    font-size: 0.78rem;
    color: var(--hud-muted);
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
    padding: 0.35rem 0.6rem 0.35rem 0;
    white-space: nowrap;
    font-size: 0.88rem;
  }
  th {
    color: var(--hud-muted);
    font-size: 0.76rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    border-bottom: 1px solid var(--hud-border-subtle);
    padding-bottom: 0.4rem;
  }
  .n {
    text-align: right;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .best {
    color: var(--hud-warning);
  }
  .warn {
    color: var(--hud-warning);
    margin-left: 0.2rem;
  }
  .ok {
    color: var(--hud-ok);
    font-weight: 600;
  }
  .tag {
    font-size: 0.72rem;
    padding: 0.1rem 0.4rem;
    border: 1px solid var(--hud-warning);
    background: rgba(255, 170, 0, 0.1);
    color: var(--hud-warning);
    border-radius: 2px;
    margin-left: 0.35rem;
    letter-spacing: 0.04em;
    font-weight: 600;
  }
  .origin-tag {
    font-size: 0.72rem;
    background: rgba(255, 255, 255, 0.05);
    padding: 0.05rem 0.35rem;
    border-radius: 2px;
    margin-left: 0.3rem;
  }
</style>
