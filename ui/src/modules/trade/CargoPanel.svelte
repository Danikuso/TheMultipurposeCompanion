<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, cssPct, decimal } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { padLabel, originLabel } from './labels';
  import { commodityName, type CargoTopic } from './types';

  const c = $derived(topic<CargoTopic>('trade.cargo'));
  const lines = $derived(c?.lines ?? []);
  const holdPct = $derived(c && c.capacity ? Math.min(100, (c.used / c.capacity) * 100) : 0);
</script>

<Panel titleKey="panel-trade-cargo" span={2}>
  {#if !c || (lines.length === 0 && !c.used)}
    <p class="muted" data-testid="cargo-empty">{t('trade-no-data')}</p>
  {:else}
    <div class="sub" data-testid="hold">
      {#if c.capacity}<b>{t('trade-cargo-capacity', { used: decimal(c.used, 0), capacity: decimal(c.capacity, 0) })}</b>{:else}<b>{decimal(c.used, 0)} t</b>{/if}
    </div>
    <div
      class="bar"
      role="progressbar"
      data-testid="hold-bar"
      aria-label={t('trade-cargo-aria')}
      aria-valuemin="0"
      aria-valuemax={c.capacity ?? c.used}
      aria-valuenow={c.used}
    ><i style:width={cssPct(holdPct)}></i></div>
    <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>{t('trade-col-commodity')}</th><th class="n">{t('trade-col-tons')}</th><th class="n">{t('trade-col-paid')}</th>
          <th class="n">{t('trade-col-price-here')}</th><th>{t('trade-col-best-sell')}</th>
          <th class="n">{t('trade-col-profit')}</th><th></th>
        </tr>
      </thead>
      <tbody>
        {#each lines as l, i (l.commodity + i)}
          {@const b = l.best_nearby}
          <tr data-testid="cargo-line">
            <td>
              {commodityName(l.commodity, l.name_localised)}
              {#if l.kind === 'Mission'}<span class="tag info">{t('trade-mission-cargo')}</span>{/if}
              {#if l.kind === 'Stolen'}<span class="tag bad">{t('trade-stolen-cargo')}</span>{/if}
            </td>
            <td class="n mono">{l.tons}</td>
            <td class="n mono">
              {l.paid_avg == null ? '—' : credits(l.paid_avg)}
              {#if l.paid_partial}<span class="warn" role="img" aria-label={t('trade-paid-partial')} title={t('trade-paid-partial')}>⚠</span>{/if}
            </td>
            <td class="n mono">{l.price_here == null ? '—' : credits(l.price_here)}</td>
            <td>
              {#if l.kind === 'Stolen'}
                <span class="muted">{t('trade-black-market-only')}</span>
              {:else if b}
                {b.station_name} · {t('mining-distance', { distance: decimal(b.distance_ly, 1) })} · {padLabel(b.pad)}
                {#if b.pad_ok === null}<span class="warn" data-testid="pad-warning" role="img" aria-label={t('trade-pad-unknown')} title={t('trade-pad-unknown')}>⚠</span>{/if}
                · {credits(b.sell_price)}
                <span class="muted small">{originLabel(b.origin)}</span>
              {:else}
                <span class="muted">—</span>
              {/if}
            </td>
            <td class="n mono ok" data-testid="cargo-profit">{l.kind === 'Trade' && l.est_profit != null ? credits(l.est_profit) : '—'}</td>
            <td>
              {#if l.kind === 'Trade' && l.advice === 'SellHere'}<span class="badge ok">{t('trade-sell-here')}</span>
              {:else if l.kind === 'Trade' && l.advice === 'Carry'}<span class="badge">{t('trade-carry')}</span>{/if}
            </td>
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
  .small {
    font-size: 0.78rem;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .sub {
    margin-top: 0.2rem;
    font-size: 0.92rem;
    color: var(--hud-text);
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
  .bar {
    height: 0.7rem;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    position: relative;
    margin: 0.35rem 0 0.75rem;
    overflow: hidden;
  }
  .bar i {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: linear-gradient(90deg, #ff9933, var(--hud-primary));
    box-shadow: 0 0 6px rgba(255, 119, 0, 0.4);
    border-radius: 1px;
    transition: width var(--transition-fast);
  }
  .warn {
    color: var(--hud-warning);
    margin-left: 0.25rem;
  }
  .ok {
    color: var(--hud-ok);
    font-weight: 600;
  }
  .tag {
    font-size: 0.72rem;
    padding: 0.1rem 0.4rem;
    border: 1px solid currentColor;
    border-radius: 2px;
    margin-left: 0.35rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
  }
  .tag.bad {
    color: var(--hud-danger);
    background: rgba(255, 68, 68, 0.1);
  }
  .tag.info {
    color: var(--hud-cyan);
    background: rgba(0, 217, 255, 0.1);
  }
  .badge {
    display: inline-block;
    border: 1px solid currentColor;
    border-radius: var(--radius-sm);
    padding: 0.15rem 0.5rem;
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .badge.ok {
    color: var(--hud-ok);
    background: rgba(68, 255, 136, 0.1);
    box-shadow: 0 0 8px rgba(68, 255, 136, 0.2);
  }
  .badge:not(.ok) {
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.1);
    box-shadow: 0 0 8px rgba(255, 170, 0, 0.2);
  }
</style>
