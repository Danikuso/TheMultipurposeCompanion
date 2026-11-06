import type { Component } from 'svelte';
import { RouteStrip, PlannerPanel, JumpPanel } from '../modules/navigation';
import { TargetPanel, ShipPanel, HuntingPanel, ConflictZonePanel, AxPanel } from '../modules/combat';
import { ProspectorPanel, MiningSessionPanel, SellPanel, MiningTargetsPanel } from '../modules/mining';
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
  MaterialsPanel, EngineersPanel, EngineerGuidePanel, ShipPanel as EngineeringShipPanel, EngineerFlow,
  ShoppingPanel, TradesPanel, SourcesPanel, EngineeringAlerts,
} from '../modules/engineering';
import { CarrierOverviewPanel, CarrierJumpPanel, CarrierFinancesPanel } from '../modules/carrier';
import { ColonisationProgressPanel, ColonisationSupplyPanel } from '../modules/colonisation';
import { PowerplayPanel } from '../modules/powerplay';
import { BgsPanel } from '../modules/bgs';
import {
  CmdrPanel, ShipPanel as DashboardShipPanel, OutfittingSearchPanel, LocationPanel, StatusPanel,
  RoutePanel as DashboardRoutePanel, SessionPanel, AlertsPanel,
} from '../modules/dashboard';
import SystemBodiesPanel from '../modules/exploration/SystemBodiesPanel.svelte';
import ExobiologyPanel from '../modules/exobiology/ExobiologyPanel.svelte';
import { AccountsPanel } from '../modules/accounts';
import { CrimePanel } from '../modules/crime';
import ExplorationContextCard from './ExplorationContextCard.svelte';
import { modules } from '../modules/registry';
import { layouts } from './layouts';
import { topic } from '../lib/ws.svelte';
import { t } from '../lib/i18n';

export type CardWidth = 1 | 2 | 3 | 'full';

export interface LayoutCtx {
  modules: { id: string; relevance: string }[];
  manual: string | null;
}

export interface CardDef {
  id: string;
  titleKey: string;
  module: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  component: Component<any>;
  defaultWidth: CardWidth;
  /** False for optional new cards that must not be appended to saved tab layouts. */
  addToSaved?: boolean;
  when?: (ctx: LayoutCtx) => boolean;
}

export interface CardPlacement { id: string; x?: number | null; w: CardWidth; h: number | null }

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
const flowVisible = () => topic<{ slot?: string | null }>('engineering.flow')?.slot != null;

