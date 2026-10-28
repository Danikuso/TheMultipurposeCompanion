import type { Component } from 'svelte';
import { RouteStrip, PlannerPanel, JumpPanel } from '../modules/navigation';
import SystemBodiesPanel from '../modules/exploration/SystemBodiesPanel.svelte';
import ExobiologyPanel from '../modules/exobiology/ExobiologyPanel.svelte';
import { TargetPanel, ShipPanel, HuntingPanel, ConflictZonePanel, AxPanel } from '../modules/combat';
import { ProspectorPanel, MiningSessionPanel, SellPanel } from '../modules/mining';
import {
  MarketPanel, CargoPanel, RoutePanel as TradeRoutePanel, TradeSessionPanel, SearchPanel,
} from '../modules/trade';
import {
  MissionsPanel, MassacrePanel, MissionAlerts, PassengersPanel, TourismPanel, RewardsPanel, CommunityGoalsPanel,
} from '../modules/missions';
import type {
  MassacreTopic, PassengersTopic, TourismTopic, CommunityGoalsTopic,
} from '../modules/missions/types';
import {
  MaterialsPanel, EngineersPanel, ShipPanel as EngineeringShipPanel, EngineerFlow,
  ShoppingPanel, TradesPanel, SourcesPanel, EngineeringAlerts,
} from '../modules/engineering';
import { CarrierOverviewPanel, CarrierJumpPanel, CarrierFinancesPanel } from '../modules/carrier';
import { ColonisationProgressPanel, ColonisationSupplyPanel } from '../modules/colonisation';
import { PowerplayPanel } from '../modules/powerplay';
import { BgsPanel } from '../modules/bgs';
import { ShipPanel as DashboardShipPanel, OutfittingSearchPanel } from '../modules/dashboard';
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
const massacreVisible = () => (topic<MassacreTopic>('missions.massacre')?.piles?.length ?? 0) > 0;
const passengersVisible = () => {
  const p = topic<PassengersTopic>('missions.passengers');
  return (p?.cabins?.seats_total ?? 0) > 0 || (p?.groups?.length ?? 0) > 0;
};
const tourismVisible = () => (topic<TourismTopic>('missions.tourism')?.trips?.length ?? 0) > 0;
const cgVisible = () => (topic<CommunityGoalsTopic>('missions.community_goals')?.goals?.length ?? 0) > 0;
const czVisible = () => (topic<unknown>('combat.conflict_zone') ?? null) !== null;

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

/** Disposición elegida por relevancia: manual > carrier > combate > colonización > minería > ingeniería > misiones > comercio > powerplay > exploración > null. */
export function autoLayout(
  mods: { id: string; relevance: string }[],
  manual: string | null,
): AutoLayout | null {
  if (manual !== null && MANUAL.includes(manual)) return manual as AutoLayout;
  if (isActive(rel(mods, 'carrier'))) return 'carrier';
  if (isActive(rel(mods, 'combat'))) return 'combat';
  if (isActive(rel(mods, 'colonisation'))) return 'colonisation';
  if (rel(mods, 'mining') === 'active') return 'mining';
  if (isActive(rel(mods, 'engineering'))) return 'engineering';
  if (isActive(rel(mods, 'missions'))) return 'missions';
  if (isActive(rel(mods, 'trade'))) return 'trade';
  if (isActive(rel(mods, 'powerplay'))) return 'powerplay';
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
  id: 'ship',
  titleKey: 'layout-ship',
  icon: 'ship',
  slots: [
    { id: 'ship-detail', panel: DashboardShipPanel, span: 2 },
    { id: 'ship-jump', panel: JumpPanel, span: 1 },
    { id: 'ship-cargo', panel: CargoPanel, span: 1 },
    { id: 'ship-outfitting-search', panel: OutfittingSearchPanel, span: 2 },
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
}, {
  id: 'trade',
  titleKey: 'layout-trade',
  icon: 'scale',
  slots: [
    { id: 'market', panel: MarketPanel, span: 2 },
    { id: 'cargo', panel: CargoPanel, span: 1 },
    { id: 'trade-session', panel: TradeSessionPanel, span: 1 },
    { id: 'trade-route', panel: TradeRoutePanel, span: 'full' },
    { id: 'search', panel: SearchPanel, span: 2 },
  ],
}, {
  id: 'missions',
  titleKey: 'layout-missions',
  icon: 'clipboard',
  slots: [
    { id: 'missions', panel: MissionsPanel, span: 2 },
    { id: 'mission-alerts', panel: MissionAlerts, span: 1 },
    { id: 'massacre', panel: MassacrePanel, span: 1, when: massacreVisible },
    { id: 'passengers', panel: PassengersPanel, span: 1, when: passengersVisible },
    { id: 'tourism', panel: TourismPanel, span: 2, when: tourismVisible },
    { id: 'rewards', panel: RewardsPanel, span: 1 },
    { id: 'community-goals', panel: CommunityGoalsPanel, span: 1, when: cgVisible },
  ],
}, {
  id: 'engineering',
  titleKey: 'layout-engineering',
  icon: 'wrench',
  slots: [
    { id: 'materials', panel: MaterialsPanel, span: 2 },
    { id: 'engineers', panel: EngineersPanel, span: 1 },
    { id: 'eng-alerts', panel: EngineeringAlerts, span: 1 },
    { id: 'eng-ship', panel: EngineeringShipPanel, span: 1 },
    { id: 'eng-flow', panel: EngineerFlow, span: 2,
      when: () => topic<{ slot?: string | null }>('engineering.flow')?.slot != null },
    { id: 'shopping', panel: ShoppingPanel, span: 2 },
    { id: 'trades', panel: TradesPanel, span: 1 },
    { id: 'sources', panel: SourcesPanel, span: 1 },
  ],
}, {
  id: 'carrier',
  titleKey: 'layout-carrier',
  icon: 'anchor',
  slots: [
    { id: 'carrier-overview', panel: CarrierOverviewPanel, span: 2 },
    { id: 'carrier-jump', panel: CarrierJumpPanel, span: 1 },
    { id: 'carrier-finances', panel: CarrierFinancesPanel, span: 1 },
  ],
}, {
  id: 'colonisation',
  titleKey: 'layout-colonisation',
  icon: 'hammer',
  slots: [
    { id: 'colonisation-progress', panel: ColonisationProgressPanel, span: 2 },
    { id: 'colonisation-supply', panel: ColonisationSupplyPanel, span: 1 },
  ],
}, {
  id: 'powerplay',
  titleKey: 'layout-powerplay',
  icon: 'crown',
  slots: [
    { id: 'powerplay-status', panel: PowerplayPanel, span: 2 },
    { id: 'bgs-system', panel: BgsPanel, span: 2 },
  ],
}];
