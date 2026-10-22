<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal, relativeAge } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
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

  // Aviso efímero de «siguiente sistema copiado»: un único timeout de 5 s, sin temporizador permanente.
  let copied = $state<string | null>(null);
  let seenSeq: number | null = null;
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (!r) return;
    const seq = r.copy_seq;
    if (seenSeq === null) {
      seenSeq = seq;
      return;
    }
    if (seq > seenSeq) {
      seenSeq = seq;
      copied = r.copy_target;
      clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => {
        copied = null;
      }, 5000);
    }
  });
  $effect(() => () => clearTimeout(copiedTimer));

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
      <p class="muted progress-head" data-testid="route-progress">
        {r.completed ? t('trade-route-done') : t('trade-route-stop', { current: r.current + 1, stops: plan.stops.length })}
      </p>
      <ol class="stops">
        {#each plan.stops as s, i (i)}
          {@const tons = s.commodities.reduce((n, c) => n + c.amount, 0)}
          <li class="hop" class:current={!r.completed && i === r.current} class:done={r.completed || i < r.current} data-testid="route-stop" aria-current={!r.completed && i === r.current ? 'step' : undefined}>
            <span class="k" aria-hidden="true">{i < r.current || r.completed ? '✓' : i + 1}</span>
            <div>
              <div>
                <b>{t('trade-route-buy', { tons: decimal(tons, 0), station: s.from_station })}</b>
                → {t('trade-route-sell', { station: s.to_station })}
                <span class="muted">· {s.to_system} · {t('mining-distance', { distance: decimal(s.distance_ly, 1) })}{#if s.jumps != null} · {t('trade-route-jumps', { jumps: s.jumps })}{/if}</span>
              </div>
              <div class="muted">
                {#each s.commodities as c, j (c.commodity + j)}
                  {#if j > 0}, {/if}{commodityName(c.commodity, c.name_localised)} ({t('trade-route-per-ton', { profit: credits(c.profit_per_ton) })})
                {/each}
                · {relativeAge(s.data_updated_at, now)}
              </div>
            </div>
            <span class="ok mono" data-testid="route-stop-profit">{credits(s.stop_profit)}</span>
          </li>
        {/each}
      </ol>
      <div class="total-card" data-testid="route-total"><b>{t('trade-route-total', { profit: credits(plan.total_profit) })}</b></div>
    {/if}
    {#if r.status !== 'blocked'}
      <button type="button" class="btn-recalc" data-testid="route-recalculate" disabled={r.status === 'loading'} onclick={recalc}>{t('trade-recalculate')}</button>
    {/if}
    {#if copied}<p class="copied-toast" role="status" data-testid="route-copied">{t('trade-route-copied', { system: copied })}</p>{/if}
    {#if failed}<p class="err" role="alert">{t('trade-route-failed')}</p>{/if}
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
  .warn {
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.08);
    border-left: 3px solid var(--hud-warning);
    padding: 0.45rem 0.75rem;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }
  .ok {
    color: var(--hud-ok);
    font-weight: 600;
  }
  .err {
    color: var(--hud-danger);
    margin-top: 0.4rem;
  }
  .progress-head {
    font-size: 0.85rem;
    margin-bottom: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stops {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .hop {
    display: grid;
    grid-template-columns: 2rem 1fr auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.6rem 0.8rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }
  .hop .k {
    width: 1.8rem;
    height: 1.8rem;
    border: 1px solid var(--hud-border-subtle);
    background: rgba(0, 0, 0, 0.3);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.82rem;
    color: var(--hud-muted);
  }
  .hop.current {
    background: rgba(255, 119, 0, 0.06);
    border-color: rgba(255, 119, 0, 0.4);
    box-shadow: 0 0 12px rgba(255, 119, 0, 0.15);
  }
  .hop.current .k {
    border-color: var(--hud-primary);
    background: rgba(255, 119, 0, 0.2);
    color: var(--hud-primary);
    box-shadow: 0 0 8px rgba(255, 119, 0, 0.4);
  }
  .hop.done {
    opacity: 0.55;
  }
  .hop.done .k {
    border-color: var(--hud-ok);
    color: var(--hud-ok);
  }
  .total-card {
    margin: 0.75rem 0 0.5rem;
    padding: 0.5rem 0.85rem;
    background: rgba(0, 217, 255, 0.05);
    border: 1px solid rgba(0, 217, 255, 0.25);
    border-radius: var(--radius-sm);
    font-size: 1rem;
    color: var(--hud-cyan);
  }
  .btn-recalc {
    margin-top: 0.5rem;
    padding: 0.45rem 1rem;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    color: var(--hud-primary);
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
  }
  .btn-recalc:hover:not(:disabled) {
    background: rgba(255, 119, 0, 0.12);
    border-color: var(--hud-primary);
    box-shadow: 0 0 10px rgba(255, 119, 0, 0.25);
  }
  .btn-recalc:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .copied-toast {
    margin-top: 0.5rem;
    padding: 0.35rem 0.75rem;
    background: rgba(68, 255, 136, 0.1);
    border: 1px solid var(--hud-ok);
    border-radius: var(--radius-sm);
    color: var(--hud-ok);
    font-size: 0.85rem;
  }
</style>
