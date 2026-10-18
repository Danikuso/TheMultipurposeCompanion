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
  const genera = $derived((body?.confirmed_genera ?? []).map(g => g.name ?? '—').join(', '));
</script>

<Panel titleKey="panel-exobiology" span="full" size="large">
  {#if body}
    <div class="header" data-testid="body-header">
      <div>
        <strong class="title">{body.body ?? '—'}</strong>
        <span class="dim"> · {body.kind ?? '—'} · {gravity(body.gravity_g)} · {kelvin(body.temperature_k)} · {body.atmosphere ?? '—'}</span>
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
                <strong class="warn number">{meters(sampling.distance_m)} / {meters(sampling.min_m)}</strong>
              {:else}
                <strong class="dim">{t('exobiology-no-data')}</strong>
              {/if}
            </div>
            <div class="bar"><i data-testid="distance-bar" style:width={barWidth}></i></div>
            <div class="dim">
              {#if sampling.remaining_m != null}{t('exobiology-remaining', { meters: meters(sampling.remaining_m) })}{/if}
              {#if sampling.direction}
                <span data-testid="direction">{t('exobiology-direction', { dir: t(`dir-${sampling.direction.rel}`), compass: t(`compass-${sampling.direction.compass}`) })}</span>
              {/if}
            </div>
          {/if}
          {#if sampling.discarded_name}
            <div class="notice" data-testid="discarded">{t('exobiology-discarded', { name: sampling.discarded_name })}</div>
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
  .header { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: baseline; gap: .75rem; margin-bottom: .75rem; }
  .title { font-size: var(--font-title-lg); color: var(--hud-primary); }
  .dim { color: var(--hud-muted); }
  .badge { display: inline-block; border: 1px solid var(--hud-border); padding: 0 .4rem; margin-left: .4rem; }
  .badge.warn, .warn { color: var(--hud-warning, var(--hud-primary)); }
  .columns { display: flex; flex-wrap: wrap; gap: 1rem; align-items: stretch; }
  .box { border: 1px solid var(--hud-border); padding: .6rem; min-width: 0; }
  .candidates { flex: 1.6 1 32rem; }
  .sampling { flex: 1 1 18rem; }
  h3 { margin: 0 0 .5rem; font-size: .85rem; letter-spacing: .06em; text-transform: uppercase; border-bottom: 1px solid var(--hud-border); }
  .scroll { overflow-x: auto; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: .2rem .6rem .2rem 0; text-align: left; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .number { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  tr.active td { color: var(--hud-primary); font-weight: 700; }
  .done { color: var(--hud-ok, var(--hud-primary)); }
  .likelihood.high { color: var(--hud-ok, var(--hud-primary)); }
  .likelihood.unknown { font-style: italic; }
  .origin { margin-left: .3rem; font-size: .85em; }
  .species { font-size: 1.25em; }
  .dots { display: flex; gap: .3rem; margin: .4rem 0 .2rem; }
  .dot { width: .9rem; height: .9rem; border: 1px solid var(--hud-primary); display: inline-block; }
  .dot.filled { background: var(--hud-primary); }
  .bar { position: relative; height: .9rem; background: var(--hud-surface-alt); border: 1px solid var(--hud-border); margin: .3rem 0; }
  .bar i { position: absolute; left: 0; top: 0; bottom: 0; background: var(--hud-primary); }
  .notice { margin-top: .5rem; color: var(--hud-danger); border: 1px solid var(--hud-danger); padding: .2rem .4rem; }
  .unsold { margin-top: .75rem; border-top: 1px solid var(--hud-border); padding-top: .5rem; }
  .amount { font-size: 1.4em; margin-right: .5rem; }
</style>
