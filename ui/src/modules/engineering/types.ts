export type MaterialCategory = 'Raw' | 'Manufactured' | 'Encoded';
export interface MaterialRow { key: string; count: number; name_localised?: string | null; name_en?: string | null; known?: boolean }
export interface InventoryTopic { by_category: Partial<Record<MaterialCategory, Record<string, MaterialRow[]>>>; caps: Record<string, number>; at_max: string[]; snapshot: boolean }
export type LockerSection = 'items' | 'components' | 'consumables' | 'data';
export interface LockerTopic { revision: number; counts: Record<LockerSection, number> }
export interface LockerRow { key: string; name_localised: string | null; count: number }
export interface SessionTopic { totals: Record<string, { in: number; out: number }>; recent: { key: string; delta: number; origin: string; lost: number; at: string }[] }
export interface PageRequest { resource: 'locker' | 'goals' | 'catalogue' | 'engineers' | 'flow'; revision: number; section: string; commander: string; scope?: string }
export interface EngineeringPage<T> { revision: number; section: string; cursor: number; next_cursor: number | null; total: number; items: T[] }

export interface EngineerRow {
  id: number;
  name: string;
  on_foot: boolean;
  status: string;
  rank: number | null;
  rank_progress: number | null;
  system: string | null;
  distance_ly: number | null;
  verified: boolean;
}

export type EngineerNextStep =
  | 'Done'
  | { Requirement: { key: string } }
  | { Invite: { key: string } }
  | { Contribute: { item: string; kind: string; have: number; need: number } };

/** Resumen del topic: filas, capacidades y requisitos se leen por REST paginado. */
export interface EngineersTopic { revision: number; total: number; selected_id: number | null; next_step: EngineerNextStep | null }
export interface EngineerCapability { engineer_id: number; module_type: string; blueprint: string; grade: number }
export type EngineerRows = { engineers: EngineerRow; capabilities: EngineerCapability };

export interface CurrentEngineering { blueprint: string; level: number; quality: number; experimental: string | null }
export interface ShipModule { slot: string; item: string; module_type: string | null; current: CurrentEngineering | null; engineerable: boolean }
export interface EngineeringShipTopic { ship: string | null; ship_id: number | null; modules: ShipModule[] }
export interface EngineeringRequest { blueprint: string; experimental: string | null; target_grade: number; rolls: number[]; module_type: string }
export interface EngineeringTrade { give: number; give_key: string; get: number; get_key: string; trader: MaterialCategory; same_family: boolean }
export interface FlowLine { key: string; name_en: string | null; grade: number; need: number; have: number; state: string; missing: number; trade: EngineeringTrade | null }
export type PlanBlocker = 'missing_base_mapping' | 'base_required' | 'ambiguous_recipe' | 'incompatible_item' | 'unknown_blueprint' | 'unknown_experimental' | { missing_recipe: { grade: number } };
/** Saldo `null` = sin fuente de datos: nunca se asume cero. */
export interface MercCoins { need: number; have: number | null; missing: number | null }
export interface FlowEngineer { nearest_any: EngineerRow | null; nearest_unlocked: EngineerRow | null; next_step: EngineerNextStep | null }
export interface EngineeringFlowTopic {
  revision: number; status: string; slot: string | null; module_type: string | null; item: string | null; current: CurrentEngineering | null;
  request: EngineeringRequest | null; percent: number; materials_ready: boolean; merc_coins: MercCoins | null; blockers: PlanBlocker[];
  replaces_blueprint: boolean; verified: boolean; engineer: FlowEngineer | null;
  counts?: Record<string, number>;
}
export interface FlowBlueprintRow { key: string; available: boolean; reason: string | null; grades: number[] }
export interface FlowExperimentalRow { key: string }
export interface FlowGradeRow { grade: number; rolls: number; merc_coins: number }
export interface FlowCostRow { grade: number | null; kind: 'material' | 'merc_coins'; material?: string; count: number }
export type FlowRows = { blueprints: FlowBlueprintRow; experimentals: FlowExperimentalRow; grades: FlowGradeRow; costs: FlowCostRow; lines: FlowLine };

export interface GoalSummary { id: string; percent: number; completable: boolean; engineer: string | null; materials_ready: boolean; merc_coins: MercCoins; blockers: PlanBlocker[] }
export interface GoalsTopic { revision: number; status: string; total: number; goals: GoalSummary[] }
export interface GoalDetail extends GoalSummary { slot: string; item: string; request: EngineeringRequest; missing: Record<string, number>; trades: EngineeringTrade[]; verified: boolean }
export interface AggregateRow { key: string; count: number; name_en?: string | null }
export interface RemoteStation { station_name: string; system_name: string; distance_ly: number; max_pad: string | null }
export type SourceStatus = 'Loading' | 'Disabled' | 'NoData' | { Ok: RemoteStation };
export interface SourcesTopic { traders: Partial<Record<MaterialCategory, SourceStatus>>; brokers: Record<string, SourceStatus>; hints: Record<string, { kind: string; detail_key: string | null }[]> }
export interface EngineeringAlert { id: string; kind: string; key?: string; grade?: number; goal?: string; engineer?: string | null; engineer_id?: number }
