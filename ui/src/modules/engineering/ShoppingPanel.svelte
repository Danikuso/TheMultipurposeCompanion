<script lang="ts">
 import { t } from '../../lib/i18n';
 import { integer, decimal } from '../../lib/format';
 import { action, live, topic } from '../../lib/ws.svelte';
 import Panel from '../../shell/Panel.svelte';
 import { createGoalsReader } from './goals.svelte';
 import { blueprintLabel, engineeringDataLabel, inventoryMaterial } from './labels';
 import type { GoalsTopic, InventoryTopic } from './types';
 const reader=createGoalsReader(true);
 const inventory=$derived(topic<InventoryTopic>('engineering.inventory'));
 const summary=$derived(topic<GoalsTopic>('engineering.goals'));
 let pending=$state(false);
 let error=$state(false);
 const commander=$derived(topic<{name?:string}>('core.cmdr')?.name ?? '');
 let generation=0;
 $effect(() => {
  commander;
  generation++;
  error=false;
  return () => { generation++; };
 });
 async function update(name: string, args: unknown) {
  if (pending) return;
  pending=true; error=false; const current=generation;
  try { await action(`engineering.${name}`,args); } catch { if(current===generation) error=true; } finally { pending=false; }
 }
</script>
<Panel titleKey="panel-engineering-shopping" span={2}>
 {#if error}<p role="alert">{t('engineering-action-failed')}</p>{/if}
 {#if reader.ready}
  {#if !reader.goals.items.length}<p>{t('engineering-no-data')}</p>{/if}
  {#each reader.goals.items as goal, index (goal.id)}
   {@const blueprint=blueprintLabel(goal.request.blueprint)}
   {@const module=engineeringDataLabel('modtype',goal.request.module_type)}
   {@const experimental=goal.request.experimental ? engineeringDataLabel('exp',goal.request.experimental) : null}
   <article data-goal-id={goal.id}>
    <h3>{module.name} · {goal.slot} · {blueprint.name} {t('engineering-grade',{grade:integer(goal.request.target_grade)})}</h3>
    {#if module.pending}<small>{t('engineering-translation-pending')}</small>{/if}
    {#if blueprint.pending}<small>{t('engineering-translation-pending')}</small>{/if}
    <p>{experimental?.name ?? t('engineering-no-experimental')}{#if experimental?.pending} <small>{t('engineering-translation-pending')}</small>{/if}</p>
    <p>{t('engineering-rolls')}: {goal.request.rolls.map(integer).join(' / ')}</p>
    <strong>{t('engineering-ready-percent',{pct:decimal(goal.percent)})}</strong>
    {#if !goal.verified}<small>{t('engineering-unverified')}</small>{/if}
    {#if goal.completable && goal.engineer}<p class="ready">{t('engineering-goal-completable',{engineer:goal.engineer})}</p>{/if}
    <ul>{#each Object.entries(goal.missing) as [key,count]}<li>{inventoryMaterial(key,inventory)}: {t('engineering-goal-missing')}: {integer(count)}</li>{/each}</ul>
    <div class="actions">
     <button data-testid="goal-up-{index}" disabled={pending || !live.connected || index===0} onclick={() => update('move_goal',{id:goal.id,to:index-1})}>{t('engineering-goal-up')}</button>
     <button data-testid="goal-down-{index}" disabled={pending || !live.connected || index >= (summary?.total ?? 0)-1} onclick={() => update('move_goal',{id:goal.id,to:index+1})}>{t('engineering-goal-down')}</button>
     <button data-testid="goal-remove-{index}" disabled={pending || !live.connected} onclick={() => update('unpin_goal',{id:goal.id})}>{t('engineering-goal-remove')}</button>
    </div>
   </article>
  {/each}
  {#each ['needs','missing','surplus'] as section}
   <section data-section={section}><h3>{t(`engineering-aggregate-${section}`)}</h3><ul>
    {#each reader[section as 'needs'|'missing'|'surplus'].items as row (row.key)}<li>{inventoryMaterial(row.key,inventory,row.name_en)}: {integer(row.count)}</li>{:else}<li>{t('engineering-no-data')}</li>{/each}
   </ul></section>
  {/each}
 {:else}<p role="status">{t(reader.state==='failed' ? 'engineering-page-failed' : reader.state==='stale' ? 'engineering-goals-stale' : reader.state==='idle' ? 'engineering-no-data' : 'engineering-page-loading')}</p>{/if}
</Panel>
<style>
 article { border-bottom:1px solid var(--hud-border); padding:.5rem 0; overflow-wrap:anywhere; }
 h3 { font-size:.95rem; color:var(--hud-primary); } p { margin:.4rem 0; } small { color:var(--hud-muted); margin-left:.5rem; }
 .ready { color:var(--hud-primary); } ul { padding-left:1.2rem; overflow-wrap:anywhere; }
 .actions { display:flex; flex-wrap:wrap; gap:.4rem; }
 button { color:var(--hud-text); background:var(--hud-bg); border:1px solid var(--hud-muted); padding:.3rem .5rem; } button:disabled { opacity:.5; }
</style>
