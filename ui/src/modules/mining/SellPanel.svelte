<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal, relativeAge } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { materialName, namesFrom, type PadSize, type ProspectorTopic, type SellTopic } from './types';

  const s = $derived(topic<SellTopic>('mining.sell'));
  const names = $derived(namesFrom(topic<ProspectorTopic>('mining.prospector'), s?.names));
  let tick = $state(Date.now());
  let skew = $state<number | null>(null);
  // Hora del diario; `null` si no se conoce (se ocultan las antigüedades).
  const now = $derived(skew == null ? null : tick + skew);
  let copiedSystem = $state<string | null>(null);

  // Un único setInterval de 30 s para las antigüedades; no corre sin plan.
  $effect(() => {
    if (s?.status !== 'ok') return;
    skew = journalSkew(s.as_of);
    tick = Date.now();
    const id = setInterval(() => {
      tick = Date.now();
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

  function copySystem(sys: string) {
    if (navigator?.clipboard) {
      navigator.clipboard.writeText(sys).catch(() => {});
      copiedSystem = sys;
      setTimeout(() => {
        if (copiedSystem === sys) copiedSystem = null;
      }, 2000);
    }
  }
</script>

<Panel titleKey="panel-mining-sell" span={1}>
  {#if !s || s.status === 'disabled' || s.status === 'no_data'}
    <p class="muted" data-testid="sell-empty">{t('mining-no-data')}</p>
  {:else if s.status === 'loading'}
    <p class="muted" data-testid="sell-empty">{t('mining-sell-loading')}</p>
  {:else if plan}
    {#if plan.best_station_total}
      <div class="best-card" data-testid="sell-best">
        <span class="best-icon" aria-hidden="true">★</span>
        <div class="best-info">
          {t('mining-best-total', { credits: credits(plan.best_station_total[1]), station: plan.best_station_total[0] })}
        </div>
      </div>
    {/if}

    {#each groups as [key, opts] (key)}
      <div class="commodity-header">
        <h3 data-testid="sell-commodity">{materialName(key, names)}</h3>
      </div>
      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th>{t('mining-col-station')}</th><th>{t('mining-col-distance')}</th><th>{t('mining-col-price')}</th>
              <th>{t('mining-col-demand')}</th><th>{t('mining-col-pad')}</th><th>{t('mining-col-age')}</th>
            </tr>
          </thead>
          <tbody>
            {#each opts as o, i (o.station_name + o.system_name + i)}
              <tr class="option-row" data-testid="sell-option">
                <td>
                  <span class="station-cell">
                    <b class:best={i === 0}>{o.station_name}</b> · {o.system_name}
                    <button
                      type="button"
                      class="btn-copy"
                      onclick={() => copySystem(o.system_name)}
                      title="Copiar sistema"
                      aria-label="Copiar {o.system_name}"
                    >{copiedSystem === o.system_name ? '✓' : '⎘'}</button>
                  </span>
                </td>
                <td class="mono">{t('mining-distance', { distance: decimal(o.distance_ly, 1) })}</td>
                <td class="mono price-val">{credits(o.sell_price)}</td>
                <td class="mono">{o.demand == null ? '—' : t('mining-demand', { demand: decimal(o.demand, 0) })}</td>
                <td>
                  <span class="pad-badge">{padLabel(o.pad)}</span>
                  {#if o.pad_ok === null}<span class="warn" data-testid="pad-warning" role="img" aria-label={t('mining-pad-unknown')} title={t('mining-pad-unknown')}>⚠</span>{/if}
                </td>
                <td class="mono muted age-cell">{relativeAge(o.updated_at, now)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/each}
    <p class="muted source-footnote" data-testid="sell-source">
      {t('mining-source', { source: sourceLabel(plan.source) })}{#if plan.fetched_at && now != null} · {relativeAge(plan.fetched_at, now)}{/if}
    </p>
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.3rem 0; font-size: 0.85rem; }
  .mono { font-family: var(--font-mono); }
  .best-card {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.55rem 0.85rem;
    background: rgba(255, 170, 0, 0.08);
    border: 1px solid rgba(255, 170, 0, 0.3);
    border-radius: var(--radius-sm);
    margin-bottom: 0.75rem;
    font-size: 0.92rem;
    color: var(--hud-text);
  }
  .best-icon {
    color: var(--hud-warning);
    font-size: 1.1rem;
    filter: drop-shadow(0 0 6px rgba(255, 170, 0, 0.5));
  }
  .commodity-header {
    margin: 0.65rem 0 0.35rem;
    padding-bottom: 0.2rem;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
    color: var(--hud-primary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
  }
  .table-container {
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
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .station-cell {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  .btn-copy {
    background: transparent;
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    color: var(--hud-muted);
    cursor: pointer;
    padding: 0.1rem 0.35rem;
    font-size: 0.75rem;
    line-height: 1;
    transition: all var(--transition-fast);
  }
  .btn-copy:hover {
    color: var(--hud-primary);
    border-color: var(--hud-primary);
    background: rgba(255, 119, 0, 0.1);
  }
  .best {
    color: var(--hud-warning);
  }
  .price-val {
    color: var(--hud-cyan);
    font-weight: 600;
  }
  .pad-badge {
    background: rgba(255, 255, 255, 0.05);
    padding: 0.1rem 0.35rem;
    border-radius: 2px;
    font-size: 0.8rem;
  }
  .warn {
    color: var(--hud-warning);
    margin-left: 0.25rem;
  }
  .source-footnote {
    margin-top: 0.6rem;
    font-size: 0.78rem;
  }
</style>
