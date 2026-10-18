<script lang="ts">
  import { t } from '../../lib/i18n';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import SystemCard from './SystemCard.svelte';
  import type { RouteTopic } from './types';

  const route = $derived(topic<RouteTopic>('navigation.route'));
  // Sin ruta activa la tira muestra el sistema actual con la misma ficha.
  const main = $derived(route ? (route.next ?? route.current) : null);

  async function recalculate() {
    try {
      await action('navigation.recalculate');
    } catch {
      // El fallo lo comunica el servidor mediante alertas.
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
        <button type="button" data-testid="recalculate" onclick={recalculate}>
          {t('strip-recalculate')}
        </button>
      </div>
    {/if}
    {#if !route.next}
      <p class="empty">{t('strip-no-route')}</p>
    {/if}
    <div class="strip">
      <SystemCard card={main} big remainingJumps={route.remaining_jumps} totalLy={route.total_ly} />
      <div class="following">
        {#each route.following.slice(0, 3) as c, i (`${i}-${c.name}`)}
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
    gap: 0.6rem;
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
    gap: 0.75rem;
    margin-bottom: 0.6rem;
    color: var(--hud-warning);
    font-weight: 700;
  }
</style>
