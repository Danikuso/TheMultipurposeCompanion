import type { Component } from 'svelte';

export interface LayoutCtx {
  modules: { id: string; relevance: string }[];
  manual: string | null;
}

export interface LayoutSlot {
  id: string;
  panel: Component;
  span: 'full' | 1 | 2;
  when?: (ctx: LayoutCtx) => boolean;
}

export interface LayoutDef {
  id: string;
  titleKey: string;
  icon: string;
  slots: LayoutSlot[];
}

export function contextPanel(
  mods: { id: string; relevance: string }[],
  manual: string | null,
): 'exploration' | 'exobiology' {
  if (manual === 'exploration' || manual === 'exobiology') return manual;
  const exo = mods.find((m) => m.id === 'exobiology');
  if (exo && (exo.relevance === 'active' || exo.relevance === 'urgent')) return 'exobiology';
  return 'exploration';
}

/** Se rellena en la Task 31. */
export const layouts: LayoutDef[] = [];