const defs: CardDef[] = [
  // Navegación y exploración
  { id: 'route', titleKey: 'panel-route-strip', module: 'navigation', component: RouteStrip, defaultWidth: 'full' },
  { id: 'planner', titleKey: 'panel-planner', module: 'navigation', component: PlannerPanel, defaultWidth: 2 },
  { id: 'jump', titleKey: 'panel-jump', module: 'navigation', component: JumpPanel, defaultWidth: 1 },
  { id: 'exploration-context', titleKey: 'layout-context', module: 'exploration',
    component: ExplorationContextCard, defaultWidth: 'full' },
  { id: 'exploration-system-bodies', titleKey: 'panel-system-bodies', module: 'exploration',
    component: SystemBodiesPanel, defaultWidth: 'full' },
  { id: 'exobiology-exobiology', titleKey: 'panel-exobiology', module: 'exobiology',
    component: ExobiologyPanel, defaultWidth: 'full' },
  // Panel de control y nave
  { id: 'dashboard-cmdr', titleKey: 'panel-cmdr', module: 'dashboard', component: CmdrPanel, defaultWidth: 1 },
  { id: 'ship-detail', titleKey: 'panel-ship', module: 'ship', component: DashboardShipPanel, defaultWidth: 2 },
  { id: 'dashboard-location', titleKey: 'panel-location', module: 'dashboard', component: LocationPanel, defaultWidth: 1 },
  { id: 'dashboard-status', titleKey: 'panel-status', module: 'dashboard', component: StatusPanel, defaultWidth: 1 },
  { id: 'dashboard-route', titleKey: 'panel-route', module: 'dashboard', component: DashboardRoutePanel, defaultWidth: 1 },
  { id: 'dashboard-session', titleKey: 'panel-session', module: 'dashboard', component: SessionPanel, defaultWidth: 1 },
  { id: 'dashboard-alerts', titleKey: 'panel-alerts', module: 'dashboard', component: AlertsPanel, defaultWidth: 1 },
  { id: 'ship-outfitting-search', titleKey: 'panel-ship-outfitting', module: 'ship',
    component: OutfittingSearchPanel, defaultWidth: 2 },
  // Combate
  { id: 'target', titleKey: 'combat-target', module: 'combat', component: TargetPanel, defaultWidth: 1 },
  { id: 'ship', titleKey: 'panel-combat-ship', module: 'combat', component: ShipPanel, defaultWidth: 1 },
  { id: 'hunting', titleKey: 'panel-combat-hunting', module: 'combat', component: HuntingPanel, defaultWidth: 2 },
  { id: 'cz', titleKey: 'panel-combat-zone', module: 'combat', component: ConflictZonePanel,
    defaultWidth: 1, when: czVisible },
  { id: 'ax', titleKey: 'panel-combat-ax', module: 'combat', component: AxPanel, defaultWidth: 1, when: axVisible },
  // Minería
  { id: 'mining-targets', titleKey: 'panel-mining-targets', module: 'mining',
    component: MiningTargetsPanel, defaultWidth: 2 },
  { id: 'prospector', titleKey: 'panel-mining-prospector', module: 'mining', component: ProspectorPanel, defaultWidth: 2 },
  { id: 'mining-session', titleKey: 'panel-mining-session', module: 'mining',
    component: MiningSessionPanel, defaultWidth: 1 },
  { id: 'sell', titleKey: 'panel-mining-sell', module: 'mining', component: SellPanel, defaultWidth: 1 },
  // Comercio
  { id: 'market', titleKey: 'panel-trade-market', module: 'trade', component: MarketPanel, defaultWidth: 2 },
  { id: 'cargo', titleKey: 'panel-trade-cargo', module: 'trade', component: CargoPanel, defaultWidth: 2 },
  { id: 'trade-route', titleKey: 'panel-trade-route', module: 'trade', component: TradeRoutePanel, defaultWidth: 2 },
  { id: 'trade-session', titleKey: 'panel-trade-session', module: 'trade', component: TradeSessionPanel, defaultWidth: 1 },
  { id: 'search', titleKey: 'panel-trade-search', module: 'trade', component: SearchPanel, defaultWidth: 1 },
  // Misiones
  { id: 'missions', titleKey: 'panel-missions-active', module: 'missions', component: MissionsPanel, defaultWidth: 2 },
  { id: 'massacre', titleKey: 'panel-missions-massacre', module: 'missions', component: MassacrePanel,
    defaultWidth: 2, when: massacreVisible },
  { id: 'mission-alerts', titleKey: 'panel-missions-alerts', module: 'missions', component: MissionAlerts, defaultWidth: 1 },
  { id: 'passengers', titleKey: 'panel-missions-passengers', module: 'missions', component: PassengersPanel,
    defaultWidth: 1, when: passengersVisible },
  { id: 'tourism', titleKey: 'panel-missions-tourism', module: 'missions', component: TourismPanel,
    defaultWidth: 2, when: tourismVisible },
  { id: 'rewards', titleKey: 'panel-missions-rewards', module: 'missions', component: RewardsPanel, defaultWidth: 1 },
  { id: 'community-goals', titleKey: 'panel-missions-community-goals', module: 'missions',
    component: CommunityGoalsPanel, defaultWidth: 2, when: cgVisible },
  // Ingeniería
  { id: 'materials', titleKey: 'panel-engineering-materials', module: 'engineering', component: MaterialsPanel, defaultWidth: 2 },
  { id: 'engineers', titleKey: 'panel-engineering-engineers', module: 'engineering', component: EngineersPanel, defaultWidth: 1 },
  { id: 'engineer-guide', titleKey: 'panel-engineering-guide', module: 'engineering', component: EngineerGuidePanel, defaultWidth: 2, addToSaved: false },
  { id: 'eng-ship', titleKey: 'panel-engineering-ship', module: 'engineering',
    component: EngineeringShipPanel, defaultWidth: 2 },
  { id: 'eng-flow', titleKey: 'panel-engineering-flow', module: 'engineering',
    component: EngineerFlow, defaultWidth: 2, when: flowVisible },
  { id: 'shopping', titleKey: 'panel-engineering-shopping', module: 'engineering', component: ShoppingPanel, defaultWidth: 2 },
  { id: 'trades', titleKey: 'panel-engineering-trades', module: 'engineering', component: TradesPanel, defaultWidth: 1 },
  { id: 'sources', titleKey: 'panel-engineering-sources', module: 'engineering', component: SourcesPanel, defaultWidth: 1 },
  { id: 'eng-alerts', titleKey: 'panel-engineering-alerts', module: 'engineering',
    component: EngineeringAlerts, defaultWidth: 1 },
  // Flota, colonización, Powerplay, BGS, cuentas y crimen
  { id: 'carrier-overview', titleKey: 'panel-carrier-overview', module: 'carrier',
    component: CarrierOverviewPanel, defaultWidth: 2 },
  { id: 'carrier-jump', titleKey: 'panel-carrier-jump', module: 'carrier', component: CarrierJumpPanel, defaultWidth: 1 },
  { id: 'carrier-finances', titleKey: 'panel-carrier-finances', module: 'carrier',
    component: CarrierFinancesPanel, defaultWidth: 1 },
  { id: 'colonisation-progress', titleKey: 'panel-colonisation-depot', module: 'colonisation',
    component: ColonisationProgressPanel, defaultWidth: 2 },
  { id: 'colonisation-supply', titleKey: 'panel-colonisation-supply', module: 'colonisation',
    component: ColonisationSupplyPanel, defaultWidth: 1 },
  { id: 'powerplay-status', titleKey: 'panel-powerplay-status', module: 'powerplay',
    component: PowerplayPanel, defaultWidth: 2 },
  { id: 'bgs-system', titleKey: 'panel-bgs-system', module: 'bgs', component: BgsPanel, defaultWidth: 2 },
  { id: 'accounts-accounts', titleKey: 'panel-accounts', module: 'accounts', component: AccountsPanel, defaultWidth: 'full' },
  { id: 'crime-crime', titleKey: 'crime-title', module: 'crime', component: CrimePanel, defaultWidth: 2 },
];

