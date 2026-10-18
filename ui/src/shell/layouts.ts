import type { Component } from 'svelte';
import { RouteStrip, PlannerPanel, JumpPanel } from '../modules/navigation';
import SystemBodiesPanel from '../modules/exploration/SystemBodiesPanel.svelte';
import ExobiologyPanel from '../modules/exobiology/ExobiologyPanel.svelte';
import { TargetPanel, ShipPanel, HuntingPanel, ConflictZonePanel, AxPanel } from '../modules/combat';
import { ProspectorPanel, MiningSessionPanel, SellPanel } from '../modules/mining';
import { topic } from '../lib/ws.svelte';

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

const isActive = (r?: string) => r === 'active' || r === 'urgent';
const rel = (mods: { id: string; relevance: string }[], id: string) =>
  mods.find((m) => m.id === id)?.relevance?.toLowerCase();
const axVisible = () => {
  const ax = topic<{ active?: boolean; presence?: boolean } | null>('combat.ax');
  return ax?.active === true || ax?.presence === true;
};
const czVisible = () => (topic<unknown>('combat.conflict_zone') ?? null) !== null;

/** Disposición elegida por relevancia: manual > combate > minería > exploración > null. */
export function autoLayout(
  mods: { id: string; relevance: string }[],
  manual: string | null,
): 'exploration' | 'combat' | 'mining' | null {
  if (manual === 'exploration' || manual === 'combat' || manual === 'mining') return manual;
  if (isActive(rel(mods, 'combat'))) return 'combat';
  if (rel(mods, 'mining') === 'active') return 'mining';
  if (isActive(rel(mods, 'exploration')) || isActive(rel(mods, 'exobiology'))) return 'exploration';
  return null;
}

/** Fija la elección manual solo hasta que el resultado automático cambie de valor. */
export function nextPin(prevAuto: string | null, newAuto: string | null, pin: string | null): string | null {
  return prevAuto === newAuto ? pin : null;
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
}, {
  id: 'combat',
  titleKey: 'layout-combat',
  icon: 'crosshair',
  slots: [
    { id: 'target', panel: TargetPanel, span: 2 },
    { id: 'hunting', panel: HuntingPanel, span: 1 },
    { id: 'ship', panel: ShipPanel, span: 1 },
    { id: 'ax', panel: AxPanel, span: 2, when: axVisible },
    { id: 'cz', panel: ConflictZonePanel, span: 2, when: czVisible },
  ],
}, {
  id: 'mining',
  titleKey: 'layout-mining',
  icon: 'pickaxe',
  slots: [
    { id: 'prospector', panel: ProspectorPanel, span: 2 },
    { id: 'mining-session', panel: MiningSessionPanel, span: 1 },
    { id: 'sell', panel: SellPanel, span: 1 },
  ],
}];
