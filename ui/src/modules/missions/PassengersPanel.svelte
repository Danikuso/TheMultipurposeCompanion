<script lang="ts">
  import { t } from '../../lib/i18n';
  import { integer } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { CabinClass, PassengersTopic } from './types';

  const p = $derived(topic<PassengersTopic>('missions.passengers'));
  const classes: CabinClass[] = ['Economy', 'Business', 'First', 'Luxury'];
  const entries = $derived(classes.flatMap((c) => (p?.cabins.by_class[c] ? [[c, p.cabins.by_class[c]!] as const] : [])));

  function cabinLabel(c: CabinClass): string {
    return t(`missions-cabin-${c === 'Economy' ? 'economy' : c === 'Business' ? 'business' : c === 'First' ? 'first' : 'luxury'}`);
  }
</script>

<Panel titleKey="panel-missions-passengers" span={1}>
  {#if !p || (p.cabins.seats_total === 0 && p.groups.length === 0)}
    <p class="muted" data-testid="passengers-empty">{t('missions-passengers-none')}</p>
  {:else}
    <div class="sub" data-testid="cabin-total">
      <b>{t('missions-cabin-seats', { seats: integer(p.cabins.occupied_total), capacity: integer(p.cabins.seats_total) })}</b>
    </div>
    <div class="cab">
      {#each entries as [c, o] (c)}
        <div data-testid={`cabin-${c}`}>
          {cabinLabel(c)}<br /><b class="mono">{o.occupied == null ? '?' : integer(o.occupied)}/{integer(o.seats)}</b>
        </div>
      {/each}
    </div>
    {#if p.groups.length > 0}
      <table>
        <thead>
          <tr><th>{t('missions-col-destination')}</th><th class="n">{t('missions-col-passengers')}</th><th></th></tr>
        </thead>
        <tbody>
          {#each p.groups as g (g.mission_id + '|' + g.destination)}
            <tr data-testid="passenger-group">
              <td>{g.destination ?? t('missions-passenger-unknown-destination')}</td>
              <td class="n mono">{integer(g.count)}</td>
              <td>
                {#if g.vip}<span class="tag warn">{t('missions-passenger-vip')}</span>{/if}
                {#if g.wanted}<span class="tag bad">{t('missions-passenger-wanted')}</span>{/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if p.wanted}
      <div class="bad" role="alert" data-testid="missions-wanted-warning">⚠ {t('missions-alert-wanted-passenger')}</div>
    {/if}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .mono { font-family: var(--font-mono); }
  .cab { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.4rem; margin: 0.4rem 0; }
  .cab div { border: 1px solid var(--hud-border); padding: 0.2rem; text-align: center; }
  table { border-collapse: collapse; width: 100%; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .n { text-align: right; }
  .tag { font-size: 0.75em; padding: 0 0.3rem; border: 1px solid currentColor; margin-left: 0.3rem; }
  .warn { color: var(--hud-warning); }
  .bad { color: var(--hud-danger); margin-top: 0.4rem; }
</style>
