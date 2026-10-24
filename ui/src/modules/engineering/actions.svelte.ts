import { action } from '../../lib/ws.svelte';

/** Las acciones de estos paneles comparten un flujo seleccionado en el servidor. */
export const engineeringActions = $state({ pending: false, selectedSlot: null as string | null });
let context: string | undefined;
let contextGeneration = 0;

/** Cambiar de comandante o nave invalida esperas de WS, pero conserva el bloqueo HTTP. */
export function syncEngineeringContext(identity: string): void {
  if (identity === context) return;
  context = identity;
  contextGeneration++;
  engineeringActions.selectedSlot = null;
}

export async function shipFlowAction(name: string, args: unknown): Promise<void> {
  if (engineeringActions.pending) return;
  engineeringActions.pending = true;
  const generation = contextGeneration;
  const previousSlot = engineeringActions.selectedSlot;
  if (name === 'select_module') engineeringActions.selectedSlot = (args as { slot: string }).slot;
  try { await action(`engineering.${name}`, args); }
  catch (error) {
    if (generation === contextGeneration) engineeringActions.selectedSlot = previousSlot;
    throw error;
  }
  finally { engineeringActions.pending = false; }
}
