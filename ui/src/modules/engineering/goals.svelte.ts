import { onMount } from 'svelte';
import { live, topic } from '../../lib/ws.svelte';
import { createPageReader, pageKey } from './pages.svelte';
import type { AggregateRow, GoalDetail, GoalsTopic } from './types';

/** Private readers per consumer: no shared cursors or partial cross-section renders. */
export function createGoalsReader(aggregates = false) {
 let visible = $state(!document.hidden);
 const revision = $derived(topic<GoalsTopic>('engineering.goals')?.revision);
 const commander = $derived(topic<{name?: string}>('core.cmdr')?.name ?? '');
 const request = $derived(revision === undefined ? null : {resource:'goals' as const,section:'goals',revision,commander});
 const currentKey = $derived(request ? pageKey(request) : null);
 const active = $derived(visible && live.connected && request !== null);
 const goals = createPageReader<GoalDetail>();
 const needs = createPageReader<AggregateRow>();
 const missing = createPageReader<AggregateRow>();
 const surplus = createPageReader<AggregateRow>();
 const readers = { goals, needs, missing, surplus };
 const sections = aggregates ? ['goals','needs','missing','surplus'] as const : ['goals'] as const;
 function clear() { for (const reader of Object.values(readers)) reader.clear(); }
 $effect(() => {
  const next = request; const enabled = active;
  if (enabled && next) for (const section of sections) void readers[section].load({...next,section});
  else clear();
  return clear;
 });
 onMount(() => {
  const changed = () => { visible = !document.hidden; };
  document.addEventListener('visibilitychange',changed);
  return () => { document.removeEventListener('visibilitychange',changed); clear(); };
 });
 const ready = $derived(active && request !== null && sections.every(section => readers[section].key === pageKey({...request!,section}) && readers[section].state === 'ready'));
 const state = $derived(!active ? 'idle' : sections.some(section => readers[section].state === 'failed') ? 'failed' : sections.some(section => readers[section].state === 'stale') ? 'stale' : 'loading');
 return { get ready() { return ready && goals.key === currentKey; }, get state() { return state; }, goals, needs, missing, surplus };
}
