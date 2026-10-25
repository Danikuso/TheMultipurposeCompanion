<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, cssPct, decimal, duration } from '../../lib/format';
  import { journalSkew } from '../../lib/clock.svelte';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { materialName, namesFrom, type MiningSessionTopic, type ProspectorTopic } from './types';

  const s = $derived(topic<MiningSessionTopic>('mining.session'));
  const names = $derived(namesFrom(topic<ProspectorTopic>('mining.prospector'), s?.names));
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
  const elapsed = $derived(elapsedMs == null ? null : duration(elapsedMs));
  const refined = $derived(Object.entries(s?.refined ?? {}).sort((a, b) => b[1] - a[1]));
  const totalTons = $derived(refined.reduce((n, [, tons]) => n + tons, 0));
  // Las tasas se calculan aquí con el reloj del diario (el topic no depende del tiempo);
  // con menos de 60 s de sesión no se muestran.
  const hours = $derived(elapsedMs != null && elapsedMs >= 60_000 ? elapsedMs / 3_600_000 : null);
  const tonsPerHour = $derived(hours == null ? null : totalTons / hours);
  const creditsPerHour = $derived(hours == null || s?.est_credits == null ? null : s.est_credits / hours);
  const maxBin = $derived(Math.max(1, ...(s?.refinery_bins ?? []).map((b) => b.tons)));
  const launchedTotal = $derived(Object.values(s?.limpets_launched ?? {}).reduce((n, v) => n + v, 0));
  const launchedDetail = $derived.by(() => {
    const l = s?.limpets_launched ?? {};
    let prospector = 0;
    let collection = 0;
    let other = 0;
    for (const [type, n] of Object.entries(l)) {
      const k = type.toLowerCase();
      if (k === 'prospector') prospector += n;
      else if (k === 'collection') collection += n;
      else other += n;
    }
    const parts: string[] = [];
    if (prospector > 0) parts.push(`${t('mining-limpet-prospector')} ${prospector}`);
    if (collection > 0) parts.push(`${t('mining-limpet-collection')} ${collection}`);
    if (other > 0) parts.push(`${t('mining-limpet-other')} ${other}`);
    return parts.join(' · ');
  });
  const cargoPct = $derived(s?.cargo != null && s.capacity ? (s.cargo / s.capacity) * 100 : 0);

  async function reset() {
    failed = false;
    try {
      await action('mining.reset_session', {});
    } catch {
      failed = true;
    }
  }
</script>

