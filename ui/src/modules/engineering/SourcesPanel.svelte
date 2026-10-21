<script lang="ts">
 import { t } from '../../lib/i18n';
 import { topic } from '../../lib/ws.svelte';
 import Panel from '../../shell/Panel.svelte';
 import { categoryLabel, inventoryMaterial, sourceLabel } from './labels';
 import type { InventoryTopic, MaterialCategory, SourcesTopic } from './types';
 const categories: MaterialCategory[]=['Raw','Manufactured','Encoded'];
 const sources=$derived(topic<SourcesTopic>('engineering.sources'));
 const inventory=$derived(topic<InventoryTopic>('engineering.inventory'));
 const hints: Record<string,string>={mining:'mining',surface_prospecting:'surface-prospecting',hge:'hge',signal_source:'signal-source',data_scan:'data-scan',mission_reward:'mission-reward',salvage:'salvage',broker:'broker',site:'site'};
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
  {#each categories as category}<li data-trader={category}><strong>{categoryLabel(category)}</strong><p>{sourceLabel(sources?.traders[category])}</p></li>{/each}
  {#each Object.entries(sources?.brokers ?? {}) as [kind,status]}<li data-broker={kind}><strong>{t('hint-broker')} · {t(kind==='Human' ? 'engineering-broker-human' : kind==='Guardian' ? 'engineering-broker-guardian' : 'engineering-no-data')}</strong><p>{sourceLabel(status)}</p></li>{/each}
 </ul>
 <ul>{#each Object.entries(sources?.hints ?? {}) as [key,entries]}<li data-hint-material={key}><strong>{inventoryMaterial(key,inventory)}</strong><ul>{#each entries as hint}<li>{hintLabel(hint.kind,hint.detail_key)}</li>{:else}<li>{t('engineering-no-data')}</li>{/each}</ul></li>{/each}</ul>
</Panel>
<style>ul { list-style:none; padding:0; } li { overflow-wrap:anywhere; padding:.2rem 0; } li ul { padding-left:1rem; color:var(--hud-muted); } p { margin:.3rem 0 .6rem; }</style>
