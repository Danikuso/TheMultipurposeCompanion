import { untrack } from 'svelte';
import { live } from '../../lib/ws.svelte';

/** Los nombres de ingeniero son estáticos: se piden una vez por id (filtro servidor) y se conservan. */
const cache = $state<Record<number, string>>({});
export function resetEngineerNames(): void { for (const key of Object.keys(cache)) delete cache[Number(key)]; }
export function engineerName(id: number | undefined): string | undefined { return id === undefined ? undefined : cache[id]; }

/** Pide solo los ids que faltan; sin sondeo: se repite únicamente tras un 409, con la revisión nueva. */
export function useEngineerNames(ids: () => number[], revision: () => number | undefined): void {
 let stalled = $state(false);
 const missing = $derived(JSON.stringify([...new Set(ids())].filter(id => cache[id] === undefined).sort((a, b) => a - b)));
 $effect(() => {
  const wanted: number[] = JSON.parse(missing);
  const current = stalled ? revision() : untrack(revision);
  if (!wanted.length || current === undefined || !live.connected || document.hidden) return;
  const controller = new AbortController();
  let live_ = true;
  untrack(() => { stalled = false; });
  for (const id of wanted) {
   void (async () => {
    try {
     const query = new URLSearchParams({ revision: String(current), cursor: '0', section: 'engineers', engineer_id: String(id) });
     const response = await fetch(`/api/engineering/engineers?${query}`, { cache: 'no-store', signal: controller.signal });
     if (!live_) return;
     if (response.status === 409) { stalled = true; return; }
     if (!response.ok) return;
     const page: { items?: { id: number; name: string }[] } = await response.json();
     const row = page.items?.find(item => item.id === id);
     if (live_ && row) cache[id] = row.name;
    } catch { /* el nombre queda como «sin datos» */ }
   })();
  }
  return () => { live_ = false; controller.abort(); };
 });
}
