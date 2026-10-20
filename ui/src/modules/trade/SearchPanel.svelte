<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal, relativeAge } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { originLabel, padLabel } from './labels';
  import { commodityName, type CommoditiesTopic, type SearchTopic } from './types';

  const s = $derived(topic<SearchTopic>('trade.search'));
  const items = $derived(topic<CommoditiesTopic>('trade.commodities')?.items ?? []);
  let text = $state('');
  let failed = $state(false);
  let tick = $state(Date.now());
  let skew = $state<number | null>(null);
  // Hora del diario; `null` si no se conoce (se ocultan las antigüedades).
  const now = $derived(skew == null ? null : tick + skew);
  const rows = $derived(s?.status === 'ok' ? s.rows : []);

  // Un único setInterval de 30 s para las antigüedades; no corre sin resultados.
  $effect(() => {
    if (!s || s.status !== 'ok' || s.rows.length === 0) return;
    skew = journalSkew(s.as_of);
    tick = Date.now();
    const id = setInterval(() => {
      tick = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  /** Clave de la mercancía escrita: por nombre localizado, por clave o tal cual. */
  function keyFor(raw: string): string {
    const v = raw.trim().toLowerCase();
    const hit = items.find(([k, n]) => (n ?? '').toLowerCase() === v || k.toLowerCase() === v);
    return hit ? hit[0] : v;
  }

  async function search(e?: Event) {
    e?.preventDefault();
    const commodity = keyFor(text);
    if (!commodity) return;
    failed = false;
    try {
      await action('trade.search', { commodity });
    } catch {
      failed = true;
    }
  }
</script>

<Panel titleKey="panel-trade-search" span={1}>
  <form onsubmit={search}>
    <label class="muted" for="trade-search-input">{t('trade-search-label')}</label>
    <input
      id="trade-search-input"
      data-testid="search-input"
      list="trade-commodities"
      placeholder={t('trade-search-placeholder')}
      bind:value={text}
    />
    <datalist id="trade-commodities">
      {#each items as [key, name] (key)}
        <option value={commodityName(key, name)}></option>
      {/each}
    </datalist>
    <button type="submit" data-testid="search-go">{t('panel-trade-search')}</button>
  </form>
  {#if failed}<p class="err" role="alert">{t('trade-route-failed')}</p>{/if}
  {#if !s || s.status === 'idle'}
    <!-- Sin búsqueda activa. -->
  {:else if s.status === 'loading'}
    <p class="muted" data-testid="search-state">{t('trade-search-loading')}</p>
  {:else if s.status === 'no_data'}
    <p class="muted" data-testid="search-state">{t('trade-no-data')}</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>{t('trade-col-station')}</th><th class="n">{t('trade-col-distance')}</th><th class="n">{t('trade-col-buy')}</th>
          <th class="n">{t('trade-col-stock')}</th><th>{t('trade-col-age')}</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as o, i (o.station_name + o.system_name + i)}
          <tr data-testid="search-row">
            <td>
              <b class:best={i === 0}>{o.station_name}</b> · {o.system_name} · {padLabel(o.pad)}
              {#if o.pad_ok === null}<span class="warn" data-testid="pad-warning" role="img" aria-label={t('trade-pad-unknown')} title={t('trade-pad-unknown')}>⚠</span>{/if}
            </td>
            <td class="n mono">{t('mining-distance', { distance: decimal(o.distance_ly, 1) })}</td>
            <td class="n mono">{credits(o.buy_price)}</td>
            <td class="n mono">{decimal(o.stock, 0)} t</td>
            <td class="mono muted">{relativeAge(o.updated_at, now)} · {originLabel(o.origin)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.3rem 0; }
  .mono { font-family: var(--font-mono); }
  .err { color: var(--hud-danger); }
  input { width: 14rem; }
  table { border-collapse: collapse; width: 100%; margin-top: 0.4rem; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .n { text-align: right; }
  .best, .warn { color: var(--hud-warning); }
</style>