<Panel titleKey="panel-mining-session" span={1} collapsible={!!s} defaultExpanded={false}>
  {#if !s}
    <p class="muted" data-testid="session-empty">{t('mining-no-data')}</p>
  {:else}
    {#if elapsed}<p class="muted session-header" data-testid="session-time">{t('mining-session-time', { time: elapsed })}</p>{/if}
    <div class="metrics-grid">
      <div class="metric-card">
        <div class="big">
          <strong data-testid="tons-total">{t('mining-tons', { tons: decimal(totalTons, 0) })}</strong>
        </div>
        <div class="rate-sub muted" data-testid="tons-hour">· {tonsPerHour == null ? '—' : t('mining-tons-per-hour', { rate: decimal(tonsPerHour, 1) })}</div>
      </div>
      <div class="metric-card">
        <div class="credits-val">
          <span data-testid="credits-total">{s.est_credits == null ? '—' : t('mining-credits-total', { credits: credits(s.est_credits) })}</span>
        </div>
        <div class="rate-sub muted" data-testid="credits-hour">· {creditsPerHour == null ? '—' : t('mining-credits-per-hour', { credits: credits(Math.round(creditsPerHour)) })}</div>
      </div>
    </div>

    <div class="sub">
      {t('mining-limpets')}
      <b data-testid="limpets-left">{s.limpets_left == null ? '—' : t('mining-limpets-left', { count: s.limpets_left })}</b>
      {#if launchedTotal > 0}
        <span class="muted" data-testid="limpets-launched">· {t('mining-limpets-launched', { total: launchedTotal, detail: launchedDetail })}</span>
      {/if}
    </div>

    <div class="sub" data-testid="cargo">
      {#if s.cargo != null && s.capacity}
        <b>{t('mining-cargo', { used: decimal(s.cargo, 0), capacity: decimal(s.capacity, 0) })}</b>
      {:else}
        <b>—</b>
      {/if}
    </div>
    <div
      class="bar"
      role="progressbar"
      data-testid="cargo-bar"
      aria-label={t('mining-cargo-aria')}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(cargoPct)}
    ><i style:width={cssPct(cargoPct)}></i></div>
  {/if}

  {#snippet details()}
    {#if s}
      <table>
        <thead><tr><th>{t('mining-refined')}</th><th>{t('mining-col-tons')}</th></tr></thead>
        <tbody>
          {#each refined as [key, tons] (key)}
            <tr data-testid="refined-row"><td>{materialName(key, names)}</td><td class="mono">{tons}</td></tr>
          {/each}
        </tbody>
      </table>

      <div class="sub" data-testid="refinery">{t('mining-refinery-estimated')}</div>
      <ul class="bins">
        {#each s.refinery_bins as b (b.key)}
          <li
            class="bin"
            data-testid="bin"
            role="img"
            aria-label={t('mining-bin-aria', { material: materialName(b.key, names), tons: b.tons })}
          >
            <i style:height={cssPct((b.tons / maxBin) * 100)}></i>
            <span class="label">{materialName(b.key, names)}</span>
          </li>
        {/each}
      </ul>

      <button type="button" class="btn-reset" data-testid="reset-session" onclick={reset}>{t('mining-reset')}</button>
      {#if failed}<p class="err" role="alert">{t('mining-reset-failed')}</p>{/if}
    {/if}
  {/snippet}
</Panel>

<style>
  .muted {
    color: var(--hud-muted);
    margin: 0;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.6rem;
    font-size: 0.85rem;
  }
  .metrics-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.6rem;
    margin-bottom: 0.75rem;
  }
  .metric-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.65rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .big {
    font-size: 1.25rem;
    color: var(--hud-primary);
    line-height: 1.2;
  }
  .credits-val {
    font-size: 1.1rem;
    color: var(--hud-cyan);
    font-weight: 600;
  }
  .rate-sub {
    font-size: 0.8rem;
    color: var(--hud-muted);
  }
  .sub {
    margin-top: 0.75rem;
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--hud-muted);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    margin-top: 0.4rem;
  }
  th, td {
    text-align: left;
    padding: 0.25rem 0.5rem 0.25rem 0;
    font-size: 0.88rem;
  }
  th {
    color: var(--hud-muted);
    font-size: 0.76rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .bins {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 0.35rem;
  }
  .bin {
    border: 1px solid var(--hud-border-subtle);
    background: rgba(0, 0, 0, 0.35);
    border-radius: 2px;
    height: 2.8rem;
    position: relative;
    font-size: 0.72rem;
    text-align: center;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }
  .bin i {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(180deg, rgba(255, 150, 50, 0.7), var(--hud-primary));
    opacity: 0.5;
    transition: height var(--transition-fast);
  }
  .bin span {
    position: relative;
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0.15rem 0.2rem;
    z-index: 1;
    font-weight: 500;
  }
  .bar {
    height: 0.65rem;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    position: relative;
    margin: 0.3rem 0 0.75rem;
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
  .btn-reset {
    margin-top: 0.5rem;
    width: 100%;
    padding: 0.45rem 0.8rem;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    color: var(--hud-muted);
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: all var(--transition-fast);
  }
  .btn-reset:hover {
    background: rgba(255, 119, 0, 0.12);
    border-color: var(--hud-primary);
    color: var(--hud-primary);
    box-shadow: 0 0 8px rgba(255, 119, 0, 0.25);
  }
  .err {
    color: var(--hud-danger);
    font-size: 0.85rem;
    margin-top: 0.4rem;
  }
</style>
