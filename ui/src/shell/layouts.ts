import type { Component } from 'svelte';
import { RouteStrip, PlannerPanel, JumpPanel } from '../modules/navigation';
import SystemBodiesPanel from '../modules/exploration/SystemBodiesPanel.svelte';
import ExobiologyPanel from '../modules/exobiology/ExobiologyPanel.svelte';

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

export const layouts: LayoutDef[] = [{
  id: 'exploration',
  titleKey: 'layout-exploration',
  icon: 'compass',
  slots: [
    { id: 'route', panel: RouteStrip, span: 'full' },
    { id: 'context-bodies', panel: SystemBodiesPanel, span: 2,
      when: (ctx) => contextPanel(ctx.modules, ctx.manual) === 'exploration' },
    { id: 'context-exobiology', panel: ExobiologyPanel, span: 2,
      when: (ctx) => contextPanel(ctx.modules, ctx.manual) === 'exobiology' },
    { id: 'planner', panel: PlannerPanel, span: 1 },
    { id: 'jump', panel: JumpPanel, span: 1 },
  ],
}];
