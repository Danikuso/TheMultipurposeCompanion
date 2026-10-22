import { onMount } from 'svelte';
import { live } from '../../lib/ws.svelte';
import { createPageReader, pageKey } from './pages.svelte';
import type { PageRequest } from './types';

export interface SectionsSource {
 /** Revisión publicada por el topic resumen; sin ella no se pide nada. */
 revision: number | undefined;
 commander: string;
 /** Falso mientras no hay nada que leer (p. ej. sin ranura abierta). */
 enabled: boolean;
 /** Parte de la clave: cambiar de ranura descarta las filas anteriores. */
 scope?: string;
}

/**
 * Lector privado por consumidor de todas las secciones de un recurso REST paginado.
 * Las filas solo se publican cuando todas las secciones pedidas son completas y de la
 * revisión, comandante y ámbito vigentes. Sin sondeo: solo se recarga al cambiar la clave.
 */
export function createSectionsReader<Rows extends Record<string, unknown>>(
 resource: PageRequest['resource'],
 sections: readonly (keyof Rows & string)[],
 source: () => SectionsSource,
) {
 let visible = $state(!document.hidden);
 // Clave primitiva: el objeto de petición solo cambia si cambia su valor, no cada vez que el topic se reemplaza.
 const requestKey = $derived.by(() => {
  const current = source();
  return current.enabled && current.revision !== undefined
   ? JSON.stringify({ resource, revision: current.revision, commander: current.commander, scope: current.scope })
   : null;
 });
 const request = $derived<Omit<PageRequest, 'section'> | null>(requestKey === null ? null : JSON.parse(requestKey));
 const active = $derived(visible && live.connected && request !== null);
 const readers = Object.fromEntries(sections.map(section => [section, createPageReader<unknown>()])) as unknown as {
  [K in keyof Rows & string]: ReturnType<typeof createPageReader<Rows[K]>>;
 };
 function clear() { for (const reader of Object.values(readers) as { clear(): void }[]) reader.clear(); }
 $effect(() => {
  const next = request; const enabled = active;
  if (enabled && next) for (const section of sections) void readers[section].load({ ...next, section });
  else clear();
  return clear;
 });
 onMount(() => {
  const changed = () => { visible = !document.hidden; };
  document.addEventListener('visibilitychange', changed);
  return () => { document.removeEventListener('visibilitychange', changed); clear(); };
 });
 const ready = $derived(active && request !== null && sections.every(section => readers[section].key === pageKey({ ...request!, section }) && readers[section].state === 'ready'));
 const state = $derived(!active ? 'idle' : sections.some(section => readers[section].state === 'failed') ? 'failed' : sections.some(section => readers[section].state === 'stale') ? 'stale' : 'loading');
 return { get ready() { return ready; }, get state() { return state; }, ...readers };
}
