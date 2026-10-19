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

  const elapsed = $derived.by(() => {
    const ts = s?.started_at ? Date.parse(s.started_at) : NaN;
    return Number.isNaN(ts) || now == null ? null : duration(Math.max(0, now - ts));
  });
  const refined = $derived(Object.entries(s?.refined ?? {}).sort((a, b) => b[1] - a[1]));
  const totalTons = $derived(refined.reduce((n, [, tons]) => n + tons, 0));
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

<Panel titleKey="panel-mining-session" span={1}>
  {#if !s}
    <p class="muted" data-testid="session-empty">{t('mining-no-data')}</p>
  {:else}
    {#if elapsed}<p class="muted" data-testid="session-time">{t('mining-session-time', { time: elapsed })}</p>{/if}
    <div class="big">
      <strong data-testid="tons-total">{t('mining-tons', { tons: decimal(totalTons, 0) })}</strong>
      <span class="muted" data-testid="tons-hour">· {s.tons_per_hour == null ? '—' : t('mining-tons-per-hour', { rate: decimal(s.tons_per_hour, 1) })}</span>
    </div>
    <div>
      <span data-testid="credits-total">{s.est_credits == null ? '—' : t('mining-credits-total', { credits: credits(s.est_credits) })}</span>
      <span class="muted" data-testid="credits-hour">· {s.credits_per_hour == null ? '—' : t('mining-credits-per-hour', { credits: credits(s.credits_per_hour) })}</span>
    </div>

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

    <button type="button" data-testid="reset-session" onclick={reset}>{t('mining-reset')}</button>
    {#if failed}<p class="err" role="alert">{t('mining-reset-failed')}</p>{/if}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0; }
  .mono { font-family: var(--font-mono); }
  .big { font-size: 1.4em; }
  .sub { margin-top: 0.6rem; }
  table { border-collapse: collapse; width: 100%; margin-top: 0.4rem; }
  th, td { text-align: left; padding: 0.1rem 0.5rem 0.1rem 0; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .bins { list-style: none; margin: 0.25rem 0 0; padding: 0; display: grid; grid-template-columns: repeat(5, 1fr); gap: 0.25rem; }
  .bin { border: 1px solid var(--hud-border); height: 2.6rem; position: relative; font-size: 0.7em; text-align: center; }
  .bin i { position: absolute; left: 0; right: 0; bottom: 0; background: var(--hud-primary); opacity: 0.35; }
  .bin span { position: relative; display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0 0.1rem; }
  .bar { height: 0.9rem; background: var(--hud-surface-alt); border: 1px solid var(--hud-border); position: relative; margin: 0.2rem 0 0.6rem; }
  .bar i { position: absolute; left: 0; top: 0; bottom: 0; background: var(--hud-primary); }
  .err { color: var(--hud-danger); }
</style>
