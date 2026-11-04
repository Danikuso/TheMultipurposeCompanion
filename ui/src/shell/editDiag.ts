import { diag } from '../lib/diag.svelte';
import { cancelEdit } from './layoutStore.svelte';

/** `tab` solo admite las disposiciones temáticas; las pestañas propias llevan nombre del usuario y se omiten. */
export function editTab(tabKey: string | null): Record<string, string> {
  return tabKey?.startsWith('layout:') ? { tab: tabKey } : {};
}

/** Cancela la edición y emite la telemetría; compartido por el botón Cancelar y por Esc. */
export function cancelEditWithDiag(tabKey: string | null): void {
  cancelEdit();
  diag('ui.layout.edit', { cancel: true, ...editTab(tabKey) });
}
