<script lang="ts">
  import { t } from '../../lib/i18n';
  import { diag, diagOn } from '../../lib/diag.svelte';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import SystemCard from './SystemCard.svelte';
  import type { RouteTopic } from './types';
  import type { SystemTopic } from '../exploration/types';

  const route = $derived(topic<RouteTopic>('navigation.route'));
  const expSystem = $derived(topic<SystemTopic>('exploration.system'));
  // Sin ruta activa la tira muestra el sistema actual con la misma ficha.
  const main = $derived(route ? (route.next ?? route.current) : null);
  const currentStarClasses = $derived(
    expSystem && expSystem.system === route?.current?.name ? expSystem.star_classes : null,
  );

  $effect(() => {
    if (!diagOn()) return;
    diag('ui.card.route_strip', {
      has_route: route !== undefined,
      has_main: main !== null && main !== undefined,
      has_next: route?.next != null,
      kind: route?.source ?? 'none',
      off_route: route?.off_route === true,
      following: route?.following?.length ?? 0,
      remaining_jumps: route?.remaining_jumps ?? -1,
      current_system_matches: currentStarClasses !== null,
    });
  });

  async function recalculate() {
    try {
      await action('navigation.recalculate');
    } catch {
      // El fallo lo comunica el servidor mediante alertas.
    }
  }

  async function clearPlan() {
    try {
      await action('navigation.clear-plan', {});
    } catch {
      // El fallo lo comunica el servidor.
    }
  }
</script>

<Panel titleKey="panel-route-strip" span="full" size="large">
  {#if !route || !main}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    {#if route.off_route}
      <div class="off-route" data-testid="off-route">
        <span>{t('strip-off-route')}</span>
        <div class="off-route-actions">
          <button type="button" data-testid="recalculate" onclick={recalculate}>
            {t('strip-recalculate')}
          </button>
          {#if route.source === 'plan'}
            <button type="button" class="btn-clear-plan" data-testid="strip-clear-plan" onclick={clearPlan}>
              {t('nav-plan-clear')}
            </button>
          {/if}
        </div>
      </div>
    {/if}
    {#if !route.next}
      <p class="empty">{t('strip-no-route')}</p>
    {/if}
    <div class="strip">
      <SystemCard
        card={main}
        big
        label={route.next ? t('strip-next-jump') : t('strip-current')}
        starClasses={!route.next ? currentStarClasses : null}
        remainingJumps={route.remaining_jumps} totalLy={route.total_ly} />
      <div class="following">
        {#each (route.following ?? []).slice(0, 3) as c, i (`${i}-${c.name}`)}
          <SystemCard card={c} />
        {/each}
      </div>
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0 0 0.5rem;
  }

  .strip {
    display: grid;
    grid-template-columns: 3fr 1fr;
    gap: 0.85rem;
    align-items: start;
  }

  .following {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  @media (max-width: 1599px) {
    .strip {
      grid-template-columns: minmax(0, 1fr);
    }

    .following {
      flex-direction: row;
      flex-wrap: wrap;
    }

    .following > :global(*) {
      flex: 1 1 14rem;
    }
  }

  @media (max-width: 899px) {
    .following {
      flex-direction: column;
    }
  }

  .off-route {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
    padding: 0.6rem 0.85rem;
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid var(--hud-warning);
    border-radius: 4px;
    color: var(--hud-warning);
    font-weight: 700;
  }

  .off-route-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-clear-plan {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: var(--hud-danger, #ef4444);
    padding: 0.3rem 0.65rem;
    border-radius: 4px;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 150ms ease;
  }

  .btn-clear-plan:hover {
    background: rgba(239, 68, 68, 0.3);
    border-color: #ef4444;
  }
</style>
