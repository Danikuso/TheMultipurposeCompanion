<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { cleanFaction, type AxTopic } from './types';

  const ax = $derived(topic<AxTopic>('combat.ax'));
  const visible = $derived(ax != null && (ax.active || ax.presence));
  const kills = $derived(Object.entries(ax?.kills_by_type ?? {}).sort((a, b) => b[1] - a[1]));

  // Nombre localizado del tipo; si no hay clave, texto crudo limpio.
  function typeName(raw: string): string {
    const key = `combat-ax-type-${raw.trim().toLowerCase()}`;
    const name = t(key);
    if (name !== key) return name;
    return cleanFaction(raw).replaceAll('_', ' ') || t('combat-ax-type-unknown');
  }
</script>

{#if ax && visible}
  <Panel titleKey="panel-combat-ax" span={1}>
    <div data-testid="ax-panel">
      {#if ax.presence}
        <p class="banner" role="status" data-testid="ax-presence">{t('combat-thargoid-presence')}</p>
      {/if}
      {#if ax.target}
        <p class="target" data-testid="ax-target">{t('combat-ax-target', { target: typeName(ax.target) })}</p>
      {/if}
      <div class="bonds-card">
        <p class="mono" data-testid="ax-bonds">{t('combat-ax-bonds', { credits: credits(ax.bonds) })}</p>
      </div>
      <ul class="kills-list">
        {#each kills as [type, n] (type)}
          <li class="kill-row" data-testid="ax-kill-{type}"><span>{typeName(type)}</span><strong class="mono count-badge">{n}</strong></li>
        {/each}
      </ul>
    </div>
  </Panel>
{/if}

<style>
  .banner {
    color: #00e676;
    background: rgba(0, 230, 118, 0.08);
    border: 1px solid rgba(0, 230, 118, 0.3);
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.75rem;
    font-weight: 700;
    font-size: 0.85rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    margin: 0 0 0.65rem;
    text-shadow: 0 0 8px rgba(0, 230, 118, 0.35);
  }
  .target {
    font-size: 0.95rem;
    color: var(--hud-primary);
    font-weight: 600;
    margin: 0.3rem 0 0.6rem;
    padding: 0.35rem 0.65rem;
    background: rgba(255, 119, 0, 0.06);
    border-left: 3px solid var(--hud-primary);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }
  .bonds-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.45rem 0.75rem;
    background: rgba(0, 217, 255, 0.05);
    border: 1px solid rgba(0, 217, 255, 0.2);
    border-radius: var(--radius-sm);
    margin: 0.4rem 0 0.75rem;
  }
  .kills-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .kill-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.65rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }
  .count-badge {
    background: rgba(255, 119, 0, 0.15);
    color: var(--hud-primary);
    padding: 0.1rem 0.5rem;
    border-radius: 2px;
    border: 1px solid rgba(255, 119, 0, 0.3);
  }
  .mono {
    font-family: var(--font-mono);
  }
</style>
