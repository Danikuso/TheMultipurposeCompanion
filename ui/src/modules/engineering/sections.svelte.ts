import { onMount, untrack } from 'svelte';
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
 /** Filtro servidor por ingeniero (`engineer_id`). */
 engineerId?: number;
 /**
  * Para datos estáticos: identidad del contenido (catálogo + filtro). Si se indica, un cambio de
  * revisión NO recarga ni vacía las filas; la revisión solo se usa al pedir y, si el servidor
  * responde 409 durante la carga, se reintenta una vez por cada revisión nueva.
  */
 cacheKey?: string;
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
 let loadedRevision = $state<number | undefined>(undefined);
 // Clave primitiva: el objeto de petición solo cambia si cambia su valor, no cada vez que el topic se reemplaza.
 const requestKey = $derived.by(() => {
  const current = source();
  return current.enabled && current.revision !== undefined
   ? JSON.stringify({ resource, revision: current.cacheKey === undefined ? current.revision : 0, commander: current.commander, scope: current.scope, engineerId: current.engineerId, cacheKey: current.cacheKey })
   : null;
 });
 const request = $derived<(Omit<PageRequest, 'section'> & { cacheKey?: string }) | null>(requestKey === null ? null : JSON.parse(requestKey));
 const active = $derived(visible && live.connected && request !== null);
 const readers = Object.fromEntries(sections.map(section => [section, createPageReader<unknown>()])) as unknown as {
  [K in keyof Rows & string]: ReturnType<typeof createPageReader<Rows[K]>>;
 };
 function clear() { loadedRevision = undefined; for (const reader of Object.values(readers) as { clear(): void }[]) reader.clear(); }
 function start(next: NonNullable<typeof request>) {
  const revision = next.cacheKey === undefined ? next.revision : untrack(() => source().revision);
  if (revision === undefined) return;
  loadedRevision = revision;
  const { cacheKey: _ignored, ...rest } = next;
  for (const section of sections) void readers[section].load({ ...rest, revision, section });
 }
 $effect(() => {
  const next = request; const enabled = active;
  if (enabled && next) start(next);
  else clear();
  return clear;
 });
 // Datos estáticos: si una carga queda obsoleta (409), se reintenta con la revisión nueva.
 $effect(() => {
  const next = request;
  if (!active || !next || next.cacheKey === undefined || !sections.some(section => readers[section].state === 'stale')) return;
  const revision = source().revision;
  if (revision !== undefined && revision !== loadedRevision) untrack(() => start(next));
 });
 onMount(() => {
  const changed = () => { visible = !document.hidden; };
  document.addEventListener('visibilitychange', changed);
  return () => { document.removeEventListener('visibilitychange', changed); clear(); };
 });
 const ready = $derived(active && request !== null && loadedRevision !== undefined && sections.every(section => {
  const { cacheKey: _ignored, ...rest } = request!;
  return readers[section].key === pageKey({ ...rest, revision: loadedRevision!, section }) && readers[section].state === 'ready';
 }));
 const state = $derived(!active ? 'idle' : sections.some(section => readers[section].state === 'failed') ? 'failed' : sections.some(section => readers[section].state === 'stale') ? 'stale' : 'loading');
 return { get ready() { return ready; }, get state() { return state; }, ...readers };
}
