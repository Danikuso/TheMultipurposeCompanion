<script lang="ts">
 import { t } from '../../lib/i18n';
 import { integer } from '../../lib/format';
 import { topic } from '../../lib/ws.svelte';
 import Panel from '../../shell/Panel.svelte';
 import { createGoalsReader } from './goals.svelte';
 import { categoryLabel, inventoryMaterial, sourceLabel } from './labels';
 import type { InventoryTopic, SourcesTopic } from './types';
 const reader=createGoalsReader();
 const inventory=$derived(topic<InventoryTopic>('engineering.inventory'));
 const sources=$derived(topic<SourcesTopic>('engineering.sources'));
 const trades=$derived(reader.ready ? reader.goals.items.flatMap(goal => goal.trades.map(trade => ({goal:goal.id,trade}))) : []);
</script>
<Panel titleKey="panel-engineering-trades">
 {#if reader.ready}
  <ul>{#each trades as {goal,trade}}<li data-trade-goal={goal}>
   <p>{t('engineering-trade-give')}: <strong>{integer(trade.give)} {inventoryMaterial(trade.give_key,inventory)}</strong> → {t('engineering-trade-receive')}: <strong>{integer(trade.get)} {inventoryMaterial(trade.get_key,inventory)}</strong></p>
   <p>{categoryLabel(trade.trader)} · {sourceLabel(sources?.traders[trade.trader])}</p>
  </li>{:else}<li>{t('engineering-no-data')}</li>{/each}</ul>
 {:else}<p role="status">{t(reader.state==='failed' ? 'engineering-page-failed' : reader.state==='stale' ? 'engineering-goals-stale' : reader.state==='idle' ? 'engineering-no-data' : 'engineering-page-loading')}</p>{/if}
</Panel>
<style>ul { list-style:none; padding:0; } li { border-bottom:1px solid var(--hud-border); padding:.4rem 0; overflow-wrap:anywhere; } p { margin:.3rem 0; }</style>
