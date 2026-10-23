<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, cssPct, decimal } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { materialName, type ProspectorTopic } from './types';

  const p = $derived(topic<ProspectorTopic>('mining.prospector'));
  const a = $derived(p?.current ?? null);

  const names = $derived.by(() => {
    const m = new Map<string, string>();
    for (const row of a?.materials ?? []) if (row.name) m.set(row.key, row.name);
    return m;
  });
  const restPct = $derived(
    Math.max(0, 100 - (a?.materials ?? []).reduce((n, m) => n + m.proportion, 0)),
  );
  const maxValue = $derived(Math.max(0, ...(a?.materials ?? []).map((m) => m.est_value ?? 0)));
  // Del más antiguo (izquierda) al más reciente (derecha).
  const history = $derived([...(p?.history ?? [])].reverse());
  const maxHist = $derived(Math.max(0, ...history.map((h) => h.est_total ?? 0)));

  function approx(v: number | null): string {
    return v == null ? '—' : t('mining-approx', { value: credits(v) });
  }
  function contentLabel(c: string): string {
    return c === 'low' || c === 'medium' || c === 'high' ? t(`mining-content-${c}`) : t('mining-content-unknown');
  }
  function barWidth(i: number): string {
    const m = a!.materials[i];
    if (maxValue > 0) return cssPct(((m.est_value ?? 0) / maxValue) * 100);
    return cssPct(m.proportion);
  }
  function histLabel(i: number, total: number | null): string {
    return t('mining-history-item', { n: i + 1, value: total == null ? t('mining-no-data') : approx(total) });
  }
</script>

