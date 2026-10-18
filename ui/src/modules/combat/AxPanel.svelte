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
        <p data-testid="ax-target">{t('combat-ax-target', { target: typeName(ax.target) })}</p>
      {/if}
      <p class="mono" data-testid="ax-bonds">{t('combat-ax-bonds', { credits: credits(ax.bonds) })}</p>
      <ul>
        {#each kills as [type, n] (type)}
          <li data-testid="ax-kill-{type}"><span>{typeName(type)}</span><strong class="mono">{n}</strong></li>
        {/each}
      </ul>
    </div>
  </Panel>
{/if}

<style>
  .banner { color: var(--hud-warning); font-weight: 600; margin: 0 0 0.4rem; }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; justify-content: space-between; gap: 0.75rem; }
  p { margin: 0.3rem 0; }
  .mono { font-family: var(--font-mono); }
</style>
