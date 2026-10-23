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
      <div class="table-wrap">
      <table>
        <thead>
          <tr><th>{t('missions-col-destination')}</th><th class="n">{t('missions-col-passengers')}</th><th></th></tr>
        </thead>
        <tbody>
          {#each p.groups as g, i (`${g.mission_id}|${g.destination ?? ''}|${i}`)}
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
      </div>
    {/if}
    {#if p.wanted}
      <div class="wanted-alert bad" role="alert" data-testid="missions-wanted-warning">⚠ {t('missions-alert-wanted-passenger')}</div>
    {/if}
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
  .sub {
    margin-bottom: 0.45rem;
    font-size: 0.92rem;
    color: var(--hud-text);
  }
  .cab {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.45rem;
    margin: 0.5rem 0 0.75rem;
  }
  .cab div {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.25rem;
    text-align: center;
    font-size: 0.78rem;
    color: var(--hud-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .cab div b {
    display: block;
    margin-top: 0.2rem;
    font-size: 0.95rem;
    color: var(--hud-primary);
  }
  .table-wrap {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    margin-top: 0.35rem;
  }
  th, td {
    text-align: left;
    padding: 0.3rem 0.5rem 0.3rem 0;
    white-space: nowrap;
    font-size: 0.88rem;
  }
  th {
    color: var(--hud-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    border-bottom: 1px solid var(--hud-border-subtle);
    padding-bottom: 0.35rem;
  }
  .n {
    text-align: right;
  }
  .tag {
    font-size: 0.72rem;
    padding: 0.1rem 0.4rem;
    border: 1px solid currentColor;
    border-radius: 2px;
    margin-left: 0.35rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .warn {
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.12);
  }
  .bad {
    color: var(--hud-danger);
    background: rgba(255, 68, 68, 0.12);
    box-shadow: 0 0 6px rgba(255, 68, 68, 0.2);
  }
  .wanted-alert {
    margin-top: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: rgba(255, 68, 68, 0.12);
    border: 1px solid var(--hud-danger);
    border-radius: var(--radius-sm);
    color: var(--hud-danger);
    font-weight: 700;
    font-size: 0.88rem;
    box-shadow: 0 0 12px rgba(255, 68, 68, 0.25);
  }
</style>
