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
    <p class="muted" data-testid="market-station">{m.station} · {m.system}</p>
    <p class="muted small">{t('trade-sale-window')}</p>
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
                <span class="muted small" data-testid="market-origin">{originLabel(b.origin)}</span>
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
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .small { font-size: 0.8em; }
  .mono { font-family: var(--font-mono); }
  table { border-collapse: collapse; width: 100%; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .n { text-align: right; }
  .best, .warn { color: var(--hud-warning); }
  .ok { color: var(--hud-success, var(--hud-primary)); }
  .tag { font-size: 0.75em; padding: 0 0.3rem; border: 1px solid var(--hud-warning); color: var(--hud-warning); margin-left: 0.3rem; }
</style>
