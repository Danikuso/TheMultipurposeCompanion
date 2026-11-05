import type { CardWidth, LayoutCtx } from './cards';

export type { LayoutCtx };

export interface LayoutSlot {
  card: string;
  width: CardWidth;
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

type AutoLayout =
  | 'exploration'
  | 'ship'
  | 'combat'
  | 'mining'
  | 'engineering'
  | 'missions'
  | 'trade'
  | 'carrier'
  | 'colonisation'
  | 'powerplay';
const MANUAL: string[] = [
  'exploration',
  'ship',
  'combat',
  'mining',
  'engineering',
  'missions',
  'trade',
  'carrier',
  'colonisation',
  'powerplay',
];

import { DEFAULT_PRIORITIES } from './priorities';
export { DEFAULT_PRIORITIES };

/** Disposición elegida por relevancia: bloqueo táctico (null) > manual > prioridades configurables > null. */
export function autoLayout(
  mods: { id: string; relevance: string }[],
  manual: string | null,
  priorities: string[] = DEFAULT_PRIORITIES,
  locked = false,
): AutoLayout | null {
  if (locked) return null;
  if (manual !== null && MANUAL.includes(manual)) return manual as AutoLayout;
  const list = priorities && priorities.length > 0 ? priorities : DEFAULT_PRIORITIES;
  for (const role of list) {
    if (role === 'carrier') {
      if (isActive(rel(mods, 'carrier'))) return 'carrier';
    } else if (role === 'combat') {
      if (isActive(rel(mods, 'combat'))) return 'combat';
    } else if (role === 'colonisation') {
      if (isActive(rel(mods, 'colonisation'))) return 'colonisation';
    } else if (role === 'mining') {
      if (rel(mods, 'mining') === 'active') return 'mining';
    } else if (role === 'engineering') {
      if (isActive(rel(mods, 'engineering'))) return 'engineering';
    } else if (role === 'missions') {
      if (isActive(rel(mods, 'missions'))) return 'missions';
    } else if (role === 'trade') {
      if (isActive(rel(mods, 'trade'))) return 'trade';
    } else if (role === 'powerplay') {
      if (isActive(rel(mods, 'powerplay'))) return 'powerplay';
    } else if (role === 'exploration') {
      if (isActive(rel(mods, 'exploration')) || isActive(rel(mods, 'exobiology'))) return 'exploration';
    } else if (MANUAL.includes(role)) {
      if (isActive(rel(mods, role))) return role as AutoLayout;
    }
  }
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
    { card: 'route', width: 'full' },
    { card: 'exploration-context', width: 2 },
    { card: 'planner', width: 1 },
    { card: 'jump', width: 1 },
  ],
}, {
  id: 'ship',
  titleKey: 'layout-ship',
  icon: 'ship',
  slots: [
    { card: 'ship-detail', width: 2 },
    { card: 'jump', width: 1 },
    { card: 'cargo', width: 1 },
    { card: 'ship-outfitting-search', width: 2 },
  ],
}, {
  id: 'combat',
  titleKey: 'layout-combat',
  icon: 'crosshair',
  slots: [
    { card: 'target', width: 2 },
    { card: 'hunting', width: 1 },
    { card: 'ship', width: 1 },
    { card: 'ax', width: 2 },
    { card: 'cz', width: 2 },
  ],
}, {
  id: 'mining',
  titleKey: 'layout-mining',
  icon: 'pickaxe',
  slots: [
    { card: 'mining-targets', width: 2 },
    { card: 'prospector', width: 2 },
    { card: 'mining-session', width: 1 },
    { card: 'sell', width: 1 },
  ],
}, {
  id: 'trade',
  titleKey: 'layout-trade',
  icon: 'scale',
  slots: [
    { card: 'market', width: 2 },
    { card: 'cargo', width: 1 },
    { card: 'trade-session', width: 1 },
    { card: 'trade-route', width: 'full' },
    { card: 'search', width: 2 },
  ],
}, {
  id: 'missions',
  titleKey: 'layout-missions',
  icon: 'clipboard',
  slots: [
    { card: 'missions', width: 2 },
    { card: 'mission-alerts', width: 1 },
    { card: 'massacre', width: 1 },
    { card: 'passengers', width: 1 },
    { card: 'tourism', width: 2 },
    { card: 'rewards', width: 1 },
    { card: 'community-goals', width: 1 },
  ],
}, {
  id: 'engineering',
  titleKey: 'layout-engineering',
  icon: 'wrench',
  slots: [
    { card: 'materials', width: 2 },
    { card: 'engineers', width: 1 },
    { card: 'engineer-guide', width: 2 },
    { card: 'eng-alerts', width: 1 },
    { card: 'eng-ship', width: 1 },
    { card: 'eng-flow', width: 2 },
    { card: 'shopping', width: 2 },
    { card: 'trades', width: 1 },
    { card: 'sources', width: 1 },
  ],
}, {
  id: 'carrier',
  titleKey: 'layout-carrier',
  icon: 'anchor',
  slots: [
    { card: 'carrier-overview', width: 2 },
    { card: 'carrier-jump', width: 1 },
    { card: 'carrier-finances', width: 1 },
  ],
}, {
  id: 'colonisation',
  titleKey: 'layout-colonisation',
  icon: 'hammer',
  slots: [
    { card: 'colonisation-progress', width: 2 },
    { card: 'colonisation-supply', width: 1 },
  ],
}, {
  id: 'powerplay',
  titleKey: 'layout-powerplay',
  icon: 'crown',
  slots: [
    { card: 'powerplay-status', width: 2 },
    { card: 'bgs-system', width: 2 },
  ],
}];
