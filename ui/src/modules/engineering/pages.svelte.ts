import type { EngineeringPage, PageRequest } from './types';
export function pageKey(request: PageRequest): string { return JSON.stringify([request.resource, request.revision, request.section, request.commander, request.scope ?? null]); }
/** A reader belongs to one mounted consumer; only a complete revision is published. */
export function createPageReader<T>() {
 let state = $state<'idle' | 'loading' | 'ready' | 'stale' | 'failed'>('idle');
 let items = $state<readonly T[]>([]);
 let key = $state<string | null>(null);
 let controller: AbortController | undefined;
 let generation = 0;
 function clear() { generation++; controller?.abort(); controller = undefined; items = []; key = null; state = 'idle'; }
 async function load(request: PageRequest) {
  clear(); const current = generation; controller = new AbortController(); const signal = controller.signal;
  key = pageKey(request); state = 'loading';
  try {
   let cursor = 0; let total: number | undefined; const rows: T[] = [];
   while (true) {
    const query = new URLSearchParams({ revision: String(request.revision), cursor: String(cursor), section: request.section });
    const response = await fetch(`/api/engineering/${request.resource}?${query}`, { cache: 'no-store', signal });
    if (current !== generation) return;
    if (response.status === 409) { state = 'stale'; return; }
    if (!response.ok) throw new Error('HTTP');
    const page: EngineeringPage<T> = await response.json();
    if (current !== generation) return;
    if (page.revision !== request.revision || page.section !== request.section || page.cursor !== cursor || !Number.isSafeInteger(page.total) || page.total < 0 || !Array.isArray(page.items) || (total !== undefined && total !== page.total)) throw new Error('Invalid page');
    total = page.total; rows.push(...page.items);
    if (rows.length > total) throw new Error('Invalid total');
    if (page.next_cursor === null) { if (rows.length !== total) throw new Error('Incomplete page'); break; }
    if (!Number.isSafeInteger(page.next_cursor) || page.next_cursor <= cursor || page.next_cursor !== cursor + page.items.length || !page.items.length || page.next_cursor >= total) throw new Error('Invalid cursor');
    cursor = page.next_cursor;
   }
   if (current === generation) { items = rows; state = 'ready'; }
  } catch { if (current === generation) { items = []; state = 'failed'; } }
 }
 return { get state() { return state; }, get items() { return items; }, get key() { return key; }, clear, load };
}
