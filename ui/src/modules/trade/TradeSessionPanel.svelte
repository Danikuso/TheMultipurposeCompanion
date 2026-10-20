<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, cssPct, decimal, duration, integer } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { commodityName, type TradeSessionTopic } from './types';

  const s = $derived(topic<TradeSessionTopic>('trade.session'));
  let tick = $state(Date.now());
  let skew = $state<number | null>(null);
  // Hora del diario; `null` si no se conoce (se oculta la duración).
  const now = $derived(skew == null ? null : tick + skew);
  let failed = $state(false);

  // Un único setInterval de 30 s para la duración; no corre sin sesión.
  $effect(() => {
    if (!s) return;
    skew = journalSkew(s.as_of);
    tick = Date.now();
    const id = setInterval(() => {
      tick = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  const elapsedMs = $derived.by(() => {
    const ts = s?.started_at ? Date.parse(s.started_at) : NaN;
    return Number.isNaN(ts) || now == null ? null : Math.max(0, now - ts);
  });
  // Créditos/hora con el reloj del diario; con menos de 60 s de sesión no se muestran.
  const perHour = $derived(elapsedMs != null && elapsedMs >= 60_000 && s ? s.profit / (elapsedMs / 3_600_000) : null);
  // Los rangos se muestran en inglés (nombres del juego), sin traducir.
  const rankName = $derived(s?.rank != null && s.rank >= 0 && s.rank <= 8 ? t(`trade-rank-${s.rank}`) : null);

  async function reset() {
    failed = false;
    try {
      await action('trade.reset_session', {});
    } catch {
      failed = true;
    }
  }
</script>

<Panel titleKey="panel-trade-session" span={1}>
  {#if !s}
    <p class="muted" data-testid="session-empty">{t('trade-no-data')}</p>
  {:else}
    {#if elapsedMs != null}<p class="muted" data-testid="session-time">{t('trade-session-duration', { time: duration(elapsedMs) })}</p>{/if}
    <div class="big">
      <strong data-testid="session-profit">{credits(s.profit)}</strong>
      <span class="muted" data-testid="session-rate">· {perHour == null ? '—' : t('trade-session-rate', { credits: credits(Math.round(perHour)) })}</span>
    </div>
    <div data-testid="session-tons">
      {t('trade-session-bought', { tons: integer(s.tons_bought) })} · {t('trade-session-sold', { tons: integer(s.tons_sold) })}
      · {t('trade-session-operations', { count: s.operations })}
    </div>
    {#if s.best_trade}
      <div class="muted" data-testid="session-best">
        {t('trade-session-best', { profit: credits(s.best_trade[1]) })} · {commodityName(s.best_trade[0], null)}
      </div>
    {/if}
    {#if rankName && s.progress != null}
      <div class="sub" data-testid="session-rank">{t('trade-rank-progress', { rank: rankName, pct: decimal(s.progress, 0) })}</div>
      <div
        class="bar"
        role="progressbar"
        data-testid="rank-bar"
        aria-label={t('trade-rank-aria')}
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(s.progress)}
      ><i style:width={cssPct(s.progress)}></i></div>
    {:else if rankName}
      <div class="sub" data-testid="session-rank">{rankName}</div>
    {/if}
    <button type="button" data-testid="session-reset" onclick={reset}>{t('trade-reset')}</button>
    {#if failed}<p class="err" role="alert">{t('trade-reset-failed')}</p>{/if}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0; }
  .big { font-size: 1.4em; }
  .sub { margin-top: 0.6rem; }
  .bar { height: 0.9rem; background: var(--hud-surface-alt); border: 1px solid var(--hud-border); position: relative; margin: 0.2rem 0 0.6rem; }
  .bar i { position: absolute; left: 0; top: 0; bottom: 0; background: var(--hud-primary); }
  .err { color: var(--hud-danger); }
</style>
