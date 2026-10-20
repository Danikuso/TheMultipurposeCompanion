<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, relativeAge } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { cr } from './labels';
  import { commodityName, type RouteTopic } from './types';

  const r = $derived(topic<RouteTopic>('trade.route'));
  let tick = $state(Date.now());
  let skew = $state<number | null>(null);
  // Hora del diario; `null` si no se conoce (se ocultan las antigüedades).
  const now = $derived(skew == null ? null : tick + skew);
  let failed = $state(false);

  const plan = $derived(r?.status === 'ok' ? r.plan : null);

  // Un único setInterval de 30 s para las antigüedades; no corre sin plan.
  $effect(() => {
    if (!r || r.status !== 'ok' || !r.plan) return;
    skew = journalSkew(r.as_of);
    tick = Date.now();
    const id = setInterval(() => {
      tick = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  async function recalc() {
    failed = false;
    try {
      await action('trade.route_request', {});
    } catch {
      failed = true;
    }
  }
</script>

<Panel titleKey="panel-trade-route" span={2}>
  {#if !r || r.status === 'disabled'}
    <p class="muted" data-testid="route-empty">{t('trade-no-data')}</p>
  {:else}
    {#if r.status === 'blocked'}
      <p class="warn" data-testid="route-dock-first">{t('trade-route-dock-first')}</p>
    {:else if r.status === 'loading'}
      <p class="muted" data-testid="route-state">{t('trade-route-loading')}</p>
    {:else if r.status === 'no_data'}
      <p class="muted" data-testid="route-state">{t('trade-route-no-data')}</p>
    {:else if r.status === 'idle'}
      <p class="muted" data-testid="route-state">{t('trade-route-idle')}</p>
    {:else if plan}
      <p class="muted" data-testid="route-progress">
        {r.completed ? t('trade-route-done') : t('trade-route-stop', { current: r.current + 1, stops: plan.stops.length })}
      </p>
      <ol class="stops">
        {#each plan.stops as s, i (i)}
          {@const tons = s.commodities.reduce((n, c) => n + c.amount, 0)}
          <li class="hop" class:current={!r.completed && i === r.current} class:done={r.completed || i < r.current} data-testid="route-stop">
            <span class="k" aria-hidden="true">{i < r.current || r.completed ? '✓' : i + 1}</span>
            <div>
              <div>
                <b>{t('trade-route-buy', { tons: decimal(tons, 0), station: s.from_station })}</b>
                → {t('trade-route-sell', { station: s.to_station })}
                <span class="muted">· {s.to_system} · {t('mining-distance', { distance: decimal(s.distance_ly, 1) })}</span>
              </div>
              <div class="muted">
                {#each s.commodities as c, j (c.commodity + j)}
                  {#if j > 0}, {/if}{commodityName(c.commodity, c.name_localised)} ({t('trade-route-per-ton', { profit: cr(c.profit_per_ton) })})
                {/each}
                · {relativeAge(s.data_updated_at, now)}
              </div>
            </div>
            <span class="ok mono" data-testid="route-stop-profit">{cr(s.stop_profit)}</span>
          </li>
        {/each}
      </ol>
      <p data-testid="route-total"><b>{t('trade-route-total', { profit: cr(plan.total_profit) })}</b></p>
    {/if}
    {#if r.status !== 'blocked'}
      <button type="button" data-testid="route-recalculate" onclick={recalc}>{t('trade-recalculate')}</button>
    {/if}
    {#if failed}<p class="err" role="alert">{t('trade-route-failed')}</p>{/if}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .mono { font-family: var(--font-mono); }
  .warn { color: var(--hud-warning); }
  .ok { color: var(--hud-success, var(--hud-primary)); }
  .err { color: var(--hud-danger); }
  .stops { list-style: none; margin: 0; padding: 0; }
  .hop { display: grid; grid-template-columns: 1.7rem 1fr auto; gap: 0.5rem; align-items: center; padding: 0.35rem 0; border-bottom: 1px dashed var(--hud-border); }
  .hop .k { width: 1.5rem; height: 1.5rem; border: 2px solid var(--hud-border); display: flex; align-items: center; justify-content: center; font-weight: bold; }
  .hop.current .k { border-color: var(--hud-warning); color: var(--hud-warning); }
  .hop.current { background: var(--hud-surface-alt); }
  .hop.done { opacity: 0.55; }
</style>
