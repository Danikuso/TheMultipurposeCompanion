<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, integer } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { TourismTopic } from './types';

  const tr = $derived(topic<TourismTopic>('missions.tourism'));
  const trips = $derived(tr?.trips ?? []);
</script>

<Panel titleKey="panel-missions-tourism" span={2}>
  {#if trips.length === 0}
    <p class="muted" data-testid="tourism-empty">{t('missions-tourism-none')}</p>
  {:else}
    {#each trips as trip (trip.mission_id)}
      <div class="trip" data-testid="tourism-trip">
        <div><b>{trip.name}</b></div>
        {#if trip.status === 'stops_unavailable'}
          <p class="muted" data-testid="missions-tourism-no-stops">{t('missions-tourism-no-stops')}</p>
        {:else if trip.status === 'loading'}
          <p class="muted" data-testid="missions-tourism-loading">{t('missions-tourism-loading')}</p>
        {:else if trip.status === 'no_coords'}
          <p class="muted" data-testid="missions-tourism-no-coords">{t('missions-tourism-no-coords')}</p>
        {:else if trip.plan}
          {@const p = trip.plan}
          <div data-testid="missions-tourism-plan">
            <table>
              <tbody>
                <tr>
                  <td>{t('missions-tourism-optimal-route')}</td>
                  <td class="n ok mono">{t('missions-tourism-distance', { distance: decimal(p.optimal_ly, 1) })}</td>
                  <td class="n mono">{p.jumps == null ? '—' : t('missions-tourism-jumps', { jumps: p.jumps })}</td>
                </tr>
                <tr>
                  <td>{t('missions-tourism-original-route')}</td>
                  <td class="n mono">{t('missions-tourism-distance', { distance: decimal(p.board_order_ly, 1) })}</td>
                  <td></td>
                </tr>
              </tbody>
            </table>
            <div class="stops">
              {#each p.order as s, i (s + i)}{#if i > 0}→{/if}<span>{s}</span>{/each}
            </div>
            <div class="mono">
              {p.cr_per_ly == null ? '—' : t('missions-tourism-cr-ly', { crly: integer(Math.round(p.cr_per_ly)) })}
              {#if p.cr_per_hour != null}· {t('missions-tourism-rate', { credits: integer(Math.round(p.cr_per_hour)) })}{/if}
            </div>
            <div class="muted small">{t('missions-tourism-reference', { crly: integer(Math.round(p.reference_cr_ly)) })}</div>
            <div class="verdict" class:bad={p.verdict === 'NotWorth'} data-testid="missions-tourism-verdict">
              {p.verdict === 'Profitable' ? '✔ ' + t('missions-tourism-profitable') : '✖ ' + t('missions-tourism-not-worth')}
            </div>
            {#if p.verdict === 'NotWorth' && p.can_abandon_now}
              <div class="warn" role="status" data-testid="missions-tourism-abandon">{t('missions-tourism-abandon-now')}</div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .small { font-size: 0.85em; }
  .mono { font-family: var(--font-mono); }
  .trip { margin-bottom: 0.8rem; }
  table { border-collapse: collapse; }
  td { padding: 0.15rem 0.8rem 0.15rem 0; white-space: nowrap; }
  .n { text-align: right; }
  .stops { display: flex; gap: 0.3rem; align-items: center; flex-wrap: wrap; margin: 0.4rem 0; }
  .stops span { border: 1px solid var(--hud-border); padding: 0 0.4rem; }
  .verdict { border: 2px solid var(--hud-success, var(--hud-primary)); color: var(--hud-success, var(--hud-primary)); font-weight: bold; text-align: center; padding: 0.2rem 0.6rem; margin: 0.4rem 0; font-size: 1.15em; }
  .verdict.bad { border-color: var(--hud-danger); color: var(--hud-danger); }
  .ok { color: var(--hud-success, var(--hud-primary)); }
  .warn { color: var(--hud-warning); }
</style>
