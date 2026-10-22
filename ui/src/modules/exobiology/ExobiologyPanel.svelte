<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, gravity, kelvin, meters } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { BodyTopic, Candidate, SamplingTopic, UnsoldTopic } from './types';

  const body = $derived(topic<BodyTopic>('exobiology.body'));
  const sampling = $derived(topic<SamplingTopic>('exobiology.sampling'));
  const unsold = $derived(topic<UnsoldTopic>('exobiology.unsold'));
  const barWidth = $derived.by(() => {
    if (!sampling || sampling.distance_m == null || !(sampling.min_m > 0)) return '0%';
    const ratio = Math.max(0, Math.min(1, sampling.distance_m / sampling.min_m));
    return `${Number((ratio * 100).toFixed(1))}%`;
  });

  function name(c: Candidate): string {
    return c.species_name ?? t('exobiology-species-unknown', { genus: c.genus_name ?? '—' });
  }
  function origin(c: Candidate): string {
    return c.sources.map(source => t(`exobiology-source-${source}`)).join(' · ');
  }
  function state(c: Candidate): string {
    if (c.status === 'done') return '✓ 3/3';
    if (c.status === 'in-progress') return `▶ ${c.progress ?? 0}/3`;
    return '—';
  }
  function label(prefix: string, key: string | null | undefined, raw: string | null | undefined): string {
    if (key) {
      const full = `${prefix}${key}`;
      const text = t(full);
      if (text !== full) return text;
    }
    return raw ?? '—';
  }
  const barNow = $derived(sampling?.distance_m == null ? undefined : Math.max(0, Math.min(sampling.min_m, sampling.distance_m)));
  const genera = $derived((body?.confirmed_genera ?? []).map(g => g.name ?? '—').join(', '));
</script>

