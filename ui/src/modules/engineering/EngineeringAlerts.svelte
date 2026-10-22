<script lang="ts">
 import { t } from '../../lib/i18n';
 import { integer } from '../../lib/format';
 import { action, live, topic } from '../../lib/ws.svelte';
 import Panel from '../../shell/Panel.svelte';
 import { goalLabel, inventoryMaterial } from './labels';
 import { createSectionsReader } from './sections.svelte';
 import type { EngineeringAlert, EngineerRows, EngineersTopic, InventoryTopic } from './types';
 const alerts=$derived(topic<{alerts: EngineeringAlert[]}>('engineering.alerts')?.alerts ?? []);
 const engineers=$derived(topic<EngineersTopic>('engineering.engineers'));
 const inventory=$derived(topic<InventoryTopic>('engineering.inventory'));
 // Los nombres salen de la página de ingenieros, solo si algún aviso los necesita.
 const names=createSectionsReader<EngineerRows>('engineers',['engineers'],() => ({revision:engineers?.revision,commander:topic<{name?:string}>('core.cmdr')?.name ?? '',enabled:alerts.some(alert => alert.engineer_id!==undefined)}));
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
 function label(alert: EngineeringAlert) {
  const engineer=(names.ready ? names.engineers.items.find(row => row.id===alert.engineer_id)?.name : undefined) ?? t('engineering-no-data');
  const material=alert.key ? inventoryMaterial(alert.key,inventory) : t('engineering-no-data');
  switch(alert.kind) {
   case 'material_at_max': return t('engineering-alert-at-max',{material,grade:integer(alert.grade ?? 0)});
   case 'engineer_unlocked': return t('engineering-alert-engineer-unlocked',{engineer});
   case 'engineer_invited': return t('engineering-alert-engineer-invited',{engineer});
   case 'scarce_obtained': return t('engineering-alert-scarce-obtained',{material});
   case 'goal_completable': return t(alert.engineer ? 'engineering-alert-goal-completable-with' : 'engineering-alert-goal-completable',{goal:goalLabel(alert.goal ?? ''),...(alert.engineer ? {engineer:alert.engineer} : {})});
   default: return t('engineering-no-data');
  }
 }
 async function dismiss(id: string) {
  if(pending) return;
  pending=true; error=false; const current=generation;
  try { await action('engineering.dismiss_alert',{id}); } catch { if(current===generation) error=true; } finally { pending=false; }
 }
</script>
<Panel titleKey="panel-engineering-alerts">
 {#if error}<p role="alert">{t('engineering-action-failed')}</p>{/if}
 <ul>{#each alerts as alert,index (alert.id)}<li data-alert-id={alert.id}><span>{label(alert)}</span><button data-testid="engineering-dismiss-{index}" disabled={pending || !live.connected} onclick={() => dismiss(alert.id)}>{t('engineering-dismiss')}</button></li>{:else}<li>{t('engineering-no-data')}</li>{/each}</ul>
</Panel>
<style>ul { list-style:none; padding:0; } li { display:flex; flex-wrap:wrap; align-items:center; justify-content:space-between; gap:.5rem; padding:.4rem 0; overflow-wrap:anywhere; } span { min-width:0; } button { color:var(--hud-text); background:var(--hud-bg); border:1px solid var(--hud-muted); padding:.3rem .5rem; } button:disabled { opacity:.5; }</style>
