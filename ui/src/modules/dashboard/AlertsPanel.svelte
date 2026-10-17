<script lang="ts">
  import { t } from '../../lib/i18n';
  import { topic, type Alert } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';

  const alerts = $derived(topic<Alert[]>('dashboard.alerts'));
</script>

<Panel titleKey="panel-alerts">
  {#if !alerts || alerts.length === 0}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <ul class="alerts-list">
      {#each alerts as a (a.id || a.key)}
        <li class="alert-item level-{a.level.toLowerCase()}">
          <span>{t(a.key, a.args)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .alerts-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .alert-item {
    padding: 0.35rem 0.6rem;
    border-radius: 3px;
    border-left: 3px solid var(--hud-primary);
    background: var(--hud-bg);
  }

  .level-warning {
    border-left-color: var(--hud-warning);
    color: var(--hud-warning);
  }

  .level-critical {
    border-left-color: var(--hud-danger);
    color: var(--hud-danger);
    font-weight: 600;
  }
</style>
