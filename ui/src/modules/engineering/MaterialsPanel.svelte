<script lang="ts">
 import { onMount } from 'svelte';
 import { t } from '../../lib/i18n';
 import { integer } from '../../lib/format';
 import { live, topic } from '../../lib/ws.svelte';
 import Panel from '../../shell/Panel.svelte';
 import type { InventoryTopic, LockerTopic, LockerRow, LockerSection, MaterialCategory, SessionTopic } from './types';
 import { materialName, originLabel } from './labels';
 import { createPageReader, pageKey } from './pages.svelte';
 const categories: MaterialCategory[] = ['Raw', 'Manufactured', 'Encoded'];
 const categoryKeys = { Raw: 'raw', Manufactured: 'manufactured', Encoded: 'encoded' };
 const sections: LockerSection[] = ['items', 'components', 'consumables', 'data'];
 let tab = $state<MaterialCategory | 'locker'>('Raw');
 let section = $state<LockerSection>('components');
 let visible = $state(!document.hidden);
 const inventory = $derived(topic<InventoryTopic>('engineering.inventory'));
 const session = $derived(topic<SessionTopic>('engineering.session'));
 const revision = $derived(topic<LockerTopic>('engineering.locker')?.revision);
 const commander = $derived(topic<{ name?: string }>('core.cmdr')?.name ?? '');
 const reader = createPageReader<LockerRow>();
 const request = $derived(revision === undefined ? null : { resource: 'locker' as const, section, revision, commander });
 const currentKey = $derived(request ? pageKey(request) : null);
 const canRead = $derived(tab === 'locker' && visible && live.connected && request !== null);
 const grades = $derived(tab === 'locker' ? [] : Object.entries(inventory?.by_category[tab] ?? {}).sort(([a], [b]) => Number(a) - Number(b)));
 $effect(() => {
  // Read primitive dependencies: an as_of-only WS update must not restart REST.
  const active = canRead; const next = request;
  if (active && next) void reader.load(next); else reader.clear();
  return () => reader.clear();
 });
 onMount(() => { const changed = () => { visible = !document.hidden; }; document.addEventListener('visibilitychange', changed); return () => { document.removeEventListener('visibilitychange', changed); reader.clear(); }; });
</script>
<Panel titleKey="panel-engineering-materials" span={2}>
 <div class="tabs" role="tablist" aria-label={t('panel-engineering-materials')}>
  {#each categories as category}<button role="tab" aria-selected={tab === category} data-testid="tab-{category}" onclick={() => tab = category}>{t(`engineering-tab-${categoryKeys[category]}`)}</button>{/each}
  <button role="tab" aria-selected={tab === 'locker'} data-testid="tab-locker" onclick={() => tab = 'locker'}>{t('engineering-tab-locker')}</button>
 </div>
 {#if tab === 'locker'}
  <label>{t('engineering-locker-section')} <select bind:value={section} data-testid="locker-section">{#each sections as entry}<option value={entry}>{t(`engineering-locker-${entry}`)}</option>{/each}</select></label>
  {#if canRead && reader.key === currentKey && reader.state === 'ready'}
   {#if reader.items.length === 0}<p class="muted">{t('engineering-no-data')}</p>{/if}
   <ul>{#each reader.items as row (row.key)}<li data-key={row.key}><span>{row.name_localised ?? row.key}</span><span class="mono">{integer(row.count)}</span></li>{/each}</ul>
  {:else}<p class="muted" role="status">{t(reader.key === currentKey && reader.state === 'failed' ? 'engineering-page-failed' : reader.key === currentKey && reader.state === 'stale' ? 'engineering-page-stale' : canRead ? 'engineering-page-loading' : 'engineering-no-data')}</p>{/if}
 {:else}
  {#if grades.length === 0}<p class="muted">{t('engineering-no-data')}</p>{/if}
  {#each grades as [grade, rows] (grade)}
   <h3>{Number(grade) > 0 ? t('engineering-grade', { grade: integer(Number(grade)) }) : t('engineering-no-data')}</h3>
   <ul>{#each rows as row (row.key)}<li data-key={row.key}>
    <span>{materialName(row)}{#if row.known === false || grade === '0'} <small class="muted">{t('engineering-no-data')}</small>{/if}</span>
    <span class="mono">{inventory?.caps[grade] ? t('engineering-count-of-max', { have: integer(row.count), max: integer(inventory.caps[grade]) }) : integer(row.count)}</span>
    {#if inventory?.at_max.includes(row.key)}<strong class="warning">{t('engineering-at-max')}</strong>{/if}
   </li>{/each}</ul>
  {/each}
 {/if}
 <h3>{t('engineering-session-changes')}</h3>
 <ul>{#each Object.entries(session?.totals ?? {}) as [origin, counts] (origin)}<li data-origin={origin}><span>{originLabel(origin)}</span><span class="mono">+{integer(counts.in)} / −{integer(counts.out)}</span></li>{/each}</ul>
</Panel>
<style>
 .tabs { display:flex; flex-wrap:wrap; gap:.4rem; margin-bottom:.8rem; }
 button, select { color:var(--hud-text); background:var(--hud-bg); border:1px solid var(--hud-muted); padding:.3rem .5rem; max-width:100%; }
 button[aria-selected='true'] { border-color:var(--hud-primary); color:var(--hud-primary); }
 ul { list-style:none; padding:0; margin:.4rem 0; }
 li { display:flex; flex-wrap:wrap; justify-content:space-between; gap:.4rem 1rem; padding:.2rem 0; overflow-wrap:anywhere; }
 li > span:first-child { min-width:0; }
 h3 { font-size:.95rem; color:var(--hud-primary); margin:.8rem 0 .3rem; }
 .mono { font-family:var(--font-mono); white-space:nowrap; }
 .muted { color:var(--hud-muted); } .warning { flex-basis:100%; color:var(--hud-warning); font-size:.8rem; }
</style>
