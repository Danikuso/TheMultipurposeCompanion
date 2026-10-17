<script lang="ts">
  import { t } from '../../lib/i18n';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { pushToast } from '../../shell/Toasts.svelte';

  interface RouteHop {
    system: string;
    star_class?: string;
  }

  interface RouteTopic {
    next_system?: string | null;
    remaining_jumps?: number | null;
    hops?: RouteHop[];
  }

  const route = $derived(topic<RouteTopic>('core.route'));
  const hasRoute = $derived(
    route != null &&
      (Boolean(route.next_system) || (route.hops != null && route.hops.length > 0)),
  );

  async function handleCopy() {
    try {
      const res = (await action('clipboard.copy-next-system')) as { system?: string } | undefined;
      const sys = res?.system ?? route?.next_system ?? '';
      if (sys) {
        pushToast(t('toast-copied', { system: sys }), 'Info');
      }
    } catch {
      // Action failure ignored in UI
    }
  }
</script>

<Panel titleKey="panel-route">
  {#if !hasRoute || !route}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="route-head">
      <div class="next-wrap">
        <span class="label">{t('field-next-system')}</span>
        <strong class="next-sys">{route.next_system ?? '—'}</strong>
      </div>
      <button
        type="button"
        data-testid="copy-next-system"
        disabled={!route.next_system}
        onclick={handleCopy}
      >
        {t('action-copy-next-system')}
      </button>
    </div>

    <div class="jumps-row">
      <span class="label">{t('field-remaining-jumps')}</span>
      <span class="mono">{route.remaining_jumps ?? 0}</span>
    </div>

    {#if route.hops && route.hops.length > 0}
      <ul class="hops-list">
        {#each route.hops as hop (hop.system)}
          <li>
            <span class="mono">{hop.system}</span>
            {#if hop.star_class}
              <span class="star-class">{hop.star_class}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .route-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.6rem;
  }

  .next-wrap {
    display: flex;
    flex-direction: column;
  }

  .label {
    color: var(--hud-muted);
    font-size: 0.8rem;
  }

  .next-sys {
    font-family: var(--font-mono);
    color: var(--hud-primary);
    font-size: 1rem;
  }

  .jumps-row {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .hops-list {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 140px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .hops-list li {
    display: flex;
    justify-content: space-between;
    font-size: 0.82rem;
    padding: 0.15rem 0.35rem;
    background: var(--hud-bg);
    border-radius: 2px;
  }

  .star-class {
    color: var(--hud-secondary);
    font-family: var(--font-mono);
  }
</style>
