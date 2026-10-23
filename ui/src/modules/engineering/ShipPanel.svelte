<script lang="ts">
  import { t } from '../../lib/i18n';
  import { integer, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import { engineeringActions, shipFlowAction, syncEngineeringContext } from './actions.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blueprintLabel, engineeringDataLabel } from './labels';
  import type { EngineeringShipTopic, EngineeringFlowTopic } from './types';
  const data = $derived(topic<EngineeringShipTopic>('engineering.ship'));
  const selected = $derived(topic<EngineeringFlowTopic>('engineering.flow')?.slot);
  const identity = $derived(JSON.stringify([topic<{ name?: string }>('core.cmdr')?.name, data?.ship_id]));
  let failed = $state(false);
  let generation = 0;
  $effect(() => {
    syncEngineeringContext(identity);
    generation++;
    failed = false;
  });
  async function select(slot: string) {
    if (engineeringActions.pending || slot === selected) return;
    const current = ++generation;
    failed = false;
    try { await shipFlowAction('select_module', { slot }); }
    catch { if (current === generation) failed = true; }
  }
</script>
<Panel titleKey="panel-engineering-ship" span={2}>
  {#if failed}<p role="alert">{t('engineering-action-failed')}</p>{/if}
  {#if !data?.modules.length}<p>{t('engineering-no-data')}</p>{/if}
  <ul>
    {#each data?.modules ?? [] as row (row.slot)}
      {@const module = row.module_type ? engineeringDataLabel('modtype', row.module_type) : { name: row.item, pending: false }}
      <li data-slot={row.slot}>
        {#if row.engineerable}<button data-testid={`module-${row.slot}`} disabled={engineeringActions.pending || selected === row.slot} aria-pressed={selected === row.slot} onclick={() => select(row.slot)}>{module.name} · {row.slot}</button>
        {:else}<span>{module.name} · {row.slot}</span>{/if}
        {#if module.pending}<small>{t('engineering-translation-pending')}</small>{/if}
        {#if row.current}
          {@const blueprint = blueprintLabel(row.current.blueprint)}
          <span>{blueprint.name}</span>{#if blueprint.pending}<small>{t('engineering-translation-pending')}</small>{/if}
          <span>{t('engineering-grade', { grade: integer(row.current.level) })}</span>
          <span>{t('engineering-col-quality')}: {pct(row.current.quality)}</span>
          {#if row.current.experimental}
            {@const experimental = engineeringDataLabel('exp', row.current.experimental)}
            <span>{experimental.name}</span>{#if experimental.pending}<small>{t('engineering-translation-pending')}</small>{/if}
          {/if}
        {:else}<span>{t('engineering-stock')}</span>{/if}
      </li>
    {/each}
  </ul>
</Panel>
<style>
  ul { list-style: none; padding: 0; }
  li { display: flex; flex-wrap: wrap; gap: .5rem; padding: .6rem 0; border-bottom: 1px solid var(--hud-border); overflow-wrap: anywhere; }
  button { background: var(--hud-bg); border: 1px solid var(--hud-border); color: var(--hud-primary); padding: .3rem; }
  small { color: var(--hud-muted); }
</style>