export const cards: Record<string, CardDef> = Object.fromEntries(defs.map((d) => [d.id, d]));

/** Pestaña activa a clave de disposición ('layout:x' | 'module:x' | 'custom:y'); null para ajustes y cuentas. */
export function tabKeyFor(activeTab: string): string | null {
  if (activeTab === 'settings' || activeTab === 'accounts') return null;
  if (activeTab.startsWith('layout:') || activeTab.startsWith('custom:')) return activeTab;
  return `module:${activeTab}`;
}

/** Disposición de fábrica de una pestaña, en orden; [] para pestañas personalizadas o desconocidas. */
export function factoryLayout(tabKey: string): CardPlacement[] {
  if (tabKey.startsWith('layout:')) {
    const layout = layouts.find((l) => `layout:${l.id}` === tabKey);
    return layout?.slots.map((s) => ({ id: s.card, w: s.width, h: null })) ?? [];
  }
  if (tabKey.startsWith('module:')) {
    const mod = modules.find((m) => `module:${m.id}` === tabKey);
    return mod?.cards.map((id) => ({ id, w: cards[id].defaultWidth, h: null })) ?? [];
  }
  return [];
}

export function isFactoryCard(tabKey: string, cardId: string): boolean {
  return factoryLayout(tabKey).some((p) => p.id === cardId);
}

export interface CatalogEntry { card: CardDef; disabled: boolean }

/**
 * Entradas del catálogo «Añadir tarjeta». Temática: tarjetas de fábrica quitadas o ausentes, en orden de fábrica.
 * Personalizada: todas las tarjetas por módulo y título, deshabilitadas las ya presentes.
 */
export function catalogFor(tabKey: string, current: CardPlacement[], removed: string[]): CatalogEntry[] {
  const present = new Set(current.map((p) => p.id));
  if (tabKey.startsWith('custom:')) {
    return Object.values(cards)
      .sort((a, b) => a.module.localeCompare(b.module) || t(a.titleKey).localeCompare(t(b.titleKey)))
      .map((card) => ({ card, disabled: present.has(card.id) }));
  }
  const gone = new Set(removed);
  const seen = new Set<string>();
  const out: CatalogEntry[] = [];
  for (const p of factoryLayout(tabKey)) {
    if (seen.has(p.id) || !cards[p.id] || !(gone.has(p.id) || !present.has(p.id))) continue;
    seen.add(p.id);
    out.push({ card: cards[p.id], disabled: false });
  }
  return out;
}
