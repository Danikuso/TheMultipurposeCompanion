<script lang="ts">
  import { t } from '../../lib/i18n';
  import { pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import PipsGauge from './PipsGauge.svelte';
  import type { ShipTopic } from './types';

  const ship = $derived(topic<ShipTopic>('combat.ship'));
  const since = $derived(
    ship?.heat === 'overheating' && ship.overheating_since ? ship.overheating_since : null,
  );
  let elapsed = $state(0);

  // Cronómetro anclado a la recepción (nunca se compara con el reloj del diario):
  // parte de 0 al recibir un nuevo `overheating_since` y cuenta con un único
  // setInterval de 1 s solo mientras hay sobrecalentamiento.
  $effect(() => {
    if (since == null) return;
    elapsed = 0;
    const receivedAt = Date.now();
    const id = setInterval(() => {
      elapsed = Math.floor((Date.now() - receivedAt) / 1000);
    }, 1000);
    return () => clearInterval(id);
  });

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
        role="img"
        aria-label={ship.shields_up == null ? t('combat-no-data') : t(ship.shields_up ? 'combat-shields-up' : 'combat-shields-down')}
        data-state={ship.shields_up == null ? 'unknown' : ship.shields_up ? 'up' : 'down'}
      >{ship.shields_up == null ? '—' : ship.shields_up ? '●' : '○'}</strong>
    </div>
    <div class="row">
      <span class="label">{t('combat-hull')}</span>
      <span class="steps" role="progressbar" aria-label={t('combat-hull')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={ship.hull_pct ?? 0}>
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
  .empty {
    color: var(--hud-muted);
    margin: 0;
    font-style: italic;
    padding: 1rem 0;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.85rem;
    margin-bottom: 0.65rem;
  }
  .label {
    color: var(--hud-muted);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    min-width: 4.8rem;
  }
  .mono {
    font-family: var(--font-mono);
    font-weight: 600;
  }
  .ind {
    font-size: 1.1rem;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .ind[data-state='up'] {
    color: var(--hud-ok);
    text-shadow: 0 0 10px rgba(68, 255, 136, 0.65);
  }
  .ind[data-state='down'] {
    color: var(--hud-danger);
    text-shadow: 0 0 10px rgba(255, 68, 68, 0.65);
  }
  .steps {
    display: flex;
    gap: 3px;
    flex: 1;
    padding: 2px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
  }
  .steps i {
    flex: 1;
    height: 0.75rem;
    border-radius: 1px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    transition: background var(--transition-fast), box-shadow var(--transition-fast);
  }
  .steps i[data-on='true'] {
    background: linear-gradient(180deg, #ff9933, var(--hud-primary));
    border-color: rgba(255, 119, 0, 0.6);
    box-shadow: 0 0 6px rgba(255, 119, 0, 0.4);
  }
  .heat {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.88rem;
    padding: 0.15rem 0.5rem;
    border-radius: var(--radius-sm);
  }
  .heat[data-heat='normal'] {
    color: var(--hud-ok);
    background: rgba(68, 255, 136, 0.08);
  }
  .heat[data-heat='overheating'] {
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.12);
    box-shadow: 0 0 8px rgba(255, 170, 0, 0.25);
  }
  .heat[data-heat='heat_damage'] {
    color: var(--hud-danger);
    background: rgba(255, 68, 68, 0.15);
    box-shadow: 0 0 10px rgba(255, 68, 68, 0.35);
  }
  .alerts {
    list-style: none;
    margin: 0.75rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .alerts li {
    padding: 0.4rem 0.65rem;
    background: rgba(255, 170, 0, 0.08);
    border-left: 3px solid var(--hud-warning);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    color: var(--hud-warning);
    font-size: 0.85rem;
    font-weight: 500;
  }
</style>