<Panel titleKey="panel-mining-prospector" span={2}>
  {#if !a}
    <p class="muted" data-testid="prospector-empty">{t('mining-no-data')}</p>
  {:else}
    <div class="badges">
      <span class="badge" data-testid="content" data-content={a.content}>{t('mining-content-level', { level: contentLabel(a.content) })}</span>
      {#if a.motherlode}
        <span class="core" data-testid="core">{t('mining-core-named', { material: a.motherlode })}</span>
      {/if}
    </div>

    <div class="stack" role="img" aria-label={t('mining-composition-aria')}>
      {#each a.materials as m, i (m.key + i)}
        <span
          data-testid="stack-segment"
          class="seg c{Math.min(i, 3)}"
          style:width={cssPct(m.proportion)}
          title="{materialName(m.key, names)} {decimal(m.proportion)} %"
        ></span>
      {/each}
      <span data-testid="stack-segment" class="seg rest" style:width={cssPct(restPct)} title={t('mining-rest-no-value')}>
        <span data-testid="stack-rest" class="sr">{t('mining-rest-no-value')}</span>
      </span>
    </div>

    <table>
      <thead>
        <tr><th>{t('mining-col-material')}</th><th>{t('mining-col-share')}</th><th>{t('mining-col-value')}</th><th></th></tr>
      </thead>
      <tbody>
        {#each a.materials as m, i (m.key + i)}
          <tr data-testid="material-row" data-interest={m.interest}>
            <td>
              {#if m.interest}<b class="star">{materialName(m.key, names)} <span role="img" aria-label={t('mining-interest-aria')}>★</span></b>{:else}{materialName(m.key, names)}{/if}
            </td>
            <td class="mono">{decimal(m.proportion)} %</td>
            <td class="mono">{approx(m.est_value)}</td>
            <td class="barcell">
              <div class="bar" aria-hidden="true"><i class="c{Math.min(i, 3)}" style:width={barWidth(i)}></i></div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    <div class="verdict" data-testid="verdict" data-kind={a.verdict.kind} role="status">
      {#if a.verdict.kind === 'worth'}
        {a.verdict.est_value == null ? t('mining-verdict-worth-unpriced') : t('mining-verdict-worth', { credits: credits(a.verdict.est_value) })}
      {:else if a.verdict.kind === 'not_worth'}
        {t('mining-verdict-not-worth')}
      {:else}
        {t('mining-no-data')}
      {/if}
    </div>

    {#if a.remaining != null}
      <div data-testid="remaining">{t('mining-remaining', { remaining: `${decimal(a.remaining, 0)} %` })}</div>
      <div
        class="bar remaining"
        role="progressbar"
        data-testid="remaining-bar"
        aria-label={t('mining-remaining-aria')}
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(a.remaining)}
      ><i style:width={cssPct(a.remaining)}></i></div>
    {/if}

    <p class="muted foot">{t('mining-footnote')}</p>

    <div class="muted">{t('mining-history')}</div>
    <ol class="hist">
      {#each history as h, i (i)}
        <li data-testid="history-item" title={histLabel(i, h.est_total)} aria-label={histLabel(i, h.est_total)}>
          <i class:top={maxHist > 0 && h.est_total === maxHist} style:height={maxHist > 0 ? cssPct(((h.est_total ?? 0) / maxHist) * 100) : '0%'}></i>
        </li>
      {/each}
    </ol>
  {/if}
</Panel>

<style>
  .muted {
    color: var(--hud-muted);
    margin: 0;
  }
  .foot {
    font-size: 0.78rem;
    margin: 0.5rem 0 0.4rem;
    color: var(--hud-muted);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.6rem;
  }
  .badge {
    border: 1px solid var(--hud-ok);
    color: var(--hud-ok);
    background: rgba(68, 255, 136, 0.08);
    border-radius: var(--radius-sm);
    padding: 0.2rem 0.55rem;
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .badge[data-content='low'] {
    border-color: var(--hud-border-subtle);
    color: var(--hud-muted);
    background: rgba(255, 255, 255, 0.04);
  }
  .badge[data-content='medium'] {
    border-color: var(--hud-warning);
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.08);
  }
  .core {
    border: 1px solid var(--hud-warning);
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.12);
    box-shadow: 0 0 10px rgba(255, 170, 0, 0.25);
    border-radius: var(--radius-sm);
    font-weight: 700;
    padding: 0.2rem 0.65rem;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stack {
    display: flex;
    height: 1.5rem;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    margin: 0.5rem 0 0.8rem;
    overflow: hidden;
    position: relative;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }
  .seg {
    display: block;
    height: 100%;
    overflow: hidden;
    transition: width var(--transition-fast);
  }
  .seg.rest {
    background: rgba(255, 255, 255, 0.05);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .c0 {
    background: linear-gradient(180deg, #ffcc00, var(--hud-warning));
    box-shadow: 0 0 6px rgba(255, 170, 0, 0.35);
  }
  .c1 {
    background: linear-gradient(180deg, #ff9933, var(--hud-primary));
    box-shadow: 0 0 6px rgba(255, 119, 0, 0.35);
  }
  .c2 {
    background: linear-gradient(180deg, #00d9ff, #0088cc);
    box-shadow: 0 0 6px rgba(0, 217, 255, 0.35);
  }
  .c3 {
    background: rgba(255, 255, 255, 0.2);
  }
  table {
    border-collapse: collapse;
    width: 100%;
    margin-bottom: 0.75rem;
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
    letter-spacing: 0.05em;
    font-weight: 600;
    border-bottom: 1px solid var(--hud-border-subtle);
    padding-bottom: 0.4rem;
  }
  tr[data-interest='true'] {
    background: rgba(255, 170, 0, 0.04);
  }
  .star {
    color: var(--hud-warning);
    font-weight: 600;
  }
  .barcell {
    width: 7rem;
  }
  .bar {
    height: 0.65rem;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }
  .bar i {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: 1px;
  }
  .remaining {
    margin: 0.3rem 0 0.6rem;
  }
  .remaining i {
    background: linear-gradient(90deg, #00cc66, var(--hud-ok));
    box-shadow: 0 0 8px rgba(68, 255, 136, 0.4);
  }
  .verdict {
    border: 1px solid var(--hud-border-subtle);
    background: rgba(255, 255, 255, 0.03);
    color: var(--hud-muted);
    font-weight: 700;
    font-size: 1.15rem;
    padding: 0.5rem 0.85rem;
    text-align: center;
    border-radius: var(--radius-sm);
    margin: 0.75rem 0;
    letter-spacing: 0.03em;
    backdrop-filter: blur(4px);
  }
  .verdict[data-kind='worth'] {
    border-color: var(--hud-ok);
    color: var(--hud-ok);
    background: rgba(68, 255, 136, 0.08);
    box-shadow: 0 0 16px rgba(68, 255, 136, 0.18);
  }
  .verdict[data-kind='not_worth'] {
    border-color: var(--hud-danger);
    color: var(--hud-danger);
    background: rgba(255, 68, 68, 0.08);
    box-shadow: 0 0 16px rgba(255, 68, 68, 0.18);
  }
  .hist {
    list-style: none;
    display: flex;
    gap: 0.3rem;
    margin: 0.4rem 0 0;
    padding: 0;
  }
  .hist li {
    flex: 1;
    height: 2.2rem;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    display: flex;
    align-items: flex-end;
    overflow: hidden;
  }
  .hist i {
    display: block;
    width: 100%;
    background: rgba(255, 255, 255, 0.15);
    transition: height var(--transition-fast);
  }
  .hist i.top {
    background: linear-gradient(180deg, #ffcc00, var(--hud-warning));
    box-shadow: 0 0 8px rgba(255, 170, 0, 0.5);
  }
</style>
