<script lang="ts">
 import { t } from '../../lib/i18n';
 import { topic } from '../../lib/ws.svelte';
 import Panel from '../../shell/Panel.svelte';
 import { categoryLabel, inventoryMaterial, sourceLabel } from './labels';
 import type { InventoryTopic, MaterialCategory, RemoteStation, SourcesTopic, SourceStatus } from './types';
 const categories: MaterialCategory[]=['Raw','Manufactured','Encoded'];
 const sources=$derived(topic<SourcesTopic>('engineering.sources'));
 const inventory=$derived(topic<InventoryTopic>('engineering.inventory'));
 let copiedSystem=$state<string | null>(null);
 const hints: Record<string,string>={mining:'mining',surface_prospecting:'surface-prospecting',hge:'hge',signal_source:'signal-source',data_scan:'data-scan',mission_reward:'mission-reward',salvage:'salvage',broker:'broker',site:'site'};
 function okStation(status: SourceStatus | undefined): RemoteStation | null {
  return status && typeof status === 'object' && 'Ok' in status ? status.Ok : null;
 }
 async function copySystem(system: string) {
  try {
   if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(system);
    copiedSystem = system;
    setTimeout(() => { if (copiedSystem === system) copiedSystem = null; }, 2000);
   }
  } catch { /* Ignorado en entornos sin portapapeles */ }
 }
 function hintLabel(kind: string, detail: string | null) {
  if (!hints[kind]) return t('engineering-no-data');
  const main=t(`hint-${hints[kind]}`);
  if (!detail) return main;
  const translated=t(detail);
  return `${main} · ${translated===detail ? t('engineering-no-data') : translated}`;
 }
</script>
<Panel titleKey="panel-engineering-sources">
 <ul>
  {#each categories as category}
   {@const st = sources?.traders[category]}
   {@const ok = okStation(st)}
   <li data-trader={category}>
    <div class="source-head">
     <strong>{categoryLabel(category)}</strong>
     {#if ok}
      <button
       type="button"
       class="btn-copy"
       data-testid={`source-copy-trader-${category}`}
       title={t('engineering-copy-system')}
       onclick={() => copySystem(ok.system_name)}
      >{copiedSystem === ok.system_name ? t('engineering-copied') : t('engineering-copy-system')}</button>
     {/if}
    </div>
    <p>{sourceLabel(st)}</p>
   </li>
  {/each}
  {#each Object.entries(sources?.brokers ?? {}) as [kind,status]}
   {@const ok = okStation(status)}
   <li data-broker={kind}>
    <div class="source-head">
     <strong>{t('hint-broker')} · {t(kind==='Human' ? 'engineering-broker-human' : kind==='Guardian' ? 'engineering-broker-guardian' : 'engineering-no-data')}</strong>
     {#if ok}
      <button
       type="button"
       class="btn-copy"
       data-testid={`source-copy-broker-${kind}`}
       title={t('engineering-copy-system')}
       onclick={() => copySystem(ok.system_name)}
      >{copiedSystem === ok.system_name ? t('engineering-copied') : t('engineering-copy-system')}</button>
     {/if}
    </div>
    <p>{sourceLabel(status)}</p>
   </li>
  {/each}
 </ul>
 <ul>{#each Object.entries(sources?.hints ?? {}) as [key,entries]}<li data-hint-material={key}><strong>{inventoryMaterial(key,inventory)}</strong><ul>{#each entries as hint}<li>{hintLabel(hint.kind,hint.detail_key)}</li>{:else}<li>{t('engineering-no-data')}</li>{/each}</ul></li>{/each}</ul>
</Panel>
<style>
 ul { list-style:none; padding:0; }
 li { overflow-wrap:anywhere; padding:.2rem 0; }
 li ul { padding-left:1rem; color:var(--hud-muted); }
 p { margin:.3rem 0 .6rem; }
 .source-head { display:flex; align-items:center; justify-content:space-between; gap:.5rem; flex-wrap:wrap; }
 .btn-copy { background:transparent; border:1px solid var(--hud-border); border-radius:3px; color:var(--hud-muted); padding:.15rem .45rem; font-size:.72rem; cursor:pointer; line-height:1.2; flex-shrink:0; }
 .btn-copy:hover { color:var(--hud-primary); border-color:var(--hud-primary); }
</style>
