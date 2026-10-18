<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal, relativeAge } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { materialName, namesFrom, type PadSize, type ProspectorTopic, type SellTopic } from './types';

  const s = $derived(topic<SellTopic>('mining.sell'));
  const names = $derived(namesFrom(topic<ProspectorTopic>('mining.prospector')));
  let now = $state(Date.now());

  // Un único setInterval de 30 s para las antigüedades; no corre sin plan.
  $effect(() => {
    if (s?.status !== 'ok') return;
    now = Date.now();
    const id = setInterval(() => {
      now = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  const plan = $derived(s?.status === 'ok' ? s : null);
  const groups = $derived(
    plan ? Object.entries(plan.by_commodity).filter(([, opts]) => opts.length > 0) : [],
  );

  function padLabel(p: PadSize | null): string {
    return p === 'Small' ? t('mining-pad-small') : p === 'Medium' ? t('mining-pad-medium') : p === 'Large' ? t('mining-pad-large') : '?';
  }
  function sourceLabel(src: string): string {
    return src === 'local_sell'
      ? t('mining-source-local-sell')
      : src === 'local_market'
        ? t('mining-source-local-market')
        : t('mining-source-remote');
  }
</script>

<Panel titleKey="panel-mining-sell" span={1}>
  {#if !s || s.status === 'disabled' || s.status === 'no_data'}
    <p class="muted" data-testid="sell-empty">{t('mining-no-data')}</p>
  {:else if s.status === 'loading'}
    <p class="muted" data-testid="sell-empty">{t('mining-sell-loading')}</p>
  {:else if plan}
    {#each groups as [key, opts] (key)}
      <h3 data-testid="sell-commodity">{materialName(key, names)}</h3>
      <table>
        <thead>
          <tr>
            <th>{t('mining-col-station')}</th><th>{t('mining-col-distance')}</th><th>{t('mining-col-price')}</th>
            <th>{t('mining-col-demand')}</th><th>{t('mining-col-pad')}</th><th>{t('mining-col-age')}</th>
          </tr>
        </thead>
        <tbody>
          {#each opts as o, i (o.station_name + o.system_name + i)}
            <tr data-testid="sell-option">
              <td><b class:best={i === 0}>{o.station_name}</b> · {o.system_name}</td>
              <td class="mono">{t('mining-distance', { distance: decimal(o.distance_ly, 1) })}</td>
              <td class="mono">{credits(o.sell_price)}</td>
              <td class="mono">{o.demand == null ? '—' : t('mining-demand', { demand: decimal(o.demand, 0) })}</td>
              <td>
                {padLabel(o.pad)}
                {#if o.pad_ok === null}<span class="warn" data-testid="pad-warning" role="img" aria-label={t('mining-pad-unknown')} title={t('mining-pad-unknown')}>⚠</span>{/if}
              </td>
              <td class="mono muted">{relativeAge(o.updated_at, now)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/each}
    <p class="muted" data-testid="sell-source">
      {t('mining-source', { source: sourceLabel(plan.source) })} · {relativeAge(plan.fetched_at, now)}
    </p>
    {#if plan.best_station_total}
      <p data-testid="sell-best">{t('mining-best-total', { credits: credits(plan.best_station_total[1]), station: plan.best_station_total[0] })}</p>
    {/if}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.3rem 0; }
  .mono { font-family: var(--font-mono); }
  h3 { margin: 0.5rem 0 0.2rem; font-size: 0.9rem; color: var(--hud-muted); }
  table { border-collapse: collapse; width: 100%; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .best { color: var(--hud-warning); }
  .warn { color: var(--hud-warning); }
</style>
