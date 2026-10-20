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
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .small { font-size: 0.8em; }
  .mono { font-family: var(--font-mono); }
  .sub { margin-top: 0.2rem; }
  table { border-collapse: collapse; width: 100%; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .n { text-align: right; }
  .bar { height: 0.9rem; background: var(--hud-surface-alt); border: 1px solid var(--hud-border); position: relative; margin: 0.2rem 0 0.6rem; }
  .bar i { position: absolute; left: 0; top: 0; bottom: 0; background: var(--hud-primary); }
  .warn { color: var(--hud-warning); }
  .ok { color: var(--hud-success, var(--hud-primary)); }
  .tag { font-size: 0.75em; padding: 0 0.3rem; border: 1px solid currentColor; margin-left: 0.3rem; }
  .tag.bad { color: var(--hud-danger); }
  .tag.info { color: var(--hud-info, var(--hud-muted)); }
  .badge { border: 1px solid currentColor; padding: 0 0.4rem; font-size: 0.85em; }
</style>
