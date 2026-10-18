<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { cleanFaction, type ZoneTopic } from './types';

  const zone = $derived(topic<ZoneTopic | null>('combat.conflict_zone'));
  const bonds = $derived(
    Object.entries(zone?.bonds_by_faction ?? {}).sort((a, b) => b[1] - a[1]),
  );
</script>

{#if zone}
  <Panel titleKey="panel-combat-zone" span={1}>
    <div data-testid="zone-panel">
      <div class="row">
        <span data-testid="zone-system">{zone.system ?? '—'}</span>
        {#if zone.side}<strong data-testid="zone-side">{t('combat-zone-side', { side: zone.side })}</strong>{/if}
      </div>
      <p data-testid="zone-kills">{t('combat-kills', { count: zone.kills })}</p>
      <ul>
        {#each bonds as [faction, amount] (faction)}
          <li data-testid="zone-bond">
            <span>{cleanFaction(faction) || t('combat-faction-unknown')}</span>
            <span class="mono">{credits(amount)}</span>
          </li>
        {/each}
      </ul>
    </div>
  </Panel>
{/if}

<style>
  .row, li { display: flex; justify-content: space-between; gap: 0.75rem; }
  ul { list-style: none; margin: 0; padding: 0; }
  p { margin: 0.3rem 0; }
  .mono { font-family: var(--font-mono); }
</style>
