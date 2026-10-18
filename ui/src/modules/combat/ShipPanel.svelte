<script lang="ts">
  import { t } from '../../lib/i18n';
  import { pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import PipsGauge from './PipsGauge.svelte';
  import type { ShipTopic } from './types';

  const ship = $derived(topic<ShipTopic>('combat.ship'));
  const since = $derived(
    ship?.heat === 'overheating' && ship.overheating_since ? Date.parse(ship.overheating_since) : null,
  );
  let nowMs = $state(Date.now());

  // Cronómetro local: un único setInterval de 1 s solo mientras hay sobrecalentamiento.
  $effect(() => {
    if (since == null) return;
    nowMs = Date.now();
    const id = setInterval(() => {
      nowMs = Date.now();
    }, 1000);
    return () => clearInterval(id);
  });

  const elapsed = $derived(since == null ? 0 : Math.max(0, Math.floor((nowMs - since) / 1000)));
  const timer = $derived(`${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, '0')}`);
  const heatKey = $derived(
    ship?.heat === 'overheating'
      ? 'combat-heat-overheated'
      : ship?.heat === 'heat_damage'
        ? 'combat-heat-damage'
        : 'combat-heat-normal',
  );
  const steps = $derived(ship?.hull_pct == null ? 0 : Math.ceil(Math.max(0, Math.min(100, ship.hull_pct)) / 10));
</script>

<Panel titleKey="panel-combat-ship" span={1}>
  {#if !ship}
    <p class="empty">{t('combat-no-data')}</p>
  {:else}
    <div class="row">
      <span class="label">{t('combat-shields')}</span>
      <strong
        class="ind"
        data-testid="ship-shields"
        data-state={ship.shields_up == null ? 'unknown' : ship.shields_up ? 'up' : 'down'}
      >{ship.shields_up == null ? '—' : ship.shields_up ? '●' : '○'}</strong>
    </div>
    <div class="row">
      <span class="label">{t('combat-hull')}</span>
      <span class="steps" aria-hidden="true">
        {#each Array.from({ length: 10 }, (_, i) => i) as i (i)}
          <i data-testid="hull-step" data-on={i < steps}></i>
        {/each}
      </span>
      <span class="mono" data-testid="hull-value">{pct(ship.hull_pct)}</span>
    </div>
    <div class="row">
      <span class="label">{t('combat-heat')}</span>
      <span class="heat" data-heat={ship.heat}>
        <span data-testid="heat-state">{t(heatKey)}</span>
        {#if since != null}
          <strong class="mono" data-testid="heat-timer">{timer}</strong>
        {/if}
      </span>
    </div>
    {#if ship.pips}
      <PipsGauge pips={ship.pips} />
    {/if}
    {#if ship.alerts.length > 0}
      <ul class="alerts" data-testid="ship-alerts">
        {#each ship.alerts as a (a)}
          <li>{t(`combat-alert-${a.replaceAll('_', '-')}`, { pct: pct(ship.hull_pct) })}</li>
        {/each}
      </ul>
    {/if}
  {/if}
</Panel>

<style>
  .empty { color: var(--hud-muted); margin: 0; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; margin-bottom: 0.4rem; }
  .label { color: var(--hud-muted); }
  .mono { font-family: var(--font-mono); }
  .ind[data-state='up'] { color: var(--hud-ok); }
  .ind[data-state='down'] { color: var(--hud-danger); }
  .steps { display: flex; gap: 2px; flex: 1; }
  .steps i { flex: 1; height: 0.7rem; border: 1px solid var(--hud-primary); }
  .steps i[data-on='true'] { background: var(--hud-primary); }
  .heat[data-heat='normal'] { color: var(--hud-ok); }
  .heat[data-heat='overheating'] { color: var(--hud-warning); }
  .heat[data-heat='heat_damage'] { color: var(--hud-danger); }
  .alerts { list-style: none; margin: 0.6rem 0 0; padding: 0; color: var(--hud-warning); }
</style>