<Panel titleKey="panel-exobiology" span="full" size="large">
  {#if body}
    <div class="header" data-testid="body-header">
      <div>
        <strong class="title">{body.body ?? '—'}</strong>
        <span class="dim"> · {label('exploration-kind-', body.kind_key, body.kind)} · {gravity(body.gravity_g)} · {kelvin(body.temperature_k)} · {label('exobiology-atmosphere-', body.atmosphere_key, body.atmosphere)}</span>
      </div>
      <div class="badges">
        <span class="badge">{t('exobiology-signals', { count: body.bio_signals })}</span>
        {#if body.first_logged}
          <span class="badge warn" data-testid="first-logged">{t('exobiology-first-logged-estimated')}</span>
        {/if}
      </div>
    </div>
    <div class="columns">
      <section class="box candidates">
        <h3>{t('exobiology-candidates')}</h3>
        {#if body.candidates.length > 0}
          <div class="scroll">
            <table>
              <thead>
                <tr>
                  <th>{t('exobiology-col-species')}</th><th>{t('exobiology-col-value')}</th><th>{t('exobiology-col-value-x5')}</th>
                  <th>{t('exobiology-col-min')}</th><th>{t('exobiology-col-likelihood')}</th><th>{t('exobiology-col-status')}</th>
                </tr>
              </thead>
              <tbody>
                {#each body.candidates as c, index (index)}
                  <tr data-testid="candidate-row" class:active={c.status === 'in-progress'}>
                    <td>{name(c)}</td>
                    <td class="number">{credits(c.value)}</td>
                    <td class="number">{credits(c.value_x5)}</td>
                    <td class="number">{meters(c.min_m)}</td>
                    <td>
                      <span class="likelihood {c.likelihood}">{t(`exobiology-likelihood-${c.likelihood}`)}</span>
                      {#if c.sources.length > 0}<span class="dim origin">({origin(c)})</span>{/if}
                    </td>
                    <td class:done={c.status === 'done'} class:dim={c.status === 'none'}>{state(c)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="dim">{t('exobiology-no-candidates')}</p>
        {/if}
        <p class="dim" data-testid="genera">
          {body.confirmed_genera.length > 0 ? t('exobiology-genera-confirmed', { genera }) : t('exobiology-genera-unconfirmed')}
          · {t('exobiology-completed', { completed: body.completed, signals: body.bio_signals })}
        </p>
      </section>
      <section class="box sampling" data-testid="sampling">
        <h3>{t('exobiology-sampling')}</h3>
        {#if sampling}
          <div class="species">{sampling.species_name ?? sampling.genus_name ?? '—'}</div>
          <div class="dots" aria-hidden="true">
            {#each [1, 2, 3] as n}<span class="dot" class:filled={n <= sampling.points}></span>{/each}
          </div>
          <div class="dim">{sampling.complete ? t('exobiology-sampling-complete') : t('exobiology-samples', { n: sampling.points })}</div>
          {#if !sampling.complete}
            <div>
              {t('exobiology-distance')}:
              {#if sampling.distance_m != null}
                <strong class={sampling.remaining_m === 0 ? 'ok number' : 'warn number'} data-testid="distance-value">{meters(sampling.distance_m)} / {meters(sampling.min_m)}</strong>
              {:else}
                <strong class="dim">{t('exobiology-no-data')}</strong>
              {/if}
            </div>
            <div class="bar" role="progressbar" aria-label={t('exobiology-distance-progress')} aria-valuemin="0" aria-valuemax={sampling.min_m} aria-valuenow={barNow}><i data-testid="distance-bar" style:width={barWidth}></i></div>
            <div class="dim">
              {#if sampling.remaining_m != null}{t('exobiology-remaining', { meters: meters(sampling.remaining_m) })}{/if}
              {#if sampling.direction}
                <span data-testid="direction">{t('exobiology-direction', { dir: t(`dir-${sampling.direction.rel}`), compass: t(`compass-${sampling.direction.compass}`) })}</span>
              {/if}
            </div>
          {/if}
          {#if sampling.discarded_name || sampling.discarded_species}
            <div class="notice" data-testid="discarded">{sampling.discarded_name ? t('exobiology-discarded', { name: sampling.discarded_name }) : t('exobiology-discarded-unknown')}</div>
          {/if}
        {:else}
          <p class="dim">{t('exobiology-waiting')}</p>
        {/if}
        {#if unsold}
          <div class="unsold" data-testid="unsold">
            <div class="dim">{t('exobiology-unsold')} · {t('exobiology-unsold-species', { count: unsold.species })}</div>
            <strong class="amount number">{credits(unsold.total)}</strong>
            {#if sampling && !sampling.complete}
              <span class="dim">{t('exobiology-unsold-after', { after: credits(sampling.unsold_after), species: sampling.species_name ?? sampling.genus_name ?? '—' })}</span>
            {/if}
          </div>
        {/if}
      </section>
    </div>
  {:else}
    <p class="dim" data-testid="empty">{t('empty-no-data')}</p>
  {/if}
</Panel>

<style>
  .header { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: baseline; gap: .75rem; margin-bottom: .85rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--hud-border-subtle); }
  .title { font-size: var(--font-title-lg); color: var(--hud-primary); }
  .dim { color: var(--hud-muted); }
  .badge { display: inline-flex; align-items: center; border: 1px solid var(--hud-border); border-radius: 3px; padding: 0.15rem .5rem; margin-left: .4rem; font-size: 0.75rem; font-family: var(--font-mono); }
  .badge.warn, .warn { color: var(--hud-warning); border-color: rgba(245, 158, 11, 0.4); background: rgba(245, 158, 11, 0.12); }
  .columns { display: flex; flex-wrap: wrap; gap: 1rem; align-items: stretch; }
  .box { background: var(--hud-surface-card); border: 1px solid var(--hud-border-subtle); border-radius: 6px; padding: .8rem; min-width: 0; box-shadow: 0 4px 15px rgba(0, 0, 0, 0.25); }
  .candidates { flex: 1.6 1 32rem; }
  .sampling { flex: 1 1 18rem; border-top: 3px solid var(--hud-purple); }
  h3 { margin: 0 0 .6rem; font-size: .8rem; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: var(--hud-primary); border-bottom: 1px solid var(--hud-border-subtle); padding-bottom: 0.35rem; }
  .sampling h3 { color: var(--hud-purple); }
  .scroll { overflow-x: auto; }
  table { border-collapse: collapse; width: 100%; font-size: 0.85rem; }
  th, td { padding: .35rem .6rem .35rem 0; text-align: left; white-space: nowrap; border-bottom: 1px solid var(--hud-border-subtle); }
  th { color: var(--hud-muted); font-size: 0.72rem; text-transform: uppercase; font-weight: 700; }
  .number { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  tr.active td { color: var(--hud-primary); font-weight: 700; background: rgba(255, 113, 0, 0.05); }
  .ok { color: var(--hud-success); font-weight: 700; }
  .done { color: var(--hud-success); font-weight: 700; }
  .likelihood.high { color: var(--hud-success); font-weight: 700; }
  .likelihood.unknown { font-style: italic; }
  .origin { margin-left: .3rem; font-size: .85em; }
  .species { font-size: 1.15em; font-weight: 700; color: #fff; margin-bottom: 0.25rem; }
  .dots { display: flex; gap: .4rem; margin: .5rem 0 .3rem; }
  .dot { width: 1.1rem; height: 1.1rem; border: 1px solid var(--hud-purple); border-radius: 50%; display: inline-block; transition: all 150ms ease; }
  .dot.filled { background: var(--hud-purple); box-shadow: 0 0 8px var(--hud-purple); }
  .bar { position: relative; height: 8px; background: rgba(0, 0, 0, 0.5); border: 1px solid var(--hud-border-subtle); border-radius: 999px; margin: .4rem 0; overflow: hidden; }
  .bar i { position: absolute; left: 0; top: 0; bottom: 0; background: linear-gradient(90deg, #7c3aed, var(--hud-success)); border-radius: 999px; transition: width 150ms ease; }
  .notice { margin-top: .5rem; color: var(--hud-danger); border: 1px solid var(--hud-danger); background: rgba(239, 68, 68, 0.1); border-radius: 4px; padding: .3rem .5rem; font-size: 0.82rem; }
  .unsold { margin-top: .85rem; border-top: 1px solid var(--hud-border-subtle); padding-top: .6rem; }
  .amount { font-size: 1.35em; color: #fbbf24; margin-right: .5rem; }
</style>
